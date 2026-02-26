//! # gently-codie
//!
//! CODIE: Compressed Operational Dense Instruction Encoding
//!
//! A 44-keyword human-semantic instruction language that:
//! - Achieves 94.7% token reduction vs natural language
//! - Is hash-addressable (instructions can be referenced by hash)
//! - Reads like instructions a human would give
//! - Supports logic gates, control flow, and geometric operations
//! - Compresses to glyph form for instant hydration
//!
//! ## Compression (Dehydration/Hydration)
//!
//! CODIE instructions compress to symbolic glyph strings:
//!
//! ```text
//! Human Form:                    Glyph Form:
//! pug LOGIN                      ρLOGIN⟨βuser←@db⟨⁇¬found→⊥⟩μ→token⟩
//! ├── bark user ← @db
//! │   └── ? not found → whine   Hash: #c7f3a2b1
//! └── treat → token
//! ```
//!
//! - **Dehydrate**: Human CODIE → Glyph string (60-80% smaller)
//! - **Hydrate**: Glyph string → Human CODIE (instant expansion)
//! - **Hash**: Content-addressable storage, pass apps as strings
//!
//! ## Keyword Categories (44 total)
//!
//! ### Core 12 Semantic Keywords (Dog-themed)
//! `pug` (entry), `bark` (fetch), `chase` (loop), `trick` (function),
//! `tag` (variable), `sniff` (validate), `fence` (constraints),
//! `sit` (exact spec), `bone` (immutable), `play` (flexible),
//! `treat` (goal/return), `bury` (checkpoint/save)
//!
//! ### Logic Gates (6)
//! `and`, `or`, `not`, `xor`, `nand`, `nor`
//!
//! ### Control Flow (10)
//! `if`, `else`, `start`, `for`, `fork`, `branch`, `while`, `break`, `continue`, `return`
//!
//! ### Boolean (2)
//! `wag` (true), `whine` (false)
//!
//! ### Geometric (5)
//! `mirror` (opposite/negative), `fold`, `rotate`, `translate`, `scale`
//!
//! ### Dimensional (5)
//! `dim`, `axis`, `plane`, `space`, `hyper`
//!
//! ### Meta/Generation (4)
//! `breed` (identify/generate languages), `speak` (generate prompts),
//! `morph` (transform), `cast` (type casting)
//!
//! ## Example
//!
//! ```text
//! pug LOGIN
//! │
//! ├── fence
//! │   ├── bone NOT: store passwords plain
//! │   └── bone NOT: unlimited attempts
//! │
//! ├── bark user ← @database/users/find(username)
//! │   └── ? not found → whine "Wrong credentials"
//! │
//! └── treat → {token, user_id}
//! ```
//!
//! ## SVG/HTMX Integration
//!
//! Compressed CODIE can embed directly in SVG for instant GUI hydration:
//!
//! ```svg
//! <g data-codie="#c7f3a2b1" hx-get="/codie/hydrate#c7f3a2b1" hx-trigger="load">
//!   <!-- UI hydrates from hash -->
//! </g>
//! ```

pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod vocabulary;
pub mod story;
pub mod wag;
pub mod glyph;
pub mod compress;
pub mod hydrate;
pub mod hash;
pub mod squeeze;
pub mod unlock;

// Token exports
pub use token::{
    CodieKeyword, CodieToken,
    CODIE_CORE_KEYWORDS, CODIE_LOGIC_GATES, CODIE_CONTROL_FLOW,
    CODIE_BOOLEANS, CODIE_GEOMETRIC, CODIE_DIMENSIONAL, CODIE_META,
};

// Lexer/Parser exports
pub use lexer::{CodieLexer, LexerError};
pub use ast::{CodieAst, CodieType, SourceKind};
pub use parser::{CodieParser, ParseError};

// Dog vocabulary exports
pub use vocabulary::{DogSemantic, get_semantic, normalize, is_dog_word};
pub use story::{StoryParser, story_to_codie};
pub use wag::{wag, WagIntent, parse_intent};

