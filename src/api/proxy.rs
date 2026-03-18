//! API 代理服务
//!
//! GraphMemory 作为中间层，自动注入记忆上下文到 LLM 请求

use crate::api::{ApiConfig, ApiInfo, ApiManager};
use crate::context::SummarySequence;
use crate::graph::{LatentGraph, MemoryGraph, NodeId, MemoryEdge, RelationType};
use crate::CacheManager;
use crate::persistence::MemoryPersistence;
use crate::package::MemoryPackage;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// 代理配置
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// 监听地址
    pub listen_addr: String,
    /// 监听端口
    pub listen_port: u16,
    /// 目标 API 配置名称
    pub target_api: String,
    /// 最大注入上下文 token 数
    pub max_context_tokens: usize,
    /// 是否启用缓存
    pub cache_enabled: bool,
    /// 记忆持久化文件路径
    pub persistence_path: String,
    /// 是否自动保存记忆
    pub auto_save: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            listen_addr: "127.0.0.1".to_string(),
            listen_port: 8080,
            target_api: "default".to_string(),
            max_context_tokens: 4000,
            cache_enabled: true,
            persistence_path: "memories.json".to_string(),
            auto_save: true,
        }
    }
}

/// 代理状态
pub struct ProxyState {
    pub api_manager: RwLock<ApiManager>,
    pub memory_graph: RwLock<MemoryGraph>,
    pub cache: RwLock<CacheManager>,
    pub http_client: Client,
    pub persistence: MemoryPersistence,
}

