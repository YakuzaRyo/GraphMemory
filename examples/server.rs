//! GraphMemory API 代理服务器
//!
//! 启动命令: cargo run --example server
//!
//! 日志: 默认输出到 stderr，可配置输出到文件
//! 环境变量 RUST_LOG=trace,debug,info,warn,error

use graph_memory::api::proxy::{ProxyConfig, ProxyRequest, ProxyResponse, ProxyState};
use graph_memory::graph::LatentGraph;
use graph_memory::ApiConfig;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{info, warn, error, debug, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Debug, Deserialize)]
struct Config {
    server: ServerConfig,
    api: ApiConfigWrapper,
    memory: MemoryConfig,
    #[serde(default)]
    logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Default)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default)]
    file: Option<String>,
}

fn default_log_level() -> String { "info".to_string() }

#[derive(Debug, Deserialize)]
struct ServerConfig {
    listen_addr: String,
    listen_port: u16,
}

#[derive(Debug, Deserialize)]
struct ApiConfigWrapper {
    active: String,
    configs: std::collections::HashMap<String, ApiProviderConfig>,
}

#[derive(Debug, Deserialize)]
struct ApiProviderConfig {
    provider: String,
    endpoint: String,
    api_key: String,
    default_model: String,
}

#[derive(Debug, Deserialize)]
struct MemoryConfig {
    #[serde(default)]
    initial_memories: Vec<MemoryEntry>,
    #[serde(default = "default_max_context_tokens")]
    max_context_tokens: usize,
    #[serde(default = "default_relevance_threshold")]
    relevance_threshold: f32,
}

#[derive(Debug, Deserialize)]
struct MemoryEntry {
    content: String,
    summary: String,
}

fn default_max_context_tokens() -> usize { 4000 }
fn default_relevance_threshold() -> f32 { 0.1 }

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

fn init_logging(config: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    // 添加文件日志
    if let Some(log_file) = &config.file {
        // 确保日志目录存在
        if let Some(parent) = std::path::Path::new(log_file).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // 创建文件 APPEND 模式，支持日志轮转
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(log_file)
            .expect("无法创建日志文件");

        let file_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file));

        let subscriber = tracing_subscriber::registry()
            .with(fmt::layer().with_target(true).with_thread_ids(true))
            .with(file_layer)
            .with(filter);

        tracing::subscriber::set_global_default(subscriber).ok();

        eprintln!("日志已写入: {}", log_file);
    } else {
        let subscriber = tracing_subscriber::registry()
            .with(fmt::layer().with_target(true).with_thread_ids(true))
            .with(filter);

        tracing::subscriber::set_global_default(subscriber).ok();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置文件
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.json".to_string());
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {}", e);
            return Err(e);
        }
    };

    // 初始化日志
    init_logging(&config.logging);

    info!("========================================");
    info!("GraphMemory API 代理服务器启动");
    info!("========================================");
    info!("配置文件: {}", config_path);

    let state = Arc::new(ProxyState::new());

    // 配置 API
    info!("配置 API 提供商");
    for (name, provider_config) in &config.api.configs {
        if !provider_config.api_key.is_empty() {
            let api_config = ApiConfig::custom(
                provider_config.provider.clone(),
                provider_config.api_key.clone(),
                provider_config.endpoint.clone(),
                provider_config.default_model.clone(),
            );
            state.add_api(name.clone(), api_config);
            info!("  [+] {} -> {}", name, provider_config.endpoint);
        }
    }

    // 激活 API
    if let Some(_) = config.api.configs.get(&config.api.active) {
        state.activate_api(&config.api.active).ok();
        info!("  [>] 激活: {}", config.api.active);
    }

    // 添加初始记忆
    info!("加载初始记忆");
    for (i, memory) in config.memory.initial_memories.iter().enumerate() {
        state.add_memory(memory.content.clone(), memory.summary.clone());
        info!("  [{}] {}", i + 1, memory.summary);
    }
    info!("共 {} 条记忆", config.memory.initial_memories.len());

    // 启动服务器
    let addr: SocketAddr = format!("{}:{}", config.server.listen_addr, config.server.listen_port)
        .parse()
        .expect("Invalid address");

    info!("监听地址: http://{}", addr);
    info!("端点:");
    info!("  POST /v1/messages - 代理聊天请求");
    info!("  GET  /stats      - 查看统计");
    info!("  GET  /memory     - 查看记忆");
    info!("  POST /memory     - 添加记忆");
    info!("  GET  /config     - 查看配置");
    info!("========================================");

    let listener = TcpListener::bind(addr).await?;
    info!("服务器已启动");

    loop {
        match listener.accept().await {
            Ok((mut stream, client_addr)) => {
                let state = Arc::clone(&state);

                tokio::spawn(async move {
                    handle_connection(&mut stream, client_addr, &state).await;
                });
            }
            Err(e) => {
                error!("接受连接失败: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    client_addr: SocketAddr,
    state: &Arc<ProxyState>,
) {
    let start_time = std::time::Instant::now();

    // 读取所有 HTTP 数据 (header + body)
    let mut total_data = Vec::new();
    let mut header_ended = false;
    let mut content_length = 0;
    let mut body_bytes_read = 0;
    let mut in_body = false;

    loop {
        let mut buf = vec![0u8; 65536];
        let n = match stream.read(&mut buf).await {
            Ok(n) if n > 0 => n,
            _ => break,
        };
        buf.truncate(n);

        if !header_ended {
            // 尝试找 \r\n\r\n
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                header_ended = true;
                in_body = true;
                // header + \r\n\r\n 之前的内容
                total_data.extend_from_slice(&buf[..pos + 4]);

                // 解析 Content-Length
                let header_str = String::from_utf8_lossy(&total_data);
                content_length = header_str
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("content-length:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    .unwrap_or(0);

                // body 的剩余部分
                let body_start = pos + 4;
                body_bytes_read = buf.len() - body_start;
                total_data.extend_from_slice(&buf[body_start..]);

                debug!(content_length = content_length, header_end_pos = pos + 4, "HTTP 头解析完成");
            } else {
                // 还没找到 \r\n\r\n，继续接收
                total_data.extend_from_slice(&buf);
            }
        } else {
            // 已经在 body 中
            body_bytes_read += buf.len();
            total_data.extend_from_slice(&buf);
        }

        // 检查是否接收完成
        // GET 请求没有 body，content_length=0 时直接完成
        if in_body && ((content_length > 0 && body_bytes_read >= content_length) || content_length == 0) {
            break;
        }
        if !in_body && total_data.len() > 16384 {
            // 防止无限循环
            break;
        }
    }

    debug!(total_len = total_data.len(), body_bytes = body_bytes_read, "数据接收完成");

    if total_data.is_empty() {
        return;
    }

    let request_str = match String::from_utf8(total_data.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "UTF-8 转换失败");
            let _ = send_error(stream, 400, "Invalid request encoding").await;
            return;
        }
    };

    let lines: Vec<&str> = request_str.lines().collect();
    let request_line = lines.first().unwrap_or(&"");

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        let _ = send_error(stream, 400, "Invalid request line").await;
        return;
    }

    let method = parts[0];
    // 去除 query string，只保留路径
    let path = parts[1].split('?').next().unwrap_or(parts[1]);

    info!(method = %method, path = %path, client = %client_addr, "-> 请求开始");

    match (method, path) {
        ("POST", "/v1/messages") => {
            handle_chat(stream, &request_str, state).await;
        }
        ("GET", "/stats") => {
            handle_stats(stream, state).await;
        }
        ("GET", "/memory") => {
            handle_memory(stream, state).await;
        }
        ("GET", "/config") => {
            handle_config(stream, state).await;
        }
        ("POST", "/memory") => {
            handle_add_memory(stream, &request_str, state).await;
        }
        _ => {
            warn!(path = %path, "404 Not Found");
            let _ = send_error(stream, 404, "Not found").await;
        }
    }

    info!(duration_ms = start_time.elapsed().as_millis(), "<- 请求完成");
}

