//! L5 Compute Cache
//! Dynamic computation cache - function-based caching

use crate::cache::{CacheLayer, CacheStats};
use std::collections::HashMap;
use std::sync::RwLock;

/// L5 Compute Cache
/// Dynamic computation layer - computes values on demand
/// Uses function callbacks to generate values when not cached
pub struct L5ComputeCache {
    stats: RwLock<CacheStats>,
    cache: RwLock<HashMap<String, String>>,
    // Compute function storage (key -> computed value)
    compute_fn: RwLock<Option<Box<dyn Fn(&str) -> String + Send + Sync>>>,
}

impl L5ComputeCache {
    pub fn new() -> Self {
        Self {
            stats: RwLock::new(CacheStats::new()),
            cache: RwLock::new(HashMap::new()),
            compute_fn: RwLock::new(None),
        }
    }

    /// Set the compute function
    pub fn set_compute_fn<F>(&self, f: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        *self.compute_fn.write().unwrap() = Some(Box::new(f));
    }

    /// Manually compute a value and cache it
    pub fn compute(&self, key: &str) -> Option<String> {
        let compute_fn = self.compute_fn.read().unwrap();
        if let Some(ref f) = *compute_fn {
            let result = f(key);
            self.cache.write().unwrap().insert(key.to_string(), result.clone());
            Some(result)
        } else {
            None
        }
    }

    /// Check if compute function is set
    pub fn has_compute_fn(&self) -> bool {
        self.compute_fn.read().unwrap().is_some()
    }

    /// Clear all cached values
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get cached value without computing
    pub fn get_cached(&self, key: &str) -> Option<String> {
        self.cache.read().unwrap().get(key).cloned()
    }
}

impl Default for L5ComputeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheLayer for L5ComputeCache {
    fn get(&self, key: &str) -> Option<String> {
        // First try to get from cache
        if let Some(value) = self.cache.read().unwrap().get(key) {
            self.stats.write().unwrap().record_hit();
            return Some(value.clone());
        }

        // Try to compute if function is set
        if let Some(result) = self.compute(key) {
            self.stats.write().unwrap().record_hit();
            return Some(result);
        }

        self.stats.write().unwrap().record_miss();
        None
    }

    fn set(&mut self, key: &str, value: &str) {
        self.cache.write().unwrap().insert(key.to_string(), value.to_string());
    }

    fn remove(&mut self, key: &str) {
        self.cache.write().unwrap().remove(key);
    }

    fn hit_rate(&self) -> f32 {
        self.stats.read().unwrap().hit_rate()
    }

    fn name(&self) -> &str {
        "L5_Compute"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l5_cache_with_compute_fn() {
        let cache = L5ComputeCache::new();
        cache.set_compute_fn(|key| format!("computed_{}", key));

        // First call should compute
        let result = cache.get("test_key");
        assert_eq!(result, Some("computed_test_key".to_string()));

        // Second call should hit cache
        let result2 = cache.get("test_key");
        assert_eq!(result2, Some("computed_test_key".to_string()));
    }

    #[test]
    fn test_l5_cache_manual_set() {
        let mut cache = L5ComputeCache::new();
        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }
}