impl ProxyState {
    pub fn new() -> Self {
        let mut cache = CacheManager::new();
        cache.add_layer(Box::new(crate::L1MemoryCache::new()));

        let persistence = MemoryPersistence::new("memories.json");

        // 尝试从文件加载记忆
        let graph = match persistence.load() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[WARNING] Failed to load memories: {}, starting fresh", e);
                MemoryGraph::new()
            }
        };

        ProxyState {
            api_manager: RwLock::new(ApiManager::new()),
            memory_graph: RwLock::new(graph),
            cache: RwLock::new(cache),
            http_client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
            persistence,
        }
    }

    /// 添加 API 配置
    pub fn add_api(&self, name: String, config: ApiConfig) {
        let mut manager = self.api_manager.write().unwrap();
        manager.add_config(name, config);
    }

    /// 激活 API
    pub fn activate_api(&self, name: &str) -> Result<(), String> {
        let mut manager = self.api_manager.write().unwrap();
        manager.activate(name)
    }

    /// 获取 API 信息
    pub fn get_api_info(&self) -> ApiInfo {
        let manager = self.api_manager.read().unwrap();
        manager.export_info()
    }

    /// 添加记忆到图（使用 MemoryPackage）
    pub fn add_memory_package(&self, package: MemoryPackage) -> NodeId {
        let mut graph = self.memory_graph.write().unwrap();
        let id = graph.add_package(package);

        // 自动保存
        if let Err(e) = self.persistence.save(&graph) {
            eprintln!("[WARNING] Failed to save memories: {}", e);
        }

        id
    }

    /// 添加记忆到图（向后兼容的简单接口）
    pub fn add_memory(&self, content: String, summary: String) -> NodeId {
        let package = MemoryPackage::from_content(
            format!("mem_{}", chrono::Utc::now().timestamp()),
            summary,
            content,
        );
        self.add_memory_package(package)
    }

    /// 手动保存记忆到文件
    pub fn save_memories(&self) -> Result<(), String> {
        let graph = self.memory_graph.read().unwrap();
        self.persistence.save(&graph).map_err(|e| e.to_string())
    }

    /// 获取增强后的上下文
    pub fn get_enhanced_context(&self, query: &str, max_memories: usize) -> String {
        let graph = self.memory_graph.read().unwrap();
        let latent = LatentGraph::new(&graph);

        // 查询相关记忆
        let memories = latent.query(query, max_memories);

        // 格式化为上下文
        if memories.is_empty() {
            String::new()
        } else {
            let content: Vec<String> = memories
                .iter()
                .enumerate()
                .map(|(i, m)| format!("[记忆 {}]\n{}", i + 1, m))
                .collect();
            format!("以下是相关记忆：\n{}\n\n", content.join("\n---\n"))
        }
    }

    /// 获取完整上下文（包含依赖，用于上下文倒置）
    pub fn get_full_context(&self, node_id: NodeId) -> Option<String> {
        let graph = self.memory_graph.read().unwrap();
        let latent = LatentGraph::new(&graph);
        latent.get_full_context(node_id)
    }

    /// 转发请求到目标 API
    pub async fn forward_request(
        &self,
        mut request: ProxyRequest,
    ) -> Result<String, ProxyError> {
        // 获取 API 配置
        let api_config = {
            let manager = self.api_manager.read().unwrap();
            manager.get_active().cloned()
        };

        let config = api_config.ok_or(ProxyError::NoApiConfigured)?;

        // 注入记忆上下文
        let context = self.get_enhanced_context(&request.get_query(), 5);
        let context_injected = !context.is_empty();

        if context_injected {
            request.inject_context(&context);
        }

        // 构建转发请求
        let target_url = format!("{}/v1/messages", config.endpoint);

        let response = self
            .http_client
            .post(&target_url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request.anthropic_request)
            .send()
            .await
            .map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| ProxyError::UpstreamError(e.to_string()))?;

        if !status.is_success() {
            return Err(ProxyError::UpstreamError(format!("{}: {}", status, body)));
        }

        Ok(body)
    }

    /// 从 LLM 回复中提取关键信息并自动存储为记忆
    pub fn extract_and_store(&self, response: &str, query: &str) -> Vec<NodeId> {
        let extracted = Self::extract_key_facts(response, query);

        if extracted.is_empty() {
            return vec![];
        }

        let mut new_ids = vec![];

        for fact in extracted {
            let package = MemoryPackage::from_content(
                format!("fact_{}", chrono::Utc::now().timestamp_millis()),
                fact.summary.clone(),
                fact.content,
            );

            let id = self.add_memory_package(package);
            new_ids.push(id);
        }

        new_ids
    }

    /// 从回复中提取关键事实
    fn extract_key_facts(response: &str, _query: &str) -> Vec<ExtractedFact> {
        let mut facts = vec![];

        // 简单的事实提取策略：
        // 1. 提取包含用户信息的句子
        // 2. 提取包含项目/技术信息的句子
        // 3. 提取包含决策/结论的句子

        let lines: Vec<&str> = response.lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        for line in lines {
            // 跳过太短的行
            if line.len() < 10 {
                continue;
            }

            // 跳过明显的系统消息
            if line.contains("<system")
                || line.contains("[SUGGESTION")
                || line.contains("TRIGGER")
                || line.contains("DO NOT TRIGGER")
            {
                continue;
            }

            // 检查是否包含关键信息模式
            let has_key_info =
                line.contains("用户") ||
                line.contains("我使用") ||
                line.contains("编程语言") ||
                line.contains("项目") ||
                line.contains("框架") ||
                line.contains("使用") && line.contains("语言") ||
                line.contains("配置") ||
                line.contains("设置") ||
                line.contains("决定") ||
                line.contains("选择");

            if has_key_info {
                let summary = if line.len() > 50 {
                    format!("{}...", &line[..50])
                } else {
                    line.to_string()
                };

                facts.push(ExtractedFact {
                    summary,
                    content: line.to_string(),
                });
            }
        }

        // 去重
        facts.dedup_by(|a, b| a.summary == b.summary);

        facts
    }
}

/// 从回复中提取的事实
struct ExtractedFact {
    summary: String,
    content: String,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self::new()
    }
}

/// 代理请求
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    /// 原始请求体
    pub raw_request: serde_json::Value,
    /// 转换后的 Anthropic 请求
    pub anthropic_request: AnthropicChatRequest,
    /// 查询字符串（用于检索记忆）
    pub query_text: String,
}

impl ProxyRequest {
    pub fn from_json(json: serde_json::Value) -> Result<Self, ProxyError> {
        let query_text = Self::extract_query(&json);

        let anthropic_request: AnthropicChatRequest = serde_json::from_value(json.clone())
            .map_err(|e| ProxyError::InvalidRequest(format!("Parse error: {}", e)))?;

        Ok(ProxyRequest {
            raw_request: json,
            anthropic_request,
            query_text,
        })
    }

