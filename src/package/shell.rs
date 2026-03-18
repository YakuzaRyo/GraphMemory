use serde::{Deserialize, Serialize};

/// Shell Layer - Lightweight wrapper
/// Entry point and wrapper scripts for easy access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shell {
    pub entry_point: String,
    pub wrapper_script: String,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            entry_point: String::new(),
            wrapper_script: String::new(),
        }
    }

    pub fn with_entry_point(entry_point: &str, wrapper_script: &str) -> Self {
        Self {
            entry_point: entry_point.to_string(),
            wrapper_script: wrapper_script.to_string(),
        }
    }

    pub fn set_entry_point(&mut self, entry_point: &str) {
        self.entry_point = entry_point.to_string();
    }

    pub fn set_wrapper_script(&mut self, script: &str) {
        self.wrapper_script = script.to_string();
    }

    pub fn execute(&self, args: &[&str]) -> String {
        format!("{} {}", self.entry_point, args.join(" "))
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}
