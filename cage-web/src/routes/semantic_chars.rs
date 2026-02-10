//! Semantic Chars -- `/semantic-chars`
//!
//! Character-level semantic analysis and safe character set enforcement.
//! Detects homoglyphs, invisible characters, RTL tricks, and other
//! Unicode attack vectors. Shows the allowed character set and flags violations.

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
enum ThreatClass {
    Homoglyph,
    Invisible,
    Bidi,
    Deprecated,
    Confusable,
    Stego,
}

impl ThreatClass {
    fn class(self) -> &'static str {
        match self {
            Self::Homoglyph => "threat-homoglyph",
            Self::Invisible => "threat-invisible",
            Self::Bidi => "threat-bidi",
            Self::Deprecated => "threat-deprecated",
            Self::Confusable => "threat-confusable",
            Self::Stego => "threat-stego",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Homoglyph => "HOMOGLYPH",
            Self::Invisible => "INVISIBLE",
            Self::Bidi => "BIDI OVERRIDE",
            Self::Deprecated => "DEPRECATED",
            Self::Confusable => "CONFUSABLE",
            Self::Stego => "STEGANOGRAPHIC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Critical,
    High,
    Medium,
    Info,
}

impl Severity {
    fn class(self) -> &'static str {
        match self {
            Self::Critical => "sev-critical",
            Self::High => "sev-high",
            Self::Medium => "sev-medium",
            Self::Info => "sev-info",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Info => "INFO",
        }
    }
}

struct CharThreat {
    codepoint: &'static str,
    name: &'static str,
    threat: ThreatClass,
    severity: Severity,
    description: &'static str,
    attack_vector: &'static str,
    example: &'static str,
}

