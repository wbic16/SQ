// -----------------------------------------------------------------------------------------------------------
// ipc.rs — phext-native shared-memory IPC primitives
//
// Replaces the raw_sync and shared_memory crates (ashfall audit FAIL verdicts; Will's ruling
// 2026-11-27: "remove raw_sync from SQ - re-implement the primitives" / "shared_memory -
// re-implement the primitives") with exactly the two primitives SQ's daemon mode consumes:
//
//   1. SharedSegment — a file-backed shared mapping with create-or-open semantics and an
//      owner flag, mirroring the Shmem surface SQ used (create / open-on-LinkExists /
//      as_ptr / is_owner). The mapping is the file itself (.sq/link, .sq/work), mapped with
//      memmap2 (ashfall PASS verdict). The owner removes the file on drop.
//
//   2. Event — a cross-process binary event living at offset 0 of a segment. raw_sync's Unix
//      event is a process-shared pthread mutex+cond pair guarding a boolean signal flag, and
//      SQ always creates it with auto_reset = true, so a successful wait consumes the signal.
//      This implementation keeps the same observable semantics with a single futex word:
//      0 = clear, 1 = signaled; wait consumes 1 -> 0; set(Signaled) stores 1 and wakes one
//      waiter. Linux only (futex); other platforms get a clean Unsupported error from wait.
// -----------------------------------------------------------------------------------------------------------

use std::error::Error as StdError;
use std::ffi::c_void;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

// -----------------------------------------------------------------------------------------------------------
// error type for both primitives
// -----------------------------------------------------------------------------------------------------------
// The full variant set mirrors the raw_sync/shared_memory error surface SQ's call sites
// distinguish; SQ exercises a subset of it.
#[allow(dead_code)]
#[derive(Debug)]
pub enum IpcError {
    /// The segment file already exists (mirrors ShmemError::LinkExists).
    LinkExists,
    Io(io::Error),
    Unsupported(&'static str),
    Corrupted(&'static str),
    Timeout,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::LinkExists => write!(f, "shared segment link already exists"),
            IpcError::Io(e) => write!(f, "shared segment io error: {}", e),
            IpcError::Unsupported(what) => write!(f, "unsupported: {}", what),
            IpcError::Corrupted(what) => write!(f, "shared segment corrupted: {}", what),
            IpcError::Timeout => write!(f, "timed out waiting for event signal"),
        }
    }
}

impl StdError for IpcError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            IpcError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for IpcError {
    fn from(e: io::Error) -> Self {
        IpcError::Io(e)
    }
}

// -----------------------------------------------------------------------------------------------------------
// SharedSegment — file-backed shared memory mapping (replaces shared_memory::Shmem)
// -----------------------------------------------------------------------------------------------------------
pub struct SharedSegment {
    mapping: memmap2::MmapMut,
    path: PathBuf,
    owner: bool,
}

impl SharedSegment {
    /// Creates a new segment file of `size` bytes and maps it. Fails with
    /// IpcError::LinkExists if the file is already there (the caller then opens it).
    pub fn create(path: &str, size: usize) -> Result<SharedSegment, IpcError> {
        let file: File = match OpenOptions::new().read(true).write(true).create_new(true).open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Err(IpcError::LinkExists),
            Err(e) => return Err(IpcError::Io(e)),
        };
        file.set_len(size as u64)?;
        // Safety: the file was just created and sized by us; SQ treats the segment as a
        // shared scratch space whose layout is defined by the daemon protocol itself.
        let mapping = unsafe { memmap2::MmapMut::map_mut(&file)? };
        Ok(SharedSegment { mapping, path: PathBuf::from(path), owner: true })
    }

    /// Opens an existing segment file created by SharedSegment::create and maps it at
    /// the file's length.
    pub fn open(path: &str) -> Result<SharedSegment, IpcError> {
        let file: File = OpenOptions::new().read(true).write(true).open(path)?;
        // The creator may not have sized the file yet; wait briefly for a non-zero length
        // rather than failing on a zero-length mapping.
        let mut waited_ms = 0;
        loop {
            if file.metadata()?.len() > 0 {
                break;
            }
            if waited_ms >= 1000 {
                return Err(IpcError::Corrupted("shared segment has zero length"));
            }
            std::thread::sleep(Duration::from_millis(5));
            waited_ms += 5;
        }
        // Safety: the segment file was sized by its creator; see create().
        let mapping = unsafe { memmap2::MmapMut::map_mut(&file)? };
        Ok(SharedSegment { mapping, path: PathBuf::from(path), owner: false })
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.mapping.as_ptr() as *mut u8
    }

    pub fn is_owner(&self) -> bool {
        self.owner
    }
}

