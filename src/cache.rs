//------------------------------------------------------------------------------------------------------------
// file: cache.rs
// purpose: SHA256-keyed prompt cache backed by phext scrolls
//
// v0.6.0 - Prompt response caching for API proxy mode
//------------------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::time::{Instant, Duration};

/// A cached prompt-response pair with TTL tracking
struct CacheEntry {
    response: String,
    created: Instant,
}

/// SHA256-keyed prompt cache with LRU eviction and TTL expiry
pub struct PromptCache {
    entries: HashMap<String, CacheEntry>,
    order: Vec<String>,      // insertion order for LRU
    max_entries: usize,
    ttl: Duration,
    hits: u64,
    misses: u64,
}

impl PromptCache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        PromptCache {
            entries: HashMap::new(),
            order: Vec::new(),
            max_entries,
            ttl: Duration::from_secs(ttl_secs),
            hits: 0,
            misses: 0,
        }
    }

    /// Normalize a prompt for cache key generation: lowercase, collapse whitespace, trim
    fn normalize(prompt: &str) -> String {
        prompt.trim().to_lowercase()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// SHA256 hash of normalized prompt
    fn hash_key(prompt: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let normalized = Self::normalize(prompt);
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Look up a cached response
    pub fn get(&mut self, prompt: &str) -> Option<String> {
        let key = Self::hash_key(prompt);

        // Check expiry first
        let expired = self.entries.get(&key)
            .map(|e| e.created.elapsed() >= self.ttl)
            .unwrap_or(false);

        if expired {
            self.entries.remove(&key);
            self.order.retain(|k| k != &key);
            self.misses += 1;
            return None;
        }

        if let Some(entry) = self.entries.get(&key) {
            self.hits += 1;
            return Some(entry.response.clone());
        }

        self.misses += 1;
        None
    }

    /// Store a prompt-response pair
    pub fn set(&mut self, prompt: &str, response: &str) {
        let key = Self::hash_key(prompt);

        // Evict LRU if at capacity
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_entries {
            if let Some(oldest_key) = self.order.first().cloned() {
                self.entries.remove(&oldest_key);
                self.order.remove(0);
            }
        }

        // Remove from order if updating existing
        self.order.retain(|k| k != &key);
        self.order.push(key.clone());

        self.entries.insert(key, CacheEntry {
            response: response.to_string(),
            created: Instant::now(),
        });
    }

    /// Check static patterns (greetings, status, ping)
    pub fn check_static(prompt: &str) -> Option<&'static str> {
        let lower = prompt.trim().to_lowercase();
        match lower.as_str() {
            "hi" | "hello" | "hey" | "hi!" | "hello!" => Some("Hello! How can I help?"),
            "ping" => Some("pong"),
            "status" => Some("SQ is running."),
            _ => None,
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.entries.len())
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn test_miss() {
        let mut cache = PromptCache::new(10, 3600);
        assert!(cache.get("something new").is_none());
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn test_set_and_get() {
        let mut cache = PromptCache::new(10, 3600);
        cache.set("hello world", "response");
        assert_eq!(cache.get("hello world"), Some("response".to_string()));
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_normalization() {
        let mut cache = PromptCache::new(10, 3600);
        cache.set("  Hello   World  ", "response");
        assert_eq!(cache.get("hello world"), Some("response".to_string()));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = PromptCache::new(2, 3600);
        cache.set("a", "1");
        cache.set("b", "2");
        cache.set("c", "3"); // should evict "a"
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b"), Some("2".to_string()));
        assert_eq!(cache.get("c"), Some("3".to_string()));
    }

    #[test]
    fn test_static_patterns() {
        assert_eq!(PromptCache::check_static("hello"), Some("Hello! How can I help?"));
        assert_eq!(PromptCache::check_static("ping"), Some("pong"));
        assert!(PromptCache::check_static("analyze this codebase").is_none());
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = PromptCache::new(10, 3600);
        cache.set("x", "y");
        cache.get("x"); // hit
        cache.get("z"); // miss
        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }
}
