use serde::{Deserialize, Serialize};
use super::{Pro, Ada, Shell};

/// MemoryPackage - Complete package structure
/// Contains Pro, Ada, and Shell layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPackage {
    pub id: String,
    pub pro: Pro,
    pub ada: Ada,
    pub shell: Shell,
    pub dependencies: Vec<String>,
}

impl MemoryPackage {
    pub fn new(id: String) -> Self {
        Self {
            id,
            pro: Pro::default(),
            ada: Ada::default(),
            shell: Shell::default(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_layers(id: String, pro: Pro, ada: Ada, shell: Shell) -> Self {
        Self {
            id,
            pro,
            ada,
            shell,
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency(&mut self, package_id: String) {
        if !self.dependencies.contains(&package_id) {
            self.dependencies.push(package_id);
        }
    }

    pub fn get_all_exports(&self) -> Vec<&String> {
        self.pro.exports.iter().collect()
    }
}

impl Default for MemoryPackage {
    fn default() -> Self {
        Self::new(String::new())
    }
}
