//! Cookie Jar -- `/cookie-jar`
//!
//! Visual representation of cookies collected across domains.
//! Each cookie has risk level, flags, plain-English explanation,
//! and per-cookie approval workflow. Features are tier-gated.
//!
//! ## Integration with gently-cookie-vault
//!
//! This UI route displays cookie data. The backend storage and encryption
//! lives in the `gently-cookie-vault` crate (L3 containment layer):
//!
//! ```ignore
//! use gently_cookie_vault::CookieVault;
//!
//! let vault = CookieVault::new(cookie_key, vault_path);
//! vault.store_cookie(domain, name, value, provenance, flags)?;
//! let token = vault.read_cookie(&id)?;     // read-once token
//! let plaintext = vault.redeem_token(token)?; // single redemption
//! ```
//!
//! This route can wire into CookieVault for encrypted persistence
//! once the full L3 integration layer is assembled.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{html_escape, is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data Model
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreatLevel {
    Safe,
    Warn,
    Breach,
}

impl ThreatLevel {
    fn class(self) -> &'static str {
        match self {
            Self::Safe => "threat-safe",
            Self::Warn => "threat-warn",
            Self::Breach => "threat-breach",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Warn => "WARN",
            Self::Breach => "BREACH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RiskLevel {
    None,
    Low,
    Medium,
    High,
}

impl RiskLevel {
    fn class(self) -> &'static str {
        match self {
            Self::None => "risk-none",
            Self::Low => "risk-low",
            Self::Medium => "risk-medium",
            Self::High => "risk-high",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }
}

struct CookieEntry {
    name: &'static str,
    value_masked: &'static str,
    expires: &'static str,
    flags: &'static [&'static str],
    tracking: bool,
    risk: RiskLevel,
    category: &'static str,
    explain: &'static str,
    tldr: &'static str,
}

struct CookieDomain {
    domain: &'static str,
    threat: ThreatLevel,
    cookies: &'static [CookieEntry],
}

// ---------------------------------------------------------------
//  28-Cookie Dataset (from gently-cookie-vault.html prototype)
// ---------------------------------------------------------------

