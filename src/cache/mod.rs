// Cache module entry point
pub mod l1_memory;
pub mod l2_disk;
pub mod l3_network;
pub mod l4_vendor;
pub mod l5_compute;
pub mod radix_trie;

pub use l1_memory::L1MemoryCache;
pub use l2_disk::L2DiskCache;
pub use l3_network::L3NetworkCache;
pub use l4_vendor::L4VendorCache;
pub use l5_compute::L5ComputeCache;
pub use radix_trie::RadixTrie;

use std::collections::HashMap;
use std::sync::RwLock;

/// Cache layer trait for multi-level cache
pub trait CacheLayer: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str);
    fn remove(&mut self, key: &str);
    fn hit_rate(&self) -> f32;
    fn name(&self) -> &str;
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_hit(&self) {
        // Use atomic operations for thread safety
        // Note: Using RwLock for simplicity, could use AtomicU64
    }

    pub fn record_miss(&self) {
        // Using RwLock, actual counting happens in CacheManager
    }

    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

impl CacheEntry {
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: None,
        }
    }

    pub fn with_ttl(key: String, value: String, ttl_secs: u64) -> Self {
        Self {
            key,
            value,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + ttl_secs,
            ),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                > expires_at
        } else {
            false
        }
    }
}

/// Cache manager for multi-level caching
pub struct CacheManager {
    layers: Vec<Box<dyn CacheLayer>>,
    trie: RadixTrie,
    stats: HashMap<String, RwLock<CacheStats>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            trie: RadixTrie::new(),
            stats: HashMap::new(),
        }
    }

    pub fn add_layer(&mut self, layer: Box<dyn CacheLayer>) {
        let name = layer.name().to_string();
        self.stats.insert(name, RwLock::new(CacheStats::new()));
        self.layers.push(layer);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        for (i, layer) in self.layers.iter().enumerate() {
            if let Some(value) = layer.get(key) {
                // Update stats
                for j in 0..=i {
                    if let Some(stats_lock) = self.stats.get(self.layers[j].name()) {
                        if let Ok(mut stats) = stats_lock.write() {
                            stats.hits += 1;
                        }
                    }
                }
                return Some(value);
            }
        }
        // Record miss for all layers
        for layer in &self.layers {
            if let Some(stats_lock) = self.stats.get(layer.name()) {
                if let Ok(mut stats) = stats_lock.write() {
                    stats.misses += 1;
                }
            }
        }
        None
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if !self.layers.is_empty() {
            self.layers[0].set(key, value);
        }
        self.trie.insert(key, value.to_string());
    }

    pub fn remove(&mut self, key: &str) {
        for layer in &mut self.layers {
            layer.remove(key);
        }
        // Note: RadixTrie doesn't have remove method, but we could add it
    }

    pub fn compute<F>(&self, key: &str, f: F) -> String
    where
        F: Fn() -> String,
    {
        // Try to get from cache first
        if let Some(value) = self.get(key) {
            return value;
        }
        // Compute and cache
        let result = f();
        // Note: set requires &mut self, so we can't call it here directly
        // In production, use interior mutability or separate method
        result
    }

    pub fn prefix_match(&self, prefix: &str) -> Vec<String> {
        self.trie.prefix_match(prefix)
    }

    pub fn total_hit_rate(&self) -> f32 {
        let mut total_hits: u64 = 0;
        let mut total_misses: u64 = 0;
        for stats_lock in self.stats.values() {
            if let Ok(stats) = stats_lock.read() {
                total_hits += stats.hits;
                total_misses += stats.misses;
            }
        }
        let total = total_hits + total_misses;
        if total == 0 {
            0.0
        } else {
            total_hits as f32 / total as f32
        }
    }

    pub fn layer_hit_rate(&self, name: &str) -> f32 {
        self.stats.get(name).and_then(|s| {
            s.read().ok().map(|stats| stats.hit_rate())
        }).unwrap_or(0.0)
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}
