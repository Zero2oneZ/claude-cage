//! CODIE Compression (Dehydration)
//!
//! Converts human-readable CODIE into compact glyph form.
//! Removes organizational syntax (tree chars, whitespace, comments)
//! and replaces keywords with single-character glyphs.
//!
//! ## Compression Pipeline
//!
//! ```text
//! Human CODIE → Strip Comments → Strip Tree Chars → Glyphify → Structure → Hash
//!
//! Input:
//!   pug LOGIN
//!   ├── bark user ← @database/users
//!   │   └── ? not found → whine
//!   └── treat → token
//!
//! Output:
//!   ρLOGIN⟨βuser←@database/users⟨⁇¬found→⊥⟩μ→token⟩
//!
//! Hash:
//!   #c7f3a2b1
//! ```
//!
//! ## Compression Ratio
//!
//! Typical compression: 60-80% size reduction
//! - Keywords (4-6 chars) → Glyphs (1 char)
//! - Tree structure (├──, └──, │) → Bracket nesting (⟨⟩)
//! - Whitespace/indentation → Eliminated

use crate::glyph::keyword_to_glyph;
use crate::squeeze::{squeeze, squeeze_identifier, SqueezeLevel};
use std::collections::VecDeque;

/// Compressed CODIE representation
#[derive(Debug, Clone)]
pub struct CompressedCodie {
    /// The glyph-encoded instruction string
    pub glyphs: String,
    /// Original byte size
    pub original_size: usize,
    /// Compressed byte size
    pub compressed_size: usize,
    /// Nesting depth (for validation)
    pub max_depth: usize,
}

impl CompressedCodie {
    /// Compression ratio (0.0 = no compression, 1.0 = fully compressed away)
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        1.0 - (self.compressed_size as f64 / self.original_size as f64)
    }

    /// Human-readable compression stats
    pub fn stats(&self) -> String {
        format!(
            "{}B → {}B ({:.1}% reduction, depth {})",
            self.original_size,
            self.compressed_size,
            self.ratio() * 100.0,
            self.max_depth
        )
    }
}

/// Tree structure characters to strip
const TREE_CHARS: &[char] = &['├', '─', '└', '│', '┌', '┐', '┘', '┴', '┬', '┤', '┼'];

/// Compress CODIE source to glyph form
pub fn compress(source: &str) -> CompressedCodie {
    let original_size = source.len();
    let mut output = String::new();
    let mut depth_stack: VecDeque<usize> = VecDeque::new();
    let mut max_depth = 0;
    let mut current_depth = 0;

    for line in source.lines() {
        let processed = process_line(line, &mut depth_stack, &mut current_depth);
        if !processed.is_empty() {
            output.push_str(&processed);
        }
        max_depth = max_depth.max(current_depth);
    }

    // Close any remaining open blocks
    while depth_stack.pop_back().is_some() {
        output.push('⟩');
    }

    let compressed_size = output.len();

    CompressedCodie {
        glyphs: output,
        original_size,
        compressed_size,
        max_depth,
    }
}

/// Process a single line of CODIE
fn process_line(line: &str, depth_stack: &mut VecDeque<usize>, current_depth: &mut usize) -> String {
    // Skip empty lines and comments
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
        return String::new();
    }

    // Calculate indentation level (tree depth)
    let indent = calculate_indent(line);

    // Close blocks for decreased indentation
    let mut result = String::new();
    while let Some(&prev_indent) = depth_stack.back() {
        if indent <= prev_indent {
            depth_stack.pop_back();
            result.push('⟩');
            *current_depth = current_depth.saturating_sub(1);
        } else {
            break;
        }
    }

    // Strip tree characters and get content
    let content = strip_tree_chars(trimmed);
    if content.is_empty() {
        return result;
    }

    // Convert content to glyphs
    let glyphed = glyphify(&content);

    // Track if this line opens a new block (has children in original)
    let opens_block = line.contains("├") || line.contains("┌") ||
                      content.ends_with(':') || is_block_opener(&content);

    result.push_str(&glyphed);

    if opens_block {
        result.push('⟨');
        depth_stack.push_back(indent);
        *current_depth += 1;
    }

    result
}

