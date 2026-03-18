# GraphMemory

基于图结构的 LLM 上下文加载优化系统，使用 Rust 实现。

## 核心特性

### 1. 上下文倒置（Context Inversion）
传统方式中，上下文由会话持有并消耗 token。在 GraphMemory 中，**产物持有上下文**：
- MemoryNode 持有关完整的记忆内容
- LLM 只看到被"完美化"的历史
- 信噪比极高

### 2. Latent Graph（隐藏图）
图结构是 latent（隐藏的），不传入 LLM：
- 图存在于记忆的关联中
- LLM 通过 SummarySequence 访问记忆
- 只暴露展开后的 Vec<String>

### 3. 五级缓存系统
| 层级 | 来源 | 命中率 |
|------|------|--------|
| L1 | 本地内存 | ~30% |
| L2 | 本地磁盘 | ~20% |
| L3 | 网络存储 | ~10% |
| L4 | 上游供应商 | ~8% |
| L5 | 动态计算 | ~5% |
| **总计** | | **73%+** |

### 4. MemoryPackage（记忆包）
与工程包同构的结构：
- **Pro**: 主要导出/接口定义
- **Ada**: 适配层/实现细节
- **Shell**: 包装/入口脚本

### 5. 万步 0 偏移
多 LLM 协作时，上下文不偏移：
- DAG 记忆图保留依赖关系
- 每个产物独立持有关键上下文
- 动态代理团支持任意复杂度任务

### 6. API 增强配置
统一的 API 配置管理，支持多种提供商：
- **Anthropic** (Claude)
- **OpenAI**
- **MiniMax**
- **自定义 API**

一键导出 Claude Code 连接配置。

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                          对话输入                                │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                     MemoryGraph (DAG)                            │
│  节点: 完整无损记忆 + 摘要嵌入                                    │
│  边: 语义关联权重 (RefersTo/Causes/RelatedTo/PartOf/Contradicts)│
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      LatentGraph                                 │
│  对 LLM 隐藏图结构，只暴露展开后的内容                            │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      LLM (只看产物)                              │
└─────────────────────────────────────────────────────────────────┘
```

## 核心数据结构

### MemoryNode
```rust
pub struct MemoryNode {
    pub id: NodeId,
    pub content: String,           // 完整记忆内容
    pub summary: String,           // 摘要（用于检索）
    pub summary_embedding: Vec<f32>, // 摘要向量
    pub created_at: DateTime<Utc>,
    pub access_count: u32,
}
```

### MemoryEdge
```rust
pub enum RelationType {
    RefersTo,   // A 提及 B
    Causes,     // A 导致 B
    RelatedTo,  // A 与 B 相关
    PartOf,     // A 是 B 的一部分
    Contradicts, // A 与 B 矛盾
}

pub struct MemoryEdge {
    pub relation: RelationType,
    pub weight: f32,  // 0.0~1.0
}
```

## 快速开始

### 构建
```bash
cargo build
```

### 测试
```bash
cargo test
```

### 运行演示
```bash
# 本地演示（不调用 LLM）
cargo run --example demo

# LLM 集成演示（需要网络）
cargo run --example llm_integration

# API 配置演示
cargo run --example api_config
```

## 项目结构

```
src/
├── lib.rs              # 公共 API
├── graph/
│   ├── mod.rs          # MemoryGraph DAG
│   ├── node.rs         # MemoryNode
│   ├── edge.rs         # MemoryEdge + RelationType
│   └── latent.rs       # LatentGraph 抽象层
├── cache/
│   ├── mod.rs          # CacheManager
│   ├── l1_memory.rs    # L1 本地内存
│   ├── l2_disk.rs      # L2 本地磁盘
│   ├── l3_network.rs   # L3 网络存储
│   ├── l4_vendor.rs    # L4 上游供应商
│   ├── l5_compute.rs   # L5 动态计算
│   └── radix_trie.rs   # 前缀匹配
├── context/
│   ├── mod.rs          # ContextLoader
│   ├── loader.rs       # 上下文加载器
│   ├── summary_sequence.rs # 摘要序列
│   └── updater.rs      # 记忆更新器
├── package/
│   ├── mod.rs
│   ├── memory_package.rs
│   ├── pro.rs
│   ├── ada.rs
│   └── shell.rs
└── api/
    └── mod.rs          # API 配置管理
```

## 使用示例

### 记忆图与上下文加载
```rust
use graph_memory::*;

// 创建记忆图
let mut graph = MemoryGraph::new();

// 添加记忆节点
let node1 = MemoryNode::new(
    NodeId(1),
    "Rust 所有权系统：每个值有唯一所有者".to_string(),
    "所有权系统".to_string(),
    vec![],
);
graph.add_node(node1);

// 使用 LatentGraph 加载上下文
let latent = LatentGraph::new(&graph);
let seq = SummarySequence::new(vec![NodeId(1)]);
let context = latent.to_llm_context(&seq);

// context 是纯文本，不包含图结构
println!("{}", context);
```

### API 配置管理
```rust
use graph_memory::*;

// 创建 API 管理器
let mut api_manager = ApiManager::new();

// 添加 Anthropic API 配置
api_manager.add_config("anthropic".to_string(), ApiConfig::anthropic(
    "sk-ant-api03-xxxx".to_string(),
    None,
));

// 激活配置
api_manager.activate("anthropic").unwrap();

// 导出 Claude Code 连接信息
let info = api_manager.export_info();
println!("{}", info.to_claude_code_env());
```

## 参考

- [Microsoft GraphRAG](https://www.microsoft.com/en-us/research/project/graphrag/) - 基于知识图的 RAG 系统
- [LangChain LangGraph](https://github.com/langchain-ai/langgraph) - 多代理协作框架
- [RadixCache](https://arxiv.org/abs/2106.01250) - Token 缓存优化

## License

MIT
