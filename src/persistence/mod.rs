//! Memory Persistence Module
//!
//! Provides JSON file-based persistence for MemoryGraph
//! - Save all packages to JSON file
//! - Load packages from JSON file on startup
//! - Automatic save on updates (optional)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::graph::{MemoryGraph, MemoryNode, NodeId, MemoryEdge, RelationType};
use crate::package::MemoryPackage;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Graph error: {0}")]
    Graph(String),
}

/// Serialization format for MemoryGraph
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub version: String,
    pub nodes: Vec<NodeSnapshot>,
    pub edges: Vec<EdgeSnapshot>,
    pub next_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub id: u64,
    pub package: MemoryPackage,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeSnapshot {
    pub from: u64,
    pub to: u64,
    pub relation: String,
    pub weight: f32,
}

/// MemoryPersistence - Handles saving/loading MemoryGraph to JSON
pub struct MemoryPersistence {
    file_path: String,
    auto_save: bool,
}

impl MemoryPersistence {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            auto_save: false,
        }
    }

    pub fn with_auto_save(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            auto_save: true,
        }
    }

    /// Save the entire graph to JSON file
    pub fn save(&self, graph: &MemoryGraph) -> Result<(), PersistenceError> {
        // Ensure directory exists
        if let Some(parent) = Path::new(&self.file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot = self.create_snapshot(graph);
        let json = serde_json::to_string_pretty(&snapshot)?;
        fs::write(&self.file_path, json)?;

        eprintln!("[PERSISTENCE] Saved {} nodes, {} edges to {}",
            graph.node_count(), graph.edge_count(), self.file_path);

        Ok(())
    }

    /// Load graph from JSON file
    pub fn load(&self) -> Result<MemoryGraph, PersistenceError> {
        if !Path::new(&self.file_path).exists() {
            eprintln!("[PERSISTENCE] No save file found at {}, starting fresh", self.file_path);
            return Ok(MemoryGraph::new());
        }

        let content = fs::read_to_string(&self.file_path)?;
        let snapshot: GraphSnapshot = serde_json::from_str(&content)?;

        let graph = self.reconstruct_graph(snapshot)?;

        eprintln!("[PERSISTENCE] Loaded {} nodes, {} edges from {}",
            graph.node_count(), graph.edge_count(), self.file_path);

        Ok(graph)
    }

    /// Check if a save file exists
    pub fn exists(&self) -> bool {
        Path::new(&self.file_path).exists()
    }

    /// Delete the save file
    pub fn delete(&self) -> Result<(), PersistenceError> {
        if Path::new(&self.file_path).exists() {
            fs::remove_file(&self.file_path)?;
        }
        Ok(())
    }

    fn create_snapshot(&self, graph: &MemoryGraph) -> GraphSnapshot {
        let nodes: Vec<NodeSnapshot> = graph.all_node_ids()
            .iter()
            .filter_map(|id| {
                graph.get_node(*id).map(|node| NodeSnapshot {
                    id: node.id.0,
                    package: node.package.clone(),
                    created_at: node.created_at,
                    updated_at: node.updated_at,
                    access_count: node.access_count,
                })
            })
            .collect();

        let edges: Vec<EdgeSnapshot> = graph.all_node_ids()
            .iter()
            .flat_map(|id| {
                graph.get_neighbors(*id)
                    .into_iter()
                    .map(|(to_id, edge)| EdgeSnapshot {
                        from: id.0,
                        to: to_id.0,
                        relation: format!("{:?}", edge.relation),
                        weight: edge.weight,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        GraphSnapshot {
            version: "1.0".to_string(),
            nodes,
            edges,
            next_id: 1, // Will be set during reconstruction
        }
    }

    fn reconstruct_graph(&self, snapshot: GraphSnapshot) -> Result<MemoryGraph, PersistenceError> {
        let mut graph = MemoryGraph::new();

        // Reconstruct nodes
        for node_snapshot in snapshot.nodes {
            let node = MemoryNode {
                id: NodeId(node_snapshot.id),
                package: node_snapshot.package,
                summary_embedding: Vec::new(),
                created_at: node_snapshot.created_at,
                updated_at: node_snapshot.updated_at,
                access_count: node_snapshot.access_count,
            };
            graph.add_node(node);
        }

        // Reconstruct edges
        for edge_snapshot in snapshot.edges {
            let relation = match edge_snapshot.relation.as_str() {
                "RefersTo" => RelationType::RefersTo,
                "Causes" => RelationType::Causes,
                "RelatedTo" => RelationType::RelatedTo,
                "PartOf" => RelationType::PartOf,
                "Contradicts" => RelationType::Contradicts,
                _ => RelationType::RelatedTo,
            };

            let edge = MemoryEdge::new(relation, edge_snapshot.weight);
            if let Err(e) = graph.add_edge(NodeId(edge_snapshot.from), NodeId(edge_snapshot.to), edge) {
                eprintln!("[PERSISTENCE] Warning: failed to restore edge: {}", e);
            }
        }

        Ok(graph)
    }

    /// Get the file path
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    /// Check if auto-save is enabled
    pub fn is_auto_save(&self) -> bool {
        self.auto_save
    }
}

impl Default for MemoryPersistence {
    fn default() -> Self {
        Self::new("memories.json")
    }
}

/// JSON export/import for external tools
pub fn export_to_json(graph: &MemoryGraph) -> Result<String, PersistenceError> {
    let snapshot = GraphSnapshot {
        version: "1.0".to_string(),
        nodes: graph.all_node_ids()
            .iter()
            .filter_map(|id| {
                graph.get_node(*id).map(|node| NodeSnapshot {
                    id: node.id.0,
                    package: node.package.clone(),
                    created_at: node.created_at,
                    updated_at: node.updated_at,
                    access_count: node.access_count,
                })
            })
            .collect(),
        edges: graph.all_node_ids()
            .iter()
            .flat_map(|id| {
                graph.get_neighbors(*id)
                    .into_iter()
                    .map(|(to_id, edge)| EdgeSnapshot {
                        from: id.0,
                        to: to_id.0,
                        relation: format!("{:?}", edge.relation),
                        weight: edge.weight,
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        next_id: 1,
    };

    Ok(serde_json::to_string_pretty(&snapshot)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_roundtrip() {
        let persistence = MemoryPersistence::new("/tmp/test_memories.json");
        let mut graph = MemoryGraph::new();

        // Add some test nodes
        let pkg1 = MemoryPackage::from_content(
            "pkg1".to_string(),
            "Test package 1".to_string(),
            "Content of test package 1".to_string()
        );
        let pkg2 = MemoryPackage::from_content(
            "pkg2".to_string(),
            "Test package 2".to_string(),
            "Content of test package 2".to_string()
        );

        graph.add_package(pkg1);
        graph.add_package(pkg2);

        // Save
        persistence.save(&graph).unwrap();

        // Load
        let loaded = persistence.load().unwrap();

        assert_eq!(loaded.node_count(), graph.node_count());

        // Cleanup
        persistence.delete().unwrap();
    }

    #[test]
    fn test_export_json() {
        let mut graph = MemoryGraph::new();
        let pkg = MemoryPackage::from_content(
            "test".to_string(),
            "Test".to_string(),
            "Test content".to_string()
        );
        graph.add_package(pkg);

        let json = export_to_json(&graph).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test"));
    }
}
