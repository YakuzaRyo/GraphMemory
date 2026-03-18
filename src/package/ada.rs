use serde::{Deserialize, Serialize};

/// Ada Layer - Internal implementation
/// Contains the actual implementation details and internal APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ada {
    pub implementation: String,
    pub internal_api: Vec<String>,
}

impl Ada {
    pub fn new() -> Self {
        Self {
            implementation: String::new(),
            internal_api: Vec::new(),
        }
    }

    pub fn with_implementation(implementation: &str, internal_api: Vec<String>) -> Self {
        Self {
            implementation: implementation.to_string(),
            internal_api,
        }
    }

    pub fn set_implementation(&mut self, implementation: &str) {
        self.implementation = implementation.to_string();
    }

    pub fn add_internal_api(&mut self, api: String) {
        if !self.internal_api.contains(&api) {
            self.internal_api.push(api);
        }
    }

    pub fn has_internal_api(&self, api: &str) -> bool {
        self.internal_api.iter().any(|a| a == api)
    }
}

impl Default for Ada {
    fn default() -> Self {
        Self::new()
    }
}