    fn extract_query(json: &serde_json::Value) -> String {
        // 尝试从 messages 中提取用户消息
        // 找到最后一个包含实际用户问题的消息
        if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
            let mut last_valid_text = String::new();

            for msg in messages.iter() {
                if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                    if let Some(content) = msg.get("content") {
                        if let Some(text) = content.as_str() {
                            // 字符串格式
                            let cleaned = text.trim().trim_matches('"').trim().to_string();
                            if !cleaned.is_empty()
                                && !cleaned.contains("<system-reminder>")
                                && !cleaned.contains("[SUGGESTION MODE:")
                                && !cleaned.contains("The following skills")
                                && !cleaned.contains("TRIGGER when")
                                && !cleaned.contains("DO NOT TRIGGER")
                            {
                                last_valid_text = cleaned;
                            }
                        } else if let Some(blocks) = content.as_array() {
                            // 内容块数组格式
                            for block in blocks {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    let cleaned = text.trim().trim_matches('"').trim().to_string();
                                    if !cleaned.is_empty()
                                        && !cleaned.contains("<system-reminder>")
                                        && !cleaned.contains("[SUGGESTION MODE:")
                                        && !cleaned.contains("The following skills")
                                        && !cleaned.contains("TRIGGER when")
                                        && !cleaned.contains("DO NOT TRIGGER")
                                    {
                                        last_valid_text = cleaned;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            return last_valid_text;
        }
        String::new()
    }

    fn get_query(&self) -> String {
        self.query_text.clone()
    }

    pub fn inject_context(&mut self, context: &str) {
        if context.is_empty() {
            return;
        }

        // 创建一个 context block
        let context_block = serde_json::json!({
            "type": "text",
            "text": context
        });

        // 在 system 消息中注入上下文
        match &mut self.anthropic_request.system {
            Some(serde_json::Value::Array(arr)) => {
                // 如果 system 已经是数组，在开头插入 context
                arr.insert(0, context_block);
            }
            Some(serde_json::Value::String(s)) => {
                // 如果 system 是字符串，创建一个新数组
                self.anthropic_request.system = Some(serde_json::json!([
                    context_block,
                    {"type": "text", "text": s}
                ]));
            }
            Some(v) => {
                // 其他情况，创建新数组
                self.anthropic_request.system = Some(serde_json::json!([
                    context_block,
                    v.clone()
                ]));
            }
            None => {
                // 没有 system，直接设置为字符串
                self.anthropic_request.system = Some(serde_json::Value::String(context.to_string()));
            }
        }
    }
}

/// Anthropic 聊天请求 - 使用 Value 来支持任意格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnthropicChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    max_tokens: u32,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    pub system: Option<serde_json::Value>,
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// 代理错误
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("无效的请求: {0}")]
    InvalidRequest(String),
    #[error("没有配置 API")]
    NoApiConfigured,
    #[error("上游错误: {0}")]
    UpstreamError(String),
}

/// 代理响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub context_injected: bool,
}

impl ProxyResponse {
    pub fn success(data: serde_json::Value, context_injected: bool) -> Self {
        ProxyResponse {
            success: true,
            data: Some(data),
            error: None,
            context_injected,
        }
    }

    pub fn error(msg: String) -> Self {
        ProxyResponse {
            success: false,
            data: None,
            error: Some(msg),
            context_injected: false,
        }
    }
}

/// 代理统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStats {
    pub requests_total: u64,
    pub context_injected_total: u64,
    pub cache_hit_rate: f32,
    pub memory_count: usize,
}

impl ProxyState {
    pub fn get_stats(&self) -> ProxyStats {
        let graph = self.memory_graph.read().unwrap();
        let cache = self.cache.read().unwrap();

        ProxyStats {
            requests_total: 0,
            context_injected_total: 0,
            cache_hit_rate: cache.total_hit_rate(),
            memory_count: graph.node_count(),
        }
    }
}
