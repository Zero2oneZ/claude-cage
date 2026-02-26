//! Semantic Chars -- `/semantic-chars`
//!
//! Character threat display for the GentlyOS IO surface.
//! Threat catalog and severity levels inlined from gently-sploit.

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// Severity level (inlined from gently_sploit::threats)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Critical,
    High,
    Medium,
    #[allow(dead_code)]
    Info,
}

// ---------------------------------------------------------------
//  Template Data
// ---------------------------------------------------------------

struct ThreatView {
    codepoint: String,
    name: String,
    threat_class: String,
    threat_label: String,
    severity_class: String,
    severity_label: String,
    description: String,
    attack_vector: String,
    example: String,
}

struct RangeView {
    name: String,
    range: String,
    count: u32,
    description: String,
}

#[derive(Template)]
#[template(path = "semantic_chars.html")]
struct SemanticCharsTemplate {
    layer_label: String,
    layer_badge: String,
    threats: Vec<ThreatView>,
    safe_ranges: Vec<RangeView>,
    total_threats: usize,
    critical_count: usize,
    high_count: usize,
    safe_codepoints: u32,
    can_scan: bool,
    can_modify_allowlist: bool,
}

// ---------------------------------------------------------------
//  Static display data (UI-specific formatting)
// ---------------------------------------------------------------

// Threat display entries mapped from gently_sploit::threats catalog.
// The detection logic is in gently-sploit; this is just display metadata.
struct DisplayThreat {
    codepoint: &'static str,
    name: &'static str,
    threat_label: &'static str,
    severity: Severity,
    description: &'static str,
    attack_vector: &'static str,
    example: &'static str,
}

static DISPLAY_THREATS: &[DisplayThreat] = &[
    DisplayThreat { codepoint: "U+200B", name: "Zero-Width Space", threat_label: "INVISIBLE", severity: Severity::High, description: "Invisible character that can split strings without visible change.", attack_vector: "Keyword filter bypass, hidden payload delimiter", example: "pass[ZWSP]word" },
    DisplayThreat { codepoint: "U+200C", name: "Zero-Width Non-Joiner", threat_label: "INVISIBLE", severity: Severity::High, description: "Prevents ligature formation. Steganographic watermarking vector.", attack_vector: "Steganographic data channel, tracking watermark", example: "ZWNJ=0 in binary encoding" },
    DisplayThreat { codepoint: "U+200D", name: "Zero-Width Joiner", threat_label: "STEGANOGRAPHIC", severity: Severity::High, description: "Forces ligature/joining. Binary stego encoding with ZWNJ.", attack_vector: "Steganographic encoding, compound emoji smuggling", example: "Hidden 8-bit ASCII via ZWJ/ZWNJ" },
    DisplayThreat { codepoint: "U+200E/U+200F", name: "LTR/RTL Mark", threat_label: "BIDI OVERRIDE", severity: Severity::Critical, description: "Invisible directional overrides. CVE-2021-42574 Trojan Source.", attack_vector: "Trojan Source attacks, filename spoofing", example: "code appears as if(isAdmin) but executes if(!isAdmin)" },
    DisplayThreat { codepoint: "U+202A-U+202E", name: "Bidi Embedding/Override", threat_label: "BIDI OVERRIDE", severity: Severity::Critical, description: "Strong directional overrides for text reversal.", attack_vector: "Filename extension spoofing, text reordering", example: "harmless.txt[RLO]txt.exe" },
    DisplayThreat { codepoint: "U+2066-U+2069", name: "Bidi Isolates", threat_label: "BIDI OVERRIDE", severity: Severity::High, description: "Newer bidi isolate characters, harder to detect.", attack_vector: "Modern Trojan Source variant", example: "Isolate blocks reversing display order" },
    DisplayThreat { codepoint: "U+00AD", name: "Soft Hyphen", threat_label: "INVISIBLE", severity: Severity::Medium, description: "Invisible except at line breaks. Filter bypass vector.", attack_vector: "Filter bypass, content fingerprinting", example: "mal[SHY]ware passes filter" },
    DisplayThreat { codepoint: "U+0430", name: "Cyrillic Small A", threat_label: "HOMOGLYPH", severity: Severity::Critical, description: "Visually identical to Latin 'a'. IDN homograph attacks.", attack_vector: "IDN homograph attacks, phishing, impersonation", example: "apple.com vs [Cyrillic a]pple.com" },
    DisplayThreat { codepoint: "U+0435", name: "Cyrillic Small IE", threat_label: "HOMOGLYPH", severity: Severity::Critical, description: "Visually identical to Latin 'e'.", attack_vector: "URL spoofing, credential phishing", example: "googl[Cyrillic e].com" },
    DisplayThreat { codepoint: "U+0456", name: "Cyrillic Small I", threat_label: "HOMOGLYPH", severity: Severity::High, description: "Identical to Latin 'i'. Dangerous due to 'i' frequency.", attack_vector: "Mixed-script URL spoofing", example: "w[Cyrillic i]k[Cyrillic i]pedia.org" },
    DisplayThreat { codepoint: "U+FEFF", name: "BOM (Byte Order Mark)", threat_label: "INVISIBLE", severity: Severity::Medium, description: "Zero-width no-break space. Disruptive mid-text.", attack_vector: "File parsing disruption", example: "JSON with BOM fails strict parsers" },
    DisplayThreat { codepoint: "U+2028/U+2029", name: "Line/Paragraph Separator", threat_label: "INVISIBLE", severity: Severity::Medium, description: "Unicode line separators not recognized by many tools.", attack_vector: "HTTP header injection, log injection", example: "Header split via U+2028" },
    DisplayThreat { codepoint: "U+FE00-U+FE0F", name: "Variation Selectors", threat_label: "CONFUSABLE", severity: Severity::Medium, description: "Alter glyph appearance without changing semantics.", attack_vector: "Visual ambiguity, fingerprinting, cache poisoning", example: "Same codepoint, different rendering" },
    DisplayThreat { codepoint: "U+E0020-U+E007F", name: "Tag Characters", threat_label: "STEGANOGRAPHIC", severity: Severity::High, description: "Invisible Plane 14 tag characters. Perfect stego channel.", attack_vector: "Steganographic data exfiltration", example: "Hidden ASCII in tag characters" },
];