async fn handle_chat(
    stream: &mut tokio::net::TcpStream,
    request_str: &str,
    state: &Arc<ProxyState>,
) {
    if let Some(body_start) = request_str.find("\r\n\r\n") {
        let body = &request_str[body_start + 4..];

        debug!(body_len = body.len(), "收到聊天请求");

        let json: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "JSON 解析失败");
                let _ = send_error(stream, 400, &format!("JSON parse error: {}", e)).await;
                return;
            }
        };

        let proxy_request = match ProxyRequest::from_json(json) {
            Ok(req) => req,
            Err(e) => {
                // 打印原始 JSON 便于调试 - 打印更多字符
                let json_len = body.len();
                let json_preview = if json_len > 1000 {
                    format!("{}...[truncated {} chars]", &body[..1000], json_len - 1000)
                } else {
                    body.to_string()
                };
                warn!(error = %e, json_len = json_len, json_preview = %json_preview, "请求创建失败");
                let _ = send_error(stream, 400, &format!("Invalid request: {}", e)).await;
                return;
            }
        };

        // 记录请求详情
        let query_preview = proxy_request.query_text.chars().take(100).collect::<String>();
        let model = &proxy_request.anthropic_request.model;
        info!(model = %model, query_preview = %query_preview, "收到聊天请求");

        // 检查是否需要注入上下文
        let context = state.get_enhanced_context(&proxy_request.query_text, 5);
        let has_context = !context.is_empty();

        if has_context {
            info!(context_len = context.len(), "已注入记忆上下文");
            debug!(context_content = %context, "上下文内容");
        }

        let mut modified_request = proxy_request.clone();
        if has_context {
            modified_request.inject_context(&context);
        }

        info!(context_injected = has_context, "转发请求到上游");

        match state.forward_request(modified_request).await {
            Ok(response_body) => {
                info!(response_len = response_body.len(), "上游响应成功");
                debug!(response_body = %response_body, "上游响应详情");

                // 检查是否是流式响应（以 "event:" 开头）
                let is_streaming = response_body.trim().starts_with("event:");
                let response_json;

                if is_streaming {
                    // 流式响应直接返回，不包装
                    response_json = response_body;
                } else {
                    // 非流式响应包装在 ProxyResponse 中
                    let proxy_response = ProxyResponse::success(
                        serde_json::from_str(&response_body).unwrap_or_else(|_| {
                            serde_json::json!({"content": response_body})
                        }),
                        has_context,
                    );
                    response_json = serde_json::to_string(&proxy_response).unwrap_or_else(|_| r#"{"success":true}"#.to_string());
                }

                let content_type = if is_streaming {
                    "text/event-stream"
                } else {
                    "application/json"
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
                    content_type,
                    response_json.len(),
                    response_json
                );

                if let Err(e) = stream.write_all(response.as_bytes()).await {
                    error!(error = %e, "发送响应失败");
                }
            }
            Err(e) => {
                error!(error = %e, "上游请求失败");
                let _ = send_error(stream, 502, &format!("Upstream error: {}", e)).await;
            }
        }
    } else {
        warn!("请求缺少 body");
        let _ = send_error(stream, 400, "Missing body").await;
    }
}

