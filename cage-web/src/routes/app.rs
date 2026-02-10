//! IO Surface Renderer -- `/app/:tenant_id`
//!
//! Loads AppConfig (Mask + ShelfState) for a tenant, applies Mask as CSS
//! custom properties, mounts only active shelf item routes, renders the
//! IO surface.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{html_escape, is_htmx, wrap_page};
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/app/{tenant_id}", get(app_surface))
}

async fn app_surface(
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    State(_state): State<Arc<AppState>>,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext.extensions().get::<Layer>().copied().unwrap_or(Layer::User);
    let tid = html_escape(&tenant_id);

    // Default palette (GentlyOS dark theme from mock)
    let palette = Palette {
        bg: "#08080c",
        shelf: "#0a0a10",
        panel: "#0e0e16",
        surface: "#12121c",
        elevated: "#18182a",
        hover: "#1e1e38",
        border: "#1a1a2e",
        focus: "#00e5a0",
        process: "#4d9fff",
        proton: "#ff6b9d",
        code: "#c77dff",
    };

    // Core items always mounted
    let core_items = ["alexandria", "claude-chat", "guarddog-dns", "env-vault", "shelf"];

    // Tier-gated items
    let mut active_items: Vec<&str> = core_items.to_vec();
    if layer.has_access(Layer::RootUser) {
        active_items.extend_from_slice(&["workbench", "python-bridge"]);
    }
    if layer.has_access(Layer::OsAdmin) {
        active_items.extend_from_slice(&["docker", "agent-swarm"]);
    }
    if layer.has_access(Layer::DevLevel) {
        active_items.extend_from_slice(&["limbo", "offensive-tools"]);
    }

    let items_html: String = active_items
        .iter()
        .map(|name| {
            let is_core = core_items.contains(name);
            format!(
                r#"<div class="shelf-item shelf-active{}">{}</div>"#,
                if is_core { " shelf-core" } else { "" },
                html_escape(name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        r##"<div class="app-surface" style="
            --app-bg: {bg};
            --app-shelf: {shelf};
            --app-panel: {panel};
            --app-surface: {surface};
            --app-elevated: {elevated};
            --app-hover: {hover};
            --app-border: {border};
            --app-focus: {focus};
            --app-process: {process};
            --app-proton: {proton};
            --app-code: {code};
        ">
    <div class="app-header">
        <h1>{tenant} IO Surface</h1>
        <span class="tier-badge {badge}">{label}</span>
    </div>
    <div class="shelf-grid">
        {items}
    </div>
    <div class="app-footer">
        <span class="powered-by">Powered by GentlyOS</span>
    </div>
</div>"##,
        bg = palette.bg,
        shelf = palette.shelf,
        panel = palette.panel,
        surface = palette.surface,
        elevated = palette.elevated,
        hover = palette.hover,
        border = palette.border,
        focus = palette.focus,
        process = palette.process,
        proton = palette.proton,
        code = palette.code,
        tenant = tid,
        badge = layer.badge_class(),
        label = layer.label(),
        items = items_html,
    );

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page(&format!("{} - IO Surface", tid), &content))
    }
}

struct Palette {
    bg: &'static str,
    shelf: &'static str,
    panel: &'static str,
    surface: &'static str,
    elevated: &'static str,
    hover: &'static str,
    border: &'static str,
    focus: &'static str,
    process: &'static str,
    proton: &'static str,
    code: &'static str,
}
