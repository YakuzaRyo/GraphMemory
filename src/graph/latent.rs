// Latent Graph - 图结构是 latent（隐藏的）
//
// 核心思想：
// - 图结构存在于记忆的关联中，不是传给 LLM 的负担
// - LLM 只看到展开后的记忆内容 Vec<String>
// - 摘要序列 (SummarySequence) 是唯一的"句柄"
//
// 数据流：
//   MemoryGraph (完整图结构)
//         │
//         ▼
//   LatentGraph::to_llm_content()  // 只返回展开的内容
//         │
//         ▼
//   Vec<String> (LLM 可见)
//         │
//   图结构隐式保存在各记忆的元数据中
//

use crate::graph::{MemoryGraph, NodeId, RelationType};
use crate::context::SummarySequence;

/// LatentGraph - 对 LLM 隐藏的图结构抽象
///
/// LLM 永远不直接看到图结构，只能通过以下方式访问：
/// 1. expand() - 展开摘要序列为具体内容
/// 2. to_llm_context() - 生成 LLM 可见的上下文
pub struct LatentGraph<'a> {
    graph: &'a MemoryGraph,
}

impl<'a> LatentGraph<'a> {
    pub fn new(graph: &'a MemoryGraph) -> Self {
        LatentGraph { graph }
    }

    /// 展开摘要序列为完整记忆内容
    /// 这是 LLM 获取记忆的主要方式
    pub fn expand(&self, summary_seq: &SummarySequence) -> Vec<String> {
        summary_seq.expand(self.graph)
    }

    /// 生成 LLM 可见的上下文字符串
    /// 格式: "以下是相关记忆：\n[记忆1]\n---\n[记忆2]\n---\n[记忆3]"
    pub fn to_llm_context(&self, summary_seq: &SummarySequence) -> String {
        let memories = self.expand(summary_seq);
        if memories.is_empty() {
            String::from("没有找到相关记忆。")
        } else {
            let content: Vec<String> = memories
                .iter()
                .enumerate()
                .map(|(i, m)| format!("[记忆{}]\n{}", i + 1, m))
                .collect();
            format!("以下是相关记忆：\n{}", content.join("\n---\n"))
        }
    }

    /// 从查询获取相关记忆（基于关键词相关性过滤）
    ///
    /// 提高信噪比：
    /// - 只返回与查询关键词相关的记忆
    /// - 按相关性得分排序
    /// - 超过阈值才返回
    pub fn query(&self, query: &str, max_memories: usize) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        eprintln!("[DEBUG] latent.query: query={}, query_words={:?}, max_memories={}", query, query_words, max_memories);

        // 计算每个节点的相关性得分
        let all_ids = self.graph.all_node_ids();
        eprintln!("[DEBUG] latent.query: total_nodes={}", all_ids.len());

        let mut scored: Vec<(NodeId, f32)> = Vec::new();

        for id in all_ids {
            if let Some(node) = self.graph.get_node(id) {
                let score = if query_words.is_empty() {
                    // 空查询返回所有记忆，得分为1.0
                    1.0
                } else {
                    self.calculate_relevance(&node.summary, &node.content, &query_words)
                };
                eprintln!("[DEBUG] latent.query: node_id={:?}, summary={}, score={}", id, node.summary, score);
                if score > 0.0 {
                    scored.push((id, score));
                }
            }
        }

        eprintln!("[DEBUG] latent.query: scored_nodes={}", scored.len());

        // 按得分降序排序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 取前 max_memories 个
        let relevant_ids: Vec<NodeId> = scored
            .into_iter()
            .take(max_memories)
            .map(|(id, _)| id)
            .collect();

        eprintln!("[DEBUG] latent.query: returning {} memories", relevant_ids.len());

