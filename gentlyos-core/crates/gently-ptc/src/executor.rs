//! Leaf execution trait and implementations.
//!
//! The `Executor` trait defines how leaf tasks are executed. Multiple
//! execution modes are supported (Design, Inspect, Shell, Claude, Plan).

use crate::task::{LeafTask, LeafResult, TaskStatus};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The execution mode for a leaf task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Design mode — generate design artifacts only
    Design,
    /// Inspect mode — read-only analysis
    Inspect,
    /// Shell mode — execute shell commands
    Shell,
    /// Claude mode — delegate to Claude for execution
    Claude,
    /// Plan mode — generate execution plan without running
    Plan,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Design => write!(f, "Design"),
            ExecutionMode::Inspect => write!(f, "Inspect"),
            ExecutionMode::Shell => write!(f, "Shell"),
            ExecutionMode::Claude => write!(f, "Claude"),
            ExecutionMode::Plan => write!(f, "Plan"),
        }
    }
}

/// Async trait for executing leaf tasks.
#[async_trait]
pub trait Executor: Send + Sync {
    /// Execute a single leaf task and return the result.
    async fn execute(&self, task: &LeafTask) -> Result<LeafResult>;
}

/// A dry-run executor that always succeeds without doing real work.
///
/// Useful for testing and plan-mode runs.
pub struct DryRunExecutor;

#[async_trait]
impl Executor for DryRunExecutor {
    async fn execute(&self, task: &LeafTask) -> Result<LeafResult> {
        Ok(LeafResult {
            task_id: task.id,
            status: TaskStatus::Completed,
            output: format!(
                "[dry-run] Would execute '{}' on node '{}' in {:?} mode",
                task.intent, task.node_id, task.mode
            ),
            artifacts: vec![],
            duration_ms: 0,
        })
    }
}
