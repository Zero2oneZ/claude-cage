//! Violation event stream and FAFO escalation.
//!
//! Captures sandbox violations (denied syscalls, blocked network, exceeded
//! resources, denied paths, capability violations) and escalates them to
//! the FAFO security system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The type of sandbox violation that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// Agent attempted a syscall not on its allowlist.
    SyscallDenied,
    /// Agent attempted a network connection that was blocked.
    NetworkBlocked,
    /// Agent exceeded a resource limit (memory, CPU, PIDs, FDs).
    ResourceExceeded,
    /// Agent attempted to access a denied filesystem path.
    PathDenied,
    /// Agent attempted to use a dropped capability.
    CapabilityDenied,
}

/// Severity level of a violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational, likely benign (e.g. transient resource spike).
    Low,
    /// Suspicious but not immediately dangerous.
    Medium,
    /// Active attempt to escape sandbox or abuse resources.
    High,
    /// Confirmed malicious behavior requiring immediate response.
    Critical,
}

/// A recorded sandbox violation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// The agent that caused the violation.
    pub agent_id: String,
    /// What type of violation occurred.
    pub violation_type: ViolationType,
    /// Human-readable detail about the violation.
    pub detail: String,
    /// When the violation occurred.
    pub timestamp: DateTime<Utc>,
    /// How severe this violation is.
    pub severity: ViolationSeverity,
}

/// A strike to be forwarded to the FAFO security system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FafoStrike {
    /// The agent that earned this strike.
    pub agent_id: String,
    /// Why the strike was issued.
    pub strike_reason: String,
    /// Severity determines which FAFO ladder rung is invoked.
    pub severity: ViolationSeverity,
}

/// Escalate a sandbox violation to a FAFO strike.
///
/// Maps the violation to a strike with an appropriate reason string
/// for the FAFO response ladder (TARPIT -> POISON -> DROWN -> DESTROY).
pub fn escalate_to_fafo(violation: &Violation) -> FafoStrike {
    let reason = match violation.violation_type {
        ViolationType::SyscallDenied => {
            format!(
                "SANDBOX: Denied syscall from agent '{}': {}",
                violation.agent_id, violation.detail
            )
        }
        ViolationType::NetworkBlocked => {
            format!(
                "SANDBOX: Blocked network access from agent '{}': {}",
                violation.agent_id, violation.detail
            )
        }
        ViolationType::ResourceExceeded => {
            format!(
                "SANDBOX: Resource limit exceeded by agent '{}': {}",
                violation.agent_id, violation.detail
            )
        }
        ViolationType::PathDenied => {
            format!(
                "SANDBOX: Denied path access from agent '{}': {}",
                violation.agent_id, violation.detail
            )
        }
        ViolationType::CapabilityDenied => {
            format!(
                "SANDBOX: Capability violation from agent '{}': {}",
                violation.agent_id, violation.detail
            )
        }
    };

    FafoStrike {
        agent_id: violation.agent_id.clone(),
        strike_reason: reason,
        severity: violation.severity,
    }
}
