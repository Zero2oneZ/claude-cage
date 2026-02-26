//! CODIE Glyph System
//!
//! Maps CODIE keywords and operators to single-character glyphs for compression.
//! Like assembly opcodes, but semantic and hash-addressable.
//!
//! ## Design
//!
//! ```text
//! Human Form:           Glyph Form:
//! pug LOGIN             ρLOGIN
//! ├── bark user         βuser
//! │   └── ? found       ⁇found
//! └── treat → token     τ→token
//!
//! Compressed: ρLOGIN⟨βuser⟨⁇found⟩τ→token⟩
//! Hash: #7f3a...
//! ```
//!
//! ## Glyph Categories
//!
//! - Greek letters: Core semantic keywords
//! - Math symbols: Logic gates and operators
//! - Brackets: Structure delimiters
//! - Arrows: Flow and assignment

use std::collections::HashMap;
use lazy_static::lazy_static;

/// A glyph representing a CODIE keyword or operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Glyph(pub char);

impl Glyph {
    pub fn as_char(&self) -> char {
        self.0
    }

    pub fn as_str(&self) -> &'static str {
        // Return static string for common glyphs
        match self.0 {
            'ρ' => "ρ", 'β' => "β", 'χ' => "χ", 'τ' => "τ",
            'σ' => "σ", 'φ' => "φ", 'ω' => "ω", 'δ' => "δ",
            'π' => "π", 'λ' => "λ", 'μ' => "μ", 'ν' => "ν",
            'ε' => "ε", 'θ' => "θ", 'ψ' => "ψ", 'ξ' => "ξ",
            'α' => "α", 'γ' => "γ", 'η' => "η", 'ι' => "ι",
            'κ' => "κ", 'ζ' => "ζ", 'Ω' => "Ω", 'Δ' => "Δ",
            'Σ' => "Σ", 'Π' => "Π", 'Λ' => "Λ", 'Φ' => "Φ",
            '∧' => "∧", '∨' => "∨", '¬' => "¬", '⊕' => "⊕",
            '⊼' => "⊼", '⊽' => "⊽", '⊤' => "⊤", '⊥' => "⊥",
            '⟨' => "⟨", '⟩' => "⟩", '→' => "→", '←' => "←",
            '⁇' => "⁇", '∴' => "∴", '∵' => "∵", '⊢' => "⊢",
            '⊣' => "⊣", '⋈' => "⋈", '↺' => "↺", '↻' => "↻",
            '⇄' => "⇄", '⊛' => "⊛", '⊙' => "⊙", '⊚' => "⊚",
            _ => "?",
        }
    }
}

impl std::fmt::Display for Glyph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

