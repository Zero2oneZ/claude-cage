use axum::{extract::State, http::StatusCode, response::{Json, Sse, sse::Event}, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

#[derive(Clone, Serialize, Deserialize)]
struct Msg { user: String, message: String, ts: u64, id: usize }

struct AppState {
    messages: RwLock<Vec<Msg>>,
    tx: broadcast::Sender<Msg>,
}

#[derive(Deserialize)]
struct SendBody { user: String, message: String }

async fn send(State(state): State<Arc<AppState>>, Json(body): Json<SendBody>) -> Result<Json<Value>, StatusCode> {
    if body.user.is_empty() || body.message.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut msgs = state.messages.write().await;
    let msg = Msg {
        user: body.user, message: body.message,
        ts: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
        id: msgs.len(),
    };
    msgs.push(msg.clone());
    if msgs.len() > 100 { msgs.remove(0); }
    let _ = state.tx.send(msg.clone());
    Ok(Json(json!(msg)))
}

async fn stream(State(state): State<Arc<AppState>>) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|r: Result<Msg, _>| {
        r.ok().map(|msg| Ok(Event::default().data(serde_json::to_string(&msg).unwrap())))
    });
    Sse::new(stream)
}

async fn history(State(state): State<Arc<AppState>>) -> Json<Value> {
    let msgs = state.messages.read().await;
    Json(json!({"messages": *msgs, "count": msgs.len()}))
}

async fn users(State(state): State<Arc<AppState>>) -> Json<Value> {
    let msgs = state.messages.read().await;
    let cutoff = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 - 300_000;
    let mut seen = std::collections::HashSet::new();
    let active: Vec<&str> = msgs.iter().rev().filter(|m| m.ts > cutoff).filter_map(|m| {
        if seen.insert(&m.user) { Some(m.user.as_str()) } else { None }
    }).collect();
    Json(json!({"users": active, "count": active.len()}))
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(256);
    let state = Arc::new(AppState { messages: RwLock::new(Vec::new()), tx });
    let app = Router::new()
        .route("/send", post(send))
        .route("/stream", get(stream))
        .route("/history", get(history))
        .route("/users", get(users))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4002").await.unwrap();
    println!("Chat server listening on :4002");
    axum::serve(listener, app).await.unwrap();
}
