use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/tier", get(tier_page))
}

struct TierRow {
    level: u8,
    name: &'static str,
    nft_tier: &'static str,
    badge_class: &'static str,
    features: Vec<&'static str>,
    is_current: bool,
}

#[derive(Template)]
#[template(path = "tier.html")]
struct TierTemplate {
    tiers: Vec<TierRow>,
    current_label: String,
    current_badge: String,
}

async fn tier_page(
    headers: HeaderMap,
    State(_state): State<Arc<AppState>>,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext.extensions().get::<Layer>().copied().unwrap_or(Layer::User);

    let tiers = vec![
        TierRow {
            level: 0,
            name: "Admin (L0)",
            nft_tier: "founder/admin",
            badge_class: "tier-founder",
            features: vec![
                "Full system access",
                "NixOS config",
                "Kernel modules",
                "Raw hardware",
                "Contract owner",
                "Upstream push",
            ],
            is_current: layer == Layer::Admin,
        },
        TierRow {
            level: 1,
            name: "GentlyDev (L1)",
            nft_tier: "internal",
            badge_class: "tier-dev",
            features: vec![
                "Internal team access",
                "Debug tools",
                "Source maps",
            ],
            is_current: layer == Layer::GentlyDev,
        },
        TierRow {
            level: 2,
            name: "DevLevel (L2)",
            nft_tier: "dev",
            badge_class: "tier-dev",
            features: vec![
                "Limbo layer",
                "Offensive tools",
                "Wine",
                "Full agent swarm (34)",
                "Contract deploy",
            ],
            is_current: layer == Layer::DevLevel,
        },
        TierRow {
            level: 3,
            name: "OsAdmin (L3)",
            nft_tier: "pro",
            badge_class: "tier-pro",
            features: vec![
                "Docker containers",
                "Agent swarm (8)",
                "Unlimited fork tree",
                "Unlimited env vault",
            ],
            is_current: layer == Layer::OsAdmin,
        },
        TierRow {
            level: 4,
            name: "RootUser (L4)",
            nft_tier: "basic",
            badge_class: "tier-basic",
            features: vec![
                "Workbench pane",
                "Python bridge",
                "Fork tree (depth 5)",
                "Env vault (10 keys)",
            ],
            is_current: layer == Layer::RootUser,
        },
        TierRow {
            level: 5,
            name: "User (L5)",
            nft_tier: "free",
            badge_class: "tier-free",
            features: vec![
                "GuardDog DNS",
                "Claude chat",
                "Core tools",
            ],
            is_current: layer == Layer::User,
        },
    ];

    let content = TierTemplate {
        current_label: layer.label().to_string(),
        current_badge: layer.badge_class().to_string(),
        tiers,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Tier Hierarchy", &content))
    }
}
