use axum::{extract::{Path, Query, State}, http::StatusCode, response::Json, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone, Serialize, Deserialize)]
struct Task {
    id: u64, title: String, status: String, priority: u8,
    tags: Vec<String>, created: u64, updated: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed: Option<u64>,
}

struct AppState { tasks: RwLock<HashMap<u64, Task>>, next_id: RwLock<u64> }

#[derive(Deserialize)]
struct CreateBody { title: String, priority: Option<u8>, tags: Option<Vec<String>> }

#[derive(Deserialize)]
struct UpdateBody { title: Option<String>, status: Option<String>, priority: Option<u8>, tags: Option<Vec<String>> }

#[derive(Deserialize)]
struct ListQuery { status: Option<String>, tag: Option<String>, sort: Option<String> }

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 }

async fn create_task(State(state): State<Arc<AppState>>, Json(body): Json<CreateBody>) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let mut id = state.next_id.write().await;
    let task = Task {
        id: *id, title: body.title, status: "open".into(),
        priority: body.priority.unwrap_or(3), tags: body.tags.unwrap_or_default(),
        created: now(), updated: now(), completed: None,
    };
    *id += 1;
    state.tasks.write().await.insert(task.id, task.clone());
    Ok((StatusCode::CREATED, Json(json!(task))))
}

async fn list_tasks(State(state): State<Arc<AppState>>, Query(q): Query<ListQuery>) -> Json<Value> {
    let tasks = state.tasks.read().await;
    let mut result: Vec<&Task> = tasks.values().collect();
    if let Some(ref s) = q.status { result.retain(|t| t.status == *s); }
    if let Some(ref tag) = q.tag { result.retain(|t| t.tags.contains(tag)); }
    match q.sort.as_deref() {
        Some("created") => result.sort_by(|a, b| b.created.cmp(&a.created)),
        _ => result.sort_by(|a, b| a.priority.cmp(&b.priority)),
    }
    Json(json!({"tasks": result, "count": result.len()}))
}

async fn get_task(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    state.tasks.read().await.get(&id).map(|t| Json(json!(t))).ok_or(StatusCode::NOT_FOUND)
}

async fn update_task(Path(id): Path<u64>, State(state): State<Arc<AppState>>, Json(body): Json<UpdateBody>) -> Result<Json<Value>, StatusCode> {
    let mut tasks = state.tasks.write().await;
    let task = tasks.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(t) = body.title { task.title = t; }
    if let Some(s) = body.status { task.status = s; }
    if let Some(p) = body.priority { task.priority = p; }
    if let Some(tags) = body.tags { task.tags = tags; }
    task.updated = now();
    Ok(Json(json!(task)))
}

async fn delete_task(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> Json<Value> {
    let existed = state.tasks.write().await.remove(&id).is_some();
    Json(json!({"id": id, "deleted": existed}))
}

async fn mark_done(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let mut tasks = state.tasks.write().await;
    let task = tasks.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    task.status = "done".into();
    task.updated = now();
    task.completed = Some(now());
    Ok(Json(json!(task)))
}

async fn stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let tasks = state.tasks.read().await;
    let mut by_status: HashMap<&str, usize> = HashMap::new();
    let mut tag_cloud: HashMap<&str, usize> = HashMap::new();
    for t in tasks.values() {
        *by_status.entry(&t.status).or_default() += 1;
        for tag in &t.tags { *tag_cloud.entry(tag).or_default() += 1; }
    }
    Json(json!({"total": tasks.len(), "byStatus": by_status, "tagCloud": tag_cloud}))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState { tasks: RwLock::new(HashMap::new()), next_id: RwLock::new(1) });
    let app = Router::new()
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", get(get_task).patch(update_task).delete(delete_task))
        .route("/tasks/{id}/done", post(mark_done))
        .route("/stats", get(stats))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4003").await.unwrap();
    println!("Task API listening on :4003");
    axum::serve(listener, app).await.unwrap();
}
