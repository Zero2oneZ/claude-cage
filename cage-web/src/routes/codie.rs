use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;

use crate::codie_parser::{self, Program};
use crate::routes::{html_escape, is_htmx, wrap_page};
use crate::subprocess;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/codie", get(codie_list))
        .route("/codie/{name}", get(codie_detail))
        .route("/codie/{name}/execute", post(codie_execute))
        .route("/codie/parse", post(codie_parse))
}

#[derive(Template)]
#[template(path = "codie.html")]
struct CodieListTemplate {
    programs: Vec<ProgramSummary>,
}

#[derive(Template)]
#[template(path = "codie_program.html")]
struct CodieProgramTemplate {
    name: String,
    entry_point: String,
    line_count: usize,
    instruction_count: usize,
    source: String,
    keywords: Vec<(String, usize)>,
}

struct ProgramSummary {
    name: String,
    entry_point: String,
    line_count: usize,
    instruction_count: usize,
    keyword_total: usize,
}

async fn codie_list(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let programs = state.codie_programs.read().await;

    let summaries: Vec<ProgramSummary> = programs
        .iter()
        .map(|p| ProgramSummary {
            name: p.name.clone(),
            entry_point: p.entry_point().unwrap_or("(none)").to_string(),
            line_count: p.line_count,
            instruction_count: p.instruction_count(),
            keyword_total: p.keyword_counts.values().sum(),
        })
        .collect();

    let content = CodieListTemplate {
        programs: summaries,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("CODIE Programs", &content))
    }
}

async fn codie_detail(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let programs = state.codie_programs.read().await;
    let program = programs.iter().find(|p| p.name == name);

    match program {
        Some(p) => {
            let mut keywords: Vec<(String, usize)> =
                p.keyword_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
            keywords.sort_by(|a, b| b.1.cmp(&a.1));

            let title = format!("CODIE: {name}");
            let content = CodieProgramTemplate {
                name: p.name.clone(),
                entry_point: p.entry_point().unwrap_or("(none)").to_string(),
                line_count: p.line_count,
                instruction_count: p.instruction_count(),
                source: p.source.clone(),
                keywords,
            }
            .render()
            .unwrap_or_default();

            if is_htmx(&headers) {
                Html(content)
            } else {
                Html(wrap_page(&title, &content))
            }
        }
        None => {
            let safe_name = html_escape(&name);
            Html(format!(
                "<div class=\"error\">Program '{}' not found</div>",
                safe_name
            ))
        }
    }
}

#[derive(Deserialize)]
struct ExecuteForm {
    intent: Option<String>,
}

async fn codie_execute(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    axum::Form(form): axum::Form<ExecuteForm>,
) -> impl IntoResponse {
    let programs = state.codie_programs.read().await;
    let program = programs.iter().find(|p| p.name == name);

    match program {
        Some(p) => {
            let intent = form.intent.unwrap_or_else(|| format!("execute {name}"));

            // Log the execution to MongoDB
            let _ = subprocess::mongo_log(
                &state.store_js,
                "coordination:phase",
                &format!("EXECUTE:codie-{name}"),
                &json!({"intent": intent, "program": name}).to_string(),
            )
            .await;

            // Execute via PTC engine
            let task_json = json!({
                "node_id": "capt:codie",
                "intent": intent,
                "codie_program": name,
                "files": [],
            })
            .to_string();

            let ptc_result = subprocess::ptc_execute(&state.cage_root, &task_json).await;

            let (status, output) = match ptc_result {
                Ok(out) => ("executed", out),
                Err(e) => ("error", e),
            };

            let result = json!({
                "status": status,
                "program": name,
                "entry_point": p.entry_point(),
                "instruction_count": p.instruction_count(),
                "intent": intent,
                "ptc_output": output,
            });

            let safe_name = html_escape(&name);
            let safe_intent = html_escape(&intent);
            let safe_entry = html_escape(p.entry_point().unwrap_or("(none)"));
            let json_pretty = serde_json::to_string_pretty(&result).unwrap_or_default();
            let safe_json = html_escape(&json_pretty);

            Html(format!(
                "<div class=\"result\">\
                    <h3>Execution: {safe_name}</h3>\
                    <p>Status: {status}</p>\
                    <p>Entry: {safe_entry}</p>\
                    <p>Instructions: {}</p>\
                    <p>Intent: {safe_intent}</p>\
                    <pre>{safe_json}</pre>\
                </div>",
                p.instruction_count(),
            ))
        }
        None => {
            let safe_name = html_escape(&name);
            Html(format!(
                "<div class=\"error\">Program '{}' not found</div>",
                safe_name
            ))
        }
    }
}

