//! Emoji Rewriter -- `/emoji-rewriter`
//!
//! Emoji-to-glyph rewrite pipeline display for the GentlyOS IO surface.
//! Pipeline stages, rules, and kill counts inlined from gently-sploit.

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Types inlined from gently_sploit::emoji
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewriteAction {
    Map,
    Drop,
    Flag,
    Block,
}

#[derive(Debug, Clone)]
struct PipelineStage {
    order: u8,
    name: &'static str,
    description: &'static str,
    drops: u32,
    maps: u32,
}

#[derive(Debug, Clone)]
struct RewriteRule {
    category: &'static str,
    pattern: &'static str,
    description: &'static str,
    action: RewriteAction,
    codepoint_range: &'static str,
    kill_count: u32,
    example_input: &'static str,
    example_output: &'static str,
}

static PIPELINE: &[PipelineStage] = &[
    PipelineStage { order: 1, name: "Codepoint Scanner", description: "Extract codepoints from UTF-8 input, handle surrogate pairs", drops: 0, maps: 0 },
    PipelineStage { order: 2, name: "ZWJ Stripper", description: "Remove Zero-Width Joiners, decompose compound sequences", drops: 1, maps: 0 },
    PipelineStage { order: 3, name: "Modifier Scrub", description: "Strip skin tone, variation selectors, keycap combining, tags", drops: 104, maps: 0 },
    PipelineStage { order: 4, name: "Flag Collapser", description: "Collapse Regional Indicator pairs to single flag glyph", drops: 0, maps: 26 },
    PipelineStage { order: 5, name: "Glyph Mapper", description: "Map known emoji codepoints to GentlyOS hex addresses", drops: 0, maps: 290 },
    PipelineStage { order: 6, name: "Unknown Catcher", description: "Flag unmapped emoji as glyph-unknown (0xFF00)", drops: 0, maps: 0 },
    PipelineStage { order: 7, name: "PUA Blocker", description: "Block Private Use Area codepoints entirely", drops: 0, maps: 0 },
    PipelineStage { order: 8, name: "Wire Encoder", description: "Encode glyph stream as GENTLY-MSG-V1 with IPFS CIDs", drops: 0, maps: 0 },
];

