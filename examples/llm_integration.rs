//! GraphMemory + LLM 集成演示
//!
//! 使用真实的 LLM API 测试上下文加载

use graph_memory::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_TOKEN: &str = "sk-cp-q213ayuZalT7kOr7JpKYcq2IwmBxeJ7xC9qVezBCrAgXAsA2yWoNAinEyTFiAW8jqAsSwvFnxwpUZVgYkW3_xtw0kDzxY8dffE_iZlJRYF0Hdwz8Ch-lFaE";
const BASE_URL: &str = "https://api.minimaxi.com/anthropic";
const MODEL: &str = "MiniMax-M2.7";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ContentBlock {
    Text { text: String },
    #[allow(dead_code)]
    Other(serde_json::Value),
}

impl ContentBlock {
    fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            ContentBlock::Other(_) => None,
        }
    }
}

struct LLMClient {
    client: Client,
    base_url: String,
    token: String,
}

impl LLMClient {
    fn new() -> Self {
        LLMClient {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: BASE_URL.to_string(),
            token: API_TOKEN.to_string(),
        }
    }

    async fn chat(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let request = ChatRequest {
            model: MODEL.to_string(),
            max_tokens: 1024,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        let chat_response: ChatResponse = response.json().await?;

        let text = chat_response
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }
}

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("GraphMemory + LLM 集成测试");
    println!("========================================\n");

    // ==========================================
    // 1. 构建记忆图
    // ==========================================
    println!("【1】构建记忆图");
    println!("----------------------------");

    let mut graph = MemoryGraph::new();

    // 添加编程相关的记忆
    let memories = vec![
        (1, "Rust 是一种系统编程语言，强调内存安全、并发安全和性能。", "Rust 语言简介"),
        (2, "所有权系统是 Rust 的核心特性：每个值有唯一所有者，所有权可转移或借用。", "Rust 所有权系统"),
        (3, "借用检查器在编译时验证引用有效性，防止数据竞争和悬挂指针。", "借用检查器"),
        (4, "生命周期标注描述引用的有效时间范围，编译器据此检查安全性。", "生命周期"),
        (5, "trait 类似其他语言的接口，定义共享行为，可实现动态分发。", "Trait 系统"),
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

    // 建立边关系
    let edges = vec![
        (1, 2, RelationType::Causes, 0.9),
        (2, 3, RelationType::Causes, 0.9),
        (3, 4, RelationType::RelatedTo, 0.7),
        (4, 5, RelationType::RelatedTo, 0.6),
    ];

    for (from, to, relation, weight) in edges {
        let edge = MemoryEdge::new(relation, weight);
        let _ = graph.add_edge(NodeId(from), NodeId(to), edge);
    }

    println!("  图构建完成: {} 节点, {} 边\n", graph.node_count(), graph.edge_count());

    // ==========================================
    // 2. 使用 LatentGraph 查询
    // ==========================================
    println!("【2】使用 LatentGraph 查询记忆");
    println!("----------------------------");

    let latent = LatentGraph::new(&graph);

    // 查询所有权相关记忆
    let query = "所有权";
    let relevant_memories = latent.query(query, 3);

    println!("  查询「{}」相关记忆:", query);
    for (i, mem) in relevant_memories.iter().enumerate() {
        println!("    [{}] {}", i + 1, mem.chars().take(40).collect::<String>() + "...");
    }
    println!();

    // ==========================================
    // 3. 生成 LLM 上下文
    // ==========================================
    println!("【3】生成 LLM 上下文");
    println!("----------------------------");

    let seq = SummarySequence::new(vec![NodeId(1), NodeId(2), NodeId(3)]);
    let llm_context = latent.to_llm_context(&seq);

    println!("  生成的上下文 (LLM 可见):");
    for line in llm_context.lines().take(8) {
        println!("    {}", line);
    }
    println!();

    // ==========================================
    // 4. 调用真实 LLM
    // ==========================================
    println!("【4】调用真实 LLM");
    println!("----------------------------");

    let prompt = format!(
        r#"你是一个 Rust 编程助手。以下是一些相关记忆：

{}

请用简洁的语言解释这些概念之间的关系，以"记忆 X:"开头。"#,
        llm_context
    );

    println!("  发送请求到 LLM (模型: {})...", MODEL);

    let client = LLMClient::new();

    match client.chat(&prompt).await {
        Ok(response) => {
            println!("\n  LLM 响应:");
            println!("  -----------------------");
            for line in response.lines().take(15) {
                println!("  {}", line);
            }
            if response.lines().count() > 15 {
                println!("  ... (共 {} 行)", response.lines().count());
            }
            println!("  -----------------------");
            println!("\n✓ LLM 集成测试成功!");
        }
        Err(e) => {
            println!("\n  LLM 调用失败: {}", e);
            println!("  请检查 API 配置是否正确");
        }
    }

    // ==========================================
    // 5. 验证上下文倒置
    // ==========================================
    println!("\n【5】验证上下文倒置");
    println!("----------------------------");

    // 确保 LLM 上下文不包含图结构
    assert!(!llm_context.contains("HashMap"));
    assert!(!llm_context.contains("MemoryGraph"));
    assert!(!llm_context.contains("NodeId"));
    assert!(llm_context.contains("Rust"));

    println!("  ✓ 上下文不包含图结构信息");
    println!("  ✓ 上下文只包含纯记忆内容");
    println!("  ✓ 上下文倒置机制验证通过");

    println!("\n========================================");
    println!("GraphMemory + LLM 集成测试完成");
    println!("========================================");
}
