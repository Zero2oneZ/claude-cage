use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::routes::{html_escape, is_htmx, wrap_page};
use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tree", get(tree_page))
        .route("/tree/{node_id}", get(node_detail))
        .route("/tree/blast-radius", get(blast_radius))
        .route("/api/gentlyos/tree", get(api_tree))
        .route("/api/gentlyos/node/{node_id}", get(api_node))
}

#[derive(Template)]
#[template(path = "tree.html")]
struct TreeTemplate {
    nodes: Vec<TreeNode>,
}

#[derive(Template)]
#[template(path = "node_detail.html")]
struct NodeDetailTemplate {
    id: String,
    name: String,
    scale: String,
    description: String,
    rules: Vec<String>,
    files: Vec<String>,
    crates_owned: Vec<String>,
    children: Vec<String>,
    sephira: String,
    department: String,
}

struct TreeNode {
    id: String,
    name: String,
    scale: String,
    depth: usize,
}

fn extract_nodes(tree: &Value) -> Vec<TreeNode> {
    let mut result = Vec::new();
    if let Some(nodes) = tree["nodes"].as_array() {
        // Build parent→depth map by walking the tree
        let mut depth_map = std::collections::HashMap::new();
        for node in nodes {
            let id = node["id"].as_str().unwrap_or("");
            let parent = node["parent"].as_str();
            let depth = match parent {
                None | Some("") => 0,
                Some(p) => depth_map.get(p).copied().unwrap_or(0) + 1,
            };
            depth_map.insert(id.to_string(), depth);
        }

        for node in nodes {
            let id = node["id"].as_str().unwrap_or("").to_string();
            let depth = depth_map.get(&id).copied().unwrap_or(0);
            result.push(TreeNode {
                id,
                name: node["name"].as_str().unwrap_or("").to_string(),
                scale: node["scale"].as_str().unwrap_or("").to_string(),
                depth,
            });
        }
    }
    result
}

fn find_node(tree: &Value, node_id: &str) -> Option<Value> {
    if let Some(nodes) = tree["nodes"].as_array() {
        nodes.iter().find(|n| n["id"].as_str() == Some(node_id)).cloned()
    } else {
        None
    }
}

async fn tree_page(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tree = subprocess::read_tree(&state.tree_path)
        .await
        .unwrap_or(json!({"nodes": []}));
    let nodes = extract_nodes(&tree);

    let content = TreeTemplate { nodes }.render().unwrap_or_default();
    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("GentlyOS Tree", &content))
    }
}

async fn node_detail(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> impl IntoResponse {
    let tree = subprocess::read_tree(&state.tree_path)
        .await
        .unwrap_or(json!({"nodes": []}));

    let node = find_node(&tree, &node_id).unwrap_or(json!({}));

    let rules: Vec<String> = node["rules"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let name = r["name"].as_str().unwrap_or("?");
                    let cond = r["condition"].as_str().unwrap_or("?");
                    let act = r["action"].as_str().unwrap_or("?");
                    Some(format!("{name}: IF {cond} THEN {act}"))
                })
                .collect()
        })
        .unwrap_or_default();

    let files: Vec<String> = node["metadata"]["files"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let crates_owned: Vec<String> = node["metadata"]["crates_owned"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let children: Vec<String> = node["children"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let title = format!("Node: {node_id}");
    let content = NodeDetailTemplate {
        id: node_id,
        name: node["name"].as_str().unwrap_or("").to_string(),
        scale: node["scale"].as_str().unwrap_or("").to_string(),
        description: node["metadata"]["description"].as_str().unwrap_or("").to_string(),
        rules,
        files,
        crates_owned,
        children,
        sephira: node["metadata"]["sephira_mapping"].as_str().unwrap_or("").to_string(),
        department: node["parent"].as_str().unwrap_or("").to_string(),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page(&title, &content))
    }
}

#[derive(Deserialize)]
struct BlastQuery {
    crates: Option<String>,
}

async fn blast_radius(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BlastQuery>,
) -> impl IntoResponse {
    let tree = subprocess::read_tree(&state.tree_path)
        .await
        .unwrap_or(json!({"nodes": []}));

    let crate_list: Vec<&str> = q
        .crates
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();

    let mut affected_nodes = Vec::new();
    let mut max_risk = 0u32;

    if let Some(nodes) = tree["nodes"].as_array() {
        for node in nodes {
            let owned: Vec<&str> = node["metadata"]["crates_owned"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            if crate_list.iter().any(|c| owned.contains(c)) {
                affected_nodes.push(json!({
                    "id": node["id"],
                    "name": node["name"],
                    "scale": node["scale"],
                }));
                let risk = match node["scale"].as_str().unwrap_or("") {
                    "executive" => 9,
                    "department" => 7,
                    "captain" => 5,
                    _ => 3,
                };
                if risk > max_risk {
                    max_risk = risk;
                }
            }
        }
    }

    let approval = if max_risk >= 9 {
        "Human"
    } else if max_risk >= 7 {
        "CTO"
    } else if max_risk >= 4 {
        "Director"
    } else {
        "Captain"
    };

    // Return HTML for HTMX instead of JSON
    let safe_crates = html_escape(&crate_list.join(", "));
    let mut html = String::new();
    html.push_str("<div class=\"result\">");
    html.push_str(&format!("<h3>Blast Radius: {safe_crates}</h3>"));
    html.push_str(&format!("<p><strong>Risk Level:</strong> {max_risk}/10</p>"));
    html.push_str(&format!("<p><strong>Approval Required:</strong> {approval}</p>"));
    html.push_str(&format!(
        "<p><strong>Affected Nodes:</strong> {}</p>",
        affected_nodes.len()
    ));
    if !affected_nodes.is_empty() {
        html.push_str("<ul>");
        for n in &affected_nodes {
            let id = html_escape(n["id"].as_str().unwrap_or(""));
            let name = html_escape(n["name"].as_str().unwrap_or(""));
            let scale = html_escape(n["scale"].as_str().unwrap_or(""));
            html.push_str(&format!(
                "<li><code>{id}</code> ({scale}) &mdash; {name}</li>"
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</div>");

    Html(html)
}

async fn api_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tree = subprocess::read_tree(&state.tree_path)
        .await
        .unwrap_or(json!({"nodes": []}));
    Json(tree)
}

async fn api_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> impl IntoResponse {
    let tree = subprocess::read_tree(&state.tree_path)
        .await
        .unwrap_or(json!({"nodes": []}));
    let node = find_node(&tree, &node_id).unwrap_or(json!(null));
    Json(node)
}
