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

    /// Get the primary summary (first export or doc)
    pub fn summary(&self) -> String {
        if !self.exports.is_empty() {
            self.exports[0].clone()
        } else if !self.doc.is_empty() {
            self.doc.split('\n').next().unwrap_or(&self.doc).to_string()
        } else {
            String::new()
        }
    }

    /// Set summary as the first export
    pub fn set_summary(&mut self, summary: &str) {
        if self.exports.is_empty() {
            self.exports.push(summary.to_string());
        } else {
            self.exports[0] = summary.to_string();
        }
    }
}

impl Default for Pro {
    fn default() -> Self {
        Self::new()
    }
}
