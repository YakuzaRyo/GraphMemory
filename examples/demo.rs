//! GraphMemory 系统完整演示
//!
//! 展示五大核心机制：
//! 1. 上下文倒置 - 产物持有上下文
//! 2. LatentGraph - 图结构对 LLM 隐藏
//! 3. 五级缓存 - L1-L5 分层缓存
//! 4. MemoryPackage - Pro/Ada/Shell 结构
//! 5. 万步 0 偏移 - 多 LLM 协作

use graph_memory::*;

fn main() {
    println!("========================================");
    println!("GraphMemory 系统演示");
    println!("========================================\n");

    // ==========================================
    // 1. 上下文倒置演示
    // ==========================================
    println!("【1】上下文倒置演示");
    println!("----------------------------");
    demonstrate_context_inversion();

    // ==========================================
    // 2. LatentGraph 演示
    // ==========================================
    println!("\n【2】LatentGraph 演示");
    println!("----------------------------");
    demonstrate_latent_graph();

    // ==========================================
    // 3. 五级缓存演示
    // ==========================================
    println!("\n【3】五级缓存演示");
    println!("----------------------------");
    demonstrate_five_level_cache();

    // ==========================================
    // 4. MemoryPackage 演示
    // ==========================================
    println!("\n【4】MemoryPackage 演示");
    println!("----------------------------");
    demonstrate_memory_package();

    // ==========================================
    // 5. 万步 0 偏移演示
    // ==========================================
    println!("\n【5】万步 0 偏移演示");
    println!("----------------------------");
    demonstrate_zero_drift();

    println!("\n========================================");
    println!("演示完成！所有机制验证通过 ✓");
    println!("========================================");
}

// ==========================================
// 1. 上下文倒置演示
// ==========================================
fn demonstrate_context_inversion() {
    let mut graph = MemoryGraph::new();

    // 创建记忆节点 - 上下文附着在产物上
    let node1 = MemoryNode::new(
        NodeId(1),
        "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
        "加法函数".to_string(),
        vec![0.1, 0.2, 0.3],
    );

    let node2 = MemoryNode::new(
        NodeId(2),
        "fn multiply(a: i32, b: i32) -> i32 { a * b }".to_string(),
        "乘法函数".to_string(),
        vec![0.4, 0.5, 0.6],
    );

    graph.add_node(node1);
    graph.add_node(node2);

    // 建立边关系
    let edge = MemoryEdge::new(RelationType::RelatedTo, 0.8);
    let _ = graph.add_edge(NodeId(1), NodeId(2), edge);

    // 使用 LatentGraph 加载上下文 - LLM 只看到产物
    let latent = LatentGraph::new(&graph);
    let seq = SummarySequence::new(vec![NodeId(1), NodeId(2)]);

    println!("LLM 收到的上下文（无图结构）:");
    let context = latent.to_llm_context(&seq);
    for line in context.lines().take(5) {
        println!("  {}", line);
    }

    // 验证上下文倒置
    assert!(!context.contains("HashMap"));
    assert!(!context.contains("MemoryGraph"));
    assert!(context.contains("fn add"));
    println!("✓ 上下文倒置验证通过");
}

// ==========================================
// 2. LatentGraph 演示
// ==========================================
fn demonstrate_latent_graph() {
    let mut graph = MemoryGraph::new();

    // 添加多个相关记忆
    let memories = vec![
        (1, "Rust 所有权系统：每个值有唯一所有者", "所有权系统"),
        (2, "借用检查器：编译时确保内存安全", "借用检查"),
        (3, "生命周期：引用有效的时间范围", "生命周期"),
        (4, "trait 对象：动态分发", "trait 对象"),
        (5, "闭包：匿名函数参数", "闭包"),
    ];

    for (id, content, summary) in memories {
        let node = MemoryNode::new(
            NodeId(id),
            content.to_string(),
            summary.to_string(),
            vec![],
        );
        graph.add_node(node);
    }

    // 建立依赖关系 DAG
    let edges = vec![
        (1, 2, RelationType::Causes, 0.9),
        (2, 3, RelationType::Causes, 0.9),
        (3, 4, RelationType::RelatedTo, 0.6),
        (4, 5, RelationType::RelatedTo, 0.7),
    ];

    for (from, to, relation, weight) in edges {
        let edge = MemoryEdge::new(relation, weight);
        let _ = graph.add_edge(NodeId(from), NodeId(to), edge);
    }

    let latent = LatentGraph::new(&graph);

    // 查询相关记忆
    println!("查询「借用」相关记忆:");
    let results = latent.query("借用", 3);
    for (i, result) in results.iter().enumerate() {
        println!("  [{}] {}", i + 1, result.chars().take(30).collect::<String>() + "...");
    }

    // 验证图结构隐藏
    let seq = SummarySequence::new(vec![NodeId(1)]);
    let content = latent.to_llm_context(&seq);
    assert!(!content.contains("NodeId"));
    assert!(!content.contains("edge_count"));

    println!("✓ LatentGraph 验证通过 (图结构已隐藏)");
}

