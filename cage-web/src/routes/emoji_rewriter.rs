//! Emoji Rewriter -- `/emoji-rewriter`
//!
//! Thin UI wrapper over gently_sploit::emoji for pipeline display.
//! All pipeline logic, rules, and kill counts live in gently-sploit crate.

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// Re-use gently-sploit emoji pipeline
use gently_sploit::emoji;

// ---------------------------------------------------------------
//  Template Data
// ---------------------------------------------------------------

struct RuleView {
    category: String,
    pattern: String,
    description: String,
    action_class: String,
    action_label: String,
    codepoint_range: String,
    kill_count: u32,
    example_input: String,
    example_output: String,
}

struct StageView {
    order: u8,
    name: String,
    description: String,
    drops: u32,
    maps: u32,
}

#[derive(Template)]
#[template(path = "emoji_rewriter.html")]
struct EmojiRewriterTemplate {
    layer_label: String,
    layer_badge: String,
    rules: Vec<RuleView>,
    pipeline: Vec<StageView>,
    total_rules: usize,
    total_kills: u32,
    total_drops: u32,
    total_maps: u32,
    can_test: bool,
    can_modify_rules: bool,
}

fn action_class(a: emoji::RewriteAction) -> &'static str {
    match a {
        emoji::RewriteAction::Map => "action-map",
        emoji::RewriteAction::Drop => "action-drop",
        emoji::RewriteAction::Flag => "action-flag",
        emoji::RewriteAction::Block => "action-block",
    }
}

fn action_label(a: emoji::RewriteAction) -> &'static str {
    match a {
        emoji::RewriteAction::Map => "MAP",
        emoji::RewriteAction::Drop => "DROP",
        emoji::RewriteAction::Flag => "FLAG",
        emoji::RewriteAction::Block => "BLOCK",
    }
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/emoji-rewriter", get(emoji_rewriter_page))
}

async fn emoji_rewriter_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let rules: Vec<RuleView> = emoji::RULES
        .iter()
        .map(|r| RuleView {
            category: r.category.to_string(),
            pattern: r.pattern.to_string(),
            description: r.description.to_string(),
            action_class: action_class(r.action).to_string(),
            action_label: action_label(r.action).to_string(),
            codepoint_range: r.codepoint_range.to_string(),
            kill_count: r.kill_count,
            example_input: r.example_input.to_string(),
            example_output: r.example_output.to_string(),
        })
        .collect();

    let pipeline: Vec<StageView> = emoji::PIPELINE
        .iter()
        .map(|s| StageView {
            order: s.order,
            name: s.name.to_string(),
            description: s.description.to_string(),
            drops: s.drops,
            maps: s.maps,
        })
        .collect();

    let total_kills = emoji::total_kill_count();
    let total_drops: u32 = emoji::PIPELINE.iter().map(|s| s.drops).sum();
    let total_maps: u32 = emoji::PIPELINE.iter().map(|s| s.maps).sum();

    let content = EmojiRewriterTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        rules,
        pipeline,
        total_rules: emoji::RULES.len(),
        total_kills,
        total_drops,
        total_maps,
        can_test: layer.has_access(Layer::RootUser),
        can_modify_rules: layer.has_access(Layer::DevLevel),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Emoji Rewriter", &content))
    }
}
