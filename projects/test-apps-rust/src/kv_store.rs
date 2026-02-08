use axum::{extract::{Path, State}, http::StatusCode, response::Json, routing::get, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

type Store = Arc<RwLock<HashMap<String, Value>>>;

#[derive(Deserialize)]
struct PutBody { value: Value }

static mut START: Option<std::time::Instant> = None;

async fn get_key(Path(key): Path<String>, State(store): State<Store>) -> Result<Json<Value>, StatusCode> {
    let s = store.read().await;
    s.get(&key).map(|v| Json(json!({"key": key, "value": v}))).ok_or(StatusCode::NOT_FOUND)
}

async fn put_key(Path(key): Path<String>, State(store): State<Store>, Json(body): Json<PutBody>) -> Json<Value> {
    store.write().await.insert(key.clone(), body.value.clone());
    Json(json!({"key": key, "value": body.value, "stored": true}))
}

async fn delete_key(Path(key): Path<String>, State(store): State<Store>) -> Json<Value> {
    let existed = store.write().await.remove(&key).is_some();
    Json(json!({"key": key, "deleted": existed}))
}

async fn list_keys(State(store): State<Store>) -> Json<Value> {
    let s = store.read().await;
    let keys: Vec<&String> = s.keys().collect();
    Json(json!({"keys": keys, "count": keys.len()}))
}

async fn stats(State(store): State<Store>) -> Json<Value> {
    let s = store.read().await;
    let uptime = unsafe { START.unwrap().elapsed().as_secs() };
    Json(json!({"keys": s.len(), "uptime": uptime}))
}

#[tokio::main]
async fn main() {
    unsafe { START = Some(std::time::Instant::now()); }
    let store: Store = Arc::new(RwLock::new(HashMap::new()));
    let app = Router::new()
        .route("/k/{key}", get(get_key).put(put_key).delete(delete_key))
        .route("/keys", get(list_keys))
        .route("/stats", get(stats))
        .with_state(store);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4001").await.unwrap();
    println!("KV store listening on :4001");
    axum::serve(listener, app).await.unwrap();
}
