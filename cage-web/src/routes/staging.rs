//! Staging -- `/staging`
//!
//! Non-destructive change management dashboard. Shows active changesets,
//! their blast radius, risk levels, approval status, and undo history.
//! Focal point partial shows the current predicted focus node.
//!
//! HTMX partials: /partials/focal-point, /partials/changesets

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;

use crate::routes::{html_escape, is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data -- Changeset
// ---------------------------------------------------------------

struct ChangesetRow {
    id: &'static str,
    title: &'static str,
    status: &'static str,
    status_shape: &'static str,
    status_class: &'static str,
    risk_level: u8,
    risk_class: &'static str,
    approval: &'static str,
    files_changed: u32,
    lines_added: u32,
    lines_removed: u32,
    blast_radius: Vec<&'static str>,
    test_status: &'static str,
    created_at: &'static str,
}

struct FocalPointData {
    primary_node: &'static str,
    primary_path: &'static str,
    primary_strength: f32,
    confidence: f32,
    stable_for: &'static str,
    tier_hot: &'static str,
    tier_warm: &'static str,
    secondary_nodes: Vec<(&'static str, f32)>,
    hot_path_prediction: &'static str,
    hot_path_confidence: f32,
}

struct UndoEntry {
    changeset_id: &'static str,
    title: &'static str,
    applied_at: &'static str,
    can_undo: bool,
    undo_class: &'static str,
}

// ---------------------------------------------------------------
//  Template
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "staging.html")]
struct StagingTemplate {
    focal: FocalPointDisplay,
    changesets: Vec<ChangesetDisplay>,
    undo_stack: Vec<UndoDisplay>,
    total_staged: usize,
    total_applied: usize,
    total_reverted: usize,
}

struct FocalPointDisplay {
    primary_node: String,
    primary_path: String,
    primary_strength: String,
    confidence: String,
    stable_for: String,
    tier_hot: String,
    tier_warm: String,
    secondary_nodes: Vec<(String, String)>,
    hot_path_prediction: String,
    hot_path_confidence: String,
}

struct ChangesetDisplay {
    id: String,
    title: String,
    status: String,
    status_shape: String,
    status_class: String,
    risk_level: u8,
    risk_class: String,
    approval: String,
    files_changed: u32,
    lines_added: u32,
    lines_removed: u32,
    blast_items: Vec<String>,
    test_status: String,
    created_at: String,
}

struct UndoDisplay {
    changeset_id: String,
    title: String,
    applied_at: String,
    can_undo: bool,
    undo_class: String,
}

// ---------------------------------------------------------------
//  Static mock data
// ---------------------------------------------------------------

fn mock_focal() -> FocalPointData {
    FocalPointData {
        primary_node: "gently-cookie-vault",
        primary_path: "crates/security/gently-cookie-vault/src/lib.rs",
        primary_strength: 0.92,
        confidence: 0.87,
        stable_for: "4m 12s",
        tier_hot: "ring_0 (self) + ring_1 (gently-crypto, gently-secrets)",
        tier_warm: "ring_2 (cage-web::cookie_jar) + ring_3 (gently-security)",
        secondary_nodes: vec![
            ("gently-crypto", 0.45),
            ("cage-web::cookie_jar", 0.31),
            ("gently-secrets", 0.22),
            ("gently-security", 0.15),
        ],
        hot_path_prediction: "focused:gently-cookie-vault -> focused:gently-crypto",
        hot_path_confidence: 0.73,
    }
}

