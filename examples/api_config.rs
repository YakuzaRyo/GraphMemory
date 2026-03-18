//! API 增强配置演示
//!
//! 展示如何：
//! 1. 配置多种 API 提供商
//! 2. 管理 API 配置
//! 3. 导出 Claude Code 连接信息

use graph_memory::*;

fn main() {
    println!("========================================");
    println!("GraphMemory API 增强配置演示");
    println!("========================================\n");

    // ==========================================
    // 1. 创建 API 管理器
    // ==========================================
    println!("【1】创建 API 管理器");
    println!("----------------------------");
    let mut api_manager = ApiManager::new();

    // ==========================================
    // 2. 添加多种 API 配置
    // ==========================================
    println!("\n【2】添加 API 配置");
    println!("----------------------------");

    // Anthropic (Claude)
    let anthropic_config = ApiConfig::anthropic(
        "sk-ant-api03-xxxx".to_string(),
        None, // 使用默认端点
    );
    println!("添加 Anthropic API:");
    println!("  端点: {}", anthropic_config.endpoint);
    println!("  默认模型: {}", anthropic_config.default_model);
    println!("  可用模型: {:?}", anthropic_config.available_models);
    api_manager.add_config("anthropic".to_string(), anthropic_config);

    // MiniMax
    let minimax_config = ApiConfig::minimax(
        "eyJh...".to_string(),
        Some("https://api.minimaxi.com/anthropic".to_string()),
    );
    println!("\n添加 MiniMax API:");
    println!("  端点: {}", minimax_config.endpoint);
    println!("  默认模型: {}", minimax_config.default_model);
    println!("  可用模型: {:?}", minimax_config.available_models);
    api_manager.add_config("minimax".to_string(), minimax_config);

    // OpenAI
    let openai_config = ApiConfig::openai(
        "sk-xxxx".to_string(),
        None,
    );
    println!("\n添加 OpenAI API:");
    println!("  端点: {}", openai_config.endpoint);
    println!("  默认模型: {}", openai_config.default_model);
    api_manager.add_config("openai".to_string(), openai_config);

    // 自定义 API (例如本地 LLM)
    let custom_config = ApiConfig::custom(
        "Local LLM".to_string(),
        "no-key-needed".to_string(),
        "http://localhost:11434".to_string(),
        "llama3".to_string(),
    );
    println!("\n添加自定义 API:");
    println!("  端点: {}", custom_config.endpoint);
    println!("  默认模型: {}", custom_config.default_model);
    api_manager.add_config("local".to_string(), custom_config);

    // ==========================================
    // 3. 管理配置
    // ==========================================
    println!("\n【3】管理 API 配置");
    println!("----------------------------");

    println!("已配置的 API:");
    for name in api_manager.list_configs() {
        println!("  - {}", name);
    }

    // 切换激活配置
    println!("\n切换到 MiniMax...");
    api_manager.activate("minimax").unwrap();
    let active = api_manager.get_active().unwrap();
    println!("当前激活: {:?}", active.provider);

    println!("\n切换到 Anthropic...");
    api_manager.activate("anthropic").unwrap();
    let active = api_manager.get_active().unwrap();
    println!("当前激活: {:?}", active.provider);

    // ==========================================
    // 4. 导出连接信息
    // ==========================================
    println!("\n【4】导出 Claude Code 连接信息");
    println!("----------------------------");

    // 激活一个配置用于导出
    api_manager.activate("minimax").unwrap();
    let info = api_manager.export_info();

    println!("\nAPI 信息:");
    println!("  激活的提供商: {:?}", info.active_provider);
    println!("  可用配置: {:?}", info.available_configs);
    println!("  端点映射: {:?}", info.endpoints);
    println!("  模型映射: {:?}", info.models);

    // 生成 Claude Code 环境变量
    println!("\n----------------------------------------");
    println!("Claude Code 环境变量配置:");
    println!("----------------------------------------");
    println!("{}", info.to_claude_code_env());
    println!("----------------------------------------");

    // ==========================================
    // 5. 与 MemoryGraph 集成
    // ==========================================
    println!("\n【5】与 MemoryGraph 集成");
    println!("----------------------------");

    // 创建记忆图
    let mut graph = MemoryGraph::new();
    let memories = vec![
        (1, "上下文管理：使用 MemoryGraph 管理对话上下文", "上下文管理"),
        (2, "API 配置：通过 ApiManager 统一管理多 API", "API 配置"),
        (3, "Latent Graph：图结构对 LLM 隐藏", "Latent Graph"),
    ];

    for (id, content, summary) in memories {
        let node = MemoryNode::new(NodeId(id), content.to_string(), summary.to_string(), vec![]);
        graph.add_node(node);
    }

    let latent = LatentGraph::new(&graph);
    let seq = SummarySequence::new(vec![NodeId(1), NodeId(2), NodeId(3)]);
    let context = latent.to_llm_context(&seq);

    println!("增强后的上下文 (可注入 LLM):");
    for line in context.lines().take(6) {
        println!("  {}", line);
    }

    println!("\n========================================");
    println!("API 增强配置演示完成");
    println!("========================================");

    // ==========================================
    // 6. 提示用户配置
    // ==========================================
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║           GraphMemory API 增强使用指南                       ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║  1. Claude Code 连接方式:                                  ║");
    println!("║     - 导出 ANTHROPIC_BASE_URL                             ║");
    println!("║     - 导出 ANTHROPIC_AUTH_TOKEN                           ║");
    println!("║     - 导出 ANTHROPIC_MODEL                                ║");
    println!("║                                                           ║");
    println!("║  2. GraphMemory 作为本地代理:                             ║");
    println!("║     - 启动 GraphMemory API 服务                           ║");
    println!("║     - 自动注入记忆上下文到请求                            ║");
    println!("║     - 转发到配置的 API 端点                               ║");
    println!("║                                                           ║");
    println!("║  3. 查看完整配置:                                          ║");
    println!("║     api_manager.export_info()                              ║");
    println!("║                                                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
}