static SAFE_RANGES: &[(&str, &str, u32, &str)] = &[
    ("Basic Latin", "U+0020-U+007E", 95, "Standard ASCII printable characters"),
    ("Latin-1 Supplement", "U+00A0-U+00FF", 96, "Western European accented characters (filtered)"),
    ("Common Punctuation", "U+2000-U+206F", 15, "Subset: em dash, en dash, quotes (bidi excluded)"),
    ("Currency Symbols", "U+20A0-U+20CF", 32, "Dollar, euro, yen, pound, etc."),
    ("Mathematical Operators", "U+2200-U+22FF", 256, "Standard math symbols (subset)"),
    ("Box Drawing", "U+2500-U+257F", 128, "TUI border characters"),
    ("Block Elements", "U+2580-U+259F", 32, "Block and shade characters for TUI"),
    ("GentlyOS Glyphs", "0x0001-0xFFFF", 57, "Mapped glyph addresses (replaces emoji)"),
];

fn severity_class(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "sev-critical",
        Severity::High => "sev-high",
        Severity::Medium => "sev-medium",
        Severity::Info => "sev-info",
    }
}

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "CRITICAL",
        Severity::High => "HIGH",
        Severity::Medium => "MEDIUM",
        Severity::Info => "INFO",
    }
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/semantic-chars", get(semantic_chars_page))
}

async fn semantic_chars_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let threat_views: Vec<ThreatView> = DISPLAY_THREATS
        .iter()
        .map(|t| ThreatView {
            codepoint: t.codepoint.to_string(),
            name: t.name.to_string(),
            threat_class: format!("threat-{}", t.threat_label.to_lowercase().replace(' ', "-")),
            threat_label: t.threat_label.to_string(),
            severity_class: severity_class(t.severity).to_string(),
            severity_label: severity_label(t.severity).to_string(),
            description: t.description.to_string(),
            attack_vector: t.attack_vector.to_string(),
            example: t.example.to_string(),
        })
        .collect();

    let safe_ranges: Vec<RangeView> = SAFE_RANGES
        .iter()
        .map(|(name, range, count, desc)| RangeView {
            name: name.to_string(),
            range: range.to_string(),
            count: *count,
            description: desc.to_string(),
        })
        .collect();

    let critical_count = DISPLAY_THREATS.iter().filter(|t| t.severity == Severity::Critical).count();
    let high_count = DISPLAY_THREATS.iter().filter(|t| t.severity == Severity::High).count();
    let safe_codepoints: u32 = SAFE_RANGES.iter().map(|(_, _, c, _)| c).sum();

    let content = SemanticCharsTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        threats: threat_views,
        safe_ranges,
        total_threats: DISPLAY_THREATS.len(),
        critical_count,
        high_count,
        safe_codepoints,
        can_scan: layer.has_access(Layer::RootUser),
        can_modify_allowlist: layer.has_access(Layer::DevLevel),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Semantic Chars", &content))
    }
}
