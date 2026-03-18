use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::package::MemoryPackage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    pub fn new(id: u64) -> Self {
        NodeId(id)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: NodeId,
    /// The memory package (pro/ada/shell)
    pub package: MemoryPackage,
    /// Cached semantic embedding for the summary
    #[serde(default, skip_deserializing)]
    pub summary_embedding: Vec<f32>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
}

impl MemoryNode {
    /// Create a new node with a MemoryPackage
    pub fn with_package(id: NodeId, package: MemoryPackage) -> Self {
        let now = Utc::now();
        MemoryNode {
            id,
            package,
            summary_embedding: Vec::new(),
            created_at: now,
            updated_at: now,
            access_count: 0,
        }
    }

    /// Create a new node with simple content (backward compatibility)
    pub fn new(id: NodeId, content: String, summary: String, _summary_embedding: Vec<f32>) -> Self {
        let package = MemoryPackage::from_content(
            format!("{}", id.0),
            summary,
            content
        );
        Self::with_package(id, package)
    }

    pub fn increment_access(&mut self) {
        self.access_count += 1;
        self.package.increment_access();
    }

    /// Get the summary from the package's pro layer
    pub fn summary(&self) -> String {
        self.package.summary()
    }

    /// Get the content from the package's ada layer
    pub fn content(&self) -> String {
        self.package.content()
    }

    /// Update the node's content and summary
    pub fn update(&mut self, summary: String, content: String) {
        self.package.update(summary, content);
        self.updated_at = Utc::now();
    }

    /// Add a dependency to another node
    pub fn add_dependency(&mut self, target_id: String) {
        self.package.add_dependency(target_id);
        self.updated_at = Utc::now();
    }
}
