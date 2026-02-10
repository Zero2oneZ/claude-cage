//! ToS Interceptor -- `/tos-interceptor`
//!
//! Intercepts and analyzes terms-of-service before user acceptance.
//! Flags concerning clauses (data collection, arbitration, IP assignment,
//! liability waivers) and shows a risk assessment for each service.

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
//  Data Model
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseRisk {
    Safe,
    Caution,
    Danger,
    Hostile,
}

impl ClauseRisk {
    fn class(self) -> &'static str {
        match self {
            Self::Safe => "clause-safe",
            Self::Caution => "clause-caution",
            Self::Danger => "clause-danger",
            Self::Hostile => "clause-hostile",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Caution => "CAUTION",
            Self::Danger => "DANGER",
            Self::Hostile => "HOSTILE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServiceVerdict {
    Accept,
    ReviewRequired,
    Reject,
}

impl ServiceVerdict {
    fn class(self) -> &'static str {
        match self {
            Self::Accept => "verdict-accept",
            Self::ReviewRequired => "verdict-review",
            Self::Reject => "verdict-reject",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::ReviewRequired => "REVIEW REQUIRED",
            Self::Reject => "REJECT",
        }
    }
}

struct FlaggedClause {
    section: &'static str,
    category: &'static str,
    risk: ClauseRisk,
    summary: &'static str,
    plain_english: &'static str,
}

