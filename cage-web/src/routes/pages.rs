use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::routes::{is_htmx, wrap_page};
use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(dashboard))
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    session_count: usize,
    docker_ok: bool,
    codie_count: usize,
}

async fn dashboard(headers: HeaderMap, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let docker_ok = subprocess::docker(&["info"]).await.is_ok();
    let sessions = subprocess::list_sessions().await.unwrap_or_default();
    let session_count = sessions.lines().filter(|l| !l.trim().is_empty()).count();
    let programs = state.codie_programs.read().await;

    let content = DashboardTemplate {
        session_count,
        docker_ok,
        codie_count: programs.len(),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Dashboard", &content))
    }
}
