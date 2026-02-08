use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde_json::json;

use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/partials/health", get(partial_health))
}

#[derive(Template)]
#[template(path = "partials/health.html")]
struct HealthPartial {
    docker_ok: bool,
    session_count: usize,
    api_key_set: bool,
}

async fn api_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let docker_ok = subprocess::docker(&["info"]).await.is_ok();
    let sessions = subprocess::list_sessions().await.unwrap_or_default();
    let session_count = sessions.lines().filter(|l| !l.trim().is_empty()).count();
    let api_key_set = std::env::var("ANTHROPIC_API_KEY").is_ok();
    let programs = state.codie_programs.read().await;

    Json(json!({
        "docker": docker_ok,
        "sessions": session_count,
        "api_key_set": api_key_set,
        "codie_programs": programs.len(),
    }))
}

async fn partial_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let docker_ok = subprocess::docker(&["info"]).await.is_ok();
    let sessions = subprocess::list_sessions().await.unwrap_or_default();
    let session_count = sessions.lines().filter(|l| !l.trim().is_empty()).count();
    let api_key_set = std::env::var("ANTHROPIC_API_KEY").is_ok();
    let _ = state;

    Html(
        HealthPartial {
            docker_ok,
            session_count,
            api_key_set,
        }
        .render()
        .unwrap_or_default(),
    )
}
