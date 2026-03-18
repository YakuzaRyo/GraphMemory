use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: NodeId,
    pub content: String,
    pub summary: String,
    pub summary_embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    pub fn new(id: u64) -> Self {
        NodeId(id)
    }
}

impl MemoryNode {
    pub fn new(id: NodeId, content: String, summary: String, summary_embedding: Vec<f32>) -> Self {
        MemoryNode {
            id,
            content,
            summary,
            summary_embedding,
            created_at: Utc::now(),
            access_count: 0,
        }
    }

    pub fn increment_access(&mut self) {
        self.access_count += 1;
    }
}
