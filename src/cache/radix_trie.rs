// Stub for Radix Trie
use std::collections::HashMap;

pub struct RadixTrie {
    root: TrieNode,
}

struct TrieNode {
    children: HashMap<char, TrieNode>,
    value: Option<String>,
    is_end: bool,
}

impl RadixTrie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: String) {
        let mut current = &mut self.root;
        for c in key.chars() {
            current = current.children.entry(c).or_insert_with(TrieNode::new);
        }
        current.is_end = true;
        current.value = Some(value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        let mut current = &self.root;
        for c in key.chars() {
            match current.children.get(&c) {
                Some(node) => current = node,
                None => return None,
            }
        }
        if current.is_end {
            current.value.as_ref()
        } else {
            None
        }
    }

    pub fn prefix_match(&self, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut current = &self.root;

        // Navigate to prefix
        for c in prefix.chars() {
            match current.children.get(&c) {
                Some(node) => current = node,
                None => return results,
            }
        }

        // Collect all values under this node
        self.collect_values(current, &mut results);
        results
    }

    fn collect_values(&self, node: &TrieNode, results: &mut Vec<String>) {
        if node.is_end {
            if let Some(ref value) = node.value {
                results.push(value.clone());
            }
        }
        for child in node.children.values() {
            self.collect_values(child, results);
        }
    }
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            value: None,
            is_end: false,
        }
    }
}

impl Default for RadixTrie {
    fn default() -> Self {
        Self::new()
    }
}
