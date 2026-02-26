//! Models page -- `/models`
//!
//! Local model management dashboard: registry, VRAM bars, daemon status,
//! load/unload/delete actions. All static mock data.
//!
//! Shape indicators (colorblind-safe):
//!   [O]  Ready     (model loaded, inference available)
//!   [/\] Stopped   (registered but not loaded)
//!   [<>] Loading   (daemon starting)
//!   [#]  Error     (daemon crashed)
//!
//! HTMX partial: /partials/model-rows

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data Model
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelStatus {
    Ready,
    Stopped,
    Loading,
    Error,
}

impl ModelStatus {
    fn shape(self) -> &'static str {
        match self {
            Self::Ready => "[O]",
            Self::Stopped => "[/\\]",
            Self::Loading => "[<>]",
            Self::Error => "[#]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Stopped => "STOPPED",
            Self::Loading => "LOADING",
            Self::Error => "ERROR",
        }
    }
    fn css_class(self) -> &'static str {
        match self {
            Self::Ready => "model-ready",
            Self::Stopped => "model-stopped",
            Self::Loading => "model-loading",
            Self::Error => "model-error",
        }
    }
}

struct ModelEntry {
    id: &'static str,
    name: &'static str,
    format: &'static str,
    quantization: &'static str,
    parameters: &'static str,
    size_gb: f32,
    vram_required_mb: u64,
    vram_used_mb: u64,
    gpu_total_mb: u64,
    status: ModelStatus,
    runtime: &'static str,
    source: &'static str,
    times_loaded: u64,
    last_used: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static MODELS: &[ModelEntry] = &[
    ModelEntry {
        id: "a1b2c3d4",
        name: "Llama 3.1 8B Instruct",
        format: "GGUF",
        quantization: "Q4_K_M",
        parameters: "8B",
        size_gb: 4.9,
        vram_required_mb: 5120,
        vram_used_mb: 5120,
        gpu_total_mb: 24576,
        status: ModelStatus::Ready,
        runtime: "llama.cpp",
        source: "HuggingFace: meta-llama/Meta-Llama-3.1-8B-Instruct",
        times_loaded: 47,
        last_used: "2026-02-24 14:15:00",
    },
    ModelEntry {
        id: "e5f6g7h8",
        name: "Mistral 7B v0.3",
        format: "GGUF",
        quantization: "Q5_K_M",
        parameters: "7B",
        size_gb: 5.3,
        vram_required_mb: 5632,
        vram_used_mb: 0,
        gpu_total_mb: 24576,
        status: ModelStatus::Stopped,
        runtime: "llama.cpp",
        source: "HuggingFace: mistralai/Mistral-7B-Instruct-v0.3",
        times_loaded: 12,
        last_used: "2026-02-23 09:30:00",
    },
    ModelEntry {
        id: "i9j0k1l2",
        name: "CodeLlama 13B",
        format: "GGUF",
        quantization: "Q4_K_M",
        parameters: "13B",
        size_gb: 7.8,
        vram_required_mb: 8192,
        vram_used_mb: 8192,
        gpu_total_mb: 24576,
        status: ModelStatus::Ready,
        runtime: "llama.cpp",
        source: "Ollama: codellama:13b",
        times_loaded: 31,
        last_used: "2026-02-24 14:20:00",
    },
    ModelEntry {
        id: "m3n4o5p6",
        name: "Phi-3 Mini",
        format: "GGUF",
        quantization: "Q4_0",
        parameters: "3.8B",
        size_gb: 2.2,
        vram_required_mb: 2560,
        vram_used_mb: 0,
        gpu_total_mb: 24576,
        status: ModelStatus::Stopped,
        runtime: "llama.cpp",
        source: "LocalImport: /mnt/usb/models/phi-3-mini.gguf",
        times_loaded: 5,
        last_used: "2026-02-22 16:00:00",
    },
    ModelEntry {
        id: "q7r8s9t0",
        name: "Whisper Large v3",
        format: "SafeTensors",
        quantization: "FP16",
        parameters: "1.5B",
        size_gb: 3.1,
        vram_required_mb: 3072,
        vram_used_mb: 0,
        gpu_total_mb: 24576,
        status: ModelStatus::Error,
        runtime: "custom",
        source: "HuggingFace: openai/whisper-large-v3",
        times_loaded: 2,
        last_used: "2026-02-20 11:45:00",
    },
];

// ---------------------------------------------------------------
//  View Models
// ---------------------------------------------------------------

struct ModelView {
    id: String,
    name: String,
    format: String,
    quantization: String,
    parameters: String,
    size_gb: String,
    status_shape: String,
    status_label: String,
    status_class: String,
    runtime: String,
    source: String,
    times_loaded: u64,
    last_used: String,
    vram_bar: String,
    vram_text: String,
    is_loaded: bool,
    can_load: bool,
    can_unload: bool,
}

// ---------------------------------------------------------------
//  Templates
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "models.html")]
struct ModelsTemplate {
    layer_label: String,
    layer_badge: String,
    models: Vec<ModelView>,
    total_models: usize,
    loaded_count: usize,
    total_vram_used_mb: u64,
    total_vram_mb: u64,
    vram_bar: String,
    vram_text: String,
    can_manage: bool,
}

