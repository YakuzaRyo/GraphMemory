use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    RefersTo,
    Causes,
    RelatedTo,
    PartOf,
    Contradicts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub relation: RelationType,
    pub weight: f32,
}

impl MemoryEdge {
    pub fn new(relation: RelationType, weight: f32) -> Self {
        let clamped_weight = weight.clamp(0.0, 1.0);
        MemoryEdge {
            relation,
            weight: clamped_weight,
        }
    }
}