static GITHUB_COOKIES: [CookieEntry; 4] = [
    CookieEntry {
        name: "_gh_sess",
        value_masked: "a]3k............f2e8",
        expires: "Session",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::None,
        category: "authentication",
        explain: "Session keeper for GitHub login during browser session.",
        tldr: "Dies when you close browser.",
    },
    CookieEntry {
        name: "logged_in",
        value_masked: "yes",
        expires: "Session",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::None,
        category: "authentication",
        explain: "Boolean flag indicating login state to show dashboard vs landing page.",
        tldr: "Simple yes/no login marker.",
    },
    CookieEntry {
        name: "_device_id",
        value_masked: "d8f2............3a1c",
        expires: "2027-02-10",
        flags: &["sec"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "security",
        explain: "Device fingerprint for security alerts; identifies new logins from unknown devices.",
        tldr: "Helps detect account takeover attempts.",
    },
    CookieEntry {
        name: "color_mode",
        value_masked: "dark",
        expires: "2027-02-10",
        flags: &[],
        tracking: false,
        risk: RiskLevel::None,
        category: "preference",
        explain: "Remembers dark/light theme preference.",
        tldr: "Pure preference storage, harmless.",
    },
];

static GOOGLE_COOKIES: [CookieEntry; 6] = [
    CookieEntry {
        name: "SID",
        value_masked: "FgiK............8dPQ",
        expires: "2028-02-10",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Primary Google session ID across all Google services.",
        tldr: "Master login cookie; deleting it logs you out of everything.",
    },
    CookieEntry {
        name: "HSID",
        value_masked: "AYwB............k3mN",
        expires: "2028-02-10",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Session integrity check paired with SID to verify session authenticity.",
        tldr: "CSRF protection for Google sessions.",
    },
    CookieEntry {
        name: "NID",
        value_masked: "511=............Xp2r",
        expires: "2026-08-10",
        flags: &["sec", "http"],
        tracking: true,
        risk: RiskLevel::High,
        category: "tracking",
        explain: "Tracking cookie. Profiles user interests from searches/browsing to personalize ads across non-Google websites.",
        tldr: "Google's cross-site ad profiler; watches your behavior.",
    },
    CookieEntry {
        name: "APISID",
        value_masked: "Lm9Q............vR3s",
        expires: "2028-02-10",
        flags: &["sec"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Authenticates API calls for embedded Google content (YouTube embeds, Google Maps).",
        tldr: "Identifies you to embedded Google widgets.",
    },
    CookieEntry {
        name: "1P_JAR",
        value_masked: "2026-02-10-08",
        expires: "2026-03-12",
        flags: &["sec", "ss"],
        tracking: true,
        risk: RiskLevel::High,
        category: "tracking",
        explain: "Tracking cookie. Collects statistics on Google service usage and ad impressions for targeting.",
        tldr: "Measures which ads you see; feeds into targeting.",
    },
    CookieEntry {
        name: "AEC",
        value_masked: "AQTF............9kPx",
        expires: "2026-08-10",
        flags: &["sec", "http"],
        tracking: true,
        risk: RiskLevel::Medium,
        category: "tracking",
        explain: "Tracking cookie. Prevents ad fraud (bot vs human), but also tracks your ad clicks.",
        tldr: "Ad fraud prevention that also tracks you.",
    },
];

static STACKOVERFLOW_COOKIES: [CookieEntry; 3] = [
    CookieEntry {
        name: "prov",
        value_masked: "3f8a............d2c1",
        expires: "2036-01-01",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Session provider; maintains login and browsing state. Expires in 2036 (unusually aggressive).",
        tldr: "Login persistence with a wild 10-year expiry date.",
    },
    CookieEntry {
        name: "acct",
        value_masked: "t=1&s=............",
        expires: "Session",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::None,
        category: "authentication",
        explain: "Session-scoped account state (tier, login status).",
        tldr: "Disappears when browser closes.",
    },
    CookieEntry {
        name: "OptanonConsent",
        value_masked: "isIABGlobal=............",
        expires: "2027-02-10",
        flags: &[],
        tracking: true,
        risk: RiskLevel::Medium,
        category: "consent-management",
        explain: "Tracking cookie. OneTrust consent platform cookie; records which tracking cookies you approved. Ironically, it's itself a tracking cookie with NO Secure flag.",
        tldr: "A tracking cookie about tracking cookies; easily intercepted.",
    },
];

static CRATES_COOKIES: [CookieEntry; 1] = [
    CookieEntry {
        name: "__crates_sess",
        value_masked: "c9e0............a7b3",
        expires: "Session",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::None,
        category: "authentication",
        explain: "Rust package registry session; textbook proper cookie: session-scoped, secure, httpOnly.",
        tldr: "Clean session cookie, does exactly what it should.",
    },
];

static DOUBLECLICK_COOKIES: [CookieEntry; 3] = [
    CookieEntry {
        name: "IDE",
        value_masked: "AHWq............R3sT",
        expires: "2027-02-10",
        flags: &["ss"],
        tracking: true,
        risk: RiskLevel::High,
        category: "surveillance",
        explain: "DoubleClick's primary ad targeting cookie. Tracks behavior across millions of websites to build behavioral profile for ad auctions.",
        tldr: "Tracks you across the entire web for ad targeting.",
    },
    CookieEntry {
        name: "test_cookie",
        value_masked: "CheckForPermission",
        expires: "Session",
        flags: &[],
        tracking: true,
        risk: RiskLevel::High,
        category: "surveillance",
        explain: "Probe cookie. DoubleClick uses this to test if your browser allows third-party cookies. If it can read this back, it knows it can track you.",
        tldr: "A probe; tests if you're trackable before surveillance starts.",
    },
    CookieEntry {
        name: "ar_debug",
        value_masked: "1",
        expires: "2026-03-12",
        flags: &[],
        tracking: true,
        risk: RiskLevel::High,
        category: "surveillance",
        explain: "Attribution reporting debug cookie. New-gen tracking infrastructure replacing third-party cookies; measures ad conversions. Zero security flags -- fully exposed.",
        tldr: "Modern ad tracking with no protections; wide open.",
    },
];

static FACEBOOK_COOKIES: [CookieEntry; 5] = [
    CookieEntry {
        name: "fr",
        value_masked: "0aB1............Xz9Y",
        expires: "2026-05-10",
        flags: &["sec", "http", "ss"],
        tracking: true,
        risk: RiskLevel::High,
        category: "surveillance",
        explain: "Facebook's primary advertising cookie. Delivered to every site with Like button, Share widget, or Pixel. Tracks browsing across web and links to Facebook identity.",
        tldr: "Facebook's cross-site tracker; follows you everywhere.",
    },
    CookieEntry {
        name: "sb",
        value_masked: "kL8m............pQ2r",
        expires: "2028-02-10",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "security",
        explain: "Browser identification for security; detects unauthorized login attempts.",
        tldr: "Browser fingerprint for account protection.",
    },
    CookieEntry {
        name: "datr",
        value_masked: "Vw3x............Yz1A",
        expires: "2028-02-10",
        flags: &["sec", "http"],
        tracking: true,
        risk: RiskLevel::High,
        category: "tracking",
        explain: "Controversial tracking cookie. Facebook claims it's for security; researchers say it tracks unique browsers across web even for non-Facebook users.",
        tldr: "Tracks browsers across web, even tracks non-users.",
    },
    CookieEntry {
        name: "c_user",
        value_masked: "100............842",
        expires: "2027-02-10",
        flags: &["sec"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Contains your Facebook user ID number; ties all session actions to your identity.",
        tldr: "Your numeric Facebook ID; links activity to you.",
    },
    CookieEntry {
        name: "xs",
        value_masked: "28:d8............:2:AQ",
        expires: "2027-02-10",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::Medium,
        category: "authentication",
        explain: "Facebook session secret; if stolen, attacker becomes you on Facebook.",
        tldr: "Session secret -- critical if compromised.",
    },
];

static ANTHROPIC_COOKIES: [CookieEntry; 1] = [
    CookieEntry {
        name: "__cf_bm",
        value_masked: "Ek9f............Mn2p",
        expires: "Session",
        flags: &["sec", "http"],
        tracking: false,
        risk: RiskLevel::None,
        category: "security",
        explain: "Cloudflare bot management; determines if you're human or bot script.",
        tldr: "\"Are you human?\" check; that's it.",
    },
];

static AMAZON_COOKIES: [CookieEntry; 5] = [
    CookieEntry {
        name: "session-id",
        value_masked: "131-............-728",
        expires: "2027-02-10",
        flags: &["sec"],
        tracking: false,
        risk: RiskLevel::Low,
        category: "authentication",
        explain: "Shopping session; links cart, browsing history, checkout. Expires after 1 year (excessive).",
        tldr: "Shopping session, but the 1-year expiry is overkill.",
    },
    CookieEntry {
        name: "i18n-prefs",
        value_masked: "USD",
        expires: "2027-02-10",
        flags: &[],
        tracking: false,
        risk: RiskLevel::None,
        category: "preference",
        explain: "Currency preference (USD). Harmless but NO Secure flag (lazy config).",
        tldr: "Currency preference; harmless but poorly secured.",
    },
    CookieEntry {
        name: "ubid-main",
        value_masked: "131-............-384",
        expires: "2028-02-10",
        flags: &["sec"],
        tracking: true,
        risk: RiskLevel::High,
        category: "tracking",
        explain: "Amazon's unique browser ID. Persists even after logout. Tracks browsing patterns and associates anonymous browsing with account upon eventual login.",
        tldr: "Persistent browser ID; tracks you logged out or in.",
    },
    CookieEntry {
        name: "ad-id",
        value_masked: "A0Bc............Yz12",
        expires: "2028-02-10",
        flags: &[],
        tracking: true,
        risk: RiskLevel::High,
        category: "surveillance",
        explain: "Amazon's ad ID. Powers ad platform showing targeted ads on Amazon and external sites. NO Secure flag, NO HttpOnly -- fully exposed.",
        tldr: "Ad targeting ID with zero security; wide open.",
    },
    CookieEntry {
        name: "ad-privacy",
        value_masked: "0",
        expires: "2028-02-10",
        flags: &[],
        tracking: true,
        risk: RiskLevel::Medium,
        category: "consent-management",
        explain: "Records ad privacy preference ('0' = not opted out). Itself has no security flags.",
        tldr: "Privacy pref without privacy protections.",
    },
];

static DOMAINS: [CookieDomain; 8] = [
    CookieDomain { domain: "github.com", threat: ThreatLevel::Safe, cookies: &GITHUB_COOKIES },
    CookieDomain { domain: "docs.google.com", threat: ThreatLevel::Warn, cookies: &GOOGLE_COOKIES },
    CookieDomain { domain: "stackoverflow.com", threat: ThreatLevel::Safe, cookies: &STACKOVERFLOW_COOKIES },
    CookieDomain { domain: "crates.io", threat: ThreatLevel::Safe, cookies: &CRATES_COOKIES },
    CookieDomain { domain: "doubleclick.net", threat: ThreatLevel::Breach, cookies: &DOUBLECLICK_COOKIES },
    CookieDomain { domain: "facebook.com", threat: ThreatLevel::Warn, cookies: &FACEBOOK_COOKIES },
    CookieDomain { domain: "api.anthropic.com", threat: ThreatLevel::Safe, cookies: &ANTHROPIC_COOKIES },
    CookieDomain { domain: "amazon.com", threat: ThreatLevel::Warn, cookies: &AMAZON_COOKIES },
];

// ---------------------------------------------------------------
//  Template data (lifetime-free copies for Askama)
// ---------------------------------------------------------------

struct DomainView {
    domain: String,
    threat_class: String,
    threat_label: String,
    cookie_count: usize,
    tracking_count: usize,
    selected: bool,
}

struct CookieView {
    name: String,
    value_masked: String,
    expires: String,
    flag_sec: bool,
    flag_http: bool,
    flag_ss: bool,
    flag_trk: bool,
    tracking: bool,
    risk_class: String,
    risk_label: String,
    category: String,
    explain: String,
    tldr: String,
}

#[derive(Template)]
#[template(path = "cookie_jar.html")]
struct CookieJarTemplate {
    layer_label: String,
    layer_badge: String,
    domains: Vec<DomainView>,
    cookies: Vec<CookieView>,
    selected_domain: String,
    selected_threat_class: String,
    selected_threat_label: String,
    total_cookies: usize,
    total_tracking: usize,
    total_domains: usize,
    breach_count: usize,
    can_explain: bool,
    can_approve: bool,
    can_export: bool,
    can_breach_sim: bool,
}

#[derive(Template)]
#[template(path = "partials/cookie_domain.html")]
struct CookieDomainPartial {
    cookies: Vec<CookieView>,
    selected_domain: String,
    selected_threat_class: String,
    selected_threat_label: String,
    can_explain: bool,
    can_approve: bool,
    can_export: bool,
}

// ---------------------------------------------------------------
//  Helpers
// ---------------------------------------------------------------

fn build_domain_views(selected: &str) -> Vec<DomainView> {
    DOMAINS
        .iter()
        .map(|d| DomainView {
            domain: d.domain.to_string(),
            threat_class: d.threat.class().to_string(),
            threat_label: d.threat.label().to_string(),
            cookie_count: d.cookies.len(),
            tracking_count: d.cookies.iter().filter(|c| c.tracking).count(),
            selected: d.domain == selected,
        })
        .collect()
}

fn build_cookie_views(domain: &str) -> Vec<CookieView> {
    let Some(d) = DOMAINS.iter().find(|d| d.domain == domain) else {
        return Vec::new();
    };
    d.cookies
        .iter()
        .map(|c| CookieView {
            name: c.name.to_string(),
            value_masked: c.value_masked.to_string(),
            expires: c.expires.to_string(),
            flag_sec: c.flags.contains(&"sec"),
            flag_http: c.flags.contains(&"http"),
            flag_ss: c.flags.contains(&"ss"),
            flag_trk: c.tracking,
            tracking: c.tracking,
            risk_class: c.risk.class().to_string(),
            risk_label: c.risk.label().to_string(),
            category: c.category.to_string(),
            explain: c.explain.to_string(),
            tldr: c.tldr.to_string(),
        })
        .collect()
}

fn vault_stats() -> (usize, usize, usize, usize) {
    let total_cookies: usize = DOMAINS.iter().map(|d| d.cookies.len()).sum();
    let total_tracking: usize = DOMAINS
        .iter()
        .flat_map(|d| d.cookies.iter())
        .filter(|c| c.tracking)
        .count();
    let total_domains = DOMAINS.len();
    let breach_count = DOMAINS
        .iter()
        .filter(|d| d.threat == ThreatLevel::Breach)
        .count();
    (total_cookies, total_tracking, total_domains, breach_count)
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/cookie-jar", get(cookie_jar_page))
        .route("/cookie-jar/domain/{domain}", get(domain_detail))
}

async fn cookie_jar_page(
    headers: HeaderMap,
    State(_state): State<Arc<AppState>>,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let default_domain = DOMAINS[0].domain;
    let (total_cookies, total_tracking, total_domains, breach_count) = vault_stats();

    let content = CookieJarTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        domains: build_domain_views(default_domain),
        cookies: build_cookie_views(default_domain),
        selected_domain: default_domain.to_string(),
        selected_threat_class: DOMAINS[0].threat.class().to_string(),
        selected_threat_label: DOMAINS[0].threat.label().to_string(),
        total_cookies,
        total_tracking,
        total_domains,
        breach_count,
        can_explain: layer.has_access(Layer::RootUser),
        can_approve: layer.has_access(Layer::RootUser),
        can_export: layer.has_access(Layer::OsAdmin),
        can_breach_sim: layer.has_access(Layer::DevLevel),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Cookie Jar", &content))
    }
}

async fn domain_detail(
    Path(domain): Path<String>,
    _headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let safe_domain = html_escape(&domain);
    let threat = DOMAINS
        .iter()
        .find(|d| d.domain == domain.as_str())
        .map(|d| d.threat)
        .unwrap_or(ThreatLevel::Safe);

    let content = CookieDomainPartial {
        cookies: build_cookie_views(&domain),
        selected_domain: safe_domain,
        selected_threat_class: threat.class().to_string(),
        selected_threat_label: threat.label().to_string(),
        can_explain: layer.has_access(Layer::RootUser),
        can_approve: layer.has_access(Layer::RootUser),
        can_export: layer.has_access(Layer::OsAdmin),
    }
    .render()
    .unwrap_or_default();

    Html(content)
}
