//! AppArmor profile generation for agent isolation.
//!
//! Generates per-agent AppArmor profiles that restrict filesystem access
//! based on the SandboxPolicy.

use serde::{Deserialize, Serialize};

use crate::SandboxPolicy;

/// A generated AppArmor profile for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppArmorProfile {
    /// Profile name (typically "gently-agent-<agent_id>").
    pub name: String,
    /// Filesystem path rules.
    pub paths: Vec<PathRule>,
}

/// A single filesystem path rule within an AppArmor profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRule {
    /// Filesystem path or glob pattern.
    pub path: String,
    /// Permission string: "r", "rw", "rwx", or "deny".
    pub permissions: String,
}

/// Generate an AppArmor profile from a SandboxPolicy for the given agent.
pub fn generate_profile(agent_id: &str, policy: &SandboxPolicy) -> AppArmorProfile {
    let mut paths = Vec::new();

    // Always deny sensitive paths first
    paths.push(PathRule {
        path: "/etc/shadow".to_string(),
        permissions: "deny".to_string(),
    });
    paths.push(PathRule {
        path: "/etc/passwd".to_string(),
        permissions: "deny".to_string(),
    });
    paths.push(PathRule {
        path: "/proc/*/mem".to_string(),
        permissions: "deny".to_string(),
    });

    // Read-only system paths
    paths.push(PathRule {
        path: "/usr/**".to_string(),
        permissions: "r".to_string(),
    });
    paths.push(PathRule {
        path: "/lib/**".to_string(),
        permissions: "r".to_string(),
    });
    paths.push(PathRule {
        path: "/etc/**".to_string(),
        permissions: "r".to_string(),
    });

    // Agent-specific working directory (read-write)
    paths.push(PathRule {
        path: format!("/tmp/gently-agents/{}/**", agent_id),
        permissions: "rw".to_string(),
    });

    // Add policy-specified paths as read-only
    for allowed in &policy.allowed_paths {
        let path_str = allowed.to_string_lossy().to_string();
        // Avoid duplicating paths already added
        if !paths.iter().any(|p| p.path.starts_with(&path_str)) {
            paths.push(PathRule {
                path: format!("{}/**", path_str),
                permissions: "r".to_string(),
            });
        }
    }

    AppArmorProfile {
        name: format!("gently-agent-{}", agent_id),
        paths,
    }
}

/// Convert an AppArmorProfile into the text format that AppArmor expects.
pub fn to_profile_string(profile: &AppArmorProfile) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "#include <tunables/global>\n\nprofile {} flags=(attach_disconnected) {{\n",
        profile.name
    ));
    out.push_str("  #include <abstractions/base>\n\n");

    // Deny rules first
    for rule in &profile.paths {
        if rule.permissions == "deny" {
            out.push_str(&format!("  deny {} rw,\n", rule.path));
        }
    }

    out.push('\n');

    // Allow rules
    for rule in &profile.paths {
        if rule.permissions != "deny" {
            out.push_str(&format!("  {} {},\n", rule.path, rule.permissions));
        }
    }

    out.push_str("}\n");
    out
}
