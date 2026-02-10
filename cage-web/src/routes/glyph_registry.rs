use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/glyph-registry", get(glyph_registry_page))
}

#[derive(Template)]
#[template(path = "glyph_registry.html")]
struct GlyphRegistryTemplate {
    layer_label: String,
    layer_badge: String,
}

async fn glyph_registry_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let content = GlyphRegistryTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Glyph Registry", &content))
    }
}
