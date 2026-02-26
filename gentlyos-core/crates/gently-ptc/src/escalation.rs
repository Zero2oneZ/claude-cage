//! Escalation cascade for PTC tasks.
//!
//! When a task encounters issues, it can escalate through levels.
//! Critical and Emergency levels trigger a halt.

use serde::{Deserialize, Serialize};

/// Escalation severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EscalationLevel {
    /// Informational — no action needed
    Info,
    /// Warning — attention recommended
    Warning,
    /// Error — task failed, retry possible
    Error,
    /// Critical — halt required, human review
    Critical,
    /// Emergency — full stop, scorched earth
    Emergency,
}

impl EscalationLevel {
    /// Numeric severity (0-4).
    pub fn severity(&self) -> u8 {
        match self {
            EscalationLevel::Info => 0,
            EscalationLevel::Warning => 1,
            EscalationLevel::Error => 2,
            EscalationLevel::Critical => 3,
            EscalationLevel::Emergency => 4,
        }
    }
}

impl std::fmt::Display for EscalationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EscalationLevel::Info => write!(f, "INFO"),
            EscalationLevel::Warning => write!(f, "WARNING"),
            EscalationLevel::Error => write!(f, "ERROR"),
            EscalationLevel::Critical => write!(f, "CRITICAL"),
            EscalationLevel::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

/// Escalate from the current level to the next one up.
///
/// Returns the bumped level. If already at Emergency, stays at Emergency.
pub fn escalate(current: EscalationLevel, reason: &str) -> EscalationLevel {
    let _ = reason; // logged by caller
    match current {
        EscalationLevel::Info => EscalationLevel::Warning,
        EscalationLevel::Warning => EscalationLevel::Error,
        EscalationLevel::Error => EscalationLevel::Critical,
        EscalationLevel::Critical => EscalationLevel::Emergency,
        EscalationLevel::Emergency => EscalationLevel::Emergency,
    }
}

/// Check if the escalation level requires a halt.
///
/// Returns true for Critical and Emergency levels.
pub fn should_halt(level: EscalationLevel) -> bool {
    matches!(level, EscalationLevel::Critical | EscalationLevel::Emergency)
}
