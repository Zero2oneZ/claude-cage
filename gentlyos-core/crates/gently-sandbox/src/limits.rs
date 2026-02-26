//! Resource limits for sandboxed agents.
//!
//! Enforces memory, CPU, PID, file descriptor, and file size limits
//! using cgroup v2 configuration.

use serde::{Deserialize, Serialize};

/// Resource limits applied to a sandboxed agent process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// CPU shares (cgroup v2 cpu.weight). Higher = more CPU time.
    pub max_cpu_shares: u64,
    /// Maximum number of PIDs (processes + threads).
    pub max_pids: u32,
    /// Maximum number of open file descriptors.
    pub max_open_files: u32,
    /// Maximum size of any single file written by the agent, in bytes.
    pub max_file_size_bytes: u64,
}

/// Return sensible default resource limits for an Ollama agent.
///
/// - 512 MB RAM
/// - 1024 CPU shares (normal weight)
/// - 64 PIDs
/// - 256 file descriptors
/// - 100 MB max file size
pub fn default_agent_limits() -> ResourceLimits {
    ResourceLimits {
        max_memory_bytes: 512 * 1024 * 1024,       // 512 MB
        max_cpu_shares: 1024,                        // normal weight
        max_pids: 64,
        max_open_files: 256,
        max_file_size_bytes: 100 * 1024 * 1024,     // 100 MB
    }
}

/// Generate a cgroup v2 configuration string from the given limits.
///
/// This is a placeholder that produces a human-readable representation
/// of the cgroup settings. In production, these values would be written
/// to the appropriate cgroup filesystem paths.
pub fn to_cgroup_config(limits: &ResourceLimits) -> String {
    let mut config = String::new();

    // memory.max
    config.push_str(&format!(
        "memory.max = {}\n",
        limits.max_memory_bytes
    ));

    // cpu.weight (cgroup v2 uses weight 1-10000, default 100)
    config.push_str(&format!(
        "cpu.weight = {}\n",
        limits.max_cpu_shares
    ));

    // pids.max
    config.push_str(&format!(
        "pids.max = {}\n",
        limits.max_pids
    ));

    // RLIMIT_NOFILE (set via setrlimit, not cgroup — noted here for completeness)
    config.push_str(&format!(
        "# rlimit.nofile = {}\n",
        limits.max_open_files
    ));

    // RLIMIT_FSIZE
    config.push_str(&format!(
        "# rlimit.fsize = {}\n",
        limits.max_file_size_bytes
    ));

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let limits = default_agent_limits();

        // Memory: 512 MB
        assert_eq!(limits.max_memory_bytes, 512 * 1024 * 1024);

        // CPU shares: standard weight
        assert!(
            limits.max_cpu_shares > 0 && limits.max_cpu_shares <= 10000,
            "CPU shares {} out of cgroup v2 range",
            limits.max_cpu_shares
        );

        // PIDs: reasonable for an agent (not 0, not unlimited)
        assert!(limits.max_pids > 0 && limits.max_pids <= 1024);

        // File descriptors: reasonable range
        assert!(limits.max_open_files > 0 && limits.max_open_files <= 4096);

        // File size: 100 MB
        assert_eq!(limits.max_file_size_bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn cgroup_config_contains_all_fields() {
        let limits = default_agent_limits();
        let config = to_cgroup_config(&limits);

        assert!(config.contains("memory.max"));
        assert!(config.contains("cpu.weight"));
        assert!(config.contains("pids.max"));
        assert!(config.contains("rlimit.nofile"));
        assert!(config.contains("rlimit.fsize"));
    }
}
