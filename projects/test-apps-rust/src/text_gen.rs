//! Text generation API — takes a prompt, streams generated tokens.
//! Uses a simple Markov chain seeded from CODIE programs as the corpus.
//! No external deps, no GPU, just architecture-driven text generation.

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct AppState {
    chain: RwLock<MarkovChain>,
}

struct MarkovChain {
    transitions: HashMap<(String, String), Vec<String>>,
    starts: Vec<(String, String)>,
}

impl MarkovChain {
    fn new() -> Self {
        Self {
            transitions: HashMap::new(),
            starts: Vec::new(),
        }
    }

    fn train(&mut self, text: &str) {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 3 {
            return;
        }
        self.starts
            .push((words[0].to_string(), words[1].to_string()));
        for window in words.windows(3) {
            let key = (window[0].to_string(), window[1].to_string());
            self.transitions
                .entry(key)
                .or_default()
                .push(window[2].to_string());
        }
    }

    fn generate(&self, max_tokens: usize, seed: Option<&str>) -> String {
        if self.starts.is_empty() {
            return String::from("(no training data)");
        }

        // Simple deterministic seed selection
        let start_idx = seed
            .map(|s| {
                s.bytes()
                    .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize))
                    % self.starts.len()
            })
            .unwrap_or(0);

        let (mut w1, mut w2) = self.starts[start_idx].clone();
        let mut result = vec![w1.clone(), w2.clone()];
        let mut counter = 0usize;

        for _ in 0..max_tokens {
            let key = (w1.clone(), w2.clone());
            match self.transitions.get(&key) {
                Some(nexts) if !nexts.is_empty() => {
                    let idx = counter % nexts.len();
                    let next = nexts[idx].clone();
                    result.push(next.clone());
                    w1 = w2;
                    w2 = next;
                    counter = counter.wrapping_add(7);
                }
                _ => break,
            }
        }

        result.join(" ")
    }
}

#[derive(Deserialize)]
struct GenRequest {
    prompt: Option<String>,
    max_tokens: Option<usize>,
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<html><head><title>text-gen</title>
<style>body{background:#1a1a2e;color:#e0e0e0;font-family:monospace;max-width:800px;margin:0 auto;padding:2rem}
textarea,input{background:#16213e;color:#e0e0e0;border:1px solid #333;padding:0.5rem;width:100%;font-family:monospace}
button{background:#0f3460;color:#e0e0e0;border:none;padding:0.5rem 1rem;cursor:pointer}
pre{background:#0a0a1a;padding:1rem;overflow:auto;white-space:pre-wrap}
.stats{color:#888;font-size:0.8rem}</style></head><body>
<h1>text-gen</h1>
<p class="stats">Markov chain trained on CODIE corpus. Architecture-native text generation.</p>
<form method="POST" action="/generate">
<textarea name="prompt" rows="3" placeholder="Enter prompt (or leave blank for random)"></textarea>
<br><input name="max_tokens" value="50" style="width:100px" placeholder="max tokens">
<button type="submit">Generate</button>
</form>
<div id="output"></div></body></html>"#,
    )
}

async fn generate(
    State(state): State<Arc<AppState>>,
    axum::Form(req): axum::Form<GenRequest>,
) -> impl IntoResponse {
    let chain = state.chain.read().await;
    let max = req.max_tokens.unwrap_or(50).min(200);
    let prompt = req.prompt.as_deref().filter(|s| !s.is_empty());

    let start = std::time::Instant::now();
    let text = chain.generate(max, prompt);
    let elapsed = start.elapsed();

    let token_count = text.split_whitespace().count();
    let ms = elapsed.as_micros() as f64 / 1000.0;
    let tps = if ms > 0.0 {
        token_count as f64 / (ms / 1000.0)
    } else {
        0.0
    };

    let body = json!({
        "text": text,
        "tokens": token_count,
        "latency_ms": format!("{ms:.2}"),
        "tokens_per_second": format!("{tps:.0}"),
        "model": "markov-codie-v1",
    });

    axum::Json(body)
}

async fn api_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let chain = state.chain.read().await;
    axum::Json(json!({
        "bigrams": chain.transitions.len(),
        "start_states": chain.starts.len(),
        "model": "markov-codie-v1",
    }))
}

#[tokio::main]
async fn main() {
    let mut chain = MarkovChain::new();

    // Train on CODIE corpus from codie-maps
    let codie_dir = std::env::var("CODIE_DIR")
        .unwrap_or_else(|_| "../../projects/Gently-nix/tools/codie-maps".to_string());

    let mut file_count = 0;
    if let Ok(entries) = std::fs::read_dir(&codie_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("codie") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    chain.train(&content);
                    file_count += 1;
                }
            }
        }
    }

    // Also train on tree.json descriptions
    if let Ok(content) = std::fs::read_to_string("../../gentlyos/tree.json") {
        chain.train(&content);
    }

    eprintln!(
        "text-gen ready: trained on {file_count} .codie files, {} bigrams, {} starts",
        chain.transitions.len(),
        chain.starts.len()
    );

    let state = Arc::new(AppState {
        chain: RwLock::new(chain),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/generate", post(generate))
        .route("/api/generate", post(generate))
        .route("/api/stats", get(api_stats))
        .with_state(state);

    let addr = "0.0.0.0:3001";
    eprintln!("text-gen listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
