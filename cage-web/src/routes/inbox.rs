//! Inbox Pipeline -- `/inbox`
//!
//! File review/scan/approve/reject workflow for ~/gently/inbox/.
//! All external files land in the inbox. Nothing executes until scanned
//! by gently-sploit and approved through this pipeline.
//!
//! Shapes for colorblind-safe status:
//!   PENDING  = [?]  -- awaiting scan
//!   CLEAN    = [O]  -- circle, no threats
//!   SUSPECT  = [/\] -- triangle, low severity
//!   DANGER   = [#]  -- square, high severity
//!   APPROVED = [+]  -- promoted to vault
//!   REJECTED = [x]  -- moved to evidence

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

#[derive(Debug, Clone, Copy)]
enum InboxStatus {
    Pending,
    Clean,
    Suspect,
    Danger,
    Approved,
    Rejected,
}

impl InboxStatus {
    fn shape(self) -> &'static str {
        match self {
            Self::Pending => "[?]",
            Self::Clean => "[O]",
            Self::Suspect => "[/\\]",
            Self::Danger => "[#]",
            Self::Approved => "[+]",
            Self::Rejected => "[x]",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Clean => "CLEAN",
            Self::Suspect => "SUSPECT",
            Self::Danger => "DANGER",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Pending => "inbox-pending",
            Self::Clean => "inbox-clean",
            Self::Suspect => "inbox-suspect",
            Self::Danger => "inbox-danger",
            Self::Approved => "inbox-approved",
            Self::Rejected => "inbox-rejected",
        }
    }

    fn can_action(self) -> bool {
        matches!(self, Self::Clean | Self::Suspect)
    }
}

struct InboxEntry {
    filename: &'static str,
    size: &'static str,
    status: InboxStatus,
    threat_level: &'static str,
    scan_summary: &'static str,
    content_type: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset (mock data for v1.0)
// ---------------------------------------------------------------

static INBOX_ITEMS: &[InboxEntry] = &[
    InboxEntry {
        filename: "config-backup.tar.gz",
        size: "2.4 MB",
        status: InboxStatus::Clean,
        threat_level: "0 -- No threats",
        scan_summary: "Archive contains 12 files. No executables, no hidden Unicode, no steganography.",
        content_type: "application/gzip",
    },
    InboxEntry {
        filename: "api-response.json",
        size: "48 KB",
        status: InboxStatus::Suspect,
        threat_level: "2 -- Low",
        scan_summary: "JSON valid. Contains 3 URLs pointing to external domains. No injection patterns.",
        content_type: "application/json",
    },
    InboxEntry {
        filename: "update-script.sh",
        size: "1.2 KB",
        status: InboxStatus::Danger,
        threat_level: "7 -- Critical",
        scan_summary: "Shell script with curl pipe to bash pattern. Downloads from unknown host. QUARANTINED.",
        content_type: "text/x-shellscript",
    },
    InboxEntry {
        filename: "readme-notes.md",
        size: "8 KB",
        status: InboxStatus::Approved,
        threat_level: "0 -- No threats",
        scan_summary: "Markdown file. No hidden characters, no embedded links, no injection.",
        content_type: "text/markdown",
    },
    InboxEntry {
        filename: "payload.bin",
        size: "256 B",
        status: InboxStatus::Rejected,
        threat_level: "9 -- Critical",
        scan_summary: "Binary with ELF header. Attempted execution from inbox path detected. Evidence preserved.",
        content_type: "application/octet-stream",
    },
];

// ---------------------------------------------------------------
//  View Models
// ---------------------------------------------------------------

struct InboxItemView {
    filename: String,
    size: String,
    status_shape: String,
    status_label: String,
    status_class: String,
    threat_level: String,
    scan_summary: String,
    content_type: String,
    can_action: bool,
}

// ---------------------------------------------------------------
//  Templates
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    layer_label: String,
    layer_badge: String,
    items: Vec<InboxItemView>,
    pending_count: usize,
    approved_count: usize,
    rejected_count: usize,
    quarantined_count: usize,
}

#[derive(Template)]
#[template(path = "partials/inbox_items.html")]
struct InboxItemsPartial {
    items: Vec<InboxItemView>,
}

// ---------------------------------------------------------------
//  Data builder
// ---------------------------------------------------------------

fn build_inbox_items() -> Vec<InboxItemView> {
    INBOX_ITEMS
        .iter()
        .map(|e| InboxItemView {
            filename: e.filename.to_string(),
            size: e.size.to_string(),
            status_shape: e.status.shape().to_string(),
            status_label: e.status.label().to_string(),
            status_class: e.status.css_class().to_string(),
            threat_level: e.threat_level.to_string(),
            scan_summary: e.scan_summary.to_string(),
            content_type: e.content_type.to_string(),
            can_action: e.status.can_action(),
        })
        .collect()
}

fn count_status(status: InboxStatus) -> usize {
    INBOX_ITEMS.iter().filter(|e| matches!(
        (e.status, status),
        (InboxStatus::Pending, InboxStatus::Pending) |
        (InboxStatus::Clean, InboxStatus::Clean) |
        (InboxStatus::Suspect, InboxStatus::Suspect) |
        (InboxStatus::Danger, InboxStatus::Danger) |
        (InboxStatus::Approved, InboxStatus::Approved) |
        (InboxStatus::Rejected, InboxStatus::Rejected)
    )).count()
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/inbox", get(inbox_page))
        .route("/partials/inbox-items", get(inbox_items_partial))
}

async fn inbox_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let items = build_inbox_items();
    let pending_count = count_status(InboxStatus::Pending) + count_status(InboxStatus::Clean) + count_status(InboxStatus::Suspect);
    let approved_count = count_status(InboxStatus::Approved);
    let rejected_count = count_status(InboxStatus::Rejected);
    let quarantined_count = count_status(InboxStatus::Danger);

    let content = InboxTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        items,
        pending_count,
        approved_count,
        rejected_count,
        quarantined_count,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Inbox Pipeline", &content))
    }
}

async fn inbox_items_partial(_headers: HeaderMap) -> impl IntoResponse {
    let items = build_inbox_items();
    Html(
        InboxItemsPartial { items }
            .render()
            .unwrap_or_default(),
    )
}
