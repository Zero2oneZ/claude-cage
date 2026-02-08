mod codie_parser;
mod routes;
mod subprocess;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use axum::Router;
use tower_http::services::ServeDir;

use codie_parser::Program;

pub struct AppState {
    pub cage_root: PathBuf,
    pub store_js: PathBuf,
    pub tree_path: PathBuf,
    pub codie_dir: PathBuf,
    pub codie_programs: RwLock<Vec<Program>>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let cage_root = std::env::var("CAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
        });

    let state = Arc::new(AppState {
        store_js: cage_root.join("mongodb/store.js"),
        tree_path: cage_root.join("gentlyos/tree.json"),
        codie_dir: cage_root.join("projects/Gently-nix/tools/codie-maps"),
        cage_root,
        codie_programs: RwLock::new(Vec::new()),
    });

    // CLI mode: --seed-codie
    if args.iter().any(|a| a == "--seed-codie") {
        routes::codie::seed_codie(state.clone()).await;
        return;
    }

    // CLI mode: --parse-codie <FILE>
    if let Some(pos) = args.iter().position(|a| a == "--parse-codie") {
        if let Some(file) = args.get(pos + 1) {
            routes::codie::parse_codie_file(file).await;
        } else {
            eprintln!("Usage: cage-web --parse-codie <FILE>");
        }
        return;
    }

    // Load CODIE programs at startup
    {
        let programs = codie_parser::load_all(&state.codie_dir);
        let mut lock = state.codie_programs.write().await;
        *lock = programs;
    }

    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");

    let app = Router::new()
        .merge(routes::pages::router())
        .merge(routes::health::router())
        .merge(routes::sessions::router())
        .merge(routes::gentlyos::router())
        .merge(routes::codie::router())
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state);

    let bind = std::env::var("CAGE_WEB_BIND").unwrap_or_else(|_| "0.0.0.0:5000".to_string());
    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {bind}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!("cage-web listening on {bind}");
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    }
}