lazy_static! {
    /// Core 12 semantic keywords → Greek letters
    pub static ref CORE_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("pug", Glyph('ρ'));      // rho - entry point
        m.insert("bark", Glyph('β'));     // beta - fetch
        m.insert("chase", Glyph('χ'));    // chi - loop
        m.insert("trick", Glyph('τ'));    // tau - define function
        m.insert("tag", Glyph('σ'));      // sigma - bind/assign
        m.insert("sniff", Glyph('φ'));    // phi - validate/incomplete
        m.insert("fence", Glyph('ω'));    // omega - constraint boundary
        m.insert("sit", Glyph('δ'));      // delta - exact/precise
        m.insert("bone", Glyph('π'));     // pi - immutable
        m.insert("play", Glyph('λ'));     // lambda - flexible
        m.insert("treat", Glyph('μ'));    // mu - return/goal
        m.insert("bury", Glyph('ν'));     // nu - save/checkpoint
        m
    };

    /// Logic gate keywords → Math symbols
    pub static ref LOGIC_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("and", Glyph('∧'));      // logical and
        m.insert("or", Glyph('∨'));       // logical or
        m.insert("not", Glyph('¬'));      // logical not
        m.insert("xor", Glyph('⊕'));      // exclusive or
        m.insert("nand", Glyph('⊼'));     // not-and
        m.insert("nor", Glyph('⊽'));      // not-or
        m
    };

    /// Boolean keywords → Truth symbols
    pub static ref BOOL_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("wag", Glyph('⊤'));      // true (tautology)
        m.insert("true", Glyph('⊤'));
        m.insert("whine", Glyph('⊥'));    // false (contradiction)
        m.insert("false", Glyph('⊥'));
        m
    };

    /// Control flow → Arrow/flow symbols
    pub static ref FLOW_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("if", Glyph('⁇'));       // conditional
        m.insert("else", Glyph('∴'));     // therefore/else
        m.insert("while", Glyph('↺'));    // loop back
        m.insert("for", Glyph('↻'));      // iterate
        m.insert("fork", Glyph('⋈'));     // parallel join
        m.insert("branch", Glyph('⊢'));   // branch right
        m.insert("break", Glyph('⊣'));    // break/stop
        m.insert("continue", Glyph('⇄')); // continue/swap
        m.insert("return", Glyph('→'));   // return arrow
        m.insert("start", Glyph('⊛'));    // start point
        m
    };

    /// Geometric operations → Transform symbols
    pub static ref GEO_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("mirror", Glyph('⊙'));   // reflection
        m.insert("fold", Glyph('⊚'));     // fold/wrap
        m.insert("rotate", Glyph('↻'));   // rotation (shared with for)
        m.insert("translate", Glyph('⇄')); // translation (shared with continue)
        m.insert("scale", Glyph('Σ'));    // sigma for scale/sum
        m
    };

    /// Dimensional keywords → Capital Greek
    pub static ref DIM_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("dim", Glyph('Δ'));      // dimension
        m.insert("axis", Glyph('Λ'));     // axis
        m.insert("plane", Glyph('Π'));    // plane (pi capital)
        m.insert("space", Glyph('Ω'));    // space (omega capital)
        m.insert("hyper", Glyph('Φ'));    // hyperspace
        m
    };

    /// Meta/generation keywords
    pub static ref META_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("breed", Glyph('ε'));    // epsilon - type/breed
        m.insert("speak", Glyph('η'));    // eta - speak/generate
        m.insert("morph", Glyph('θ'));    // theta - transform
        m.insert("cast", Glyph('ι'));     // iota - cast
        m.insert("roll", Glyph('κ'));     // kappa - roll/transform
        m.insert("pack", Glyph('ξ'));     // xi - pack/group
        m.insert("stay", Glyph('ψ'));     // psi - wait/stay
        m.insert("stop", Glyph('ζ'));     // zeta - stop
        m
    };

    /// Structural delimiters
    pub static ref STRUCT_GLYPHS: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.insert("open", Glyph('⟨'));     // open block
        m.insert("close", Glyph('⟩'));    // close block
        m.insert("assign", Glyph('←'));   // assignment
        m.insert("arrow", Glyph('→'));    // arrow/return
        m.insert("at", Glyph('@'));       // source reference
        m.insert("hash", Glyph('#'));     // hash reference
        m.insert("vault", Glyph('$'));    // vault/secure reference
        m
    };

    /// Combined glyph lookup (keyword → glyph)
    pub static ref KEYWORD_TO_GLYPH: HashMap<&'static str, Glyph> = {
        let mut m = HashMap::new();
        m.extend(CORE_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(LOGIC_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(BOOL_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(FLOW_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(GEO_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(DIM_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(META_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m.extend(STRUCT_GLYPHS.iter().map(|(k, v)| (*k, *v)));
        m
    };

    /// Reverse lookup (glyph → keyword)
    pub static ref GLYPH_TO_KEYWORD: HashMap<char, &'static str> = {
        let mut m = HashMap::new();
        // Core (prefer canonical forms)
        m.insert('ρ', "pug");
        m.insert('β', "bark");
        m.insert('χ', "chase");
        m.insert('τ', "trick");
        m.insert('σ', "tag");
        m.insert('φ', "sniff");
        m.insert('ω', "fence");
        m.insert('δ', "sit");
        m.insert('π', "bone");
        m.insert('λ', "play");
        m.insert('μ', "treat");
        m.insert('ν', "bury");
        // Logic
        m.insert('∧', "and");
        m.insert('∨', "or");
        m.insert('¬', "not");
        m.insert('⊕', "xor");
        m.insert('⊼', "nand");
        m.insert('⊽', "nor");
        // Bool
        m.insert('⊤', "wag");
        m.insert('⊥', "whine");
        // Flow
        m.insert('⁇', "if");
        m.insert('∴', "else");
        m.insert('↺', "while");
        m.insert('↻', "for");
        m.insert('⋈', "fork");
        m.insert('⊢', "branch");
        m.insert('⊣', "break");
        m.insert('⇄', "continue");
        m.insert('⊛', "start");
        // Geo
        m.insert('⊙', "mirror");
        m.insert('⊚', "fold");
        // Dim
        m.insert('Δ', "dim");
        m.insert('Λ', "axis");
        m.insert('Π', "plane");
        m.insert('Ω', "space");
        m.insert('Φ', "hyper");
        // Meta
        m.insert('ε', "breed");
        m.insert('η', "speak");
        m.insert('θ', "morph");
        m.insert('ι', "cast");
        m.insert('κ', "roll");
        m.insert('ξ', "pack");
        m.insert('ψ', "stay");
        m.insert('ζ', "stop");
        // Struct
        m.insert('⟨', "open");
        m.insert('⟩', "close");
        m.insert('←', "assign");
        m.insert('→', "arrow");
        m
    };
}

/// Get glyph for a keyword
pub fn keyword_to_glyph(keyword: &str) -> Option<Glyph> {
    KEYWORD_TO_GLYPH.get(keyword.to_lowercase().as_str()).copied()
}

/// Get keyword for a glyph
pub fn glyph_to_keyword(glyph: char) -> Option<&'static str> {
    GLYPH_TO_KEYWORD.get(&glyph).copied()
}

/// Check if character is a CODIE glyph
pub fn is_glyph(c: char) -> bool {
    GLYPH_TO_KEYWORD.contains_key(&c)
}

/// Check if character is a structural glyph
pub fn is_structural(c: char) -> bool {
    matches!(c, '⟨' | '⟩' | '←' | '→' | '@' | '#' | '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_glyphs() {
        assert_eq!(keyword_to_glyph("pug"), Some(Glyph('ρ')));
        assert_eq!(keyword_to_glyph("bark"), Some(Glyph('β')));
        assert_eq!(keyword_to_glyph("treat"), Some(Glyph('μ')));
    }

    #[test]
    fn test_logic_glyphs() {
        assert_eq!(keyword_to_glyph("and"), Some(Glyph('∧')));
        assert_eq!(keyword_to_glyph("or"), Some(Glyph('∨')));
        assert_eq!(keyword_to_glyph("not"), Some(Glyph('¬')));
    }

    #[test]
    fn test_reverse_lookup() {
        assert_eq!(glyph_to_keyword('ρ'), Some("pug"));
        assert_eq!(glyph_to_keyword('β'), Some("bark"));
        assert_eq!(glyph_to_keyword('⊤'), Some("wag"));
    }

    #[test]
    fn test_roundtrip() {
        let keywords = ["pug", "bark", "chase", "trick", "tag", "treat"];
        for kw in keywords {
            let glyph = keyword_to_glyph(kw).unwrap();
            let back = glyph_to_keyword(glyph.0).unwrap();
            assert_eq!(kw, back);
        }
    }
}
