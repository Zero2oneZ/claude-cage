//! Seccomp syscall allowlist management.
//!
//! Provides a hardcoded safe-syscall profile for Ollama agent isolation
//! and utilities for loading/serializing custom profiles.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A complete seccomp profile describing the default action and per-syscall overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompProfile {
    /// Action taken when a syscall is not explicitly listed (e.g. "SCMP_ACT_ERRNO").
    pub default_action: String,
    /// Rules that override the default action for specific syscalls.
    pub syscalls: Vec<SyscallRule>,
}

/// A single seccomp rule mapping a set of syscall names to an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallRule {
    /// Syscall names this rule applies to (e.g. ["read", "write"]).
    pub names: Vec<String>,
    /// Action to take (e.g. "SCMP_ACT_ALLOW").
    pub action: String,
}

/// Load a seccomp profile from a JSON file on disk.
pub fn load_profile(path: &Path) -> Result<SeccompProfile> {
    let contents = std::fs::read_to_string(path)?;
    let profile: SeccompProfile = serde_json::from_str(&contents)?;
    Ok(profile)
}

/// Return the default agent seccomp profile with ~30 safe syscalls allowed.
///
/// Everything not on this list is denied with SCMP_ACT_ERRNO.
pub fn default_agent_profile() -> SeccompProfile {
    SeccompProfile {
        default_action: "SCMP_ACT_ERRNO".to_string(),
        syscalls: vec![SyscallRule {
            names: vec![
                "read".into(),
                "write".into(),
                "openat".into(),
                "close".into(),
                "stat".into(),
                "fstat".into(),
                "lseek".into(),
                "mmap".into(),
                "mprotect".into(),
                "munmap".into(),
                "brk".into(),
                "ioctl".into(),
                "access".into(),
                "pipe".into(),
                "dup2".into(),
                "fork".into(),
                "execve".into(),
                "exit_group".into(),
                "wait4".into(),
                "fcntl".into(),
                "getpid".into(),
                "getuid".into(),
                "getgid".into(),
                "gettid".into(),
                "clock_gettime".into(),
                "nanosleep".into(),
                "poll".into(),
                "epoll_create".into(),
                "epoll_wait".into(),
                "epoll_ctl".into(),
                "socket".into(),
                "connect".into(),
                "sendto".into(),
                "recvfrom".into(),
            ],
            action: "SCMP_ACT_ALLOW".to_string(),
        }],
    }
}

/// Serialize a seccomp profile to a JSON string.
pub fn serialize_profile(profile: &SeccompProfile) -> Result<String> {
    let json = serde_json::to_string_pretty(profile)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_has_syscalls() {
        let profile = default_agent_profile();
        assert_eq!(profile.default_action, "SCMP_ACT_ERRNO");
        assert!(!profile.syscalls.is_empty(), "must have at least one rule");

        let allowed: &Vec<String> = &profile.syscalls[0].names;
        assert!(
            allowed.len() >= 30,
            "expected >= 30 safe syscalls, got {}",
            allowed.len()
        );
        assert!(allowed.contains(&"read".to_string()));
        assert!(allowed.contains(&"write".to_string()));
        assert!(allowed.contains(&"mmap".to_string()));
        assert!(allowed.contains(&"exit_group".to_string()));
    }

    #[test]
    fn serialize_roundtrip() {
        let profile = default_agent_profile();
        let json = serialize_profile(&profile).unwrap();
        let deserialized: SeccompProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.default_action, profile.default_action);
        assert_eq!(
            deserialized.syscalls[0].names.len(),
            profile.syscalls[0].names.len()
        );
    }
}