struct ServiceTos {
    service: &'static str,
    url: &'static str,
    last_updated: &'static str,
    word_count: u32,
    read_time_min: u32,
    verdict: ServiceVerdict,
    clauses: &'static [FlaggedClause],
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static SERVICES: &[ServiceTos] = &[
    ServiceTos {
        service: "GitHub",
        url: "github.com/site/terms",
        last_updated: "2024-11-15",
        word_count: 4200,
        read_time_min: 17,
        verdict: ServiceVerdict::Accept,
        clauses: &[
            FlaggedClause {
                section: "Section D",
                category: "IP Rights",
                risk: ClauseRisk::Safe,
                summary: "You retain ownership of your content.",
                plain_english: "Your code stays yours. GitHub gets a license to host/display it, but you own it.",
            },
            FlaggedClause {
                section: "Section D.5",
                category: "License Grant",
                risk: ClauseRisk::Caution,
                summary: "License to run user content on GitHub's servers.",
                plain_english: "They need permission to actually run your code for features like Actions and Codespaces. Standard for hosting platforms.",
            },
            FlaggedClause {
                section: "Section L",
                category: "Disclaimer",
                risk: ClauseRisk::Safe,
                summary: "Standard AS-IS warranty disclaimer.",
                plain_english: "If GitHub goes down and you lose a deploy, they're not liable. Normal for any SaaS.",
            },
        ],
    },
    ServiceTos {
        service: "Google (Workspace)",
        url: "policies.google.com/terms",
        last_updated: "2024-12-20",
        word_count: 3800,
        read_time_min: 15,
        verdict: ServiceVerdict::ReviewRequired,
        clauses: &[
            FlaggedClause {
                section: "Your Content",
                category: "License Grant",
                risk: ClauseRisk::Caution,
                summary: "Broad license to use, host, reproduce, modify, publish, and distribute your content.",
                plain_english: "Google gets a wide license to your content. They say it's for 'operating and improving services,' but the scope is very broad.",
            },
            FlaggedClause {
                section: "Privacy",
                category: "Data Collection",
                risk: ClauseRisk::Danger,
                summary: "Collects search history, location, voice recordings, browsing patterns across all Google services.",
                plain_english: "They track almost everything. Your searches, where you go, what you say to Assistant, sites you visit. All linked to your identity.",
            },
            FlaggedClause {
                section: "AI Features",
                category: "Training Data",
                risk: ClauseRisk::Danger,
                summary: "Content may be used to improve AI models and services.",
                plain_english: "Your docs, emails, and content could train Google's AI. You can opt out in settings, but it's opt-out, not opt-in.",
            },
            FlaggedClause {
                section: "Disputes",
                category: "Arbitration",
                risk: ClauseRisk::Caution,
                summary: "Mandatory arbitration with class-action waiver.",
                plain_english: "You can't sue them in court or join a class action. Disputes go to arbitration, which historically favors corporations.",
            },
        ],
    },
    ServiceTos {
        service: "Anthropic (Claude)",
        url: "anthropic.com/legal/consumer-terms",
        last_updated: "2025-02-01",
        word_count: 2900,
        read_time_min: 12,
        verdict: ServiceVerdict::Accept,
        clauses: &[
            FlaggedClause {
                section: "Inputs/Outputs",
                category: "IP Rights",
                risk: ClauseRisk::Safe,
                summary: "You own your inputs and outputs. Anthropic does not claim rights to your content.",
                plain_english: "What you type in and what Claude generates — you own it. Anthropic doesn't claim any IP.",
            },
            FlaggedClause {
                section: "Safety",
                category: "Usage Monitoring",
                risk: ClauseRisk::Caution,
                summary: "Conversations may be reviewed for safety and abuse prevention.",
                plain_english: "They might look at flagged conversations to catch abuse. Not mass surveillance, but your chats aren't fully private.",
            },
            FlaggedClause {
                section: "API Terms",
                category: "Training Data",
                risk: ClauseRisk::Safe,
                summary: "API inputs are NOT used for training by default.",
                plain_english: "If you use the API, your data doesn't train their models. Clear policy, no opt-out needed.",
            },
        ],
    },
    ServiceTos {
        service: "Facebook (Meta)",
        url: "facebook.com/legal/terms",
        last_updated: "2024-07-26",
        word_count: 5200,
        read_time_min: 21,
        verdict: ServiceVerdict::Reject,
        clauses: &[
            FlaggedClause {
                section: "Section 3",
                category: "License Grant",
                risk: ClauseRisk::Hostile,
                summary: "Non-exclusive, transferable, sub-licensable, royalty-free, worldwide license to your content.",
                plain_english: "They can do almost anything with your photos, posts, and messages. They can sublicense it to anyone. And it's free.",
            },
            FlaggedClause {
                section: "Data Policy",
                category: "Data Collection",
                risk: ClauseRisk::Hostile,
                summary: "Collects data from on-platform activity, off-platform tracking (Pixel, SDK), device sensors, Bluetooth, WiFi, and third-party data brokers.",
                plain_english: "They track you everywhere — on Facebook, across the web via Pixel, from your phone sensors, and they buy data about you from brokers.",
            },
            FlaggedClause {
                section: "Section 3.2",
                category: "Training Data",
                risk: ClauseRisk::Danger,
                summary: "Content used to train AI/ML models including Llama and Meta AI.",
                plain_english: "Your posts, photos, and interactions train Meta's AI models. They announced this broadly in 2024, with limited opt-out.",
            },
            FlaggedClause {
                section: "Section 5",
                category: "Liability",
                risk: ClauseRisk::Danger,
                summary: "Maximum liability capped at $100 or fees paid in last 12 months.",
                plain_english: "If they leak your data or cause you harm, most they'll pay is $100. Even if the damage is millions.",
            },
            FlaggedClause {
                section: "Disputes",
                category: "Arbitration",
                risk: ClauseRisk::Danger,
                summary: "Mandatory binding arbitration, class-action waiver, venue in Northern District of California.",
                plain_english: "Can't sue them, can't join a class action, must fly to California for any dispute. Maximum friction for you.",
            },
        ],
    },
    ServiceTos {
        service: "Amazon (AWS)",
        url: "aws.amazon.com/service-terms",
        last_updated: "2025-01-10",
        word_count: 48000,
        read_time_min: 192,
        verdict: ServiceVerdict::ReviewRequired,
        clauses: &[
            FlaggedClause {
                section: "Section 1.1",
                category: "Service Changes",
                risk: ClauseRisk::Caution,
                summary: "AWS may modify or discontinue any service at any time.",
                plain_english: "They can kill any service you depend on with no notice. Remember when they deprecated stuff? Same risk.",
            },
            FlaggedClause {
                section: "Section 4.2",
                category: "Data Access",
                risk: ClauseRisk::Caution,
                summary: "AWS may access your content to provide support or comply with law.",
                plain_english: "If you open a support ticket or they get a subpoena, they can look at your data. Standard for cloud providers.",
            },
            FlaggedClause {
                section: "Section 11",
                category: "Liability",
                risk: ClauseRisk::Danger,
                summary: "Liability capped at fees paid in the 12 months preceding the claim.",
                plain_english: "If their outage costs you millions, you'll only get back what you paid them in the last year. Could be pennies for a startup.",
            },
        ],
    },
    ServiceTos {
        service: "DoubleClick / Google Ads",
        url: "marketingplatform.google.com/about/analytics/terms",
        last_updated: "2024-09-01",
        word_count: 8500,
        read_time_min: 34,
        verdict: ServiceVerdict::Reject,
        clauses: &[
            FlaggedClause {
                section: "Section 7",
                category: "Data Collection",
                risk: ClauseRisk::Hostile,
                summary: "User data collected across the entire Google Display Network (2M+ sites) for behavioral profiling.",
                plain_english: "Every website with a Google ad or Analytics tag feeds your browsing data into Google's profile of you. That's most of the internet.",
            },
            FlaggedClause {
                section: "Section 4",
                category: "Data Sharing",
                risk: ClauseRisk::Hostile,
                summary: "Data shared with advertisers, publishers, and Google affiliates for ad targeting.",
                plain_english: "Your behavioral profile is the product. It's sold to advertisers in real-time auctions, hundreds of times per day.",
            },
            FlaggedClause {
                section: "Section 11",
                category: "Retention",
                risk: ClauseRisk::Danger,
                summary: "Data retained for up to 26 months; some data retained indefinitely in aggregated form.",
                plain_english: "They keep detailed tracking data for over 2 years. 'Aggregated' data (which can often be re-identified) is kept forever.",
            },
        ],
    },
];

// ---------------------------------------------------------------
//  Template Data
// ---------------------------------------------------------------

struct ClauseView {
    section: String,
    category: String,
    risk_class: String,
    risk_label: String,
    summary: String,
    plain_english: String,
}

struct ServiceView {
    service: String,
    url: String,
    last_updated: String,
    word_count: u32,
    read_time_min: u32,
    verdict_class: String,
    verdict_label: String,
    clauses: Vec<ClauseView>,
    hostile_count: usize,
    danger_count: usize,
}

#[derive(Template)]
#[template(path = "tos_interceptor.html")]
struct TosInterceptorTemplate {
    layer_label: String,
    layer_badge: String,
    services: Vec<ServiceView>,
    total_services: usize,
    total_clauses: usize,
    hostile_total: usize,
    danger_total: usize,
    accept_count: usize,
    review_count: usize,
    reject_count: usize,
    can_override: bool,
    can_add_service: bool,
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/tos-interceptor", get(tos_interceptor_page))
}

async fn tos_interceptor_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let services: Vec<ServiceView> = SERVICES
        .iter()
        .map(|s| {
            let clauses: Vec<ClauseView> = s.clauses.iter().map(|c| ClauseView {
                section: c.section.to_string(),
                category: c.category.to_string(),
                risk_class: c.risk.class().to_string(),
                risk_label: c.risk.label().to_string(),
                summary: c.summary.to_string(),
                plain_english: c.plain_english.to_string(),
            }).collect();
            let hostile_count = s.clauses.iter().filter(|c| c.risk == ClauseRisk::Hostile).count();
            let danger_count = s.clauses.iter().filter(|c| c.risk == ClauseRisk::Danger).count();
            ServiceView {
                service: s.service.to_string(),
                url: s.url.to_string(),
                last_updated: s.last_updated.to_string(),
                word_count: s.word_count,
                read_time_min: s.read_time_min,
                verdict_class: s.verdict.class().to_string(),
                verdict_label: s.verdict.label().to_string(),
                clauses,
                hostile_count,
                danger_count,
            }
        })
        .collect();

    let total_clauses: usize = SERVICES.iter().map(|s| s.clauses.len()).sum();
    let hostile_total = SERVICES.iter().flat_map(|s| s.clauses.iter()).filter(|c| c.risk == ClauseRisk::Hostile).count();
    let danger_total = SERVICES.iter().flat_map(|s| s.clauses.iter()).filter(|c| c.risk == ClauseRisk::Danger).count();
    let accept_count = SERVICES.iter().filter(|s| s.verdict == ServiceVerdict::Accept).count();
    let review_count = SERVICES.iter().filter(|s| s.verdict == ServiceVerdict::ReviewRequired).count();
    let reject_count = SERVICES.iter().filter(|s| s.verdict == ServiceVerdict::Reject).count();

    let content = TosInterceptorTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        services,
        total_services: SERVICES.len(),
        total_clauses,
        hostile_total,
        danger_total,
        accept_count,
        review_count,
        reject_count,
        can_override: layer.has_access(Layer::OsAdmin),
        can_add_service: layer.has_access(Layer::RootUser),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("ToS Interceptor", &content))
    }
}
