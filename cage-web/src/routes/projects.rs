//! Projects page -- `/projects`
//!
//! Project scoping dashboard: per-project secret/tool/model configuration,
//! the ".env killer". Each project gets toggle overrides for what secrets,
//! tools, and models are available.
//!
//! Toggle indicators:
//!   [O] Enabled          (inherited from global, active)
//!   [_] Disabled         (explicitly turned off for this project)
//!   [*] Project-specific (project-only secret or default model)

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

struct SecretEntry {
    name: &'static str,
    category: &'static str,
    scope: SecretScopeData,
}

#[derive(Clone, Copy)]
enum SecretScopeData {
    Enabled,
    Disabled,
    ProjectSpecific,
}

impl SecretScopeData {
    fn shape(self) -> &'static str {
        match self {
            Self::Enabled => "[O]",
            Self::Disabled => "[_]",
            Self::ProjectSpecific => "[*]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Enabled => "Enabled",
            Self::Disabled => "Disabled",
            Self::ProjectSpecific => "Project-specific",
        }
    }
    fn css_class(self) -> &'static str {
        match self {
            Self::Enabled => "scope-enabled",
            Self::Disabled => "scope-disabled",
            Self::ProjectSpecific => "scope-project",
        }
    }
}

struct ToolOverride {
    name: &'static str,
    enabled: bool,
}

struct ModelOverride {
    name: &'static str,
    enabled: bool,
    is_default: bool,
}

