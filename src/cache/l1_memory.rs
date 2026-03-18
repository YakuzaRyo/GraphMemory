//! L1 Local Memory Cache
//! Fast in-memory cache using HashMap

use crate::cache::{CacheEntry, CacheLayer, CacheStats};
use std::collections::HashMap;
use std::sync::RwLock;

/// L1 Local Memory Cache
/// Fastest cache layer, stores data in memory using HashMap
pub struct L1MemoryCache {
    store: RwLock<HashMap<String, CacheEntry>>,
    stats: RwLock<CacheStats>,
    max_size: usize,
}

impl L1MemoryCache {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::new()),
            max_size: 10000,
        }
    }

    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::new()),
            max_size,
        }
    }

    pub fn len(&self) -> usize {
        self.store.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.read().unwrap().is_empty()
    }

    pub fn clear(&self) {
        self.store.write().unwrap().clear();
    }
}

impl Default for L1MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheLayer for L1MemoryCache {
    fn get(&self, key: &str) -> Option<String> {
        let store = self.store.read().unwrap();
        if let Some(entry) = store.get(key) {
            if entry.is_expired() {
                drop(store);
                // Note: We can't remove here due to borrow checker
                // In production, use a separate cleanup mechanism
                return None;
            }
            self.stats.write().unwrap().record_hit();
            Some(entry.value.clone())
        } else {
            self.stats.write().unwrap().record_miss();
            None
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        let mut store = self.store.write().unwrap();
        // Simple eviction: if full, remove oldest entry
        if store.len() >= self.max_size && !store.contains_key(key) {
            if let Some(first_key) = store.keys().next().cloned() {
                store.remove(&first_key);
            }
        }
        store.insert(
            key.to_string(),
            CacheEntry::new(key.to_string(), value.to_string()),
        );
    }

    fn remove(&mut self, key: &str) {
        self.store.write().unwrap().remove(key);
    }

    fn hit_rate(&self) -> f32 {
        self.stats.read().unwrap().hit_rate()
    }

    fn name(&self) -> &str {
        "L1_Memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_cache_basic() {
        let mut cache = L1MemoryCache::new();
        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_l1_cache_remove() {
        let mut cache = L1MemoryCache::new();
        cache.set("key1", "value1");
        cache.remove("key1");
        assert_eq!(cache.get("key1"), None);
    }
}
