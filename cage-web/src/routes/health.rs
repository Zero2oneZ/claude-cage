use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::json;

use crate::routes::html_escape;
use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/mongo/query", get(api_mongo_query))
        .route("/api/ptc/route", get(api_ptc_route))
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

#[derive(Deserialize)]
struct MongoQuery {
    collection: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
}

async fn api_mongo_query(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MongoQuery>,
) -> impl IntoResponse {
    let collection = q.collection.as_deref().unwrap_or("events");
    let query = q.query.as_deref().unwrap_or("{}");
    let limit = q.limit.unwrap_or(10);

    match subprocess::mongo_get(&state.store_js, collection, query, limit).await {
        Ok(raw) => {
            let docs: Vec<serde_json::Value> = raw
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            Json(json!({ "results": docs, "count": docs.len() }))
        }
        Err(e) => Json(json!({ "error": html_escape(&e) })),
    }
}

#[derive(Deserialize)]
struct RouteQuery {
    intent: Option<String>,
}

async fn api_ptc_route(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RouteQuery>,
) -> impl IntoResponse {
    let intent = q.intent.as_deref().unwrap_or("health check");
    let tree_path = state.tree_path.to_str().unwrap_or("gentlyos/tree.json");

    match subprocess::ptc_run(&state.cage_root, tree_path, intent).await {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => Json(json!({ "raw": raw })),
        },
        Err(e) => Json(json!({ "error": html_escape(&e) })),
    }
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
