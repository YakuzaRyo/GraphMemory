use serde::{Deserialize, Serialize};

/// Pro Layer - Persistent public API
/// Exports the public interface that consumers interact with
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pro {
    pub exports: Vec<String>,
    pub doc: String,
}

impl Pro {
    pub fn new() -> Self {
        Self {
            exports: Vec::new(),
            doc: String::new(),
        }
    }

    pub fn with_exports(exports: Vec<String>, doc: &str) -> Self {
        Self {
            exports,
            doc: doc.to_string(),
        }
    }

    pub fn add_export(&mut self, export: String) {
        if !self.exports.contains(&export) {
            self.exports.push(export);
        }
    }

    pub fn set_doc(&mut self, doc: &str) {
        self.doc = doc.to_string();
    }
}

impl Default for Pro {
    fn default() -> Self {
        Self::new()
    }
}
