//! CODIE Hydration
//!
//! Expands compressed glyph form back to human-readable CODIE.
//! Like XML/SVG hydration - instant expansion from compact form.
//!
//! ## Hydration Pipeline
//!
//! ```text
//! Glyph String → Parse Structure → Expand Glyphs → Format Tree → Human CODIE
//!
//! Input:
//!   ρLOGIN⟨βuser←@database/users⟨⁇¬found→⊥⟩μ→token⟩
//!
//! Output:
//!   pug LOGIN
//!   ├── bark user ← @database/users
//!   │   └── ? not found → whine
//!   └── treat → token
//! ```
//!
//! ## Hydration Modes
//!
//! - `hydrate()` - Full tree-formatted output with box drawing
//! - `hydrate_flat()` - Indentation-based output (simpler)
//! - `hydrate_minimal()` - Single-line keywords only

use crate::glyph::{glyph_to_keyword, is_glyph, is_structural};

/// Hydrated CODIE representation
#[derive(Debug, Clone)]
pub struct HydratedCodie {
    /// Human-readable CODIE source
    pub source: String,
    /// Original compressed size
    pub compressed_size: usize,
    /// Hydrated size
    pub hydrated_size: usize,
    /// Parse errors (if any)
    pub errors: Vec<String>,
}

impl HydratedCodie {
    /// Expansion ratio
    pub fn expansion(&self) -> f64 {
        if self.compressed_size == 0 {
            return 0.0;
        }
        self.hydrated_size as f64 / self.compressed_size as f64
    }
}

/// Hydrate compressed CODIE to human-readable form
pub fn hydrate(compressed: &str) -> HydratedCodie {
    let compressed_size = compressed.len();
    let mut output = String::new();
    let mut errors = Vec::new();
    let mut depth = 0;
    let mut chars = compressed.chars().peekable();
    let mut line_buffer = String::new();
    let mut is_first_at_depth: Vec<bool> = vec![true];

    while let Some(c) = chars.next() {
        match c {
            // Block open
            '⟨' => {
                // Flush current line
                if !line_buffer.is_empty() {
                    output.push_str(&format_line(&line_buffer, depth, is_first_at_depth.last() == Some(&true)));
                    output.push('\n');
                    if let Some(first) = is_first_at_depth.last_mut() {
                        *first = false;
                    }
                    line_buffer.clear();
                }
                depth += 1;
                is_first_at_depth.push(true);
            }
            // Block close
            '⟩' => {
                // Flush current line
                if !line_buffer.is_empty() {
                    output.push_str(&format_line(&line_buffer, depth, is_first_at_depth.last() == Some(&true)));
                    output.push('\n');
                    line_buffer.clear();
                }
                depth = depth.saturating_sub(1);
                is_first_at_depth.pop();
                if let Some(first) = is_first_at_depth.last_mut() {
                    *first = false;
                }
            }
            // Glyph - expand to keyword
            c if is_glyph(c) => {
                if let Some(keyword) = glyph_to_keyword(c) {
                    if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                        line_buffer.push(' ');
                    }
                    line_buffer.push_str(keyword);
                    // Add space after keyword for readability
                    line_buffer.push(' ');
                } else {
                    errors.push(format!("Unknown glyph: {}", c));
                    line_buffer.push(c);
                }
            }
            // Structural characters - keep as-is with spacing
            '←' | '→' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
                line_buffer.push(' ');
            }
            '?' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
                line_buffer.push(' ');
            }
            '@' | '#' | '$' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
            }
            // Whitespace - add space if needed
            c if c.is_whitespace() => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
            }
            // Other characters (identifiers, literals)
            _ => {
                line_buffer.push(c);
            }
        }
    }

    // Flush remaining buffer
    if !line_buffer.is_empty() {
        output.push_str(&format_line(&line_buffer, depth, is_first_at_depth.last() == Some(&true)));
        output.push('\n');
    }

    // Trim trailing newline
    let source = output.trim_end().to_string();
    let hydrated_size = source.len();

    HydratedCodie {
        source,
        compressed_size,
        hydrated_size,
        errors,
    }
}

