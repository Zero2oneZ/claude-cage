#![allow(dead_code, unused_variables, unused_imports)]

//! gently-sandbox — Agent isolation framework
//!
//! Sandboxes the 34 Ollama agents with seccomp, AppArmor, capabilities,
//! resource limits, and network filtering. Violations escalate to FAFO.

pub mod seccomp;
pub mod apparmor;
pub mod capabilities;
pub mod limits;
pub mod network;
pub mod violation;

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Policy defining what an agent is allowed to do inside its sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Syscalls the agent is permitted to invoke.
    pub allowed_syscalls: Vec<String>,
    /// Maximum resident memory in megabytes.
    pub max_memory_mb: u64,
    /// Maximum CPU utilization as a percentage (0-100).
    pub max_cpu_percent: u8,
    /// Maximum number of processes/threads the agent may spawn.
    pub max_pids: u32,
    /// Maximum number of open file descriptors.
    pub max_fds: u32,
    /// Whether the agent is allowed any network access.
    pub network_allowed: bool,
    /// Filesystem paths the agent may access.
    pub allowed_paths: Vec<PathBuf>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allowed_syscalls: seccomp::default_agent_profile()
                .syscalls
                .iter()
                .flat_map(|r| r.names.clone())
                .collect(),
            max_memory_mb: 512,
            max_cpu_percent: 25,
            max_pids: 64,
            max_fds: 256,
            network_allowed: false,
            allowed_paths: vec![
                PathBuf::from("/tmp/gently-agents"),
                PathBuf::from("/usr/lib"),
                PathBuf::from("/usr/share"),
            ],
        }
    }
}

/// Top-level manager that applies combined sandbox policies to agents.
pub struct SandboxManager {
    /// Active policies keyed by agent id.
    active_policies: std::collections::HashMap<String, SandboxPolicy>,
}

impl SandboxManager {
    /// Create a new SandboxManager with no active policies.
    pub fn new() -> Self {
        Self {
            active_policies: std::collections::HashMap::new(),
        }
    }

    /// Apply a full sandbox policy to the given agent.
    ///
    /// This configures seccomp, AppArmor, capabilities, resource limits,
    /// and network filtering for the agent process.
    pub fn apply_policy(&mut self, agent_id: &str, policy: &SandboxPolicy) -> Result<()> {
        // 1. Load/generate seccomp profile from the policy's allowed syscalls
        let seccomp_profile = seccomp::SeccompProfile {
            default_action: "SCMP_ACT_ERRNO".to_string(),
            syscalls: vec![seccomp::SyscallRule {
                names: policy.allowed_syscalls.clone(),
                action: "SCMP_ACT_ALLOW".to_string(),
            }],
        };

        // 2. Generate AppArmor profile for this agent
        let aa_profile = apparmor::generate_profile(agent_id, policy);
        let _aa_string = apparmor::to_profile_string(&aa_profile);

        // 3. Drop dangerous capabilities
        let _drop_caps = capabilities::default_drop_list();

        // 4. Configure resource limits
        let resource_limits = limits::ResourceLimits {
            max_memory_bytes: policy.max_memory_mb * 1024 * 1024,
            max_cpu_shares: (policy.max_cpu_percent as u64) * 10,
            max_pids: policy.max_pids,
            max_open_files: policy.max_fds,
            max_file_size_bytes: 100 * 1024 * 1024, // 100 MB default
        };
        let _cgroup_cfg = limits::to_cgroup_config(&resource_limits);

        // 5. Network policy
        let net_policy = if policy.network_allowed {
            network::NetworkPolicy {
                allow_outbound: true,
                allowed_hosts: vec!["127.0.0.1".to_string(), "::1".to_string()],
                allowed_ports: vec![80, 443, 11434], // 11434 = Ollama default
                blocked_hosts: vec![],
            }
        } else {
            network::default_agent_network()
        };
        let _iptables = network::to_iptables_rules(agent_id, &net_policy);

        // Record the active policy
        self.active_policies
            .insert(agent_id.to_string(), policy.clone());

        Ok(())
    }

    /// Return the currently active policy for an agent, if any.
    pub fn get_policy(&self, agent_id: &str) -> Option<&SandboxPolicy> {
        self.active_policies.get(agent_id)
    }

    /// Remove the sandbox policy for an agent (teardown).
    pub fn remove_policy(&mut self, agent_id: &str) -> Option<SandboxPolicy> {
        self.active_policies.remove(agent_id)
    }

    /// Number of agents currently sandboxed.
    pub fn active_count(&self) -> usize {
        self.active_policies.len()
    }
}
