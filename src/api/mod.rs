//! API 增强模块
//!
//! 提供统一的 API 配置和代理功能，支持：
//! - 自定义 API 端点配置
//! - API 密钥管理
//! - 自动上下文注入
//! - 兼容 Claude Code 的 API 格式

pub mod proxy;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API 提供商类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProvider {
    /// Anthropic API (Claude)
    Anthropic,
    /// OpenAI API
    OpenAI,
    /// Azure OpenAI
    Azure,
    /// MiniMax API
    MiniMax,
    /// 自定义 API
    Custom(String),
}

/// API 消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

/// API 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: MessageRole,
    pub content: String,
}

/// API 请求配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub model: String,
    pub messages: Vec<ApiMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
}

fn default_max_tokens() -> u32 {
    1024
}

/// API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// 提供商
    pub provider: ApiProvider,
    /// API 端点
    pub endpoint: String,
    /// API 密钥
    pub api_key: String,
    /// 默认模型
    pub default_model: String,
    /// 可用模型列表
    pub available_models: Vec<String>,
    /// 其他配置
    pub extra: HashMap<String, String>,
}

impl ApiConfig {
    /// 创建 Anthropic API 配置
    pub fn anthropic(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.anthropic.com".to_string());
        ApiConfig {
            provider: ApiProvider::Anthropic,
            endpoint,
            api_key,
            default_model: "claude-3-5-sonnet-20241022".to_string(),
            available_models: vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
            ],
            extra: HashMap::new(),
        }
    }

    /// 创建 MiniMax API 配置
    pub fn minimax(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.minimaxi.com/anthropic".to_string());
        ApiConfig {
            provider: ApiProvider::MiniMax,
            endpoint,
            api_key,
            default_model: "MiniMax-M2.7".to_string(),
            available_models: vec![
                "MiniMax-M2.7".to_string(),
                "MiniMax-M2.1".to_string(),
                "MiniMax-M2".to_string(),
            ],
            extra: HashMap::new(),
        }
    }

    /// 创建 OpenAI API 配置
    pub fn openai(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.openai.com".to_string());
        ApiConfig {
            provider: ApiProvider::OpenAI,
            endpoint,
            api_key,
            default_model: "gpt-4o".to_string(),
            available_models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
            ],
            extra: HashMap::new(),
        }
    }

    /// 创建自定义 API 配置
    pub fn custom(name: String, api_key: String, endpoint: String, default_model: String) -> Self {
        ApiConfig {
            provider: ApiProvider::Custom(name),
            endpoint,
            api_key,
            default_model: default_model.clone(),
            available_models: vec![default_model],
            extra: HashMap::new(),
        }
    }
}

/// API 配置管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiManager {
    /// 当前激活的配置
    pub active_config: Option<ApiConfig>,
    /// 所有已保存的配置
    configs: HashMap<String, ApiConfig>,
}

impl ApiManager {
    pub fn new() -> Self {
        ApiManager {
            active_config: None,
            configs: HashMap::new(),
        }
    }

    /// 添加配置
    pub fn add_config(&mut self, name: String, config: ApiConfig) {
        self.configs.insert(name, config.clone());
        if self.active_config.is_none() {
            self.active_config = Some(config);
        }
    }

    /// 激活配置
    pub fn activate(&mut self, name: &str) -> Result<(), String> {
        match self.configs.get(name) {
            Some(config) => {
                self.active_config = Some(config.clone());
                Ok(())
            }
            None => Err(format!("配置 '{}' 不存在", name)),
        }
    }

    /// 获取当前配置
    pub fn get_active(&self) -> Option<&ApiConfig> {
        self.active_config.as_ref()
    }

    /// 获取所有配置名称
    pub fn list_configs(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }

    /// 移除配置
    pub fn remove_config(&mut self, name: &str) -> bool {
        self.configs.remove(name).is_some()
    }

    /// 导出配置信息（不含密钥）
    pub fn export_info(&self) -> ApiInfo {
        ApiInfo {
            active_provider: self.active_config.as_ref().map(|c| c.provider.clone()),
            available_configs: self.configs.keys().cloned().collect(),
            endpoints: self.configs.iter()
                .map(|(k, v)| (k.clone(), v.endpoint.clone()))
                .collect(),
            models: self.configs.iter()
                .map(|(k, v)| (k.clone(), v.available_models.clone()))
                .collect(),
        }
    }
}

impl Default for ApiManager {
    fn default() -> Self {
        Self::new()
    }
}

/// API 信息（用于连接外部服务）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    /// 当前激活的提供商
    pub active_provider: Option<ApiProvider>,
    /// 可用配置名称
    pub available_configs: Vec<String>,
    /// 各配置的端点
    pub endpoints: HashMap<String, String>,
    /// 各配置的可用模型
    pub models: HashMap<String, Vec<String>>,
}

impl ApiInfo {
    /// 生成 Claude Code 环境变量配置
    pub fn to_claude_code_env(&self) -> String {
        let mut lines = vec![
            "# Claude Code API 配置".to_string(),
            "# 在 Claude Code 设置中添加以下环境变量：".to_string(),
            "".to_string(),
        ];

        if let Some(config_name) = self.available_configs.first() {
            if let Some(endpoint) = self.endpoints.get(config_name) {
                lines.push(format!("ANTHROPIC_BASE_URL={}", endpoint));
            }
            lines.push("ANTHROPIC_AUTH_TOKEN=your_api_key_here".to_string());
            if let Some(models) = self.models.get(config_name) {
                if let Some(model) = models.first() {
                    lines.push(format!("ANTHROPIC_MODEL={}", model));
                }
            }
        }

        lines.push("".to_string());
        lines.push("# 或直接使用 GraphMemory API 代理：".to_string());
        lines.push("# 设置 GraphMemory 为本地代理，自动注入记忆上下文".to_string());

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_anthropic() {
        let config = ApiConfig::anthropic(
            "sk-test-key".to_string(),
            None,
        );
        assert_eq!(config.provider, ApiProvider::Anthropic);
        assert_eq!(config.default_model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_api_config_minimax() {
        let config = ApiConfig::minimax(
            "test-key".to_string(),
            Some("https://api.minimaxi.com/anthropic".to_string()),
        );
        assert_eq!(config.provider, ApiProvider::MiniMax);
        assert_eq!(config.endpoint, "https://api.minimaxi.com/anthropic");
    }

    #[test]
    fn test_api_manager() {
        let mut manager = ApiManager::new();

        manager.add_config("anthropic".to_string(), ApiConfig::anthropic("key1".to_string(), None));
        manager.add_config("minimax".to_string(), ApiConfig::minimax("key2".to_string(), None));

        assert_eq!(manager.list_configs(), vec!["anthropic".to_string(), "minimax".to_string()]);

        manager.activate("minimax").unwrap();
        assert!(matches!(manager.get_active().unwrap().provider, ApiProvider::MiniMax));
    }

    #[test]
    fn test_export_info() {
        let mut manager = ApiManager::new();
        manager.add_config("test".to_string(), ApiConfig::minimax("key".to_string(), None));

        let info = manager.export_info();
        let env = info.to_claude_code_env();

        assert!(env.contains("ANTHROPIC_BASE_URL"));
        assert!(env.contains("ANTHROPIC_AUTH_TOKEN"));
    }
}