#[derive(Deserialize)]
struct ParseForm {
    source: String,
}

async fn codie_parse(axum::Form(form): axum::Form<ParseForm>) -> impl IntoResponse {
    match Program::parse("user-input", &form.source) {
        Ok(program) => {
            let result = program.to_json();
            let safe_entry = html_escape(program.entry_point().unwrap_or("(none)"));
            let json_pretty = serde_json::to_string_pretty(&result).unwrap_or_default();
            let safe_json = html_escape(&json_pretty);

            Html(format!(
                "<div class=\"result\">\
                    <h3>Parse Result</h3>\
                    <p>Entry: {safe_entry}</p>\
                    <p>Lines: {}, Instructions: {}</p>\
                    <pre>{safe_json}</pre>\
                </div>",
                program.line_count,
                program.instruction_count(),
            ))
        }
        Err(e) => {
            let safe_error = html_escape(&e.to_string());
            Html(format!(
                "<div class=\"error\">Parse error: {safe_error}</div>"
            ))
        }
    }
}

/// CLI: seed all .codie programs into MongoDB.
pub async fn seed_codie(state: Arc<AppState>) {
    eprintln!("Seeding CODIE programs from {}...", state.codie_dir.display());

    let programs = codie_parser::load_all(&state.codie_dir);
    eprintln!("Parsed {} programs, seeding to MongoDB...", programs.len());

    for program in &programs {
        let doc = json!({
            "name": program.name,
            "source": program.source,
            "parsed": {
                "entry_point": program.entry_point(),
                "instruction_count": program.instruction_count(),
                "keyword_counts": program.keyword_counts,
                "node_count": program.nodes.len(),
            },
            "metadata": {
                "file": format!("{}.codie", program.name),
                "line_count": program.line_count,
                "size_bytes": program.source.len(),
            },
            "project": "claude-cage",
            "_ts": chrono_now(),
        });

        match subprocess::mongo_put(
            &state.store_js,
            "codie_programs",
            &doc.to_string(),
        )
        .await
        {
            Ok(_) => eprintln!("  Seeded: {}", program.name),
            Err(e) => eprintln!("  WARN: Failed to seed {}: {e}", program.name),
        }
    }

    // Log the seeding event
    let _ = subprocess::mongo_log(
        &state.store_js,
        "coordination:phase",
        "INTAKE:codie-seed",
        &json!({"programs": programs.len()}).to_string(),
    )
    .await;

    eprintln!("Done. Seeded {} CODIE programs.", programs.len());
}

/// CLI: parse a single .codie file and print the AST.
pub async fn parse_codie_file(path: &str) {
    let path = std::path::Path::new(path);
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    match std::fs::read_to_string(path) {
        Ok(source) => match Program::parse(name, &source) {
            Ok(program) => {
                println!("{}", serde_json::to_string_pretty(&program.to_json()).unwrap());
            }
            Err(e) => {
                eprintln!("Parse error: {e}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Failed to read {}: {e}", path.display());
            std::process::exit(1);
        }
    }
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 UTC without external crate
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Days since epoch to y/m/d (simplified civil calendar)
    let mut y = 1970i64;
    let mut rem = days as i64;
    loop {
        let ylen = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if rem < ylen { break; }
        rem -= ylen;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 0usize;
    for md in &mdays {
        if rem < *md { break; }
        rem -= md;
        mo += 1;
    }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", mo + 1, rem + 1)
}