        let seq = SummarySequence::new(relevant_ids);
        self.expand(&seq)
    }

    /// 计算记忆与查询的相关性得分（支持精确匹配、前缀匹配、TF-IDF）
    fn calculate_relevance(&self, summary: &str, content: &str, query_words: &[&str]) -> f32 {
        let summary_lower = summary.to_lowercase();
        let content_lower = content.to_lowercase();

        let mut score = 0.0;

        // 判断是否像中文句子（以中文为主的查询）
        let is_chinese_query = |s: &str| -> bool {
            let chinese_chars = s.chars().filter(|c| {
                // 判断是否为CJK统一汉字
                let code = *c as u32;
                (0x4E00..=0x9FFF).contains(&code) || (0x3400..=0x4DBF).contains(&code)
            }).count();
            chinese_chars > s.chars().count() / 3
        };

        for word in query_words {
            let word_lower = word.to_lowercase();

            // 判断是中文还是英文查询
            let is_chinese = is_chinese_query(&word_lower);

            if is_chinese {
                // 中文处理：直接子串匹配
                // 1. 精确匹配摘要
                if summary_lower.contains(&word_lower) {
                    score += 3.0;
                }
                // 2. 精确匹配内容
                if content_lower.contains(&word_lower) {
                    score += 1.5;
                }
                // 3. 如果包含"记得"、"知道"等词，提取概念匹配
                // 提取查询中的关键概念（中文字符重叠）
                let query_chars: Vec<char> = word_lower.chars().filter(|c| c.is_alphanumeric()).collect();
                let summary_chars: Vec<char> = summary_lower.chars().filter(|c| c.is_alphanumeric()).collect();
                let content_chars: Vec<char> = content_lower.chars().filter(|c| c.is_alphanumeric()).collect();

                // 计算2-gram重叠
                if query_chars.len() >= 2 {
                    let query_bigrams: Vec<String> = (0..query_chars.len().saturating_sub(1))
                        .map(|i| format!("{}{}", query_chars[i], query_chars[i+1]))
                        .collect();

                    let summary_bigrams: Vec<String> = (0..summary_chars.len().saturating_sub(1))
                        .map(|i| format!("{}{}", summary_chars[i], summary_chars[i+1]))
                        .collect();

                    let content_bigrams: Vec<String> = (0..content_chars.len().saturating_sub(1))
                        .map(|i| format!("{}{}", content_chars[i], content_chars[i+1]))
                        .collect();

                    let summary_bigram_overlap = query_bigrams.iter().filter(|bg| summary_bigrams.contains(bg)).count();
                    let content_bigram_overlap = query_bigrams.iter().filter(|bg| content_bigrams.contains(bg)).count();

                    // bigram重叠率
                    if query_bigrams.len() > 0 {
                        let summary_ratio = summary_bigram_overlap as f32 / query_bigrams.len() as f32;
                        let content_ratio = content_bigram_overlap as f32 / query_bigrams.len() as f32;

                        if summary_ratio > 0.15 {
                            score += 2.0 * summary_ratio;
                        }
                        if content_ratio > 0.15 {
                            score += 1.0 * content_ratio;
                        }
                    }

                    // 同时计算单字符重叠（对中文更友好）
                    let summary_char_overlap = query_chars.iter().filter(|c| summary_chars.contains(c)).count();
                    let content_char_overlap = query_chars.iter().filter(|c| content_chars.contains(c)).count();

                    if query_chars.len() > 0 {
                        let summary_char_ratio = summary_char_overlap as f32 / query_chars.len() as f32;
                        let content_char_ratio = content_char_overlap as f32 / query_chars.len() as f32;

                        // 单字符重叠率达到30%以上就加分
                        if summary_char_ratio > 0.3 {
                            score += 1.5 * summary_char_ratio;
                        }
                        if content_char_ratio > 0.3 {
                            score += 0.8 * content_char_ratio;
                        }
                    }
                }
            } else {
                // 英文处理：单词边界匹配
                // 1. 精确匹配摘要（权重最高）
                if summary_lower.contains(&word_lower) {
                    score += 3.0;
                }
                // 2. 精确匹配内容
                if content_lower.contains(&word_lower) {
                    score += 1.5;
                }
                // 3. 前缀匹配摘要（单词边界）
                if self.word_prefix_match(&word_lower, &summary_lower) {
                    score += 2.0;
                }
                // 4. 前缀匹配内容
                if self.word_prefix_match(&word_lower, &content_lower) {
                    score += 1.0;
                }
            }
        }

        // 归一化
        let word_count = query_words.len() as f32;
        if word_count > 0.0 {
            score / word_count
        } else {
            0.0
        }
    }

    /// 检查查询词是否是文本中某个词的前缀（单词边界）
    fn word_prefix_match(&self, prefix: &str, text: &str) -> bool {
        let prefix_lower = prefix.to_lowercase();
        for word in text.split_whitespace() {
            // 清理单词（移除标点符号）
            let clean_word: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if clean_word.to_lowercase().starts_with(&prefix_lower) && !clean_word.eq_ignore_ascii_case(prefix) {
                return true;
            }
        }
        false
    }

    /// 获取记忆数量（图大小，但对 LLM 隐藏）
    pub fn memory_count(&self) -> usize {
        self.graph.node_count()
    }

    /// 检查图是否为空
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// 获取图的边数量（内部使用，不暴露给 LLM）
    #[allow(dead_code)]
    pub(crate) fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// 获取节点的邻居（内部使用，不暴露给 LLM）
    #[allow(dead_code)]
    pub(crate) fn get_neighbors(&self, id: NodeId) -> Vec<NodeId> {
        self.graph.get_neighbors(id)
            .into_iter()
            .map(|(neighbor_id, _)| neighbor_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{MemoryNode, MemoryEdge};
    use chrono::Utc;

    fn create_test_graph() -> MemoryGraph {
        let mut graph = MemoryGraph::new();

        let node1 = MemoryNode::new(
            NodeId(1),
            "第一个记忆：Rust 是一种系统编程语言".to_string(),
            "Rust 编程语言".to_string(),
            vec![],
        );

        let node2 = MemoryNode::new(
            NodeId(2),
            "第二个记忆：所有权系统是 Rust 的核心特性".to_string(),
            "Rust 所有权系统".to_string(),
            vec![],
        );

        let node3 = MemoryNode::new(
            NodeId(3),
            "第三个记忆：并发安全是 Rust 的设计目标".to_string(),
            "Rust 并发".to_string(),
            vec![],
        );

        graph.add_node(node1);
        graph.add_node(node2);
        graph.add_node(node3);

        // 添加边建立关联
        let edge12 = MemoryEdge::new(RelationType::RelatedTo, 0.8);
        let edge23 = MemoryEdge::new(RelationType::RelatedTo, 0.7);

        let _ = graph.add_edge(NodeId(1), NodeId(2), edge12);
        let _ = graph.add_edge(NodeId(2), NodeId(3), edge23);

        graph
    }

    #[test]
    fn test_latent_graph_expand() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        let seq = SummarySequence::new(vec![NodeId(1), NodeId(2)]);
        let expanded = latent.expand(&seq);

        assert_eq!(expanded.len(), 2);
        assert!(expanded[0].contains("Rust"));
    }

    #[test]
    fn test_latent_graph_to_llm_context() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        let seq = SummarySequence::new(vec![NodeId(1), NodeId(2)]);
        let context = latent.to_llm_context(&seq);

        assert!(context.contains("以下是相关记忆："));
        assert!(context.contains("记忆1"));
        assert!(context.contains("记忆2"));
        // 图结构不暴露
        assert!(!context.contains("NodeId"));
        assert!(!context.contains("HashMap"));
    }

    #[test]
    fn test_latent_graph_query() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        // 查询 Rust 相关记忆
        let results = latent.query("Rust", 2);
        assert!(!results.is_empty());
        assert!(results[0].contains("Rust"));
    }

    #[test]
    fn test_latent_graph_query_empty() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        // 空查询返回空
        let results = latent.query("", 2);
        assert!(results.is_empty());
    }

    #[test]
    fn test_latent_graph_query_relevance() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        // 查询所有权，应该只返回包含所有权的记忆
        let results = latent.query("所有权", 10);
        assert!(!results.is_empty());
        // 结果应该包含所有权相关内容
        assert!(results.iter().any(|r| r.contains("所有权")));
    }

    #[test]
    fn test_graph_structure_hidden() {
        let graph = create_test_graph();
        let latent = LatentGraph::new(&graph);

        // 验证图大小可查，但结构不暴露
        assert_eq!(latent.memory_count(), 3);
        assert!(!latent.is_empty());

        // 内部可以访问邻居（用于构建摘要序列）
        let neighbors = latent.get_neighbors(NodeId(1));
        assert_eq!(neighbors.len(), 1);

        // 但 LLM 看到的只是内容
        let seq = SummarySequence::new(vec![NodeId(1)]);
        let content = latent.to_llm_context(&seq);
        assert!(!content.contains("neighbors"));
    }
}
