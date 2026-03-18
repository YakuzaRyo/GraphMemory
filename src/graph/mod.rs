use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub mod node;
pub mod edge;
pub mod latent;

pub use node::{MemoryNode, NodeId};
pub use edge::{MemoryEdge, RelationType};
pub use latent::LatentGraph;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
    #[error("Edge already exists from {0:?} to {1:?}")]
    EdgeAlreadyExists(NodeId, NodeId),
    #[error("Invalid edge: from={0:?}, to={1:?}")]
    InvalidEdge(NodeId, NodeId),
}

pub struct MemoryGraph {
    nodes: HashMap<NodeId, MemoryNode>,
    edges: HashMap<NodeId, HashMap<NodeId, MemoryEdge>>,
    next_id: u64,
}

impl MemoryGraph {
    pub fn new() -> Self {
        MemoryGraph {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_node(&mut self, node: MemoryNode) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, edge: MemoryEdge) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&from) {
            return Err(GraphError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(GraphError::NodeNotFound(to));
        }
        if from == to {
            return Err(GraphError::InvalidEdge(from, to));
        }

        self.edges
            .entry(from)
            .or_insert_with(HashMap::new)
            .insert(to, edge);

        Ok(())
    }

    pub fn get_node(&self, id: NodeId) -> Option<&MemoryNode> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut MemoryNode> {
        self.nodes.get_mut(&id)
    }

    pub fn get_neighbors(&self, id: NodeId) -> Vec<(NodeId, &MemoryEdge)> {
        self.edges
            .get(&id)
            .map(|neighbors| {
                neighbors
                    .iter()
                    .map(|(to_id, edge)| (*to_id, edge))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn traverse(&self, start_ids: &[NodeId], max_nodes: usize) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut queue: Vec<NodeId> = start_ids.to_vec();
        let mut result = Vec::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if self.nodes.contains_key(&current) {
                result.push(current);
            }

            if result.len() >= max_nodes {
                break;
            }

            for (neighbor, _) in self.get_neighbors(current) {
                if !visited.contains(&neighbor) {
                    queue.push(neighbor);
                }
            }
        }

        result
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|m| m.len()).sum()
    }

    pub fn next_node_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}
