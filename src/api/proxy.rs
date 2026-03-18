//! API 代理服务
//!
//! GraphMemory 作为中间层，自动注入记忆上下文到 LLM 请求

use crate::api::{ApiConfig, ApiInfo, ApiManager};
use crate::context::SummarySequence;
use crate::graph::{LatentGraph, MemoryGraph};
use crate::CacheManager;
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
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            listen_addr: "127.0.0.1".to_string(),
            listen_port: 8080,
            target_api: "default".to_string(),
            max_context_tokens: 4000,
            cache_enabled: true,
        }
    }
}

/// 代理状态
pub struct ProxyState {
    pub api_manager: RwLock<ApiManager>,
    pub memory_graph: RwLock<MemoryGraph>,
    pub cache: RwLock<CacheManager>,
    pub http_client: Client,
}

impl ProxyState {
    pub fn new() -> Self {
        let mut cache = CacheManager::new();
        cache.add_layer(Box::new(crate::L1MemoryCache::new()));

        ProxyState {
            api_manager: RwLock::new(ApiManager::new()),
            memory_graph: RwLock::new(MemoryGraph::new()),
            cache: RwLock::new(cache),
            http_client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
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

    /// 添加记忆到图
    pub fn add_memory(&self, content: String, summary: String) -> crate::NodeId {
        let mut graph = self.memory_graph.write().unwrap();
        let id = graph.next_node_id();
        let node = crate::MemoryNode::new(id, content, summary, vec![]);
        graph.add_node(node);
        id
    }

    /// 获取增强后的上下文
    pub fn get_enhanced_context(&self, query: &str, max_memories: usize) -> String {
        let graph = self.memory_graph.read().unwrap();
        let latent = LatentGraph::new(&graph);

        // 查询相关记忆
        let memories = latent.query(query, max_memories);

        eprintln!("[DEBUG] get_enhanced_context: query={}, memories_found={}", query, memories.len());

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

        if !context.is_empty() {
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