struct SafeRange {
    name: &'static str,
    range: &'static str,
    count: u32,
    description: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset
// ---------------------------------------------------------------

static THREATS: &[CharThreat] = &[
    CharThreat {
        codepoint: "U+200B",
        name: "Zero-Width Space",
        threat: ThreatClass::Invisible,
        severity: Severity::High,
        description: "Invisible character that can split strings without visible change. Used to bypass keyword filters and inject hidden content.",
        attack_vector: "Keyword filter bypass, hidden payload delimiter",
        example: "pass​word (contains ZWSP between 'pass' and 'word')",
    },
    CharThreat {
        codepoint: "U+200C",
        name: "Zero-Width Non-Joiner",
        threat: ThreatClass::Invisible,
        severity: Severity::High,
        description: "Prevents ligature formation. Used in steganographic watermarking by encoding binary data as sequences of ZWNJ/ZWJ.",
        attack_vector: "Steganographic data channel, tracking watermark",
        example: "Binary encoding: ZWNJ=0, ZWJ=1 hidden in text",
    },
    CharThreat {
        codepoint: "U+200D",
        name: "Zero-Width Joiner",
        threat: ThreatClass::Stego,
        severity: Severity::High,
        description: "Forces ligature/joining. Combined with ZWNJ for binary stego encoding. Also creates compound emoji that resist decomposition.",
        attack_vector: "Steganographic encoding, compound emoji smuggling",
        example: "Hidden 8-bit ASCII via 8 ZWJ/ZWNJ sequences",
    },
    CharThreat {
        codepoint: "U+200E/U+200F",
        name: "LTR/RTL Mark",
        threat: ThreatClass::Bidi,
        severity: Severity::Critical,
        description: "Invisible directional overrides. CVE-2021-42574: can make source code appear different from what executes. 'Trojan Source' attack.",
        attack_vector: "Trojan Source attacks, filename spoofing, UI reordering",
        example: "code appears as `if (isAdmin)` but executes `if (!isAdmin)`",
    },
    CharThreat {
        codepoint: "U+202A-U+202E",
        name: "Bidi Embedding/Override",
        threat: ThreatClass::Bidi,
        severity: Severity::Critical,
        description: "Strong directional overrides. Entire text segments can be visually reversed. Used in filename spoofing (exe -> gpj.exe looks like exe.jpg).",
        attack_vector: "Filename extension spoofing, text reordering attacks",
        example: "harmless.txt\\u202Etxt.exe appears as harmless.txtexe.txt",
    },
    CharThreat {
        codepoint: "U+2066-U+2069",
        name: "Bidi Isolates",
        threat: ThreatClass::Bidi,
        severity: Severity::High,
        description: "Newer bidi isolate characters. Same category of attacks as legacy bidi overrides but harder to detect.",
        attack_vector: "Modern Trojan Source variant, UI manipulation",
        example: "Isolate blocks that reverse display order of code",
    },
    CharThreat {
        codepoint: "U+00AD",
        name: "Soft Hyphen",
        threat: ThreatClass::Invisible,
        severity: Severity::Medium,
        description: "Invisible except at line break points. Can split words for filter bypass while appearing normal to users.",
        attack_vector: "Filter bypass, content fingerprinting",
        example: "mal\\u00ADware passes 'malware' keyword filter",
    },
    CharThreat {
        codepoint: "U+0430",
        name: "Cyrillic Small A",
        threat: ThreatClass::Homoglyph,
        severity: Severity::Critical,
        description: "Visually identical to Latin 'a' (U+0061). Foundation of IDN homograph attacks. apple.com vs аpple.com (Cyrillic а).",
        attack_vector: "IDN homograph attacks, phishing domains, impersonation",
        example: "аpple.com (Cyrillic а) vs apple.com (Latin a)",
    },
    CharThreat {
        codepoint: "U+0435",
        name: "Cyrillic Small IE",
        threat: ThreatClass::Homoglyph,
        severity: Severity::Critical,
        description: "Visually identical to Latin 'e'. Combined with other Cyrillic homoglyphs to construct convincing phishing URLs.",
        attack_vector: "URL spoofing, credential phishing",
        example: "googlе.com (Cyrillic е in 'google')",
    },
    CharThreat {
        codepoint: "U+0456",
        name: "Cyrillic Small Byelorussian-Ukrainian I",
        threat: ThreatClass::Homoglyph,
        severity: Severity::High,
        description: "Identical to Latin 'i'. One of the most dangerous homoglyphs due to frequency of 'i' in English text and URLs.",
        attack_vector: "Mixed-script URL spoofing",
        example: "wіkіpedіa.org (Cyrillic і)",
    },
    CharThreat {
        codepoint: "U+FEFF",
        name: "BOM (Byte Order Mark)",
        threat: ThreatClass::Invisible,
        severity: Severity::Medium,
        description: "Zero-width no-break space / BOM. Valid at file start, but invisible and disruptive mid-text. Can cause parsing failures.",
        attack_vector: "File parsing disruption, invisible content injection",
        example: "JSON with BOM fails strict parsers",
    },
    CharThreat {
        codepoint: "U+2028/U+2029",
        name: "Line/Paragraph Separator",
        threat: ThreatClass::Invisible,
        severity: Severity::Medium,
        description: "Unicode line separators not recognized by many tools. Can inject line breaks that bypass single-line validation.",
        attack_vector: "HTTP header injection, log injection, validation bypass",
        example: "HTTP header split via U+2028 in user input",
    },
    CharThreat {
        codepoint: "U+FE00-U+FE0F",
        name: "Variation Selectors",
        threat: ThreatClass::Confusable,
        severity: Severity::Medium,
        description: "Alter glyph appearance without changing semantic identity. Can create distinct visual representations of same string.",
        attack_vector: "Visual ambiguity, fingerprinting, cache poisoning",
        example: "Same codepoint, different visual rendering across systems",
    },
    CharThreat {
        codepoint: "U+E0020-U+E007F",
        name: "Tag Characters",
        threat: ThreatClass::Stego,
        severity: Severity::High,
        description: "Invisible tag characters in Plane 14. Encode full ASCII (A-Z, 0-9) invisibly within text. Perfect stego channel.",
        attack_vector: "Steganographic data exfiltration, invisible watermarking",
        example: "Hidden ASCII message encoded in tag characters",
    },
];

static SAFE_RANGES: &[SafeRange] = &[
    SafeRange { name: "Basic Latin", range: "U+0020-U+007E", count: 95, description: "Standard ASCII printable characters" },
    SafeRange { name: "Latin-1 Supplement", range: "U+00A0-U+00FF", count: 96, description: "Western European accented characters (filtered)" },
    SafeRange { name: "Common Punctuation", range: "U+2000-U+206F", count: 15, description: "Subset: em dash, en dash, quotes (bidi chars excluded)" },
    SafeRange { name: "Currency Symbols", range: "U+20A0-U+20CF", count: 32, description: "Dollar, euro, yen, pound, etc." },
    SafeRange { name: "Mathematical Operators", range: "U+2200-U+22FF", count: 256, description: "Standard math symbols (subset)" },
    SafeRange { name: "Box Drawing", range: "U+2500-U+257F", count: 128, description: "TUI border characters" },
    SafeRange { name: "Block Elements", range: "U+2580-U+259F", count: 32, description: "Block and shade characters for TUI" },
    SafeRange { name: "GentlyOS Glyphs", range: "0x0001-0xFFFF", count: 57, description: "Mapped glyph addresses (replaces emoji)" },
];

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

    let threats: Vec<ThreatView> = THREATS
        .iter()
        .map(|t| ThreatView {
            codepoint: t.codepoint.to_string(),
            name: t.name.to_string(),
            threat_class: t.threat.class().to_string(),
            threat_label: t.threat.label().to_string(),
            severity_class: t.severity.class().to_string(),
            severity_label: t.severity.label().to_string(),
            description: t.description.to_string(),
            attack_vector: t.attack_vector.to_string(),
            example: t.example.to_string(),
        })
        .collect();

    let safe_ranges: Vec<RangeView> = SAFE_RANGES
        .iter()
        .map(|r| RangeView {
            name: r.name.to_string(),
            range: r.range.to_string(),
            count: r.count,
            description: r.description.to_string(),
        })
        .collect();

    let critical_count = THREATS.iter().filter(|t| t.severity == Severity::Critical).count();
    let high_count = THREATS.iter().filter(|t| t.severity == Severity::High).count();
    let safe_codepoints: u32 = SAFE_RANGES.iter().map(|r| r.count).sum();

    let content = SemanticCharsTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        threats,
        safe_ranges,
        total_threats: THREATS.len(),
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
