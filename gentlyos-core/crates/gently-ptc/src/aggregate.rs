//! Result aggregation — combine leaf results back up to the root.
//!
//! BFS walks UP from leaf results, applying status rules to produce
//! a single aggregated result.

use crate::task::{LeafResult, TaskStatus};
use crate::tree::Tree;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The final aggregated result after combining all leaf results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResult {
    /// Overall status: "completed", "partial", "failed", "blocked", "escalated"
    pub status: String,
    /// Per-node summaries
    pub summaries: HashMap<String, NodeSummary>,
    /// Total tasks executed
    pub total_tasks: usize,
    /// Tasks that completed successfully
    pub completed_tasks: usize,
    /// Tasks that failed
    pub failed_tasks: usize,
}

/// Summary for a single node after aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub node_id: String,
    pub status: String,
    pub outputs: Vec<String>,
    pub artifact_count: usize,
}

/// Aggregate leaf results back up the tree.
///
/// Status logic:
/// - All completed -> "completed"
/// - Any failed but some completed -> "partial"
/// - All failed -> "failed"
/// - Any blocked -> "blocked"
/// - Any escalated -> "escalated"
pub fn aggregate_results(tree: &Tree, results: &[LeafResult]) -> AggregatedResult {
    if results.is_empty() {
        return AggregatedResult {
            status: "completed".to_string(),
            summaries: HashMap::new(),
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
        };
    }

    let total_tasks = results.len();
    let mut completed_tasks = 0;
    let mut failed_tasks = 0;
    let mut has_blocked = false;
    let mut has_escalated = false;

    // Build node summaries by mapping task_id back via node_id
    // We group results by their associated node_id through a reverse lookup
    let mut summaries: HashMap<String, NodeSummary> = HashMap::new();

    for result in results {
        match result.status {
            TaskStatus::Completed => completed_tasks += 1,
            TaskStatus::Failed => failed_tasks += 1,
            TaskStatus::Blocked => has_blocked = true,
            TaskStatus::Escalated => has_escalated = true,
            _ => {}
        }

        // For aggregation, we use task_id as a proxy for grouping.
        // In a full implementation, we'd cross-reference with the leaf tasks.
        let task_key = result.task_id.to_string();
        let entry = summaries
            .entry(task_key.clone())
            .or_insert_with(|| NodeSummary {
                node_id: task_key,
                status: "pending".to_string(),
                outputs: Vec::new(),
                artifact_count: 0,
            });

        entry.status = format!("{:?}", result.status);
        if !result.output.is_empty() {
            entry.outputs.push(result.output.clone());
        }
        entry.artifact_count += result.artifacts.len();
    }

    // Determine overall status (priority order)
    let status = if has_escalated {
        "escalated".to_string()
    } else if has_blocked {
        "blocked".to_string()
    } else if failed_tasks == total_tasks {
        "failed".to_string()
    } else if completed_tasks == total_tasks {
        "completed".to_string()
    } else if failed_tasks > 0 {
        "partial".to_string()
    } else {
        "completed".to_string()
    };

    AggregatedResult {
        status,
        summaries,
        total_tasks,
        completed_tasks,
        failed_tasks,
    }
}

/// Walk UP from leaves to the root, applying aggregation rules.
///
/// This is a BFS traversal that builds parent status from child statuses.
/// Currently the simple `aggregate_results` handles the common case;
/// this function is available for tree-aware aggregation.
pub fn aggregate_up_tree(
    tree: &Tree,
    results: &[LeafResult],
    _leaf_node_map: &HashMap<uuid::Uuid, String>,
) -> AggregatedResult {
    // For now, delegate to the flat aggregation.
    // A full implementation would BFS from leaves up through parent nodes.
    aggregate_results(tree, results)
}
