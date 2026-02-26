//! Tools page -- `/tools`
//!
//! MCP tool sovereignty dashboard: registered tools, transport types,
//! permissions, enable/disable toggles. All static mock data.
//!
//! Status shapes (colorblind-safe):
//!   [O]  Connected  (socket listening, ready)
//!   [/\] Registered (installed but not connected)
//!   [#]  Error      (socket dead or crash)
//!
//! Transport shapes:
//!   [O]  Socket     (Unix domain socket)
//!   [/\] HTTP       (HTTP bridge, port-remapped)
//!   [<>] Stdio      (stdin/stdout pipe)

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data Model
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolStatus {
    Connected,
    Registered,
    Error,
}

impl ToolStatus {
    fn shape(self) -> &'static str {
        match self {
            Self::Connected => "[O]",
            Self::Registered => "[/\\]",
            Self::Error => "[#]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Connected => "CONNECTED",
            Self::Registered => "REGISTERED",
            Self::Error => "ERROR",
        }
    }
    fn css_class(self) -> &'static str {
        match self {
            Self::Connected => "tool-connected",
            Self::Registered => "tool-registered",
            Self::Error => "tool-error",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TransportType {
    Socket,
    HttpBridge,
    Stdio,
}

impl TransportType {
    fn shape(self) -> &'static str {
        match self {
            Self::Socket => "[O]",
            Self::HttpBridge => "[/\\]",
            Self::Stdio => "[<>]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Socket => "Unix Socket",
            Self::HttpBridge => "HTTP Bridge",
            Self::Stdio => "Stdio Pipe",
        }
    }
}

struct ToolEntry {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    version: &'static str,
    transport: TransportType,
    status: ToolStatus,
    permissions: &'static [&'static str],
    secret_refs: &'static [&'static str],
    enabled: bool,
    requests_served: u64,
    socket_path: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static TOOLS: &[ToolEntry] = &[
    ToolEntry {
        id: "t1a2b3c4",
        name: "web-search",
        description: "Search the web via Brave Search API",
        version: "2.1.0",
        transport: TransportType::Socket,
        status: ToolStatus::Connected,
        permissions: &["NetworkAccess", "SecretAccess(BRAVE_API_KEY)"],
        secret_refs: &["BRAVE_API_KEY"],
        enabled: true,
        requests_served: 142,
        socket_path: "/run/gently/sockets/tool-web-search.sock",
    },
    ToolEntry {
        id: "t5d6e7f8",
        name: "file-reader",
        description: "Read files from the project workspace",
        version: "1.0.0",
        transport: TransportType::Socket,
        status: ToolStatus::Connected,
        permissions: &["FileSystemRead"],
        secret_refs: &[],
        enabled: true,
        requests_served: 891,
        socket_path: "/run/gently/sockets/tool-file-reader.sock",
    },
    ToolEntry {
        id: "t9g0h1i2",
        name: "code-interpreter",
        description: "Execute Python code in sandboxed environment",
        version: "3.0.1",
        transport: TransportType::HttpBridge,
        status: ToolStatus::Connected,
        permissions: &["ProcessSpawn", "NetworkAccess", "FileSystemWrite"],
        secret_refs: &[],
        enabled: true,
        requests_served: 67,
        socket_path: "/run/gently/sockets/tool-code-interpreter.sock",
    },
    ToolEntry {
        id: "tj3k4l5m",
        name: "database-query",
        description: "Query PostgreSQL databases via connection string",
        version: "1.2.0",
        transport: TransportType::Socket,
        status: ToolStatus::Registered,
        permissions: &["DatabaseQuery", "SecretAccess(DATABASE_URL)"],
        secret_refs: &["DATABASE_URL"],
        enabled: false,
        requests_served: 0,
        socket_path: "/run/gently/sockets/tool-database-query.sock",
    },
    ToolEntry {
        id: "tn6o7p8q",
        name: "image-gen",
        description: "Generate images via local Stable Diffusion model",
        version: "0.9.0",
        transport: TransportType::Stdio,
        status: ToolStatus::Error,
        permissions: &["GpuAccess", "FileSystemWrite"],
        secret_refs: &[],
        enabled: true,
        requests_served: 3,
        socket_path: "/run/gently/sockets/tool-image-gen.sock",
    },
    ToolEntry {
        id: "tr9s0t1u",
        name: "git-operations",
        description: "Git clone, pull, push, branch management",
        version: "1.1.0",
        transport: TransportType::Socket,
        status: ToolStatus::Connected,
        permissions: &["ProcessSpawn", "FileSystemRead", "FileSystemWrite", "SecretAccess(SSH_KEY)"],
        secret_refs: &["SSH_KEY"],
        enabled: true,
        requests_served: 234,
        socket_path: "/run/gently/sockets/tool-git-operations.sock",
    },
];

