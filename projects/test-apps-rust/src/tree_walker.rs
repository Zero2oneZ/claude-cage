//! Tree walker — navigates the GentlyOS org, routes intents, shows blast radius.
//! Reads tree.json directly. Pure Rust, no Python dependency.

use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

struct AppState {
    tree: Value,
    nodes: HashMap<String, Value>,
    depth_map: HashMap<String, usize>,
}

impl AppState {
    fn from_file(path: &str) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read tree.json");
        let tree: Value = serde_json::from_str(&content).expect("Invalid JSON");

        let mut nodes = HashMap::new();
        let mut depth_map = HashMap::new();

        if let Some(arr) = tree["nodes"].as_array() {
            // First pass: build depth map
            for node in arr {
                let id = node["id"].as_str().unwrap_or("").to_string();
                let parent = node["parent"].as_str();
                let depth = match parent {
                    None | Some("") => 0,
                    Some(p) => depth_map.get(p).copied().unwrap_or(0) + 1,
                };
                depth_map.insert(id.clone(), depth);
                nodes.insert(id, node.clone());
            }
        }

        Self {
            tree,
            nodes,
            depth_map,
        }
    }

    fn route_intent(&self, intent: &str) -> Vec<(&str, f64)> {
        let lowered = intent.to_lowercase();
        let words: Vec<&str> = lowered.split_whitespace().collect();
        let mut scores: Vec<(&str, f64)> = Vec::new();

        for (id, node) in &self.nodes {
            let name = node["name"].as_str().unwrap_or("").to_lowercase();
            let scale = node["scale"].as_str().unwrap_or("");

            // Only route to captains (leaf workers)
            if scale != "captain" {
                continue;
            }

            let mut score = 0.0;
            for word in &words {
                if name.contains(word) {
                    score += 2.0;
                }
                if id.contains(word) {
                    score += 3.0;
                }
                // Check metadata keywords
                if let Some(meta) = node.get("metadata") {
                    let meta_str = meta.to_string().to_lowercase();
                    if meta_str.contains(word) {
                        score += 1.0;
                    }
                }
            }

            if score > 0.0 {
                scores.push((id.as_str(), score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores
    }

    fn blast_radius(&self, node_id: &str) -> Vec<String> {
        let mut affected = vec![node_id.to_string()];
        // Walk up to parent
        if let Some(node) = self.nodes.get(node_id) {
            if let Some(parent) = node["parent"].as_str() {
                affected.push(parent.to_string());
                // Walk up to grandparent
                if let Some(pnode) = self.nodes.get(parent) {
                    if let Some(gp) = pnode["parent"].as_str() {
                        affected.push(gp.to_string());
                    }
                }
            }
        }
        affected
    }
}

async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let node_count = state.nodes.len();
    let mut html = format!(
        r#"<html><head><title>tree-walker</title>
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:monospace;max-width:900px;margin:0 auto;padding:2rem}}
a{{color:#4fc3f7;text-decoration:none}}a:hover{{text-decoration:underline}}
.node{{padding:4px 0}}.scale-executive{{color:#ff6b6b}}.scale-department{{color:#ffd93d}}.scale-captain{{color:#6bcf7f}}
input{{background:#16213e;color:#e0e0e0;border:1px solid #333;padding:0.5rem;width:60%;font-family:monospace}}
button{{background:#0f3460;color:#e0e0e0;border:none;padding:0.5rem 1rem;cursor:pointer}}</style></head>
<body><h1>tree-walker</h1>
<p>{node_count} nodes in the org</p>
<form action="/route" method="GET">
<input name="intent" placeholder="Route an intent..." value="">
<button>Route</button></form>
<h2>Tree</h2>"#
    );

    if let Some(nodes) = state.tree["nodes"].as_array() {
        for node in nodes {
            let id = node["id"].as_str().unwrap_or("");
            let name = node["name"].as_str().unwrap_or("");
            let scale = node["scale"].as_str().unwrap_or("");
            let depth = state.depth_map.get(id).copied().unwrap_or(0);
            let indent = depth * 24;
            html.push_str(&format!(
                "<div class=\"node scale-{scale}\" style=\"padding-left:{indent}px\">\
                 <a href=\"/node/{id}\">{id}</a> — {name}</div>"
            ));
        }
    }

    html.push_str("</body></html>");
    Html(html)
}

async fn node_detail(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> impl IntoResponse {
    match state.nodes.get(&node_id) {
        Some(node) => {
            let blast = state.blast_radius(&node_id);
            let children: Vec<&str> = node["children"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let rules: Vec<String> = node["rules"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|r| {
                            format!(
                                "{}: IF {} THEN {}",
                                r["name"].as_str().unwrap_or("?"),
                                r["condition"].as_str().unwrap_or("?"),
                                r["action"].as_str().unwrap_or("?")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            axum::Json(json!({
                "id": node_id,
                "name": node["name"],
                "scale": node["scale"],
                "parent": node["parent"],
                "children": children,
                "rules": rules,
                "blast_radius": blast,
                "depth": state.depth_map.get(&node_id),
                "metadata": node["metadata"],
            }))
        }
        None => axum::Json(json!({"error": "node not found"})),
    }
}

#[derive(Deserialize)]
struct RouteQuery {
    intent: String,
}

async fn route_intent(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RouteQuery>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let routes = state.route_intent(&q.intent);
    let elapsed = start.elapsed();

    let results: Vec<Value> = routes
        .iter()
        .take(5)
        .map(|(id, score)| {
            let blast = state.blast_radius(id);
            json!({
                "node_id": id,
                "score": score,
                "blast_radius": blast,
                "lineage": state.nodes.get(*id).and_then(|_n| {
                    let mut chain = vec![id.to_string()];
                    let mut current = *id;
                    while let Some(node) = state.nodes.get(current) {
                        if let Some(p) = node["parent"].as_str() {
                            chain.push(p.to_string());
                            current = p;
                        } else { break; }
                    }
                    chain.reverse();
                    Some(chain)
                }),
            })
        })
        .collect();

    axum::Json(json!({
        "intent": q.intent,
        "routes": results,
        "latency_us": elapsed.as_micros(),
    }))
}

#[tokio::main]
async fn main() {
    let tree_path = std::env::var("TREE_PATH")
        .unwrap_or_else(|_| "../../gentlyos/tree.json".to_string());

    let state = Arc::new(AppState::from_file(&tree_path));
    eprintln!(
        "tree-walker ready: {} nodes, {} depth levels",
        state.nodes.len(),
        state.depth_map.values().max().unwrap_or(&0) + 1
    );

    let app = Router::new()
        .route("/", get(index))
        .route("/node/{node_id}", get(node_detail))
        .route("/route", get(route_intent))
        .with_state(state);

    let addr = "0.0.0.0:3002";
    eprintln!("tree-walker listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
