use crate::graph::{NodeId, MemoryGraph};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct SummarySequence {
    pub node_ids: Vec<NodeId>,
    pub total_tokens: usize,
    pub last_updated: DateTime<Utc>,
}

impl SummarySequence {
    pub fn new(node_ids: Vec<NodeId>) -> Self {
        let total_tokens = 0;
        let last_updated = Utc::now();
        SummarySequence {
            node_ids,
            total_tokens,
            last_updated,
        }
    }

    pub fn expand(&self, graph: &MemoryGraph) -> Vec<String> {
        self.node_ids
            .iter()
            .filter_map(|id| graph.get_node(*id).map(|n| n.content.clone()))
            .collect()
    }

    pub fn estimate_tokens(&self, graph: &MemoryGraph) -> usize {
        self.expand(graph).iter().map(|s| s.len() / 4).sum()
    }

    pub fn update_tokens(&mut self, graph: &MemoryGraph) {
        self.total_tokens = self.estimate_tokens(graph);
        self.last_updated = Utc::now();
    }
}
