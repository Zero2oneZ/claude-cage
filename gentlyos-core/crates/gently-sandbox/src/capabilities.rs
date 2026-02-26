//! Linux capability management for agent sandboxing.
//!
//! Lists dangerous capabilities that must be dropped from agent processes
//! to prevent privilege escalation.

use serde::{Deserialize, Serialize};

/// Linux capabilities that can be granted or dropped from a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Send raw network packets (CAP_NET_RAW).
    NetRaw,
    /// Bypass all kernel permission checks (CAP_SYS_ADMIN).
    SysAdmin,
    /// Trace arbitrary processes (CAP_SYS_PTRACE).
    SysPtrace,
    /// Network administration (CAP_NET_ADMIN).
    NetAdmin,
    /// Change file ownership (CAP_CHOWN).
    Chown,
    /// Bypass file read/write/execute permission checks (CAP_DAC_OVERRIDE).
    DacOverride,
    /// Bypass permission checks on operations that normally require
    /// the filesystem UID of the process to match the file UID (CAP_FOWNER).
    Fowner,
    /// Send signals to arbitrary processes (CAP_KILL).
    Kill,
    /// Change process GID (CAP_SETGID).
    SetGid,
    /// Change process UID (CAP_SETUID).
    SetUid,
}

/// Return the list of capabilities that MUST be dropped from every agent process.
///
/// Agents should run with minimal capabilities. All dangerous capabilities
/// listed here are dropped to prevent privilege escalation, raw network
/// manipulation, or process interference.
pub fn default_drop_list() -> Vec<Capability> {
    vec![
        Capability::NetRaw,
        Capability::SysAdmin,
        Capability::SysPtrace,
        Capability::NetAdmin,
        Capability::Chown,
        Capability::DacOverride,
        Capability::Fowner,
        Capability::Kill,
        Capability::SetGid,
        Capability::SetUid,
    ]
}

/// Map a Capability enum variant to its Linux kernel name string.
pub fn capability_name(cap: Capability) -> &'static str {
    match cap {
        Capability::NetRaw => "CAP_NET_RAW",
        Capability::SysAdmin => "CAP_SYS_ADMIN",
        Capability::SysPtrace => "CAP_SYS_PTRACE",
        Capability::NetAdmin => "CAP_NET_ADMIN",
        Capability::Chown => "CAP_CHOWN",
        Capability::DacOverride => "CAP_DAC_OVERRIDE",
        Capability::Fowner => "CAP_FOWNER",
        Capability::Kill => "CAP_KILL",
        Capability::SetGid => "CAP_SETGID",
        Capability::SetUid => "CAP_SETUID",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_list_is_non_empty() {
        let list = default_drop_list();
        assert!(
            !list.is_empty(),
            "drop list must contain dangerous capabilities"
        );
        // All 10 capabilities should be dropped
        assert_eq!(list.len(), 10);
    }

    #[test]
    fn all_dangerous_caps_are_dropped() {
        let list = default_drop_list();
        assert!(list.contains(&Capability::SysAdmin));
        assert!(list.contains(&Capability::SysPtrace));
        assert!(list.contains(&Capability::NetRaw));
        assert!(list.contains(&Capability::Kill));
    }

    #[test]
    fn capability_names_are_correct() {
        assert_eq!(capability_name(Capability::SysAdmin), "CAP_SYS_ADMIN");
        assert_eq!(capability_name(Capability::NetRaw), "CAP_NET_RAW");
        assert_eq!(capability_name(Capability::SetUid), "CAP_SETUID");
        assert_eq!(capability_name(Capability::DacOverride), "CAP_DAC_OVERRIDE");
    }
}
