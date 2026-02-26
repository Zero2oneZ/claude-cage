//! 7-phase PTC pipeline.
//!
//! Every PTC run progresses through these phases:
//! Intake -> Triage -> Plan -> Execute -> Verify -> Integrate -> Ship

use serde::{Deserialize, Serialize};

/// The 7 phases of a PTC run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Receive and validate the intent
    Intake,
    /// Route intent to tree nodes
    Triage,
    /// Generate leaf task plan
    Plan,
    /// Execute leaf tasks
    Execute,
    /// Verify results and quality
    Verify,
    /// Integrate results up the tree
    Integrate,
    /// Ship — final delivery
    Ship,
}

impl Phase {
    /// Advance to the next phase. Returns None if already at Ship.
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Intake => Some(Phase::Triage),
            Phase::Triage => Some(Phase::Plan),
            Phase::Plan => Some(Phase::Execute),
            Phase::Execute => Some(Phase::Verify),
            Phase::Verify => Some(Phase::Integrate),
            Phase::Integrate => Some(Phase::Ship),
            Phase::Ship => None,
        }
    }

    /// Check if this is a terminal phase (Ship).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Phase::Ship)
    }

    /// Return the phase index (0-6).
    pub fn index(&self) -> usize {
        match self {
            Phase::Intake => 0,
            Phase::Triage => 1,
            Phase::Plan => 2,
            Phase::Execute => 3,
            Phase::Verify => 4,
            Phase::Integrate => 5,
            Phase::Ship => 6,
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Intake => write!(f, "Intake"),
            Phase::Triage => write!(f, "Triage"),
            Phase::Plan => write!(f, "Plan"),
            Phase::Execute => write!(f, "Execute"),
            Phase::Verify => write!(f, "Verify"),
            Phase::Integrate => write!(f, "Integrate"),
            Phase::Ship => write!(f, "Ship"),
        }
    }
}
