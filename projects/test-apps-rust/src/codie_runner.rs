//! CODIE runner — loads .codie files, parses them, simulates execution.
//! Walks the instruction tree and produces a step-by-step trace.

use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

struct AppState {
    programs: HashMap<String, CodieProgram>,
}

struct CodieProgram {
    name: String,
    source: String,
    instructions: Vec<Instruction>,
    line_count: usize,
}

#[derive(Clone)]
enum Instruction {
    Entry(String),
    Fetch { target: String, source: String },
    Bind { name: String, value: String },
    Call { name: String, args: String },
    Guard(String),
    Rule { name: String, negated: bool, body: String },
    Loop { var: String, collection: String },
    Conditional { condition: String, action: String },
    Return(String),
    Checkpoint(String),
    Const { name: String, value: String },
    Comment(String),
}

impl CodieProgram {
    fn parse(name: &str, source: &str) -> Self {
        let mut instructions = Vec::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Strip pipe prefixes
            let content = trimmed
                .trim_start_matches('|')
                .trim()
                .trim_start_matches("+--")
                .trim();
            if content.is_empty() {
                continue;
            }

            let first = content.split_whitespace().next().unwrap_or("");
            let rest = content[first.len()..].trim();

            let inst = match first.to_lowercase().as_str() {
                "pug" => Instruction::Entry(rest.to_string()),
                "bark" => {
                    if let Some((target, source)) = rest.split_once("<-") {
                        Instruction::Fetch {
                            target: target.trim().to_string(),
                            source: source.trim().to_string(),
                        }
                    } else if let Some((target, source)) = rest.split_once(" from ") {
                        Instruction::Fetch {
                            target: target.trim().to_string(),
                            source: source.trim().to_string(),
                        }
                    } else {
                        Instruction::Fetch {
                            target: String::new(),
                            source: rest.to_string(),
                        }
                    }
                }
                "elf" => {
                    if let Some((name, value)) = rest.split_once("<-") {
                        Instruction::Bind {
                            name: name.trim().to_string(),
                            value: value.trim().to_string(),
                        }
                    } else {
                        Instruction::Bind {
                            name: rest.to_string(),
                            value: String::new(),
                        }
                    }
                }
                "cali" => {
                    let (name, args) = rest.split_once('(').unwrap_or((rest, ""));
                    Instruction::Call {
                        name: name.trim().to_string(),
                        args: args.trim_end_matches(')').to_string(),
                    }
                }
                "fence" => Instruction::Guard(rest.to_string()),
                "bone" => {
                    let negated = rest.starts_with("NOT:");
                    let body = if negated {
                        rest.trim_start_matches("NOT:").trim()
                    } else {
                        rest
                    };
                    Instruction::Rule {
                        name: String::new(),
                        negated,
                        body: body.to_string(),
                    }
                }
                "spin" => {
                    let (var, collection) = rest.split_once(" IN ").unwrap_or((rest, ""));
                    Instruction::Loop {
                        var: var.to_string(),
                        collection: collection.to_string(),
                    }
                }
                "?" => {
                    let (cond, action) = rest.split_once("->").unwrap_or((rest, ""));
                    Instruction::Conditional {
                        condition: cond.trim().to_string(),
                        action: action.trim().to_string(),
                    }
                }
                "biz" => Instruction::Return(rest.trim_start_matches("->").trim().to_string()),
                "anchor" => Instruction::Checkpoint(rest.trim_start_matches('#').to_string()),
                "pin" => {
                    let (name, value) = rest.split_once('=').unwrap_or((rest, ""));
                    Instruction::Const {
                        name: name.trim().to_string(),
                        value: value.trim().to_string(),
                    }
                }
                "//" => Instruction::Comment(rest.to_string()),
                _ => {
                    if content.starts_with("//") {
                        Instruction::Comment(content[2..].trim().to_string())
                    } else {
                        Instruction::Comment(content.to_string())
                    }
                }
            };
            instructions.push(inst);
        }

