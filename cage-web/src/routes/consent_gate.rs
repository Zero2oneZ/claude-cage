//! Consent Gate -- `/consent-gate`
//!
//! Tier-gated approval workflow. Shows pending consent requests,
//! recent denials, and the approval cascade. Users see what actions
//! need approval and which tier can grant it.

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data Model
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl GateStatus {
    fn class(self) -> &'static str {
        match self {
            Self::Pending => "gate-pending",
            Self::Approved => "gate-approved",
            Self::Denied => "gate-denied",
            Self::Expired => "gate-expired",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Denied => "DENIED",
            Self::Expired => "EXPIRED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn class(self) -> &'static str {
        match self {
            Self::Low => "risk-low",
            Self::Medium => "risk-medium",
            Self::High => "risk-high",
            Self::Critical => "risk-critical",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
    fn score(self) -> u8 {
        match self {
            Self::Low => 2,
            Self::Medium => 5,
            Self::High => 7,
            Self::Critical => 9,
        }
    }
}

struct ConsentRequest {
    id: &'static str,
    action: &'static str,
    description: &'static str,
    requester: &'static str,
    required_tier: Layer,
    risk: RiskLevel,
    status: GateStatus,
    timestamp: &'static str,
    cascade_path: &'static [&'static str],
}

struct CascadeRule {
    risk_range: &'static str,
    approver: &'static str,
    tier: &'static str,
    example: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static REQUESTS: &[ConsentRequest] = &[
    ConsentRequest {
        id: "CG-001",
        action: "docker.container.create",
        description: "Create new sandboxed container with filtered network",
        requester: "session-manager",
        required_tier: Layer::OsAdmin,
        risk: RiskLevel::Medium,
        status: GateStatus::Approved,
        timestamp: "2026-02-10 14:23:01",
        cascade_path: &["Captain", "Director"],
    },
    ConsentRequest {
        id: "CG-002",
        action: "network.filter.modify",
        description: "Add api.openai.com to allowed hosts whitelist",
        requester: "guarddog-dns",
        required_tier: Layer::OsAdmin,
        risk: RiskLevel::High,
        status: GateStatus::Pending,
        timestamp: "2026-02-10 14:31:15",
        cascade_path: &["Captain", "Director", "CTO"],
    },
    ConsentRequest {
        id: "CG-003",
        action: "env.vault.read",
        description: "Read MONGODB_URI from encrypted vault",
        requester: "mongo-analyst",
        required_tier: Layer::RootUser,
        risk: RiskLevel::Low,
        status: GateStatus::Approved,
        timestamp: "2026-02-10 14:12:44",
        cascade_path: &["Captain"],
    },
    ConsentRequest {
        id: "CG-004",
        action: "agent.swarm.spawn",
        description: "Spawn 8-agent parallel analysis swarm",
        requester: "orchestrator",
        required_tier: Layer::OsAdmin,
        risk: RiskLevel::High,
        status: GateStatus::Denied,
        timestamp: "2026-02-10 13:55:20",
        cascade_path: &["Captain", "Director", "CTO"],
    },
    ConsentRequest {
        id: "CG-005",
        action: "offensive.tools.enable",
        description: "Enable penetration testing toolkit in container",
        requester: "security-auditor",
        required_tier: Layer::DevLevel,
        risk: RiskLevel::Critical,
        status: GateStatus::Pending,
        timestamp: "2026-02-10 14:35:00",
        cascade_path: &["Captain", "Director", "CTO", "Human"],
    },
    ConsentRequest {
        id: "CG-006",
        action: "shell.exec.arbitrary",
        description: "Execute user-provided shell command in sandbox",
        requester: "workbench",
        required_tier: Layer::RootUser,
        risk: RiskLevel::Medium,
        status: GateStatus::Approved,
        timestamp: "2026-02-10 14:28:33",
        cascade_path: &["Captain", "Director"],
    },
    ConsentRequest {
        id: "CG-007",
        action: "cookie.export.bulk",
        description: "Export all cookie data to external JSON",
        requester: "cookie-jar",
        required_tier: Layer::OsAdmin,
        risk: RiskLevel::Medium,
        status: GateStatus::Expired,
        timestamp: "2026-02-10 12:01:10",
        cascade_path: &["Captain", "Director"],
    },
    ConsentRequest {
        id: "CG-008",
        action: "breach.simulate",
        description: "Run breach simulation on honeypot layer",
        requester: "cookie-jar",
        required_tier: Layer::DevLevel,
        risk: RiskLevel::Critical,
        status: GateStatus::Pending,
        timestamp: "2026-02-10 14:40:22",
        cascade_path: &["Captain", "Director", "CTO", "Human"],
    },
];

static CASCADE_RULES: &[CascadeRule] = &[
    CascadeRule { risk_range: "1-3", approver: "Captain", tier: "Any", example: "env.vault.read, file.read" },
    CascadeRule { risk_range: "4-6", approver: "Director", tier: "RootUser+", example: "docker.create, shell.exec" },
    CascadeRule { risk_range: "7-8", approver: "CTO", tier: "OsAdmin+", example: "network.modify, agent.swarm" },
    CascadeRule { risk_range: "9-10", approver: "Human", tier: "DevLevel+", example: "offensive.enable, breach.sim" },
];

// ---------------------------------------------------------------
//  Template Data
// ---------------------------------------------------------------

struct RequestView {
    id: String,
    action: String,
    description: String,
    requester: String,
    required_tier: String,
    risk_class: String,
    risk_label: String,
    risk_score: u8,
    status_class: String,
    status_label: String,
    timestamp: String,
    cascade_path: String,
    can_approve: bool,
}

struct CascadeView {
    risk_range: String,
    approver: String,
    tier: String,
    example: String,
}

#[derive(Template)]
#[template(path = "consent_gate.html")]
struct ConsentGateTemplate {
    layer_label: String,
    layer_badge: String,
    requests: Vec<RequestView>,
    cascade_rules: Vec<CascadeView>,
    total_requests: usize,
    pending_count: usize,
    approved_count: usize,
    denied_count: usize,
    can_approve_medium: bool,
    can_approve_high: bool,
    can_approve_critical: bool,
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/consent-gate", get(consent_gate_page))
}

async fn consent_gate_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let requests: Vec<RequestView> = REQUESTS
        .iter()
        .map(|r| {
            let can_approve = layer.has_access(r.required_tier) && r.status == GateStatus::Pending;
            RequestView {
                id: r.id.to_string(),
                action: r.action.to_string(),
                description: r.description.to_string(),
                requester: r.requester.to_string(),
                required_tier: format!("{:?}", r.required_tier),
                risk_class: r.risk.class().to_string(),
                risk_label: r.risk.label().to_string(),
                risk_score: r.risk.score(),
                status_class: r.status.class().to_string(),
                status_label: r.status.label().to_string(),
                timestamp: r.timestamp.to_string(),
                cascade_path: r.cascade_path.join(" -> "),
                can_approve,
            }
        })
        .collect();

    let cascade_rules: Vec<CascadeView> = CASCADE_RULES
        .iter()
        .map(|c| CascadeView {
            risk_range: c.risk_range.to_string(),
            approver: c.approver.to_string(),
            tier: c.tier.to_string(),
            example: c.example.to_string(),
        })
        .collect();

    let pending_count = REQUESTS.iter().filter(|r| r.status == GateStatus::Pending).count();
    let approved_count = REQUESTS.iter().filter(|r| r.status == GateStatus::Approved).count();
    let denied_count = REQUESTS.iter().filter(|r| r.status == GateStatus::Denied).count();

    let content = ConsentGateTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        requests,
        cascade_rules,
        total_requests: REQUESTS.len(),
        pending_count,
        approved_count,
        denied_count,
        can_approve_medium: layer.has_access(Layer::RootUser),
        can_approve_high: layer.has_access(Layer::OsAdmin),
        can_approve_critical: layer.has_access(Layer::DevLevel),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Consent Gate", &content))
    }
}
