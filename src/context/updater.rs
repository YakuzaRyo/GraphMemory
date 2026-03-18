use crate::graph::{MemoryGraph, MemoryNode, NodeId, MemoryEdge, RelationType};

const RELEVANCE_THRESHOLD: f32 = 0.15;

pub struct MemoryUpdater<'a> {
    graph: &'a mut MemoryGraph,
}

impl<'a> MemoryUpdater<'a> {
    pub fn new(graph: &'a mut MemoryGraph) -> Self {
        MemoryUpdater { graph }
    }

    pub fn update_from_output(
        &mut self,
        output: &str,
        summary_seq: &[NodeId],
    ) -> Vec<NodeId> {
        let new_memories = self.extract_memories(output);
        let mut new_ids = Vec::new();

        for memory in new_memories {
            for prev_id in summary_seq {
                let relevance = self.calculate_relevance(&memory, prev_id);
                if relevance > RELEVANCE_THRESHOLD {
                    let edge = MemoryEdge::new(RelationType::RelatedTo, relevance);
                    let new_id = self.graph.next_node_id();
                    let node = MemoryNode::new(new_id, memory.content(), memory.summary(), vec![]);
                    self.graph.add_node(node);
                    let _ = self.graph.add_edge(new_id, *prev_id, edge);
                    new_ids.push(new_id);
                    break;
                }
            }
        }

        new_ids
    }

    fn extract_memories(&self, text: &str) -> Vec<MemoryNode> {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let summary = line.chars().take(50).collect::<String>();
                MemoryNode::new(
                    NodeId(0),
                    line.to_string(),
                    summary,
                    vec![],
                )
            })
            .collect()
    }

    fn calculate_relevance(&self, memory: &MemoryNode, prev_id: &NodeId) -> f32 {
        if let Some(prev) = self.graph.get_node(*prev_id) {
            let common: usize = memory.summary()
                .split_whitespace()
                .filter(|w| prev.summary().contains(w))
                .count();
            // Base relevance + word match bonus
            0.1 + (common as f32) * 0.2
        } else {
            0.0
        }
    }
}