async fn handle_stats(stream: &mut tokio::net::TcpStream, state: &Arc<ProxyState>) {
    info!("[STATS] 查看统计请求");
    let stats = state.get_stats();

    debug!(requests_total = stats.requests_total,
           context_injected = stats.context_injected_total,
           cache_hit_rate = stats.cache_hit_rate,
           memory_count = stats.memory_count,
           "统计数据详情");

    let json = serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string());

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        error!(error = %e, "[STATS] 发送响应失败");
    } else {
        info!(response_len = json.len(), "[STATS] 统计响应已发送");
    }
}

async fn handle_memory(stream: &mut tokio::net::TcpStream, state: &Arc<ProxyState>) {
    info!("[MEMORY] 查看记忆列表请求");
    let memories: Vec<String> = {
        let graph = state.memory_graph.read().unwrap();
        let latent = LatentGraph::new(&*graph);
        debug!(graph_node_count = graph.node_count(), "当前记忆图节点数");
        latent.query("", 100)
    };

    debug!(memory_count = memories.len(), "获取记忆列表");

    let response_data = serde_json::json!({
        "count": memories.len(),
        "memories": memories
    });

    let json = serde_json::to_string(&response_data).unwrap_or_else(|_| "{}".to_string());
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        error!(error = %e, "[MEMORY] 发送响应失败");
    } else {
        info!(response_len = json.len(), "[MEMORY] 记忆列表已发送");
    }
}

async fn handle_config(stream: &mut tokio::net::TcpStream, state: &Arc<ProxyState>) {
    info!("[CONFIG] 查看配置请求");
    let info = state.get_api_info();

    debug!(active_provider = ?info.active_provider,
           available_configs = ?info.available_configs,
           endpoints = ?info.endpoints,
           "API 配置详情");

    let json = serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string());

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json.len(),
        json
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        error!(error = %e, "[CONFIG] 发送响应失败");
    } else {
        info!(response_len = json.len(), "[CONFIG] 配置信息已发送");
    }
}

async fn handle_add_memory(
    stream: &mut tokio::net::TcpStream,
    request_str: &str,
    state: &Arc<ProxyState>,
) {
    if let Some(body_start) = request_str.find("\r\n\r\n") {
        let body = &request_str[body_start + 4..];

        debug!(body = %body, "收到添加记忆请求");

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            let content = json.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let summary = json.get("summary").and_then(|s| s.as_str()).unwrap_or("");

            if !content.is_empty() {
                let id = state.add_memory(content.to_string(), summary.to_string());
                info!(id = ?id, summary = %summary, content_len = content.len(), "[MEMORY] 添加新记忆成功");
                let response = serde_json::json!({
                    "success": true,
                    "id": format!("{:?}", id)
                });
                let json = serde_json::to_string(&response).unwrap();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    json.len(),
                    json
                );
                if let Err(e) = stream.write_all(resp.as_bytes()).await {
                    error!(error = %e, "[MEMORY] 发送响应失败");
                }
                return;
            } else {
                warn!("[MEMORY] 添加记忆失败: content 为空");
            }
        } else {
            warn!("[MEMORY] 添加记忆失败: JSON 解析错误");
        }
    } else {
        warn!("[MEMORY] 添加记忆失败: 缺少 body");
    }
    let _ = send_error(stream, 400, "Invalid request").await;
}

async fn send_error(stream: &mut tokio::net::TcpStream, code: u16, message: &str) -> Result<(), std::io::Error> {
    let body = serde_json::json!({
        "error": message,
        "code": code
    });

    let json = serde_json::to_string(&body).unwrap_or_else(|_| r#"{"error":"Internal error"}"#.to_string());
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        code,
        match code {
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            _ => "Error",
        },
        json.len(),
        json
    );

    stream.write_all(response.as_bytes()).await
}