/// Format a line with tree structure
fn format_line(content: &str, depth: usize, _is_first: bool) -> String {
    if depth == 0 {
        return content.to_string();
    }

    let mut prefix = String::new();
    for i in 0..depth {
        if i == depth - 1 {
            prefix.push_str("├── ");
        } else {
            prefix.push_str("│   ");
        }
    }

    format!("{}{}", prefix, content)
}

/// Hydrate to flat indentation-based format
pub fn hydrate_flat(compressed: &str) -> String {
    let mut output = String::new();
    let mut depth = 0;
    let mut chars = compressed.chars().peekable();
    let mut line_buffer = String::new();

    while let Some(c) = chars.next() {
        match c {
            '⟨' => {
                if !line_buffer.is_empty() {
                    output.push_str(&"  ".repeat(depth));
                    output.push_str(&line_buffer);
                    output.push('\n');
                    line_buffer.clear();
                }
                depth += 1;
            }
            '⟩' => {
                if !line_buffer.is_empty() {
                    output.push_str(&"  ".repeat(depth));
                    output.push_str(&line_buffer);
                    output.push('\n');
                    line_buffer.clear();
                }
                depth = depth.saturating_sub(1);
            }
            c if is_glyph(c) => {
                if let Some(keyword) = glyph_to_keyword(c) {
                    if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                        line_buffer.push(' ');
                    }
                    line_buffer.push_str(keyword);
                    line_buffer.push(' ');
                }
            }
            '←' | '→' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
                line_buffer.push(' ');
            }
            '?' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
                line_buffer.push(' ');
            }
            '@' | '#' | '$' => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
                line_buffer.push(c);
            }
            c if c.is_whitespace() => {
                if !line_buffer.is_empty() && !line_buffer.ends_with(' ') {
                    line_buffer.push(' ');
                }
            }
            _ => {
                line_buffer.push(c);
            }
        }
    }

    if !line_buffer.is_empty() {
        output.push_str(&"  ".repeat(depth));
        // Trim extra spaces
        output.push_str(line_buffer.trim());
    }

    output.trim_end().to_string()
}

