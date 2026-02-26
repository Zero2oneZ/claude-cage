#![allow(dead_code, unused_variables, unused_imports)]

//! PTC Brain Engine
//!
//! Permission To Change — tree decomposition, leaf execution, result aggregation.
//!
//! The PTC engine decomposes a high-level intent into leaf tasks using
//! a universal tree, executes each leaf via a pluggable executor, and
//! aggregates results back up the tree.

pub mod tree;
pub mod decompose;
pub mod aggregate;
pub mod executor;
pub mod task;
pub mod escalation;
pub mod phase;
pub mod storage;

// Re-export key types
pub use tree::{Tree, UniversalNode, NodeScale};
pub use decompose::{route_intent, walk_down};
pub use aggregate::{aggregate_results, AggregatedResult};
pub use executor::{Executor, ExecutionMode, DryRunExecutor};
pub use task::{LeafTask, LeafResult, TaskStatus};
pub use escalation::{EscalationLevel, escalate, should_halt};
pub use phase::Phase;
pub use storage::{PtcStorage, PtcEvent, NullStorage};

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

/// The main PTC engine.
///
/// Holds the universal tree, an executor for running leaf tasks,
/// and a storage backend for persisting events.
pub struct PtcEngine {
    pub tree: Arc<Tree>,
    pub executor: Box<dyn Executor>,
    pub storage: Box<dyn PtcStorage>,
}

/// The result of a full PTC run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunResult {
    pub intent: String,
    pub leaf_results: Vec<LeafResult>,
    pub aggregated: AggregatedResult,
    pub phase: Phase,
}

impl PtcEngine {
    /// Create a new PTC engine.
    pub fn new(
        tree: Arc<Tree>,
        executor: Box<dyn Executor>,
        storage: Box<dyn PtcStorage>,
    ) -> Self {
        Self {
            tree,
            executor,
            storage,
        }
    }

    /// Run the PTC engine on an intent string.
    ///
    /// Pipeline: decompose intent -> execute leaves -> aggregate results.
    pub async fn run(&self, intent: &str) -> Result<RunResult> {
        // Phase 1: Intake — store the incoming event
        let intake_event = PtcEvent {
            timestamp: chrono::Utc::now(),
            phase: Phase::Intake,
            event_type: "intent_received".to_string(),
            data: serde_json::json!({ "intent": intent }),
        };
        let _ = self.storage.store_event(&intake_event).await;

        // Phase 2: Triage — route intent to matching nodes
        let matched_node_ids = route_intent(&self.tree, intent);

        let triage_event = PtcEvent {
            timestamp: chrono::Utc::now(),
            phase: Phase::Triage,
            event_type: "nodes_matched".to_string(),
            data: serde_json::json!({
                "matched_count": matched_node_ids.len(),
                "node_ids": matched_node_ids,
            }),
        };
        let _ = self.storage.store_event(&triage_event).await;

        // Phase 3: Plan — walk down to leaf tasks
        let leaf_tasks = walk_down(&self.tree, &matched_node_ids);

        let plan_event = PtcEvent {
            timestamp: chrono::Utc::now(),
            phase: Phase::Plan,
            event_type: "tasks_planned".to_string(),
            data: serde_json::json!({ "task_count": leaf_tasks.len() }),
        };
        let _ = self.storage.store_event(&plan_event).await;

        // Phase 4: Execute — run each leaf task
        let mut leaf_results = Vec::with_capacity(leaf_tasks.len());
        for task in &leaf_tasks {
            let start = std::time::Instant::now();
            match self.executor.execute(task).await {
                Ok(result) => leaf_results.push(result),
                Err(e) => {
                    leaf_results.push(LeafResult {
                        task_id: task.id,
                        status: TaskStatus::Failed,
                        output: format!("Execution error: {}", e),
                        artifacts: vec![],
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        let execute_event = PtcEvent {
            timestamp: chrono::Utc::now(),
            phase: Phase::Execute,
            event_type: "execution_complete".to_string(),
            data: serde_json::json!({
                "total": leaf_results.len(),
                "completed": leaf_results.iter().filter(|r| r.status == TaskStatus::Completed).count(),
                "failed": leaf_results.iter().filter(|r| r.status == TaskStatus::Failed).count(),
            }),
        };
        let _ = self.storage.store_event(&execute_event).await;

        // Phase 5: Verify + Integrate — aggregate results up the tree
        let aggregated = aggregate_results(&self.tree, &leaf_results);

        let integrate_event = PtcEvent {
            timestamp: chrono::Utc::now(),
            phase: Phase::Integrate,
            event_type: "aggregation_complete".to_string(),
            data: serde_json::json!({
                "status": aggregated.status,
                "summary_count": aggregated.summaries.len(),
            }),
        };
        let _ = self.storage.store_event(&integrate_event).await;

        // Determine final phase based on aggregation status
        let final_phase = match aggregated.status.as_str() {
            "completed" => Phase::Ship,
            _ => Phase::Verify,
        };

        Ok(RunResult {
            intent: intent.to_string(),
            leaf_results,
            aggregated,
            phase: final_phase,
        })
    }
}
