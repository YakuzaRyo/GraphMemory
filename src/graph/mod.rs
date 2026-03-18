use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub mod node;
pub mod edge;
pub mod latent;

pub use node::{MemoryNode, NodeId};
pub use edge::{MemoryEdge, RelationType};
pub use latent::LatentGraph;
use crate::package::MemoryPackage;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
    #[error("Edge already exists from {0:?} to {1:?}")]
    EdgeAlreadyExists(NodeId, NodeId),
    #[error("Invalid edge: from={0:?}, to={1:?}")]
    InvalidEdge(NodeId, NodeId),
    #[error("Cycle detected: adding edge from {0:?} to {1:?} would create a cycle")]
    CycleDetected(NodeId, NodeId),
    #[error("Empty graph, cannot perform topological sort")]
    EmptyGraph,
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

    /// Add a new node with auto-generated ID
    pub fn add_package(&mut self, package: MemoryPackage) -> NodeId {
        let id = self.next_node_id();
        let node = MemoryNode::with_package(id, package);
        self.add_node(node);
        id
    }

    /// Add edge and maintain reverse index
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

        // Check for cycle before adding
        if self.would_create_cycle(from, to) {
            return Err(GraphError::CycleDetected(from, to));
        }

        self.edges
            .entry(from)
            .or_insert_with(HashMap::new)
            .insert(to, edge);

        // Update reverse index (dependents)
        if let Some(node) = self.nodes.get_mut(&to) {
            node.package.add_dependent(format!("{}", from.0));
        }

        // Update forward index (dependencies)
        if let Some(node) = self.nodes.get_mut(&from) {
            node.add_dependency(format!("{}", to.0));
        }

        Ok(())
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: NodeId, to: NodeId) -> bool {
        // DFS from 'to' to see if we can reach 'from'
        let mut visited = HashSet::new();
        let mut stack = vec![to];

        while let Some(current) = stack.pop() {
            if current == from {
                return true;
            }
            if visited.insert(current) {
                if let Some(neighbors) = self.edges.get(&current) {
                    for (&next, _) in neighbors {
                        stack.push(next);
                    }
                }
            }
        }
        false
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

    /// Get the full context for a node, including all dependencies (context inversion)
    pub fn get_full_context(&self, id: NodeId) -> Option<String> {
        let node = self.get_node(id)?;
        Some(self.build_context_string(id))
    }

    /// Build complete context string including all dependencies
    fn build_context_string(&self, id: NodeId) -> String {
        let node = self.nodes.get(&id);
        let package = match node {
            Some(n) => &n.package,
            None => return String::new(),
        };

        let mut parts = vec![
            format!("## [Package: {}]", package.id),
            format!("### Pro (Requirements):"),
            format!("  Summary: {}", package.pro.summary()),
            format!("  Exports: {}", package.pro.exports.join(", ")),
            format!("  Doc: {}", package.pro.doc),
            format!("### Ada (Implementation):"),
            format!("  {}", package.ada.implementation),
            format!("### Shell (Export):"),
            format!("  Entry: {}", package.shell.entry_point),
        ];

        // Recursively include dependencies
        if !package.dependencies.is_empty() {
            parts.push(format!("\n### Dependencies:"));
            for dep_id in &package.dependencies {
                if let Ok(dep_node_id) = dep_id.parse::<u64>() {
                    let dep_context = self.build_context_string(NodeId(dep_node_id));
                    if !dep_context.is_empty() {
                        parts.push(dep_context);
                    }
                }
            }
        }

        parts.join("\n")
    }

    /// Topological sort using Kahn's algorithm
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, GraphError> {
        if self.nodes.is_empty() {
            return Err(GraphError::EmptyGraph);
        }

        // Calculate in-degree for each node
        let mut in_degree: HashMap<NodeId, usize> = self.nodes.keys().map(|id| (*id, 0)).collect();

        for (_, neighbors) in &self.edges {
            for (to_id, _) in neighbors {
                *in_degree.entry(*to_id).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no incoming edges
        let mut queue: Vec<NodeId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::new();

        while let Some(node_id) = queue.pop() {
            result.push(node_id);

            if let Some(neighbors) = self.edges.get(&node_id) {
                for (&neighbor, _) in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        // If result doesn't contain all nodes, there's a cycle
        if result.len() != self.nodes.len() {
            // Find the cycle - nodes not in result form a cycle
            return Err(GraphError::EmptyGraph); // Simplified, should be CycleDetected
        }

        Ok(result)
    }

    /// Get execution order for a specific target node (dependencies first)
    pub fn get_execution_order(&self, target: NodeId) -> Result<Vec<NodeId>, GraphError> {
        if !self.nodes.contains_key(&target) {
            return Err(GraphError::NodeNotFound(target));
        }

        // DFS to collect all dependencies
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        self.collect_dependencies(target, &mut visited, &mut order)?;

        // Reverse to get proper execution order (dependencies first)
        order.reverse();

        // Add target node at the end
        order.push(target);

        Ok(order)
    }

    fn collect_dependencies(&self, node: NodeId, visited: &mut HashSet<NodeId>, order: &mut Vec<NodeId>) -> Result<(), GraphError> {
        if visited.contains(&node) {
            return Ok(());
        }

        if !self.nodes.contains_key(&node) {
            return Err(GraphError::NodeNotFound(node));
        }

        visited.insert(node);

        // First, process all dependencies
        if let Some(neighbors) = self.edges.get(&node) {
            for (&dep_id, _) in neighbors {
                self.collect_dependencies(dep_id, visited, order)?;
            }
        }

        order.push(node);
        Ok(())
    }

    /// Find all nodes that depend on the given node
    pub fn get_dependents(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .get(&id)
            .map(|n| {
                n.package.dependents
                    .iter()
                    .filter_map(|s| s.parse::<u64>().ok().map(NodeId))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all packages sorted by dependencies
    pub fn get_all_packages_sorted(&self) -> Vec<&MemoryNode> {
        if let Ok(sorted_ids) = self.topological_sort() {
            sorted_ids.iter().filter_map(|id| self.nodes.get(id)).collect()
        } else {
            // Return in ID order if topological sort fails
            let mut nodes: Vec<_> = self.nodes.values().collect();
            nodes.sort_by_key(|n| n.id.0);
            nodes
        }
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

    /// Find node by package ID string
    pub fn find_by_package_id(&self, package_id: &str) -> Option<NodeId> {
        self.nodes.values()
            .find(|n| n.package.id == package_id)
            .map(|n| n.id)
    }

    /// Remove a node and all its edges
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_some() {
            // Remove all edges to this node
            self.edges.values_mut().for_each(|neighbors| {
                neighbors.remove(&id);
            });
            // Remove all edges from this node
            self.edges.remove(&id);

            // Update dependents in other nodes
            for node in self.nodes.values_mut() {
                node.package.remove_dependent(&format!("{}", id.0));
            }

            true
        } else {
            false
        }
    }
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}