fn mock_changesets() -> Vec<ChangesetRow> {
    vec![
        ChangesetRow {
            id: "cs-a1b2c3",
            title: "Add expiry rotation to CookieVault",
            status: "Staged",
            status_shape: "[~]",
            status_class: "cs-staged",
            risk_level: 3,
            risk_class: "risk-low",
            approval: "captain (auto-approve)",
            files_changed: 2,
            lines_added: 47,
            lines_removed: 12,
            blast_radius: vec!["gently-cookie-vault", "cage-web::cookie_jar"],
            test_status: "12 passed, 0 failed",
            created_at: "2026-02-24 14:32:01",
        },
        ChangesetRow {
            id: "cs-d4e5f6",
            title: "Refactor encrypt() signature in gently-crypto",
            status: "Testing",
            status_shape: "[<>]",
            status_class: "cs-testing",
            risk_level: 7,
            risk_class: "risk-high",
            approval: "CTO required",
            files_changed: 8,
            lines_added: 134,
            lines_removed: 89,
            blast_radius: vec![
                "gently-crypto",
                "gently-cookie-vault",
                "gently-secrets",
                "gently-security",
                "cage-web::cookie_jar",
            ],
            test_status: "running...",
            created_at: "2026-02-24 14:28:45",
        },
        ChangesetRow {
            id: "cs-789abc",
            title: "Add HTMX partial for focal point display",
            status: "Approved",
            status_shape: "[O]",
            status_class: "cs-approved",
            risk_level: 2,
            risk_class: "risk-low",
            approval: "captain (auto-approve)",
            files_changed: 3,
            lines_added: 85,
            lines_removed: 0,
            blast_radius: vec!["cage-web::gentlyos", "cage-web::staging"],
            test_status: "5 passed, 0 failed",
            created_at: "2026-02-24 14:15:22",
        },
        ChangesetRow {
            id: "cs-def012",
            title: "Fix consensus gate edge case in empty vault",
            status: "Applied",
            status_shape: "[+]",
            status_class: "cs-applied",
            risk_level: 4,
            risk_class: "risk-medium",
            approval: "director",
            files_changed: 1,
            lines_added: 8,
            lines_removed: 3,
            blast_radius: vec!["gently-security", "gently-init"],
            test_status: "28 passed, 0 failed",
            created_at: "2026-02-24 13:55:10",
        },
    ]
}

fn mock_undo_stack() -> Vec<UndoEntry> {
    vec![
        UndoEntry {
            changeset_id: "cs-def012",
            title: "Fix consensus gate edge case in empty vault",
            applied_at: "2026-02-24 14:01:33",
            can_undo: true,
            undo_class: "undo-available",
        },
        UndoEntry {
            changeset_id: "cs-old001",
            title: "Update gently-core Hash display impl",
            applied_at: "2026-02-24 12:44:00",
            can_undo: true,
            undo_class: "undo-available",
        },
        UndoEntry {
            changeset_id: "cs-old002",
            title: "Add tier auth middleware",
            applied_at: "2026-02-24 11:20:15",
            can_undo: false,
            undo_class: "undo-expired",
        },
    ]
}

// ---------------------------------------------------------------
//  Router
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/staging", get(staging_page))
        .route("/partials/focal-point", get(focal_point_partial))
        .route("/partials/changesets", get(changesets_partial))
        .route("/staging/{id}/approve", post(approve_changeset))
        .route("/staging/{id}/revert", post(revert_changeset))
}

// ---------------------------------------------------------------
//  Handlers
// ---------------------------------------------------------------

async fn staging_page(headers: HeaderMap) -> impl IntoResponse {
    let focal_data = mock_focal();
    let changesets_data = mock_changesets();
    let undo_data = mock_undo_stack();

    let total_staged = changesets_data
        .iter()
        .filter(|c| c.status == "Staged" || c.status == "Testing")
        .count();
    let total_applied = changesets_data
        .iter()
        .filter(|c| c.status == "Applied")
        .count();
    let total_reverted = 1usize; // mock

    let focal = FocalPointDisplay {
        primary_node: focal_data.primary_node.to_string(),
        primary_path: focal_data.primary_path.to_string(),
        primary_strength: format!("{:.0}%", focal_data.primary_strength * 100.0),
        confidence: format!("{:.0}%", focal_data.confidence * 100.0),
        stable_for: focal_data.stable_for.to_string(),
        tier_hot: focal_data.tier_hot.to_string(),
        tier_warm: focal_data.tier_warm.to_string(),
        secondary_nodes: focal_data
            .secondary_nodes
            .iter()
            .map(|(name, s)| (name.to_string(), format!("{:.0}%", s * 100.0)))
            .collect(),
        hot_path_prediction: focal_data.hot_path_prediction.to_string(),
        hot_path_confidence: format!("{:.0}%", focal_data.hot_path_confidence * 100.0),
    };

    let changesets: Vec<ChangesetDisplay> = changesets_data
        .into_iter()
        .map(|c| ChangesetDisplay {
            id: c.id.to_string(),
            title: c.title.to_string(),
            status: c.status.to_string(),
            status_shape: c.status_shape.to_string(),
            status_class: c.status_class.to_string(),
            risk_level: c.risk_level,
            risk_class: c.risk_class.to_string(),
            approval: c.approval.to_string(),
            files_changed: c.files_changed,
            lines_added: c.lines_added,
            lines_removed: c.lines_removed,
            blast_items: c.blast_radius.iter().map(|s| s.to_string()).collect(),
            test_status: c.test_status.to_string(),
            created_at: c.created_at.to_string(),
        })
        .collect();

    let undo_stack: Vec<UndoDisplay> = undo_data
        .into_iter()
        .map(|u| UndoDisplay {
            changeset_id: u.changeset_id.to_string(),
            title: u.title.to_string(),
            applied_at: u.applied_at.to_string(),
            can_undo: u.can_undo,
            undo_class: u.undo_class.to_string(),
        })
        .collect();

    let content = StagingTemplate {
        focal,
        changesets,
        undo_stack,
        total_staged,
        total_applied,
        total_reverted,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Staging", &content))
    }
}

