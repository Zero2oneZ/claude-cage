//! Intent decomposition — route a high-level intent to leaf tasks.
//!
//! `route_intent` scores tree nodes by keyword matching against the intent,
//! then `walk_down` performs a DFS from matched nodes to find leaf tasks.

use crate::task::LeafTask;
use crate::tree::Tree;
use crate::executor::ExecutionMode;
use uuid::Uuid;

/// Route an intent string to matching node IDs in the tree.
///
/// Scoring: each node scores by counting how many words in the intent
/// match words in the node's name or metadata "keywords" array.
/// Nodes with score > 0 are returned, sorted by score descending.
pub fn route_intent(tree: &Tree, intent: &str) -> Vec<String> {
    let intent_words: Vec<String> = intent
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut scored: Vec<(String, usize)> = tree
        .nodes
        .values()
        .filter_map(|node| {
            let score = score_node(node, &intent_words);
            if score > 0 {
                Some((node.id.clone(), score))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    scored.into_iter().map(|(id, _)| id).collect()
}

/// Score a node against intent words.
fn score_node(node: &crate::tree::UniversalNode, intent_words: &[String]) -> usize {
    let mut score = 0;

    // Match against node name
    let name_lower = node.name.to_lowercase();
    for word in intent_words {
        if name_lower.contains(word.as_str()) {
            score += 1;
        }
    }

    // Match against metadata keywords
    if let Some(keywords) = node.metadata.get("keywords").and_then(|v| v.as_array()) {
        for kw_val in keywords {
            if let Some(kw) = kw_val.as_str() {
                let kw_lower = kw.to_lowercase();
                for word in intent_words {
                    if kw_lower.contains(word.as_str()) {
                        score += 2; // metadata keywords weighted higher
                    }
                }
            }
        }
    }

    score
}

/// Walk down from matched nodes to leaf tasks via DFS.
///
/// For each matched node ID, if the node is a leaf, create a LeafTask directly.
/// Otherwise, DFS into children until leaves are found.
pub fn walk_down(tree: &Tree, node_ids: &[String]) -> Vec<LeafTask> {
    let mut tasks = Vec::new();
    let mut visited = std::collections::HashSet::new();

    for node_id in node_ids {
        collect_leaf_tasks(tree, node_id, &mut tasks, &mut visited);
    }

    tasks
}

/// Recursively collect leaf tasks from a node via DFS.
fn collect_leaf_tasks(
    tree: &Tree,
    node_id: &str,
    tasks: &mut Vec<LeafTask>,
    visited: &mut std::collections::HashSet<String>,
) {
    if visited.contains(node_id) {
        return;
    }
    visited.insert(node_id.to_string());

    if let Some(node) = tree.get(node_id) {
        if node.is_leaf() {
            tasks.push(LeafTask {
                id: Uuid::new_v4(),
                node_id: node.id.clone(),
                intent: node.name.clone(),
                mode: ExecutionMode::Inspect,
                context: node.metadata.clone(),
            });
        } else {
            for child_id in &node.children {
                collect_leaf_tasks(tree, child_id, tasks, visited);
            }
        }
    }
}