/// Calculate indentation level from tree structure
fn calculate_indent(line: &str) -> usize {
    let mut indent = 0;
    for c in line.chars() {
        match c {
            ' ' | '\t' => indent += 1,
            '│' => indent += 4,
            '├' | '└' => {
                indent += 4;
                break;
            }
            _ => break,
        }
    }
    indent
}

/// Strip tree drawing characters from line
fn strip_tree_chars(line: &str) -> String {
    line.chars()
        .filter(|c| !TREE_CHARS.contains(c) && *c != ' ' || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Check if content opens a block
fn is_block_opener(content: &str) -> bool {
    let first_word = content.split_whitespace().next().unwrap_or("");
    matches!(first_word.to_lowercase().as_str(),
        "pug" | "fence" | "trick" | "chase" | "if" | "while" | "for" | "fork"
    )
}

/// Convert text content to glyph form
fn glyphify(content: &str) -> String {
    let mut result = String::new();
    let mut words = content.split_whitespace().peekable();

    while let Some(word) = words.next() {
        // Handle special operators
        if word == "←" || word == "→" || word == "?" {
            result.push_str(word);
            continue;
        }

        // Handle source references (@, #, $)
        if word.starts_with('@') || word.starts_with('#') || word.starts_with('$') {
            result.push_str(word);
            continue;
        }

        // Try to convert keyword to glyph
        let lower = word.to_lowercase();
        if let Some(glyph) = keyword_to_glyph(&lower) {
            result.push(glyph.0);
        } else {
            // Keep identifiers and literals as-is
            result.push_str(word);
        }
    }

    result
}

/// Compress with explicit block markers (alternative format)
pub fn compress_explicit(source: &str) -> String {
    let mut output = String::new();
    let mut prev_indent = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = calculate_indent(line);
        let content = strip_tree_chars(trimmed);

        // Handle depth changes
        if indent > prev_indent {
            output.push('⟨');
        } else if indent < prev_indent {
            let levels = (prev_indent - indent) / 4;
            for _ in 0..levels {
                output.push('⟩');
            }
        }

        output.push_str(&glyphify(&content));
        prev_indent = indent;
    }

    // Close remaining blocks
    for _ in 0..(prev_indent / 4) {
        output.push('⟩');
    }

    output
}

/// Ultra-compact mode: strip all non-essential characters
pub fn compress_ultra(source: &str) -> String {
    let normal = compress(source);
    // Further compress by removing spaces between glyphs
    normal.glyphs
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Compress with squeeze: glyphs + vowel removal for maximum compression
/// AI-friendly lossy compression - identifiers are squeezed but still understandable
pub fn compress_squeezed(source: &str, level: SqueezeLevel) -> CompressedCodie {
    let original_size = source.len();
    let mut output = String::new();
    let mut depth_stack: VecDeque<usize> = VecDeque::new();
    let mut max_depth = 0;
    let mut current_depth = 0;

    for line in source.lines() {
        let processed = process_line_squeezed(line, &mut depth_stack, &mut current_depth, level);
        if !processed.is_empty() {
            output.push_str(&processed);
        }
        max_depth = max_depth.max(current_depth);
    }

    // Close any remaining open blocks
    while depth_stack.pop_back().is_some() {
        output.push('⟩');
    }

    let compressed_size = output.len();

    CompressedCodie {
        glyphs: output,
        original_size,
        compressed_size,
        max_depth,
    }
}

/// Process a line with squeeze enabled
fn process_line_squeezed(
    line: &str,
    depth_stack: &mut VecDeque<usize>,
    current_depth: &mut usize,
    level: SqueezeLevel,
) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
        return String::new();
    }

    let indent = calculate_indent(line);

    let mut result = String::new();
    while let Some(&prev_indent) = depth_stack.back() {
        if indent <= prev_indent {
            depth_stack.pop_back();
            result.push('⟩');
            *current_depth = current_depth.saturating_sub(1);
        } else {
            break;
        }
    }

    let content = strip_tree_chars(trimmed);
    if content.is_empty() {
        return result;
    }

    let glyphed = glyphify_squeezed(&content, level);

    let opens_block = line.contains("├") || line.contains("┌") ||
                      content.ends_with(':') || is_block_opener(&content);

    result.push_str(&glyphed);

    if opens_block {
        result.push('⟨');
        depth_stack.push_back(indent);
        *current_depth += 1;
    }

    result
}