        Self {
            name: name.to_string(),
            source: source.to_string(),
            instructions,
            line_count: source.lines().count(),
        }
    }

    fn execute_trace(&self) -> Vec<Value> {
        let mut trace = Vec::new();
        let mut env: HashMap<String, String> = HashMap::new();
        let mut step = 0;

        for inst in &self.instructions {
            step += 1;
            let entry = match inst {
                Instruction::Entry(name) => {
                    json!({"step": step, "op": "ENTRY", "name": name})
                }
                Instruction::Fetch { target, source } => {
                    env.insert(target.clone(), format!("<fetched:{source}>"));
                    json!({"step": step, "op": "FETCH", "target": target, "source": source})
                }
                Instruction::Bind { name, value } => {
                    env.insert(name.clone(), value.clone());
                    json!({"step": step, "op": "BIND", "name": name, "value": value})
                }
                Instruction::Call { name, args } => {
                    json!({"step": step, "op": "CALL", "function": name, "args": args})
                }
                Instruction::Guard(name) => {
                    json!({"step": step, "op": "GUARD", "name": name})
                }
                Instruction::Rule { negated, body, .. } => {
                    json!({"step": step, "op": "RULE", "negated": negated, "body": body})
                }
                Instruction::Loop { var, collection } => {
                    json!({"step": step, "op": "LOOP", "var": var, "collection": collection})
                }
                Instruction::Conditional { condition, action } => {
                    json!({"step": step, "op": "CONDITIONAL", "condition": condition, "action": action})
                }
                Instruction::Return(val) => {
                    json!({"step": step, "op": "RETURN", "value": val})
                }
                Instruction::Checkpoint(name) => {
                    json!({"step": step, "op": "CHECKPOINT", "name": name})
                }
                Instruction::Const { name, value } => {
                    env.insert(name.clone(), value.clone());
                    json!({"step": step, "op": "CONST", "name": name, "value": value})
                }
                Instruction::Comment(_) => continue,
            };
            trace.push(entry);
        }

        trace
    }
}

async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut html = String::from(
        r#"<html><head><title>codie-runner</title>
<style>body{background:#1a1a2e;color:#e0e0e0;font-family:monospace;max-width:900px;margin:0 auto;padding:2rem}
a{color:#4fc3f7;text-decoration:none}a:hover{text-decoration:underline}
.program{padding:8px 0;border-bottom:1px solid #333}
.stats{color:#888;font-size:0.85rem}</style></head>
<body><h1>codie-runner</h1>
<p class="stats">CODIE program interpreter with execution tracing</p>"#,
    );

    let mut programs: Vec<&CodieProgram> = state.programs.values().collect();
    programs.sort_by_key(|p| &p.name);

    for p in &programs {
        let inst_count = p
            .instructions
            .iter()
            .filter(|i| !matches!(i, Instruction::Comment(_)))
            .count();
        html.push_str(&format!(
            "<div class=\"program\"><a href=\"/run/{name}\">{name}</a> \
             <span class=\"stats\">{lines} lines, {inst} instructions</span></div>",
            name = p.name,
            lines = p.line_count,
            inst = inst_count,
        ));
    }

    html.push_str("</body></html>");
    Html(html)
}

async fn run_program(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.programs.get(&name) {
        Some(program) => {
            let start = std::time::Instant::now();
            let trace = program.execute_trace();
            let elapsed = start.elapsed();

            axum::Json(json!({
                "program": name,
                "line_count": program.line_count,
                "instruction_count": program.instructions.iter()
                    .filter(|i| !matches!(i, Instruction::Comment(_))).count(),
                "trace": trace,
                "trace_steps": trace.len(),
                "latency_us": elapsed.as_micros(),
            }))
        }
        None => axum::Json(json!({"error": "program not found"})),
    }
}

async fn api_list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let programs: Vec<Value> = state
        .programs
        .values()
        .map(|p| {
            json!({
                "name": p.name,
                "line_count": p.line_count,
                "instruction_count": p.instructions.iter()
                    .filter(|i| !matches!(i, Instruction::Comment(_))).count(),
            })
        })
        .collect();
    axum::Json(json!({"programs": programs}))
}

#[tokio::main]
async fn main() {
    let codie_dir = std::env::var("CODIE_DIR")
        .unwrap_or_else(|_| "../../codie-maps".to_string());

    let mut programs = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(&codie_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("codie") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                if let Ok(source) = std::fs::read_to_string(&path) {
                    let program = CodieProgram::parse(&name, &source);
                    eprintln!(
                        "  loaded: {} ({} lines, {} instructions)",
                        name,
                        program.line_count,
                        program
                            .instructions
                            .iter()
                            .filter(|i| !matches!(i, Instruction::Comment(_)))
                            .count()
                    );
                    programs.insert(name, program);
                }
            }
        }
    }

    eprintln!("codie-runner ready: {} programs loaded", programs.len());

    let state = Arc::new(AppState { programs });

    let app = Router::new()
        .route("/", get(index))
        .route("/run/{name}", get(run_program))
        .route("/api/programs", get(api_list))
        .with_state(state);

    let addr = "0.0.0.0:3003";
    eprintln!("codie-runner listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
