//! L2 Local Disk Cache
//! Persistent cache using local filesystem

use crate::cache::{CacheLayer, CacheStats};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::RwLock;

/// L2 Local Disk Cache
/// Persistent cache layer using filesystem
pub struct L2DiskCache {
    cache_dir: PathBuf,
    stats: RwLock<CacheStats>,
    memory_index: RwLock<HashMap<String, u64>>,
}

impl L2DiskCache {
    pub fn new(cache_dir: PathBuf) -> std::io::Result<Self> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            stats: RwLock::new(CacheStats::new()),
            memory_index: RwLock::new(HashMap::new()),
        })
    }

    pub fn with_default_dir() -> std::io::Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("graph_memory_l2");
        Self::new(cache_dir)
    }

    fn key_to_filename(&self, key: &str) -> String {
        // Simple encoding: replace invalid chars
        key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    }

    fn get_file_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(self.key_to_filename(key))
    }

    pub fn exists(&self, key: &str) -> bool {
        self.get_file_path(key).exists()
    }

    pub fn clear(&self) -> std::io::Result<()> {
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        fs::remove_file(path)?;
                    }
                }
            }
        }
        self.memory_index.write().unwrap().clear();
        Ok(())
    }
}

impl CacheLayer for L2DiskCache {
    fn get(&self, key: &str) -> Option<String> {
        let file_path = self.get_file_path(key);
        if file_path.exists() {
            match fs::File::open(&file_path) {
                Ok(mut file) => {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        self.stats.write().unwrap().record_hit();
                        // Update access time in index
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        self.memory_index
                            .write()
                            .unwrap()
                            .insert(key.to_string(), now);
                        return Some(contents);
                    }
                }
                Err(_) => {}
            }
        }
        self.stats.write().unwrap().record_miss();
        None
    }

    fn set(&mut self, key: &str, value: &str) {
        let file_path = self.get_file_path(key);
        if let Ok(mut file) = fs::File::create(&file_path) {
            let _ = file.write_all(value.as_bytes());
            // Update index
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.memory_index
                .write()
                .unwrap()
                .insert(key.to_string(), now);
        }
    }

    fn remove(&mut self, key: &str) {
        let file_path = self.get_file_path(key);
        let _ = fs::remove_file(file_path);
        self.memory_index.write().unwrap().remove(key);
    }

    fn hit_rate(&self) -> f32 {
        self.stats.read().unwrap().hit_rate()
    }

    fn name(&self) -> &str {
        "L2_Disk"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_l2_cache_basic() -> std::io::Result<()> {
        let temp_dir = temp_dir().join("l2_cache_test");
        let mut cache = L2DiskCache::new(temp_dir.clone())?;
        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        cache.remove("key1");
        assert_eq!(cache.get("key1"), None);
        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
        Ok(())
    }
}