/// Convert text to glyph form with squeeze applied to identifiers
fn glyphify_squeezed(content: &str, level: SqueezeLevel) -> String {
    let mut result = String::new();
    let words = content.split_whitespace();

    for word in words {
        // Handle special operators
        if word == "←" || word == "→" || word == "?" {
            result.push_str(word);
            continue;
        }

        // Handle source references (@, #, $) - squeeze the path part
        if let Some(prefix) = word.chars().next() {
            if prefix == '@' || prefix == '#' || prefix == '$' {
                result.push(prefix);
                let rest = &word[1..];
                // Squeeze each path segment
                let squeezed_path: String = rest
                    .split('/')
                    .map(|seg| squeeze_identifier(seg, level))
                    .collect::<Vec<_>>()
                    .join("/");
                result.push_str(&squeezed_path);
                continue;
            }
        }

        // Try to convert keyword to glyph
        let lower = word.to_lowercase();
        if let Some(glyph) = keyword_to_glyph(&lower) {
            result.push(glyph.0);
        } else {
            // Squeeze identifier
            let squeezed = squeeze_identifier(word, level);
            result.push_str(&squeezed);
        }
    }

    result
}

/// Maximum compression: glyphs + aggressive squeeze + strip whitespace
pub fn compress_max(source: &str) -> CompressedCodie {
    let squeezed = compress_squeezed(source, SqueezeLevel::Aggressive);
    let ultra: String = squeezed.glyphs
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    CompressedCodie {
        glyphs: ultra,
        original_size: squeezed.original_size,
        compressed_size: squeezed.glyphs.len(),
        max_depth: squeezed.max_depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_compression() {
        let source = "pug LOGIN\ntreat → token";
        let compressed = compress(source);
        assert!(compressed.glyphs.contains('ρ')); // pug
        assert!(compressed.glyphs.contains('μ')); // treat
        assert!(compressed.ratio() > 0.0);
    }

    #[test]
    fn test_tree_stripping() {
        let source = r#"pug TEST
├── bark data
└── treat → result"#;
        let compressed = compress(source);
        // Should not contain tree chars
        assert!(!compressed.glyphs.contains('├'));
        assert!(!compressed.glyphs.contains('└'));
        assert!(!compressed.glyphs.contains('─'));
    }

    #[test]
    fn test_logic_gates() {
        let source = "sniff x and y or not z";
        let compressed = compress(source);
        assert!(compressed.glyphs.contains('∧')); // and
        assert!(compressed.glyphs.contains('∨')); // or
        assert!(compressed.glyphs.contains('¬')); // not
    }

    #[test]
    fn test_booleans() {
        let source = "? valid → wag\n? invalid → whine";
        let compressed = compress(source);
        assert!(compressed.glyphs.contains('⊤')); // wag/true
        assert!(compressed.glyphs.contains('⊥')); // whine/false
    }

    #[test]
    fn test_nested_structure() {
        let source = r#"pug OUTER
├── fence
│   └── bone NOT: fail
└── treat → done"#;
        let compressed = compress(source);
        // Should have opening/closing brackets
        assert!(compressed.glyphs.contains('⟨'));
        assert!(compressed.glyphs.contains('⟩'));
    }

    #[test]
    fn test_compression_ratio() {
        let source = r#"pug AUTHENTICATE
├── fence
│   ├── bone NOT: store passwords plain
│   └── bone NOT: unlimited attempts
├── bark user ← @database/users
│   └── ? not found → whine "Invalid"
└── treat → {token, user_id}"#;

        let compressed = compress(source);
        // Should achieve significant compression
        assert!(compressed.ratio() > 0.3, "Expected >30% compression, got {:.1}%", compressed.ratio() * 100.0);
        println!("Compression: {}", compressed.stats());
    }

    #[test]
    fn test_source_refs_preserved() {
        let source = "bark data ← @database/users\nbury key $vault/secrets";
        let compressed = compress(source);
        assert!(compressed.glyphs.contains("@database"));
        assert!(compressed.glyphs.contains("$vault"));
    }
}