// ---------------------------------------------------------------
//  View Models
// ---------------------------------------------------------------

struct ToolView {
    id: String,
    name: String,
    description: String,
    version: String,
    transport_shape: String,
    transport_label: String,
    status_shape: String,
    status_label: String,
    status_class: String,
    permissions: Vec<String>,
    secret_refs: Vec<String>,
    enabled: bool,
    enabled_shape: String,
    requests_served: u64,
    socket_path: String,
    can_toggle: bool,
}

// ---------------------------------------------------------------
//  Templates
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "tools.html")]
struct ToolsTemplate {
    layer_label: String,
    layer_badge: String,
    tools: Vec<ToolView>,
    total_tools: usize,
    connected_count: usize,
    enabled_count: usize,
    can_manage: bool,
}

#[derive(Template)]
#[template(path = "partials/tool_row.html")]
struct ToolRowPartial {
    tools: Vec<ToolView>,
}

// ---------------------------------------------------------------
//  Data builders
// ---------------------------------------------------------------

fn build_tools(can_manage: bool) -> Vec<ToolView> {
    TOOLS
        .iter()
        .map(|t| ToolView {
            id: t.id.to_string(),
            name: t.name.to_string(),
            description: t.description.to_string(),
            version: t.version.to_string(),
            transport_shape: t.transport.shape().to_string(),
            transport_label: t.transport.label().to_string(),
            status_shape: t.status.shape().to_string(),
            status_label: t.status.label().to_string(),
            status_class: t.status.css_class().to_string(),
            permissions: t.permissions.iter().map(|s| s.to_string()).collect(),
            secret_refs: t.secret_refs.iter().map(|s| s.to_string()).collect(),
            enabled: t.enabled,
            enabled_shape: if t.enabled { "[O]" } else { "[_]" }.to_string(),
            requests_served: t.requests_served,
            socket_path: t.socket_path.to_string(),
            can_toggle: can_manage,
        })
        .collect()
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tools", get(tools_page))
        .route("/tools/{id}/enable", post(tool_enable))
        .route("/tools/{id}/disable", post(tool_disable))
        .route("/partials/tool-rows", get(tool_rows_partial))
}

async fn tools_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let can_manage = layer.has_access(Layer::RootUser);
    let tools = build_tools(can_manage);

    let connected_count = TOOLS
        .iter()
        .filter(|t| matches!(t.status, ToolStatus::Connected))
        .count();
    let enabled_count = TOOLS.iter().filter(|t| t.enabled).count();

    let content = ToolsTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        tools,
        total_tools: TOOLS.len(),
        connected_count,
        enabled_count,
        can_manage,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Tools", &content))
    }
}

async fn tool_enable(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Tool {} enabled (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn tool_disable(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-warn\">Tool {} disabled (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn tool_rows_partial(_headers: HeaderMap) -> impl IntoResponse {
    let tools = build_tools(false);
    Html(
        ToolRowPartial { tools }
            .render()
            .unwrap_or_default(),
    )
}
