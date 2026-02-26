//! Task and LeafResult structs.
//!
//! Defines the leaf task that gets executed and the result that comes back.

use crate::executor::ExecutionMode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a task through its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Not yet started
    Pending,
    /// Currently executing
    Running,
    /// Finished successfully
    Completed,
    /// Finished with error
    Failed,
    /// Blocked by dependency or rule
    Blocked,
    /// Escalated to higher authority
    Escalated,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
            TaskStatus::Blocked => write!(f, "Blocked"),
            TaskStatus::Escalated => write!(f, "Escalated"),
        }
    }
}

/// A leaf task to be executed by an Executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafTask {
    /// Unique task identifier
    pub id: Uuid,
    /// The tree node this task targets
    pub node_id: String,
    /// The intent/description for this task
    pub intent: String,
    /// How this task should be executed
    pub mode: ExecutionMode,
    /// Additional context from the tree node metadata
    pub context: serde_json::Value,
}

/// The result of executing a leaf task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafResult {
    /// The task ID this result corresponds to
    pub task_id: Uuid,
    /// Final status of the task
    pub status: TaskStatus,
    /// Human-readable output or error message
    pub output: String,
    /// Paths or identifiers of artifacts produced
    pub artifacts: Vec<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_leaf_task() {
        let task = LeafTask {
            id: Uuid::new_v4(),
            node_id: "security-fafo".to_string(),
            intent: "Check FAFO defense status".to_string(),
            mode: ExecutionMode::Inspect,
            context: serde_json::json!({ "priority": "high" }),
        };

        assert_eq!(task.node_id, "security-fafo");
        assert_eq!(task.intent, "Check FAFO defense status");
        assert_eq!(task.mode, ExecutionMode::Inspect);
        assert!(task.context.get("priority").is_some());
    }

    #[test]
    fn test_create_leaf_result() {
        let task_id = Uuid::new_v4();
        let result = LeafResult {
            task_id,
            status: TaskStatus::Completed,
            output: "FAFO defense is active".to_string(),
            artifacts: vec!["report.json".to_string()],
            duration_ms: 42,
        };

        assert_eq!(result.task_id, task_id);
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.output, "FAFO defense is active");
        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(result.duration_ms, 42);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(format!("{}", TaskStatus::Pending), "Pending");
        assert_eq!(format!("{}", TaskStatus::Running), "Running");
        assert_eq!(format!("{}", TaskStatus::Completed), "Completed");
        assert_eq!(format!("{}", TaskStatus::Failed), "Failed");
        assert_eq!(format!("{}", TaskStatus::Blocked), "Blocked");
        assert_eq!(format!("{}", TaskStatus::Escalated), "Escalated");
    }

    #[test]
    fn test_task_serialization_roundtrip() {
        let task = LeafTask {
            id: Uuid::new_v4(),
            node_id: "test-node".to_string(),
            intent: "test intent".to_string(),
            mode: ExecutionMode::Design,
            context: serde_json::json!(null),
        };

        let json = serde_json::to_string(&task).expect("serialize");
        let deserialized: LeafTask = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.node_id, task.node_id);
        assert_eq!(deserialized.mode, ExecutionMode::Design);
    }

    #[test]
    fn test_result_serialization_roundtrip() {
        let result = LeafResult {
            task_id: Uuid::new_v4(),
            status: TaskStatus::Failed,
            output: "something went wrong".to_string(),
            artifacts: vec![],
            duration_ms: 100,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: LeafResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.status, TaskStatus::Failed);
        assert_eq!(deserialized.duration_ms, 100);
    }
}
