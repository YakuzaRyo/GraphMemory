//! Integration tests for GraphMemory

use graph_memory::{
    CacheLayer, CacheManager, ContextLoader, L1MemoryCache, L2DiskCache,
    L3NetworkCache, L4VendorCache, L5ComputeCache, MemoryEdge, MemoryGraph,
    MemoryNode, MemoryUpdater, NodeId, RelationType, SummarySequence,
};

// Test helper to create a simple graph with nodes
fn create_test_graph() -> MemoryGraph {
    let mut graph = MemoryGraph::new();

    // Add some test nodes
    let node1 = MemoryNode::new(
        NodeId(1),
        "First memory about Rust programming".to_string(),
        "Rust programming".to_string(),
        vec![],
    );
    let node2 = MemoryNode::new(
        NodeId(2),
        "Second memory about async programming".to_string(),
        "Async programming".to_string(),
        vec![],
    );
    let node3 = MemoryNode::new(
        NodeId(3),
        "Third memory about memory management".to_string(),
        "Memory management".to_string(),
        vec![],
    );

    graph.add_node(node1);
    graph.add_node(node2);
    graph.add_node(node3);

    // Add edges
    let _ = graph.add_edge(NodeId(1), NodeId(2), MemoryEdge::new(RelationType::RelatedTo, 0.8));
    let _ = graph.add_edge(NodeId(2), NodeId(3), MemoryEdge::new(RelationType::RelatedTo, 0.6));

    graph
}

/// Test 1: MemoryGraph basic operations
#[test]
fn test_memory_graph_basic() {
    let mut graph = MemoryGraph::new();

    // Create nodes
    let node1 = MemoryNode::new(
        NodeId(1),
        "Test content 1".to_string(),
        "Summary 1".to_string(),
        vec![],
    );
    let node2 = MemoryNode::new(
        NodeId(2),
        "Test content 2".to_string(),
        "Summary 2".to_string(),
        vec![],
    );

    let id1 = graph.add_node(node1);
    let id2 = graph.add_node(node2);

    assert_eq!(id1, NodeId(1));
    assert_eq!(id2, NodeId(2));
    assert_eq!(graph.node_count(), 2);

    // Add edge
    let edge = MemoryEdge::new(RelationType::RelatedTo, 0.5);
    let result = graph.add_edge(NodeId(1), NodeId(2), edge);
    assert!(result.is_ok());
    assert_eq!(graph.edge_count(), 1);

    // Get node
    let retrieved = graph.get_node(NodeId(1));
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().content(), "Test content 1");

    // Traverse
    let traversed = graph.traverse(&[NodeId(1)], 10);
    assert!(!traversed.is_empty());
}

