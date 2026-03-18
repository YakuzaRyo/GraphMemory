use crate::graph::{MemoryGraph, NodeId};
use crate::context::SummarySequence;

pub struct ContextLoader {
    graph: MemoryGraph,
    max_tokens: usize,
}

pub struct ContextResult {
    pub content: Vec<String>,
    pub summary_sequence: SummarySequence,
}

impl ContextLoader {
    pub fn new(graph: MemoryGraph, max_tokens: usize) -> Self {
        ContextLoader { graph, max_tokens }
    }

    pub fn load_context(&self, _query: &str) -> ContextResult {
        let all_nodes: Vec<NodeId> = if self.graph.node_count() > 0 {
            self.graph.traverse(&self.graph.all_node_ids(), 10)
        } else {
            Vec::new()
        };
        let node_ids: Vec<NodeId> = all_nodes.into_iter().take(5).collect();

        let summary_seq = SummarySequence::new(node_ids);
        let content = summary_seq.expand(&self.graph);

        ContextResult {
            content,
            summary_sequence: summary_seq,
        }
    }

    pub fn vector_search(&self, _query: &str, top_k: usize) -> Vec<NodeId> {
        if self.graph.node_count() > 0 {
            self.graph.traverse(&self.graph.all_node_ids(), top_k)
        } else {
            Vec::new()
        }
    }

    pub fn graph_traverse(&self, seeds: &[NodeId], max_tokens: usize) -> Vec<NodeId> {
        self.graph.traverse(seeds, max_tokens)
    }

    pub fn get_graph(&self) -> &MemoryGraph {
        &self.graph
    }
}