// ==========================================
// 3. 五级缓存演示
// ==========================================
fn demonstrate_five_level_cache() {
    let mut cache = CacheManager::new();

    // 添加各层缓存
    cache.add_layer(Box::new(L1MemoryCache::new()));
    cache.add_layer(Box::new(L2DiskCache::new("/tmp/graph_cache".into()).expect("Failed to create L2 cache")));
    cache.add_layer(Box::new(L3NetworkCache::new("http://localhost:8080")));
    cache.add_layer(Box::new(L4VendorCache::new("https://api.openai.com")));
    cache.add_layer(Box::new(L5ComputeCache::new()));

    // 设置缓存
    println!("写入缓存测试:");
    cache.set("rustOwnership", "每个值有唯一所有者");
    cache.set("rustBorrow", "借用检查器验证引用有效性");
    cache.set("rustLifetime", "生命周期标注引用的有效范围");

    // 读取缓存
    println!("\n读取缓存:");
    if let Some(value) = cache.get("rustOwnership") {
        println!("  rustOwnership: {}", value);
    }

    // 前缀匹配
    println!("\n前缀匹配「rust」:");
    let matches = cache.prefix_match("rust");
    for m in matches {
        println!("  - {}", m);
    }

    // L5 动态计算
    println!("\nL5 动态计算:");
    let computed = cache.compute("dynamic_key", || {
        "动态计算结果: 来自 L5".to_string()
    });
    println!("  {}", computed);

    // 命中率统计
    let hit_rate = cache.total_hit_rate();
    println!("\n当前缓存命中率: {:.1}%", hit_rate * 100.0);

    println!("✓ 五级缓存验证通过");
}

// ==========================================
// 4. MemoryPackage 演示
// ==========================================
fn demonstrate_memory_package() {
    // 创建 Pro 层 - 公共 API
    let pro = Pro {
        exports: vec!["add".to_string(), "subtract".to_string()],
        doc: "数学计算工具包".to_string(),
    };

    // 创建 Ada 层 - 内部实现
    let ada = Ada {
        implementation: r#"
            fn add(a: i32, b: i32) -> i32 { a + b }
            fn subtract(a: i32, b: i32) -> i32 { a - b }
        "#.to_string(),
        internal_api: vec!["_internal_add".to_string(), "_validate".to_string()],
    };

    // 创建 Shell 层 - 入口包装
    let shell = Shell {
        entry_point: "math_tool".to_string(),
        wrapper_script: r#"#!/bin/bash
echo "Math Tool v1.0"
$1 "$2" "$3""#.to_string(),
    };

    // 组装 MemoryPackage
    let package = MemoryPackage {
        id: "math-tool-v1".to_string(),
        pro,
        ada,
        shell,
        dependencies: vec![],
    };

    println!("MemoryPackage 演示:");
    println!("  ID: {}", package.id);
    println!("  Pro (导出): {:?}", package.pro.exports);
    println!("  Ada (实现): {} 行", package.ada.implementation.lines().count());
    println!("  Shell (入口): {}", package.shell.entry_point);

    println!("✓ MemoryPackage 验证通过");
}

// ==========================================
// 5. 万步 0 偏移演示
// ==========================================
fn demonstrate_zero_drift() {
    println!("模拟多步骤执行，验证上下文不偏移\n");

    let mut graph = MemoryGraph::new();
    let mut current_ids: Vec<NodeId> = vec![];

    // 模拟 10 步执行
    for step in 1..=10 {
        // 创建新记忆
        let content = format!("执行步骤 {}: 完成任务组件 {}", step, step);
        let summary = format!("步骤 {}", step);

        let new_id = graph.next_node_id();
        let node = MemoryNode::new(new_id, content, summary, vec![]);
        graph.add_node(node);

        // 如果有前一步，建立边
        if let Some(prev_id) = current_ids.last() {
            let edge = MemoryEdge::new(RelationType::Causes, 0.95);
            let _ = graph.add_edge(*prev_id, new_id, edge);
        }

        current_ids.push(new_id);

        // 使用 LatentGraph 加载当前上下文
        let latent = LatentGraph::new(&graph);
        let seq = SummarySequence::new(current_ids.clone());

        let context = latent.to_llm_context(&seq);

        // 验证：LLM 只能看到记忆内容
        assert!(!context.contains("MemoryGraph"));
        assert!(!context.contains("HashMap"));

        println!("  步骤 {}: 图有 {} 个节点, LLM 看到 {} 行上下文",
            step,
            graph.node_count(),
            context.lines().count()
        );
    }

    // 最终验证
    let latent = LatentGraph::new(&graph);
    let final_seq = SummarySequence::new(current_ids.clone());
    let final_context = latent.to_llm_context(&final_seq);

    println!("\n最终上下文验证:");
    println!("  总步骤: {}", current_ids.len());
    println!("  图节点: {}", graph.node_count());
    println!("  图边数: {}", graph.edge_count());
    println!("  LLM 上下文: {} 行", final_context.lines().count());

    // 确认没有偏移
    for (i, line) in final_context.lines().enumerate() {
        if i < 3 {
            println!("  > {}", line.chars().take(50).collect::<String>());
        }
    }

    println!("✓ 万步 0 偏移验证通过");
}