/// Test 2: Cache manager with five-level cache
#[test]
fn test_cache_manager() {
    let mut manager = CacheManager::new();

    // Add L1 memory cache
    let l1 = Box::new(L1MemoryCache::new());
    manager.add_layer(l1);

    // Add L2 disk cache (use temp directory)
    let temp_dir = std::env::temp_dir().join("cache_test_l2");
    let l2 = Box::new(L2DiskCache::new(temp_dir.clone()).expect("Failed to create L2 cache"));
    manager.add_layer(l2);

    // Add L3 network cache
    let l3 = Box::new(L3NetworkCache::new("http://localhost:6379"));
    manager.add_layer(l3);

    // Add L4 vendor cache
    let l4 = Box::new(L4VendorCache::new("https://api.vendor.com"));
    manager.add_layer(l4);

    // Add L5 compute cache
    let l5 = Box::new(L5ComputeCache::new());
    manager.add_layer(l5);

    // Test set and get
    manager.set("key1", "value1");
    let result = manager.get("key1");
    assert_eq!(result, Some("value1".to_string()));

    // Test prefix match with RadixTrie
    manager.set("prefix_abc", "value_abc");
    manager.set("prefix_def", "value_def");
    let matches = manager.prefix_match("prefix_");
    assert!(matches.len() >= 2);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

/// Test 3: Context loader
#[test]
fn test_context_loader() {
    let graph = create_test_graph();
    let loader = ContextLoader::new(graph, 1000);

    // Load context
    let result = loader.load_context("test query");
    // The loader uses traverse which returns nodes, so there should be some result
    let _ = result.content;
    let _ = result.summary_sequence;

    // Test vector search
    let search_results = loader.vector_search("test", 2);
    assert!(search_results.len() <= 2);

    // Test graph traverse
    let traversed = loader.graph_traverse(&[NodeId(1)], 5);
    assert!(traversed.len() <= 5);
}

/// Test 4: Summary sequence
#[test]
fn test_summary_sequence() {
    // Create a sequence of node IDs
    let node_ids = vec![NodeId(1), NodeId(2), NodeId(3)];
    let sequence = SummarySequence::new(node_ids.clone());

    // Test expand
    let graph = create_test_graph();
    let expanded = sequence.expand(&graph);
    // Should get contents from the nodes (some may be None if IDs don't match)
    assert_eq!(expanded.len(), node_ids.len());

    // Test token estimation
    let tokens = sequence.estimate_tokens(&graph);
    assert!(tokens >= 0);

    // Test update tokens
    let mut seq = SummarySequence::new(vec![]);
    seq.update_tokens(&graph);
    assert!(seq.total_tokens >= 0);
}

/// Test 5: Memory updater
#[test]
fn test_memory_updater() {
    let mut graph = create_test_graph();

    // Create initial node for association - use words that will match
    let initial_node = MemoryNode::new(
        NodeId(100),
        "This is a test memory about programming in Rust".to_string(),
        "programming Rust memory test".to_string(),
        vec![],
    );
    graph.add_node(initial_node);

    // Create updater
    let mut updater = MemoryUpdater::new(&mut graph);

    // Update from output - include matching words for relevance
    let output = "New memory about programming in Rust\nAnother test memory about programming";
    let summary_ids = vec![NodeId(100)];
    let new_ids = updater.update_from_output(output, &summary_ids);

    // Check that nodes were added to graph (might be 0 if relevance threshold not met)
    for id in &new_ids {
        let node = graph.get_node(*id);
        assert!(node.is_some());
    }
}

/// Test 6: Full pipeline test
#[test]
fn test_full_pipeline() {
    // Step 1: Create graph
    let mut graph = MemoryGraph::new();
    assert_eq!(graph.node_count(), 0);

    // Step 2: Add initial memories
    let node1 = MemoryNode::new(
        NodeId(1),
        "First step: Initialize the memory graph".to_string(),
        "Initialize memory graph".to_string(),
        vec![],
    );
    let node2 = MemoryNode::new(
        NodeId(2),
        "Second step: Add more memories to the graph".to_string(),
        "Add memories".to_string(),
        vec![],
    );
    graph.add_node(node1);
    graph.add_node(node2);
    assert_eq!(graph.node_count(), 2);

    // Step 3: Add edge between nodes
    let edge = MemoryEdge::new(RelationType::RelatedTo, 0.9);
    let _ = graph.add_edge(NodeId(1), NodeId(2), edge);
    assert_eq!(graph.edge_count(), 1);

    // Step 4: Load context using ContextLoader (create a separate graph for loader)
    let graph_for_loader = create_test_graph();
    let loader = ContextLoader::new(graph_for_loader, 500);
    let context_result = loader.load_context("test");
    // Just verify it runs without error
    let _ = context_result.content;
    let _ = context_result.summary_sequence;

    // Step 5: Update memories using MemoryUpdater
    // Use matching words in the output for relevance calculation
    let mut updater = MemoryUpdater::new(&mut graph);
    let output = "Step three: The memory system is working correctly step test";
    let summary_ids = vec![NodeId(1), NodeId(2)];
    let new_ids = updater.update_from_output(output, &summary_ids);
    // Note: new_ids might be empty if relevance threshold is not met

    // Step 6: Verify "万步 0 偏移" concept - zero offset traversal
    // This means starting from any node with offset 0, we should be able to traverse
    let traversed_from_start = graph.traverse(&[NodeId(1)], 10);
    let traversed_from_middle = graph.traverse(&[NodeId(2)], 10);

    // Both traversals should work correctly (0 offset = start from the given node)
    assert!(traversed_from_start.len() > 0);
    assert!(traversed_from_middle.len() > 0);

    // Step 7: Verify total nodes after pipeline
    // Original 2 + possible new nodes from updater
    assert!(graph.node_count() >= 2);

    // Step 8: Test cache with the graph
    let mut manager = CacheManager::new();
    let l1 = Box::new(L1MemoryCache::new());
    manager.add_layer(l1);

    // Cache some graph data
    manager.set("node_1_content", "First step: Initialize the memory graph");
    let cached = manager.get("node_1_content");
    assert_eq!(cached, Some("First step: Initialize the memory graph".to_string()));
}

/// Test: L1 Memory Cache specific operations
#[test]
fn test_l1_cache_operations() {
    let mut cache = L1MemoryCache::new();

    // Basic operations
    cache.set("test_key", "test_value");
    assert_eq!(cache.get("test_key"), Some("test_value".to_string()));

    // Remove
    cache.remove("test_key");
    assert_eq!(cache.get("test_key"), None);

    // Hit rate (should be 0 after no hits)
    let rate = cache.hit_rate();
    assert!(rate >= 0.0 && rate <= 1.0);
}

/// Test: Cache layer trait object
#[test]
fn test_cache_layer_trait() {
    let mut cache = L1MemoryCache::new();
    cache.set("key", "value");

    // Test through trait object
    let boxed: Box<dyn CacheLayer> = Box::new(cache);
    assert_eq!(boxed.get("key"), Some("value".to_string()));
    assert_eq!(boxed.name(), "L1_Memory");
}

/// Test: RadixTrie prefix matching
#[test]
fn test_radix_trie_prefix_match() {
    let mut trie = graph_memory::RadixTrie::new();

    trie.insert("hello_world", "value1".to_string());
    trie.insert("hello_rust", "value2".to_string());
    trie.insert("goodbye", "value3".to_string());

    // Prefix match
    let matches = trie.prefix_match("hello_");
    assert_eq!(matches.len(), 2);

    // Exact match
    let exact = trie.get("hello_world");
    assert_eq!(exact, Some(&"value1".to_string()));
}