struct ProjectEntry {
    id: &'static str,
    name: &'static str,
    last_opened: &'static str,
    is_active: bool,
    secrets: &'static [SecretEntry],
    tools: &'static [ToolOverride],
    models: &'static [ModelOverride],
    env_vars: &'static [(&'static str, &'static str)],
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static PROJECT_A_SECRETS: &[SecretEntry] = &[
    SecretEntry { name: "OPENAI_API_KEY", category: "ApiKey", scope: SecretScopeData::Enabled },
    SecretEntry { name: "BRAVE_API_KEY", category: "ApiKey", scope: SecretScopeData::Enabled },
    SecretEntry { name: "DATABASE_URL", category: "Password", scope: SecretScopeData::Disabled },
    SecretEntry { name: "PROJECT_A_TOKEN", category: "Token", scope: SecretScopeData::ProjectSpecific },
];

static PROJECT_A_TOOLS: &[ToolOverride] = &[
    ToolOverride { name: "web-search", enabled: true },
    ToolOverride { name: "file-reader", enabled: true },
    ToolOverride { name: "code-interpreter", enabled: true },
    ToolOverride { name: "database-query", enabled: false },
    ToolOverride { name: "git-operations", enabled: true },
];

static PROJECT_A_MODELS: &[ModelOverride] = &[
    ModelOverride { name: "Llama 3.1 8B Instruct", enabled: true, is_default: true },
    ModelOverride { name: "CodeLlama 13B", enabled: true, is_default: false },
    ModelOverride { name: "Mistral 7B v0.3", enabled: false, is_default: false },
];

static PROJECT_B_SECRETS: &[SecretEntry] = &[
    SecretEntry { name: "OPENAI_API_KEY", category: "ApiKey", scope: SecretScopeData::Enabled },
    SecretEntry { name: "DATABASE_URL", category: "Password", scope: SecretScopeData::Enabled },
    SecretEntry { name: "STRIPE_SECRET", category: "ApiKey", scope: SecretScopeData::ProjectSpecific },
];

static PROJECT_B_TOOLS: &[ToolOverride] = &[
    ToolOverride { name: "web-search", enabled: false },
    ToolOverride { name: "file-reader", enabled: true },
    ToolOverride { name: "database-query", enabled: true },
    ToolOverride { name: "git-operations", enabled: true },
];

static PROJECT_B_MODELS: &[ModelOverride] = &[
    ModelOverride { name: "Llama 3.1 8B Instruct", enabled: true, is_default: false },
    ModelOverride { name: "CodeLlama 13B", enabled: true, is_default: true },
];

static PROJECTS: &[ProjectEntry] = &[
    ProjectEntry {
        id: "p1a2b3c4",
        name: "gently-core",
        last_opened: "2026-02-24 14:30:00",
        is_active: true,
        secrets: PROJECT_A_SECRETS,
        tools: PROJECT_A_TOOLS,
        models: PROJECT_A_MODELS,
        env_vars: &[
            ("OPENAI_API_KEY", "OPENAI_API_KEY"),
            ("BRAVE_KEY", "BRAVE_API_KEY"),
            ("AUTH_TOKEN", "PROJECT_A_TOKEN"),
        ],
    },
    ProjectEntry {
        id: "p5d6e7f8",
        name: "ecommerce-api",
        last_opened: "2026-02-23 18:15:00",
        is_active: false,
        secrets: PROJECT_B_SECRETS,
        tools: PROJECT_B_TOOLS,
        models: PROJECT_B_MODELS,
        env_vars: &[
            ("OPENAI_API_KEY", "OPENAI_API_KEY"),
            ("DATABASE_URL", "DATABASE_URL"),
            ("STRIPE_SECRET_KEY", "STRIPE_SECRET"),
        ],
    },
    ProjectEntry {
        id: "p9g0h1i2",
        name: "docs-site",
        last_opened: "2026-02-22 10:00:00",
        is_active: false,
        secrets: &[
            SecretEntry { name: "OPENAI_API_KEY", category: "ApiKey", scope: SecretScopeData::Enabled },
        ],
        tools: &[
            ToolOverride { name: "file-reader", enabled: true },
            ToolOverride { name: "web-search", enabled: true },
        ],
        models: &[
            ModelOverride { name: "Llama 3.1 8B Instruct", enabled: true, is_default: true },
        ],
        env_vars: &[
            ("OPENAI_API_KEY", "OPENAI_API_KEY"),
        ],
    },
];

// ---------------------------------------------------------------
//  View Models
// ---------------------------------------------------------------

struct ProjectListView {
    id: String,
    name: String,
    last_opened: String,
    is_active: bool,
    active_shape: String,
    secret_count: usize,
    tool_count: usize,
    model_count: usize,
    default_model: String,
}

struct SecretView {
    name: String,
    category: String,
    scope_shape: String,
    scope_label: String,
    scope_class: String,
}

struct ToolOverrideView {
    name: String,
    enabled: bool,
    enabled_shape: String,
    enabled_class: String,
}

struct ModelOverrideView {
    name: String,
    enabled: bool,
    is_default: bool,
    enabled_shape: String,
    enabled_class: String,
    default_shape: String,
}

struct EnvVarView {
    env_name: String,
    secret_name: String,
}

// ---------------------------------------------------------------
//  Templates
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "projects.html")]
struct ProjectsTemplate {
    layer_label: String,
    layer_badge: String,
    projects: Vec<ProjectListView>,
    total_projects: usize,
    active_project: String,
    can_manage: bool,
}

#[derive(Template)]
#[template(path = "partials/project_detail.html")]
struct ProjectDetailTemplate {
    id: String,
    name: String,
    is_active: bool,
    secrets: Vec<SecretView>,
    tools: Vec<ToolOverrideView>,
    models: Vec<ModelOverrideView>,
    env_vars: Vec<EnvVarView>,
    can_manage: bool,
}

// ---------------------------------------------------------------
//  Data builders
// ---------------------------------------------------------------

fn build_project_list() -> Vec<ProjectListView> {
    PROJECTS
        .iter()
        .map(|p| {
            let default_model = p
                .models
                .iter()
                .find(|m| m.is_default)
                .map(|m| m.name.to_string())
                .unwrap_or_else(|| "none".to_string());

            ProjectListView {
                id: p.id.to_string(),
                name: p.name.to_string(),
                last_opened: p.last_opened.to_string(),
                is_active: p.is_active,
                active_shape: if p.is_active { "[*]" } else { "[ ]" }.to_string(),
                secret_count: p.secrets.len(),
                tool_count: p.tools.iter().filter(|t| t.enabled).count(),
                model_count: p.models.iter().filter(|m| m.enabled).count(),
                default_model,
            }
        })
        .collect()
}

