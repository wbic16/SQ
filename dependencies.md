# SQ dependencies — audited state (2026-11-24)

SQ's manifest carries **6 direct dependencies**. This note records a
minimization pass over the tree at v0.6.3 (`exo` HEAD 6d7aa6dc): what was
measured, what was tried, and what deliberately stays.

## Counts (Linux/x86_64 host tree, committed Cargo.lock)

| Measure | v0.6.0 | v0.6.3 |
|---|---|---|
| Direct dependencies | 6 | 6 (same set) |
| Unique package names | 30 | 30 |
| Unique name+version | 33 | 33 |
| Packages in Cargo.lock (all targets) | 55 | 55 |

Nothing entered the tree between 0.6.0 and 0.6.3; the shipped lockfiles
differ in exactly two lines (libphext, sq). No direct dependency has
entered the manifest since v0.5.5 (serde + serde_json). The manifest's
libphext floor is `"0.6.4"`; the committed lock pins **libphext 0.6.6**,
which is what a source build at this commit embeds.

## Every direct dependency is load-bearing

Verified by call-site grep over `src/*.rs` — no unused declarations:

- **libphext** — the phext engine (main.rs, sq.rs, tests.rs). Pulls only
  xxhash-rust. The leanest dep in the tree.
- **serde** (derive) — config/API types across api.rs, router.rs,
  mesh.rs, config.rs, triage.rs.
- **serde_json** — the REST API's parse/serialize path (91 refs).
- **shared_memory** — the daemon's IPC core: `ShmemConf` segments in
  main.rs's server/client paths.
- **raw_sync** — the other half of that IPC: `Event` waits on the
  shared segments.
- **percent-encoding** — one call site (`percent_decode`, main.rs),
  zero transitive weight.

## Feature trims attempted — all no-ops, none landed

Each was applied, re-resolved, and reverted when the tree came back
identical (the features in question gate no additional crates):

- `shared_memory` `default-features = false` — its default set is
  already empty; rand and nix are hard dependencies, not features.
- `raw_sync` `default-features = false` — same result.
- `serde_json` `default-features = false` + `alloc` — same package set;
  `std` gates no extra crates.
- `serde` `default-features = false` + `derive, alloc` — same set.

## Suspect, kept — the IPC pair's cluster

The tree's weight is transitive and old. **shared_memory 0.12.4 +
raw_sync 0.1.5 account for ~17 of the 32 dependency packages** and
every duplicated major:

- nix 0.23.2 (shared_memory's pin) beside nix 0.31.3 (raw_sync's)
- bitflags 1.3.2 beside 2.13.2; cfg-if 0.1.10 beside 1.0.5
- the full rand stack (rand, rand_chacha, rand_core, getrandom,
  ppv-lite86, zerocopy) — present only so shared_memory can mint
  random segment names; SQ itself never calls rand
- memoffset + autocfg (build) under shared_memory; cfg_aliases (build)
  under raw_sync

Both crates are at their lines' final versions (0.12.4 / 0.1.5), so no
version bump sheds any of this. The lock's Windows-only cluster
(windows 0.34 family, winapi, win-sys, rand 0.10 line) rides in via the
same pair's Windows support and is never compiled on Linux.

**The one real cut, flagged and not attempted:** retiring the pair for
a thin in-house shmem/event wrapper over libc (main.rs already does
the raw-pointer work itself) would remove ~17 packages at once. That
is a design change to the daemon's IPC core, not a dependency bump —
it is the maintainer's call.

## Acceptance at the audited tree

cargo test 78/78 · release build clean · REST write-roundtrip
byte-equal and chain detection on the canonical example
("1 chain(s), 8 universe(s), closed=true. Hosting universe 1."),
run on the tree this note describes.