static RULES: &[RewriteRule] = &[
    RewriteRule { category: "ZWJ Sequences", pattern: "U+200D (Zero-Width Joiner)", description: "Silently joins emoji into compound glyphs. Stripped to decompose back to atomic units.", action: RewriteAction::Drop, codepoint_range: "U+200D", kill_count: 1, example_input: "man + ZWJ + computer", example_output: "[0x0301:smile] [0x0400:computer]" },
    RewriteRule { category: "Skin Tone Modifiers", pattern: "U+1F3FB..U+1F3FF", description: "Fitzpatrick skin tone modifiers. Dropped -- glyphs are tone-neutral by design.", action: RewriteAction::Drop, codepoint_range: "U+1F3FB-U+1F3FF", kill_count: 5, example_input: "wave + medium tone", example_output: "[0x0001:wave]" },
    RewriteRule { category: "Variation Selectors", pattern: "U+FE0E, U+FE0F", description: "Text/emoji presentation selectors. Dropped -- GentlyOS controls rendering.", action: RewriteAction::Drop, codepoint_range: "U+FE0E-U+FE0F", kill_count: 2, example_input: "heart + VS16", example_output: "[0x0300:heart]" },
    RewriteRule { category: "Keycap Sequences", pattern: "U+20E3 (Combining Enclosing Keycap)", description: "Combines with digits to form keycap emoji. Stripped to plain digit.", action: RewriteAction::Drop, codepoint_range: "U+20E3", kill_count: 1, example_input: "3 + VS16 + keycap", example_output: "3" },
    RewriteRule { category: "Regional Indicators", pattern: "U+1F1E6..U+1F1FF", description: "Flag emoji pairs. Mapped to system flag glyph -- no national flags rendered.", action: RewriteAction::Map, codepoint_range: "U+1F1E6-U+1F1FF", kill_count: 26, example_input: "US flag", example_output: "[0x0F00:flag]" },
    RewriteRule { category: "Tag Characters", pattern: "U+E0020..U+E007F", description: "Subdivision flag tags (invisible). Dropped entirely -- stego risk vector.", action: RewriteAction::Drop, codepoint_range: "U+E0020-U+E007F", kill_count: 96, example_input: "England flag tags", example_output: "[0x0F00:flag]" },
    RewriteRule { category: "Emoticons", pattern: "U+1F600..U+1F64F", description: "Smileys and emotion faces. Mapped to GentlyOS emotion glyphs.", action: RewriteAction::Map, codepoint_range: "U+1F600-U+1F64F", kill_count: 80, example_input: "laugh think fear", example_output: "[0x0302:laugh] [0x0307:think] [0x0305:fear]" },
    RewriteRule { category: "Gesture & Body", pattern: "U+1F44B..U+1F596", description: "Hand gestures, body parts. Mapped to gesture glyphs.", action: RewriteAction::Map, codepoint_range: "U+1F44B-U+1F596", kill_count: 50, example_input: "wave thumbs-up peace", example_output: "[0x0001:wave] [0x0002:thumbs-up] [0x0009:peace]" },
    RewriteRule { category: "Objects & Symbols", pattern: "U+1F4BB..U+1F6E1", description: "Common objects (computer, phone, house, etc). Mapped to object glyphs.", action: RewriteAction::Map, codepoint_range: "U+1F4BB-U+1F6E1", kill_count: 120, example_input: "computer phone house key", example_output: "[0x0400:computer] [0x0401:phone] [0x0402:house] [0x0408:key]" },
    RewriteRule { category: "Nature & Weather", pattern: "U+2600..U+1F33F", description: "Sun, moon, stars, clouds, trees. Mapped to nature glyphs.", action: RewriteAction::Map, codepoint_range: "U+2600-U+1F33F", kill_count: 40, example_input: "sun moon star wave-n", example_output: "[0x0500:sun] [0x0501:moon] [0x0502:star] [0x0505:wave-n]" },
    RewriteRule { category: "Unknown Emoji", pattern: "U+1F000..U+1FBFF (unmapped)", description: "Any emoji in the supplementary range without a mapped glyph. Mapped to glyph-unknown.", action: RewriteAction::Flag, codepoint_range: "U+1F000-U+1FBFF", kill_count: 0, example_input: "mahjong nazar amulet hamsa", example_output: "[0xFF00:glyph-unknown] x3" },
    RewriteRule { category: "Private Use Area", pattern: "U+E000..U+F8FF, U+F0000..U+10FFFF", description: "Private Use Area codepoints. Blocked -- cannot verify intent or rendering.", action: RewriteAction::Block, codepoint_range: "U+E000-U+F8FF", kill_count: 0, example_input: "(vendor-specific icons)", example_output: "BLOCKED" },
];

fn total_kill_count() -> u32 {
    RULES.iter().map(|r| r.kill_count).sum()
}

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

fn action_class(a: RewriteAction) -> &'static str {
    match a {
        RewriteAction::Map => "action-map",
        RewriteAction::Drop => "action-drop",
        RewriteAction::Flag => "action-flag",
        RewriteAction::Block => "action-block",
    }
}

fn action_label(a: RewriteAction) -> &'static str {
    match a {
        RewriteAction::Map => "MAP",
        RewriteAction::Drop => "DROP",
        RewriteAction::Flag => "FLAG",
        RewriteAction::Block => "BLOCK",
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

    let rules: Vec<RuleView> = RULES
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

    let pipeline: Vec<StageView> = PIPELINE
        .iter()
        .map(|s| StageView {
            order: s.order,
            name: s.name.to_string(),
            description: s.description.to_string(),
            drops: s.drops,
            maps: s.maps,
        })
        .collect();

    let total_kills = total_kill_count();
    let total_drops: u32 = PIPELINE.iter().map(|s| s.drops).sum();
    let total_maps: u32 = PIPELINE.iter().map(|s| s.maps).sum();

    let content = EmojiRewriterTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        rules,
        pipeline,
        total_rules: RULES.len(),
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
