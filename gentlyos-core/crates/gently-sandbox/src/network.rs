//! Network filtering for sandboxed agents.
//!
//! Generates iptables rules that restrict agent network access to only
//! explicitly allowed hosts and ports.

use serde::{Deserialize, Serialize};

/// Network access policy for a sandboxed agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Whether outbound connections are allowed at all.
    pub allow_outbound: bool,
    /// Hosts the agent may connect to (IP addresses or hostnames).
    pub allowed_hosts: Vec<String>,
    /// Destination ports the agent may connect to.
    pub allowed_ports: Vec<u16>,
    /// Hosts that are always blocked regardless of other rules.
    pub blocked_hosts: Vec<String>,
}

/// Return the default network policy for an Ollama agent.
///
/// By default, agents may only reach localhost (for Ollama API at 11434).
/// All other outbound traffic is denied.
pub fn default_agent_network() -> NetworkPolicy {
    NetworkPolicy {
        allow_outbound: true,
        allowed_hosts: vec!["127.0.0.1".to_string(), "::1".to_string()],
        allowed_ports: vec![11434], // Ollama default port
        blocked_hosts: vec![],
    }
}

/// Generate iptables rules that enforce the given network policy for an agent.
///
/// Rules are returned as a Vec of iptables command strings. The agent is
/// identified by a dedicated chain name derived from its agent_id.
pub fn to_iptables_rules(agent_id: &str, policy: &NetworkPolicy) -> Vec<String> {
    let chain = format!("GENTLY-{}", agent_id.to_uppercase().replace('-', "_"));
    let mut rules = Vec::new();

    // Create a dedicated chain for this agent
    rules.push(format!("iptables -N {}", chain));

    // Block explicitly denied hosts first
    for host in &policy.blocked_hosts {
        rules.push(format!(
            "iptables -A {} -d {} -j DROP",
            chain, host
        ));
    }

    if !policy.allow_outbound {
        // Drop all outbound if not allowed
        rules.push(format!(
            "iptables -A {} -j DROP",
            chain
        ));
        return rules;
    }

    // Allow established/related connections (return traffic)
    rules.push(format!(
        "iptables -A {} -m state --state ESTABLISHED,RELATED -j ACCEPT",
        chain
    ));

    // Allow traffic to each permitted host on permitted ports
    for host in &policy.allowed_hosts {
        for port in &policy.allowed_ports {
            rules.push(format!(
                "iptables -A {} -d {} -p tcp --dport {} -j ACCEPT",
                chain, host, port
            ));
        }
    }

    // Allow loopback (always)
    rules.push(format!(
        "iptables -A {} -o lo -j ACCEPT",
        chain
    ));

    // Drop everything else
    rules.push(format!(
        "iptables -A {} -j DROP",
        chain
    ));

    rules
}
