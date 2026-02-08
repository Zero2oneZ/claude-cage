use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::routes::{is_htmx, wrap_page};
use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/new", post(create_session))
        .route("/sessions/{name}", get(session_detail))
        .route("/sessions/{name}/stop", post(stop_session))
        .route("/sessions/{name}/start", post(start_session))
        .route("/sessions/{name}/destroy", delete(destroy_session))
        .route("/sessions/{name}/logs", get(session_logs))
        .route("/api/sessions", get(api_sessions))
}

#[derive(Template)]
#[template(path = "sessions.html")]
struct SessionsTemplate {
    sessions: Vec<SessionInfo>,
}

#[derive(Template)]
#[template(path = "session_detail.html")]
struct SessionDetailTemplate {
    name: String,
    status: String,
    image: String,
    created: String,
    ports: String,
    logs: String,
}

#[derive(Template)]
#[template(path = "partials/session_row.html")]
struct SessionRowTemplate {
    name: String,
    status: String,
    image: String,
    ports: String,
}

#[derive(Clone)]
struct SessionInfo {
    name: String,
    status: String,
    image: String,
    ports: String,
}

#[derive(Deserialize)]
struct CreateForm {
    mode: Option<String>,
    network: Option<String>,
}

fn parse_sessions(raw: &str) -> Vec<SessionInfo> {
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            Some(SessionInfo {
                name: v["Names"].as_str().unwrap_or("").to_string(),
                status: v["Status"].as_str().unwrap_or("unknown").to_string(),
                image: v["Image"].as_str().unwrap_or("").to_string(),
                ports: v["Ports"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect()
}

async fn list_sessions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let raw = subprocess::list_sessions().await.unwrap_or_default();
    let sessions = parse_sessions(&raw);

    Html(
        SessionsTemplate { sessions }
            .render()
            .unwrap_or_default(),
    )
}

async fn api_sessions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let raw = subprocess::list_sessions().await.unwrap_or_default();
    let sessions: Vec<Value> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Json(json!({ "sessions": sessions }))
}

async fn session_detail(
    headers: HeaderMap,
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let container = format!("cage-{name}");
    let inspect_raw = subprocess::inspect_container(&container)
        .await
        .unwrap_or_default();
    let logs_raw = subprocess::container_logs(&container, "50")
        .await
        .unwrap_or_default();

    let inspect: Vec<Value> = serde_json::from_str(&inspect_raw).unwrap_or_default();
    let info = inspect.first().cloned().unwrap_or(json!({}));

    let title = format!("Session: {name}");
    let content = SessionDetailTemplate {
        name,
        status: info["State"]["Status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        image: info["Config"]["Image"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        created: info["Created"].as_str().unwrap_or("").to_string(),
        ports: format!("{}", info["NetworkSettings"]["Ports"]),
        logs: logs_raw,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page(&title, &content))
    }
}

async fn create_session(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<CreateForm>,
) -> impl IntoResponse {
    let mode = form.mode.unwrap_or_else(|| "cli".to_string());
    let network = form.network.unwrap_or_else(|| "filtered".to_string());

    let _ = subprocess::create_session(&state.cage_root, &mode, &network).await;

    // Re-render session list
    let raw = subprocess::list_sessions().await.unwrap_or_default();
    let sessions = parse_sessions(&raw);

    Html(
        SessionsTemplate { sessions }
            .render()
            .unwrap_or_default(),
    )
}

async fn stop_session(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let container = format!("cage-{name}");
    let _ = subprocess::stop_container(&container).await;

    let raw = subprocess::list_sessions().await.unwrap_or_default();
    let sessions = parse_sessions(&raw);
    let session = sessions
        .iter()
        .find(|s| s.name.contains(&name))
        .cloned()
        .unwrap_or(SessionInfo {
            name: name.clone(),
            status: "stopped".to_string(),
            image: String::new(),
            ports: String::new(),
        });

    Html(
        SessionRowTemplate {
            name: session.name,
            status: session.status,
            image: session.image,
            ports: session.ports,
        }
        .render()
        .unwrap_or_default(),
    )
}

async fn start_session(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let container = format!("cage-{name}");
    let _ = subprocess::start_container(&container).await;

    let raw = subprocess::list_sessions().await.unwrap_or_default();
    let sessions = parse_sessions(&raw);
    let session = sessions
        .iter()
        .find(|s| s.name.contains(&name))
        .cloned()
        .unwrap_or(SessionInfo {
            name: name.clone(),
            status: "running".to_string(),
            image: String::new(),
            ports: String::new(),
        });

    Html(
        SessionRowTemplate {
            name: session.name,
            status: session.status,
            image: session.image,
            ports: session.ports,
        }
        .render()
        .unwrap_or_default(),
    )
}

async fn destroy_session(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let container = format!("cage-{name}");
    let _ = subprocess::destroy_container(&container).await;
    // Return empty so hx-swap="outerHTML" removes the row
    Html(String::new())
}

async fn session_logs(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let container = format!("cage-{name}");
    let logs = subprocess::container_logs(&container, "200")
        .await
        .unwrap_or_else(|e| format!("Error: {e}"));

    // Return as SSE-formatted text for hx-ext="sse"
    let mut sse = String::new();
    for line in logs.lines() {
        sse.push_str(&format!("data: {line}\n\n"));
    }
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/event-stream; charset=utf-8",
        )],
        sse,
    )
}