#[derive(Template)]
#[template(path = "partials/model_row.html")]
struct ModelRowPartial {
    models: Vec<ModelView>,
}

// ---------------------------------------------------------------
//  Data builders
// ---------------------------------------------------------------

fn vram_bar(used: u64, total: u64) -> String {
    let ratio = if total > 0 {
        (used as f64 / total as f64).min(1.0)
    } else {
        0.0
    };
    let filled = (ratio * 12.0) as usize;
    let empty = 12 - filled;
    format!(
        "[{}{}]",
        "#".repeat(filled),
        ".".repeat(empty)
    )
}

fn vram_text(used: u64, total: u64) -> String {
    format!(
        "{:.1} / {:.1} GB",
        used as f64 / 1024.0,
        total as f64 / 1024.0
    )
}

fn build_models(can_manage: bool) -> Vec<ModelView> {
    MODELS
        .iter()
        .map(|m| {
            let is_loaded = matches!(m.status, ModelStatus::Ready | ModelStatus::Loading);
            ModelView {
                id: m.id.to_string(),
                name: m.name.to_string(),
                format: m.format.to_string(),
                quantization: m.quantization.to_string(),
                parameters: m.parameters.to_string(),
                size_gb: format!("{:.1} GB", m.size_gb),
                status_shape: m.status.shape().to_string(),
                status_label: m.status.label().to_string(),
                status_class: m.status.css_class().to_string(),
                runtime: m.runtime.to_string(),
                source: m.source.to_string(),
                times_loaded: m.times_loaded,
                last_used: m.last_used.to_string(),
                vram_bar: vram_bar(m.vram_used_mb, m.gpu_total_mb),
                vram_text: vram_text(m.vram_used_mb, m.gpu_total_mb),
                is_loaded,
                can_load: can_manage && !is_loaded,
                can_unload: can_manage && is_loaded,
            }
        })
        .collect()
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/models", get(models_page))
        .route("/models/{id}/load", post(model_load))
        .route("/models/{id}/unload", post(model_unload))
        .route("/models/{id}/delete", post(model_delete))
        .route("/partials/model-rows", get(model_rows_partial))
}

async fn models_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let can_manage = layer.has_access(Layer::RootUser);
    let models = build_models(can_manage);

    let loaded_count = MODELS
        .iter()
        .filter(|m| matches!(m.status, ModelStatus::Ready | ModelStatus::Loading))
        .count();
    let total_vram_used: u64 = MODELS.iter().map(|m| m.vram_used_mb).sum();
    let total_vram: u64 = 24576; // Single GPU total

    let content = ModelsTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        models,
        total_models: MODELS.len(),
        loaded_count,
        total_vram_used_mb: total_vram_used,
        total_vram_mb: total_vram,
        vram_bar: vram_bar(total_vram_used, total_vram),
        vram_text: vram_text(total_vram_used, total_vram),
        can_manage,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Models", &content))
    }
}

async fn model_load(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Model {} load requested (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn model_unload(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Model {} unload requested (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn model_delete(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-warn\">Model {} delete requested (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn model_rows_partial(_headers: HeaderMap) -> impl IntoResponse {
    let models = build_models(false);
    Html(
        ModelRowPartial { models }
            .render()
            .unwrap_or_default(),
    )
}
