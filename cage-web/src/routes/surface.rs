use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/surface", get(surface_page))
}

struct ShelfEntry {
    name: &'static str,
    icon: &'static str,
    status: &'static str,
    locked: bool,
    core: bool,
}

#[derive(Template)]
#[template(path = "surface.html")]
struct SurfaceTemplate {
    layer_label: String,
    layer_badge: String,
    shelf_items: Vec<ShelfEntry>,
    active_count: usize,
    locked_count: usize,
}

async fn surface_page(
    headers: HeaderMap,
    State(_state): State<Arc<AppState>>,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext.extensions().get::<Layer>().copied().unwrap_or(Layer::User);

    let shelf_items = build_shelf(layer);
    let active_count = shelf_items.iter().filter(|s| !s.locked).count();
    let locked_count = shelf_items.iter().filter(|s| s.locked).count();

    let content = SurfaceTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        shelf_items,
        active_count,
        locked_count,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("IO Surface", &content))
    }
}

fn build_shelf(layer: Layer) -> Vec<ShelfEntry> {
    let mut items = vec![
        // Core items (always active, locked_on)
        ShelfEntry { name: "alexandria", icon: "LIB", status: "active", locked: false, core: true },
        ShelfEntry { name: "claude-chat", icon: "AI", status: "active", locked: false, core: true },
        ShelfEntry { name: "guarddog-dns", icon: "DNS", status: "active", locked: false, core: true },
        ShelfEntry { name: "env-vault", icon: "KEY", status: "active", locked: false, core: true },
        ShelfEntry { name: "shelf", icon: "SHF", status: "active", locked: false, core: true },
    ];

    // Basic+ items
    if layer.has_access(Layer::RootUser) {
        items.push(ShelfEntry { name: "workbench", icon: "WRK", status: "active", locked: false, core: false });
        items.push(ShelfEntry { name: "python-bridge", icon: "PY", status: "active", locked: false, core: false });
    } else {
        items.push(ShelfEntry { name: "workbench", icon: "WRK", status: "locked", locked: true, core: false });
        items.push(ShelfEntry { name: "python-bridge", icon: "PY", status: "locked", locked: true, core: false });
    }

    // Pro+ items
    if layer.has_access(Layer::OsAdmin) {
        items.push(ShelfEntry { name: "docker", icon: "DKR", status: "active", locked: false, core: false });
        items.push(ShelfEntry { name: "agent-swarm", icon: "AGT", status: "active", locked: false, core: false });
    } else {
        items.push(ShelfEntry { name: "docker", icon: "DKR", status: "locked", locked: true, core: false });
        items.push(ShelfEntry { name: "agent-swarm", icon: "AGT", status: "locked", locked: true, core: false });
    }

    // Dev+ items
    if layer.has_access(Layer::DevLevel) {
        items.push(ShelfEntry { name: "limbo", icon: "LMB", status: "active", locked: false, core: false });
        items.push(ShelfEntry { name: "offensive-tools", icon: "OFS", status: "active", locked: false, core: false });
    } else {
        items.push(ShelfEntry { name: "limbo", icon: "LMB", status: "locked", locked: true, core: false });
        items.push(ShelfEntry { name: "offensive-tools", icon: "OFS", status: "locked", locked: true, core: false });
    }

    items
}
