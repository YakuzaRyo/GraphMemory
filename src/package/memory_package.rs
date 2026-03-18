use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::package::{Pro, Ada, Shell};

/// MemoryPackage - Complete package structure
/// Contains Pro, Ada, and Shell layers
/// MemoryPackage is isomorphic to Engineering Package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPackage {
    /// Unique identifier for this package
    pub id: String,
    /// Pro Layer - Persistent public API (requirements/interface)
    pub pro: Pro,
    /// Ada Layer - Internal implementation (adaptation)
    pub ada: Ada,
    /// Shell Layer - Lightweight wrapper (exports)
    pub shell: Shell,
    /// Dependencies: package IDs this package depends on (DAG edges)
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Dependents: package IDs that depend on this package (reverse edges for quick lookup)
    #[serde(default, skip)]
    pub dependents: Vec<String>,
    /// Creation timestamp
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default)]
    pub updated_at: DateTime<Utc>,
    /// Access count
    #[serde(default)]
    pub access_count: u32,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl MemoryPackage {
    pub fn new(id: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            pro: Pro::default(),
            ada: Ada::default(),
            shell: Shell::default(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            created_at: now,
            updated_at: now,
            access_count: 0,
            tags: Vec::new(),
        }
    }

    pub fn with_layers(id: String, pro: Pro, ada: Ada, shell: Shell) -> Self {
        let now = Utc::now();
        Self {
            id,
            pro,
            ada,
            shell,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            created_at: now,
            updated_at: now,
            access_count: 0,
            tags: Vec::new(),
        }
    }

    /// Create from simple key-value (for backward compatibility)
    pub fn from_content(id: String, summary: String, content: String) -> Self {
        let pro = Pro::with_exports(vec![summary.clone()], &summary);
        let ada = Ada::with_implementation(&content, vec![]);
        let shell = Shell::with_entry_point(&format!("export {}", summary), "");
        Self::with_layers(id, pro, ada, shell)
    }

    pub fn add_dependency(&mut self, package_id: String) {
        if !self.dependencies.contains(&package_id) {
            self.dependencies.push(package_id);
        }
    }

    pub fn remove_dependency(&mut self, package_id: &str) {
        self.dependencies.retain(|id| id != package_id);
    }

    pub fn add_dependent(&mut self, package_id: String) {
        if !self.dependents.contains(&package_id) {
            self.dependents.push(package_id);
        }
    }

    pub fn remove_dependent(&mut self, package_id: &str) {
        self.dependents.retain(|id| id != package_id);
    }

    pub fn increment_access(&mut self) {
        self.access_count += 1;
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn get_all_exports(&self) -> Vec<&String> {
        self.pro.exports.iter().collect()
    }

    /// Get the primary summary from pro layer
    pub fn summary(&self) -> String {
        self.pro.summary()
    }

    /// Get the full content from ada layer
    pub fn content(&self) -> String {
        self.ada.implementation.clone()
    }

    /// Get export interface
    pub fn export_interface(&self) -> String {
        self.shell.entry_point.clone()
    }

    /// Update the package with new content
    pub fn update(&mut self, summary: String, content: String) {
        self.pro.set_summary(&summary);
        self.ada.set_implementation(&content);
        self.updated_at = Utc::now();
    }

    /// Check if this package depends on another
    pub fn depends_on(&self, package_id: &str) -> bool {
        self.dependencies.contains(&package_id.to_string())
    }

    /// Check if there's a cycle if we add this dependency (for DAG validation)
    pub fn would_create_cycle(&self, new_dep: &str) -> bool {
        // If new_dep depends on self.id, adding self -> new_dep would create a cycle
        new_dep == self.id
    }
}

impl Default for MemoryPackage {
    fn default() -> Self {
        Self::new(String::new())
    }
}