fn build_project_detail(project: &ProjectEntry, can_manage: bool) -> ProjectDetailTemplate {
    let secrets: Vec<SecretView> = project
        .secrets
        .iter()
        .map(|s| SecretView {
            name: s.name.to_string(),
            category: s.category.to_string(),
            scope_shape: s.scope.shape().to_string(),
            scope_label: s.scope.label().to_string(),
            scope_class: s.scope.css_class().to_string(),
        })
        .collect();

    let tools: Vec<ToolOverrideView> = project
        .tools
        .iter()
        .map(|t| ToolOverrideView {
            name: t.name.to_string(),
            enabled: t.enabled,
            enabled_shape: if t.enabled { "[O]" } else { "[_]" }.to_string(),
            enabled_class: if t.enabled {
                "scope-enabled"
            } else {
                "scope-disabled"
            }
            .to_string(),
        })
        .collect();

    let models: Vec<ModelOverrideView> = project
        .models
        .iter()
        .map(|m| ModelOverrideView {
            name: m.name.to_string(),
            enabled: m.enabled,
            is_default: m.is_default,
            enabled_shape: if m.enabled { "[O]" } else { "[_]" }.to_string(),
            enabled_class: if m.enabled {
                "scope-enabled"
            } else {
                "scope-disabled"
            }
            .to_string(),
            default_shape: if m.is_default { "[*]" } else { "[ ]" }.to_string(),
        })
        .collect();

    let env_vars: Vec<EnvVarView> = project
        .env_vars
        .iter()
        .map(|(env, secret)| EnvVarView {
            env_name: env.to_string(),
            secret_name: secret.to_string(),
        })
        .collect();

    ProjectDetailTemplate {
        id: project.id.to_string(),
        name: project.name.to_string(),
        is_active: project.is_active,
        secrets,
        tools,
        models,
        env_vars,
        can_manage,
    }
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects", get(projects_page))
        .route("/projects/{id}", get(project_detail))
        .route("/projects/create", post(project_create))
        .route("/projects/{id}/open", post(project_open))
        .route("/projects/{id}/secret/{name}/toggle", post(secret_toggle))
        .route("/projects/{id}/tool/{tool_id}/toggle", post(tool_toggle))
        .route("/projects/{id}/model/{model_id}/toggle", post(model_toggle))
        .route("/projects/{id}/model/{model_id}/default", post(model_set_default))
}

async fn projects_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let can_manage = layer.has_access(Layer::RootUser);
    let projects = build_project_list();

    let active_project = PROJECTS
        .iter()
        .find(|p| p.is_active)
        .map(|p| p.name.to_string())
        .unwrap_or_else(|| "none".to_string());

    let content = ProjectsTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        projects,
        total_projects: PROJECTS.len(),
        active_project,
        can_manage,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Projects", &content))
    }
}

async fn project_detail(
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);
    let can_manage = layer.has_access(Layer::RootUser);

    let project = PROJECTS.iter().find(|p| p.id == id);

    let content = match project {
        Some(p) => build_project_detail(p, can_manage)
            .render()
            .unwrap_or_default(),
        None => "<div class=\"flash-warn\">Project not found</div>".to_string(),
    };

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Project Detail", &content))
    }
}

async fn project_create() -> impl IntoResponse {
    Html("<div class=\"flash-ok\">Project created (mock)</div>".to_string())
}

async fn project_open(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Project {} activated (mock)</div>",
        crate::routes::html_escape(&id)
    ))
}

async fn secret_toggle(
    axum::extract::Path((id, name)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Secret {} toggled in project {} (mock)</div>",
        crate::routes::html_escape(&name),
        crate::routes::html_escape(&id)
    ))
}

async fn tool_toggle(
    axum::extract::Path((id, tool_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Tool {} toggled in project {} (mock)</div>",
        crate::routes::html_escape(&tool_id),
        crate::routes::html_escape(&id)
    ))
}

async fn model_toggle(
    axum::extract::Path((id, model_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Model {} toggled in project {} (mock)</div>",
        crate::routes::html_escape(&model_id),
        crate::routes::html_escape(&id)
    ))
}

async fn model_set_default(
    axum::extract::Path((id, model_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    Html(format!(
        "<div class=\"flash-ok\">Model {} set as default in project {} (mock)</div>",
        crate::routes::html_escape(&model_id),
        crate::routes::html_escape(&id)
    ))
}