async fn focal_point_partial() -> impl IntoResponse {
    let f = mock_focal();
    let mut html = String::new();

    html.push_str("<div class=\"focal-card\">");
    html.push_str(&format!(
        "<div class=\"focal-primary\"><span class=\"focal-shape\">[*]</span> <strong>{}</strong></div>",
        html_escape(f.primary_node)
    ));
    html.push_str(&format!(
        "<div class=\"focal-path\">{}</div>",
        html_escape(f.primary_path)
    ));
    html.push_str(&format!(
        "<div class=\"focal-stats\">Strength: <strong>{:.0}%</strong> | Confidence: <strong>{:.0}%</strong> | Stable: <strong>{}</strong></div>",
        f.primary_strength * 100.0,
        f.confidence * 100.0,
        f.stable_for
    ));
    html.push_str("<div class=\"focal-tiers\">");
    html.push_str(&format!(
        "<span class=\"tier-hot\">HOT: {}</span>",
        html_escape(f.tier_hot)
    ));
    html.push_str(&format!(
        "<span class=\"tier-warm\">WARM: {}</span>",
        html_escape(f.tier_warm)
    ));
    html.push_str("</div>");

    if !f.secondary_nodes.is_empty() {
        html.push_str("<div class=\"focal-secondary\">");
        for (name, strength) in &f.secondary_nodes {
            html.push_str(&format!(
                "<span class=\"focal-sec-node\">{} ({:.0}%)</span> ",
                html_escape(name),
                strength * 100.0
            ));
        }
        html.push_str("</div>");
    }

    html.push_str(&format!(
        "<div class=\"focal-hotpath\">Prediction: {} (confidence: {:.0}%)</div>",
        html_escape(f.hot_path_prediction),
        f.hot_path_confidence * 100.0
    ));
    html.push_str("</div>");

    Html(html)
}

async fn changesets_partial() -> impl IntoResponse {
    let changesets = mock_changesets();
    let mut html = String::new();

    for c in &changesets {
        html.push_str(&format!(
            "<div class=\"cs-row {}\">",
            c.status_class
        ));
        html.push_str(&format!(
            "<div class=\"cs-header\"><span class=\"cs-shape\">{}</span> <strong>{}</strong> <span class=\"cs-id\">{}</span></div>",
            c.status_shape,
            html_escape(c.title),
            html_escape(c.id)
        ));
        html.push_str(&format!(
            "<div class=\"cs-meta\"><span class=\"{}\">Risk: {}/10</span> | {} | +{} -{} in {} files</div>",
            c.risk_class,
            c.risk_level,
            html_escape(c.approval),
            c.lines_added,
            c.lines_removed,
            c.files_changed
        ));
        html.push_str(&format!(
            "<div class=\"cs-tests\">Tests: {}</div>",
            html_escape(c.test_status)
        ));
        html.push_str(&format!(
            "<div class=\"cs-blast\">Blast: {}</div>",
            c.blast_radius.iter().map(|s| html_escape(s)).collect::<Vec<_>>().join(", ")
        ));
        html.push_str("</div>");
    }

    Html(html)
}

async fn approve_changeset(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"cs-action-result\">Changeset {} approved (mock)</div>",
        html_escape(&id)
    ))
}

async fn revert_changeset(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"cs-action-result\">Changeset {} reverted (mock)</div>",
        html_escape(&id)
    ))
}