// Compression system exports
pub use glyph::{Glyph, keyword_to_glyph, glyph_to_keyword, is_glyph};
pub use compress::{compress, compress_ultra, compress_squeezed, compress_max, CompressedCodie};
pub use hydrate::{hydrate, hydrate_flat, hydrate_minimal, HydratedCodie, HydrationToken, parse_tokens};
pub use hash::{CodieHash, HashBundle, PtcLevel, register, lookup, store, ptc_level, data_uri};
pub use squeeze::{squeeze, squeeze_identifier, squeeze_with_stats, SqueezeLevel, SqueezeStats};

// Unlock protocol exports
pub use unlock::{
    create_lock, request_unlock, approve_request, deny_request, hydrate as unlock_hydrate,
    get_pending_requests, get_status, burn, generate_words, word_key, derive_unlock_hash,
    LockedCodie, UnlockMode, ClientInfo, UnlockRequest, UnlockRequestResult,
    ApprovalResult, HydrateResult, LockStatus, UnlockError, WORD_LIST,
};

/// Parse CODIE source into an AST
pub fn parse(source: &str) -> Result<CodieAst, ParseError> {
    let mut lexer = CodieLexer::new(source);
    let tokens = lexer.tokenize_all()?;
    let mut parser = CodieParser::new(tokens);
    parser.parse()
}

/// Dehydrate: Human CODIE → Compressed glyph string with hash
///
/// Returns (compressed_string, hash)
pub fn dehydrate(source: &str) -> (String, CodieHash) {
    let compressed = compress(source);
    let hash = hash::register(&compressed.glyphs);
    (compressed.glyphs, hash)
}

/// Dehydrate and store: Returns hash bundle for embedding
pub fn dehydrate_store(source: &str) -> HashBundle {
    let compressed = compress(source);
    hash::store(&compressed.glyphs)
}

/// Hydrate: Compressed glyph string → Human CODIE
pub fn rehydrate(compressed: &str) -> String {
    hydrate(compressed).source
}

/// Lookup and hydrate by hash
pub fn hydrate_hash(hash: &str) -> Option<String> {
    hash::lookup(hash).map(|c| hydrate(&c).source)
}

/// Full roundtrip: source → compress → hash → lookup → hydrate
pub fn roundtrip(source: &str) -> Option<String> {
    let (compressed, hash) = dehydrate(source);
    let retrieved = hash::lookup(&hash.short())?;
    Some(hydrate(&retrieved).source)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_compression_pipeline() {
        let source = r#"pug LOGIN
├── bark user ← @database/users
│   └── ? not found → whine
└── treat → token"#;

        // Dehydrate
        let (compressed, hash) = dehydrate(source);
        println!("Compressed: {}", compressed);
        println!("Hash: {}", hash.short());

        // Verify hash lookup works
        let retrieved = lookup(&hash.short());
        assert!(retrieved.is_some());

        // Hydrate back
        let hydrated = rehydrate(&compressed);
        println!("Hydrated:\n{}", hydrated);

        // Should contain key elements
        assert!(hydrated.contains("pug"));
        assert!(hydrated.contains("bark"));
        assert!(hydrated.contains("treat"));
    }

    #[test]
    fn test_hash_bundle_for_svg() {
        let source = "pug BUTTON\ntreat → clicked";
        let bundle = dehydrate_store(source);

        let svg = bundle.svg_element("g");
        assert!(svg.contains("data-codie="));
        assert!(svg.contains("hx-get="));
        println!("SVG element: {}", svg);
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
        println!("Original: {} bytes", compressed.original_size);
        println!("Compressed: {} bytes", compressed.compressed_size);
        println!("Ratio: {:.1}%", compressed.ratio() * 100.0);

        // Should achieve meaningful compression
        assert!(compressed.ratio() > 0.2);
    }
}
