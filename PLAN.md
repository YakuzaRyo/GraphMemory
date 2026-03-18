# GraphMemory 记忆图引擎开发计划

## 一、核心概念理解

### 1.1 记忆包结构（与工程包同构）

每个记忆包包含三个部分：
- **pro (process/需求)**：这个记忆包解决的问题/需求描述
- **ada (adapter/适配)**：实现方案、适配逻辑
- **shell (导出)**：对外暴露的接口、产物

```rust
struct MemoryPackage {
    pro: String,    // 需求描述
    ada: String,    // 适配实现
    shell: String,  // 导出接口
}
```

### 1.2 记忆图（DAG）

- 节点：记忆包
- 边：包间依赖关系
- 拓扑：严格有向无环图，支持并行处理

```
[记忆包A] ──依赖──> [记忆包B]
                    │
                    └──依赖──> [记忆包C]
```

### 1.3 上下文倒置（核心创新）

| 传统方式 | 上下文倒置 |
|---------|-----------|
| 会话/Agent 持有上下文 | 产物/结果持有上下文 |
| 上下文随会话结束丢失 | 上下文归数据持有，持久化 |
| 多 Agent 共享困难 | 任意 Agent 可读取任意产物的完整上下文 |

### 1.4 完美历史

每个 LLM 在执行任务时，看到的历史记录是"每一步都完美成功"的：
- 不展示错误尝试
- 不展示中间失败
- 只展示最终成功的路径

这让 LLM 以为任务是一气呵成的，降低认知负担。

### 1.5 隐式能力传递

不同工序由不同 LLM 负责时，它们：
- 不知道有其他 LLM 存在
- 只看到完整的上下文（包含其他工序的产物）
- 能力通过上下文隐式传递和叠加

## 二、当前系统状态

### 2.1 已实现

- [x] TCP 代理服务器（8080 端口）
- [x] HTTP 请求解析与转发
- [x] 记忆图基本结构（MemoryGraph）
- [x] LatentGraph（隐藏图结构，只暴露内容）
- [x] 相关性检索（中文 bigram/字符重叠）
- [x] 上下文注入到 LLM 请求
- [x] MiniMax API 集成

### 2.2 待实现

- [ ] 记忆包结构（pro/ada/shell）
- [ ] DAG 依赖管理
- [ ] 上下文倒置检索机制
- [ ] 记忆持久化（JSON 文件）
- [ ] 自动记忆提取（对话后自动分析）
- [ ] 多 Agent 协作框架

## 三、后续目标

### 3.1 第一阶段：记忆包结构

```rust
// 记忆包 = 工程包同构
struct MemoryPackage {
    id: PackageId,
    pro: Requirement,      // 需求/问题描述
    ada: Adaptation,       // 适配实现
    shell: Export,         // 导出接口
    dependencies: Vec<PackageId>,  // 依赖的包
}

struct Requirement {
    summary: String,       // 简短摘要
    description: String,  // 详细描述
    constraints: Vec<String>,
}

struct Adaptation {
    solution: String,      // 解决方案
    implementation: String, // 具体实现
}

struct Export {
    interface: String,     // 接口定义
    artifacts: Vec<String>, // 产物列表
}
```

### 3.2 第二阶段：DAG 管理

```rust
trait DAGManager {
    fn add_package(&mut self, package: MemoryPackage);
    fn add_dependency(&mut self, from: PackageId, to: PackageId);
    fn get_execution_order(&self) -> Vec<PackageId>;  // 拓扑排序
    fn get_dependencies(&self, id: PackageId) -> Vec<PackageId>;
}
```

### 3.3 第三阶段：上下文倒置检索

```rust
trait ContextInversion {
    // 根据需求查找最合适的记忆包
    fn find_relevant_packages(&self, query: &str) -> Vec<PackageId>;

    // 获取某个记忆包的完整上下文（包含所有依赖的上下文）
    fn get_full_context(&self, id: PackageId) -> String;

    // 上下文合并策略
    fn merge_contexts(&self, packages: &[PackageId]) -> String;
}
```

### 3.4 第四阶段：自动记忆提取

```rust
trait AutoMemoryExtraction {
    // 从 LLM 回复中提取关键信息
    fn extract_key_facts(&self, response: &str) -> Vec<MemoryPackage>;

    // 更新已有记忆
    fn update_memory(&self, id: PackageId, new_info: &str);
}
```

### 3.5 第五阶段：多 Agent 框架

```rust
trait AgentFramework {
    // 注册 Agent
    fn register_agent(&mut self, id: AgentId, capability: Capability);

    // 分发任务
    fn dispatch_task(&self, task: Task) -> AgentId;

    // 执行并记录
    fn execute_and_record(&self, agent: AgentId, task: Task) -> Result<PackageId>;
}
```

## 四、技术路线

### 4.1 存储层
- JSON 文件持久化（memories.json）
- 每个记忆包独立文件（packages/*.json）

### 4.2 图结构
- 使用 petgraph 库管理 DAG
- 拓扑排序确定执行顺序

### 4.3 检索
- 中文语义检索（继续优化 bigram/字符重叠）
- 未来可引入向量检索（embedding）

### 4.4 上下文合并
- 按依赖顺序拼接
- 去重和冲突检测

## 五、预期效果

1. **万步0偏移**：N 个 LLM 做 N 步操作，最终结果正确
2. **上下文复用**：同一记忆包可被多个 Agent 并行使用
3. **能力叠加**：LLM 能力通过上下文隐式传递
4. **无中心**：不存在中心化的会话管理，每个 Agent 独立工作

## 六、参考项目

- LangGraph：图结构 agent
- AutoGen：多 agent 协作
- MemGPT：长期记忆管理

但本项目的核心差异是**上下文倒置**和**记忆包同构工程包**的设计理念。

---

*本文档由 Claude Code 与用户对话总结生成*