impl Drop for SharedSegment {
    fn drop(&mut self) {
        // The owner deletes the segment, mirroring shared_memory's owner-drop behavior.
        if self.owner {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// -----------------------------------------------------------------------------------------------------------
// Event — cross-process binary event in shared memory (replaces raw_sync::events::Event)
// -----------------------------------------------------------------------------------------------------------
// Clear and Timeout::Val are part of the raw_sync-parity surface; SQ's call sites only
// use Signaled and Timeout::Infinite.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventState {
    Clear,
    Signaled,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum Timeout {
    Infinite,
    Val(Duration),
}

/// Bytes of shared memory an Event occupies: one futex word.
pub const EVENT_USED_BYTES: usize = 4;

pub struct Event {
    word: *mut u32,
}

// The event state lives in shared memory and the handle is just a pointer to it; SQ uses
// each handle from a single thread at a time, matching how raw_sync handles were used.
unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
    /// Initializes a new event at `mem` in the clear state and returns it with the number
    /// of shared-memory bytes it occupies. SQ passes auto_reset = true: this event is
    /// auto-reset by design (a successful wait consumes the signal), so the flag is
    /// accepted for call-site parity and the behavior is fixed.
    ///
    /// Safety: `mem` must point to at least EVENT_USED_BYTES of writable shared memory,
    /// 4-byte aligned, that outlives the returned handle.
    pub unsafe fn new(mem: *mut u8, _auto_reset: bool) -> Result<(Event, usize), IpcError> {
        let evt = Event { word: Self::checked_word(mem)? };
        evt.atomic().store(0, Ordering::SeqCst);
        Ok((evt, EVENT_USED_BYTES))
    }

    /// Attaches to an event previously initialized by Event::new at `mem`.
    ///
    /// Safety: same requirements as Event::new, and the event must already be initialized.
    pub unsafe fn from_existing(mem: *mut u8) -> Result<(Event, usize), IpcError> {
        let evt = Event { word: Self::checked_word(mem)? };
        if evt.atomic().load(Ordering::SeqCst) > 1 {
            return Err(IpcError::Corrupted("existing event state is not 0 or 1"));
        }
        Ok((evt, EVENT_USED_BYTES))
    }

    fn checked_word(mem: *mut u8) -> Result<*mut u32, IpcError> {
        if (mem as usize) % std::mem::align_of::<u32>() != 0 {
            return Err(IpcError::Corrupted("event address is not 4-byte aligned"));
        }
        Ok(mem as *mut u32)
    }

    fn atomic(&self) -> &AtomicU32 {
        // Safety: `word` came from checked_word over shared memory that outlives the
        // handle (the owning SharedSegment is held by the same server/client loop).
        unsafe { AtomicU32::from_ptr(self.word) }
    }

    /// Sets the event state. Signaled wakes one waiter (auto-reset semantics, matching
    /// raw_sync's pthread_cond_signal path); Clear resets the flag without waking anyone.
    pub fn set(&self, state: EventState) -> Result<(), IpcError> {
        match state {
            EventState::Clear => {
                self.atomic().store(0, Ordering::SeqCst);
            }
            EventState::Signaled => {
                let prev = self.atomic().swap(1, Ordering::SeqCst);
                if prev == 0 {
                    #[cfg(target_os = "linux")]
                    futex_wake(self.word, 1);
                }
            }
        }
        Ok(())
    }

    /// Blocks until the event is signaled, consuming the signal (auto-reset), or until
    /// the timeout elapses.
    pub fn wait(&self, timeout: Timeout) -> Result<(), IpcError> {
        #[cfg(target_os = "linux")]
        {
            self.wait_futex(timeout)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = timeout;
            Err(IpcError::Unsupported("daemon IPC events require Linux (futex)"))
        }
    }

    #[cfg(target_os = "linux")]
    fn wait_futex(&self, timeout: Timeout) -> Result<(), IpcError> {
        let deadline = match timeout {
            Timeout::Infinite => None,
            Timeout::Val(d) => Some(Instant::now() + d),
        };
        loop {
            // Consume the signal if it is present (auto-reset).
            if self
                .atomic()
                .compare_exchange(1, 0, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
            // The word is 0: sleep while it stays 0. futex_wait returns immediately with
            // EAGAIN if the word changed under us, and EINTR on signals; both just loop.
            let remaining = match deadline {
                None => None,
                Some(dl) => {
                    let now = Instant::now();
                    if now >= dl {
                        return Err(IpcError::Timeout);
                    }
                    Some(dl - now)
                }
            };
            futex_wait(self.word, 0, remaining);
        }
    }
}

// -----------------------------------------------------------------------------------------------------------
// futex syscalls (Linux)
// -----------------------------------------------------------------------------------------------------------
#[cfg(target_os = "linux")]
fn futex_wait(word: *mut u32, expected: u32, timeout: Option<Duration>) {
    // For FUTEX_WAIT the timeout is relative.
    let ts = timeout.map(|d| libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    });
    let ts_ptr = match &ts {
        Some(t) => t as *const libc::timespec,
        None => std::ptr::null(),
    };
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            word as *mut c_void,
            libc::FUTEX_WAIT,
            expected as libc::c_int,
            ts_ptr,
            std::ptr::null_mut::<c_void>(),
            0 as libc::c_int,
        );
    }
}

#[cfg(target_os = "linux")]
fn futex_wake(word: *mut u32, count: i32) {
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            word as *mut c_void,
            libc::FUTEX_WAKE,
            count as libc::c_int,
            std::ptr::null::<libc::timespec>(),
            std::ptr::null_mut::<c_void>(),
            0 as libc::c_int,
        );
    }
}