/// Hydrate to minimal single-line format
pub fn hydrate_minimal(compressed: &str) -> String {
    let mut output = String::new();

    for c in compressed.chars() {
        match c {
            '⟨' => output.push_str(" { "),
            '⟩' => output.push_str(" } "),
            c if is_glyph(c) => {
                if let Some(keyword) = glyph_to_keyword(c) {
                    if !output.is_empty() && !output.ends_with(' ') && !output.ends_with('{') {
                        output.push(' ');
                    }
                    output.push_str(keyword);
                }
            }
            _ => output.push(c),
        }
    }

    // Clean up extra spaces
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse compressed string to token stream (for execution)
pub fn parse_tokens(compressed: &str) -> Vec<HydrationToken> {
    let mut tokens = Vec::new();

    let mut chars = compressed.chars().peekable();
    let mut identifier = String::new();

    while let Some(c) = chars.next() {
        // Flush identifier if we hit a special char
        if is_glyph(c) || is_structural(c) || c == '⟨' || c == '⟩' {
            if !identifier.is_empty() {
                tokens.push(HydrationToken::Identifier(identifier.clone()));
                identifier.clear();
            }
        }

        match c {
            '⟨' => tokens.push(HydrationToken::BlockOpen),
            '⟩' => tokens.push(HydrationToken::BlockClose),
            '←' => tokens.push(HydrationToken::Assign),
            '→' => tokens.push(HydrationToken::Arrow),
            '?' => tokens.push(HydrationToken::Conditional),
            '@' => {
                // Collect source reference
                let mut source_ref = String::from("@");
                while let Some(&next) = chars.peek() {
                    if is_glyph(next) || next == '⟨' || next == '⟩' || next.is_whitespace() {
                        break;
                    }
                    source_ref.push(chars.next().unwrap());
                }
                tokens.push(HydrationToken::SourceRef(source_ref));
            }
            '#' => {
                // Collect hash reference
                let mut hash_ref = String::from("#");
                while let Some(&next) = chars.peek() {
                    if is_glyph(next) || next == '⟨' || next == '⟩' || next.is_whitespace() {
                        break;
                    }
                    hash_ref.push(chars.next().unwrap());
                }
                tokens.push(HydrationToken::HashRef(hash_ref));
            }
            '$' => {
                // Collect vault reference
                let mut vault_ref = String::from("$");
                while let Some(&next) = chars.peek() {
                    if is_glyph(next) || next == '⟨' || next == '⟩' || next.is_whitespace() {
                        break;
                    }
                    vault_ref.push(chars.next().unwrap());
                }
                tokens.push(HydrationToken::VaultRef(vault_ref));
            }
            c if is_glyph(c) => {
                if let Some(keyword) = glyph_to_keyword(c) {
                    tokens.push(HydrationToken::Keyword(keyword.to_string()));
                }
            }
            c if c.is_whitespace() => continue,
            _ => identifier.push(c),
        }
    }

    // Flush remaining identifier
    if !identifier.is_empty() {
        tokens.push(HydrationToken::Identifier(identifier));
    }

    tokens
}

/// Token type for parsed compressed CODIE
#[derive(Debug, Clone, PartialEq)]
pub enum HydrationToken {
    Keyword(String),
    Identifier(String),
    BlockOpen,
    BlockClose,
    Assign,
    Arrow,
    Conditional,
    SourceRef(String),
    HashRef(String),
    VaultRef(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hydration() {
        let compressed = "ρLOGINμ→token";
        let hydrated = hydrate(compressed);
        assert!(hydrated.source.contains("pug"));
        assert!(hydrated.source.contains("LOGIN"));
        assert!(hydrated.source.contains("treat"));
        assert!(hydrated.source.contains("token"));
    }

    #[test]
    fn test_nested_hydration() {
        let compressed = "ρTEST⟨βdata⟨⁇found→⊤⟩μ→result⟩";
        let hydrated = hydrate(compressed);
        eprintln!("Nested hydration output:\n{}", hydrated.source);
        // Check for key elements (may have varying whitespace)
        assert!(hydrated.source.contains("pug"), "Missing 'pug'");
        assert!(hydrated.source.contains("TEST"), "Missing 'TEST'");
        assert!(hydrated.source.contains("bark"), "Missing 'bark'");
        assert!(hydrated.source.contains("if"), "Missing 'if'");
        assert!(hydrated.source.contains("wag"), "Missing 'wag'");
        assert!(hydrated.source.contains("treat"), "Missing 'treat'");
    }

    #[test]
    fn test_logic_gates_hydration() {
        let compressed = "φx∧y∨¬z";
        let hydrated = hydrate(compressed);
        assert!(hydrated.source.contains("sniff"));
        assert!(hydrated.source.contains("and"));
        assert!(hydrated.source.contains("or"));
        assert!(hydrated.source.contains("not"));
    }

    #[test]
    fn test_source_refs_preserved() {
        let compressed = "βdata←@database/users";
        let hydrated = hydrate(compressed);
        assert!(hydrated.source.contains("bark"));
        assert!(hydrated.source.contains("@database/users"));
    }

    #[test]
    fn test_flat_format() {
        let compressed = "ρTEST⟨βdata⟨μ→ok⟩⟩";
        let flat = hydrate_flat(compressed);
        eprintln!("Flat hydration output:\n{}", flat);
        // Check for key elements (whitespace may vary)
        assert!(flat.contains("pug"), "Missing 'pug'");
        assert!(flat.contains("TEST"), "Missing 'TEST'");
        assert!(flat.contains("bark"), "Missing 'bark'");
        assert!(flat.contains("treat"), "Missing 'treat'");
    }

    #[test]
    fn test_token_parsing() {
        let compressed = "ρTEST⟨βdata←@db⟩";
        let tokens = parse_tokens(compressed);

        assert!(tokens.iter().any(|t| matches!(t, HydrationToken::Keyword(k) if k == "pug")));
        assert!(tokens.iter().any(|t| matches!(t, HydrationToken::Keyword(k) if k == "bark")));
        assert!(tokens.iter().any(|t| matches!(t, HydrationToken::BlockOpen)));
        assert!(tokens.iter().any(|t| matches!(t, HydrationToken::SourceRef(s) if s.starts_with("@"))));
    }

    #[test]
    fn test_roundtrip_minimal() {
        // Simple case without nested structure
        let original = "pug TEST treat → result";
        let compressed = crate::compress::compress(original);
        let hydrated = hydrate_minimal(&compressed.glyphs);

        // Check key elements are present
        assert!(hydrated.contains("pug"));
        assert!(hydrated.contains("treat"));
    }
}
