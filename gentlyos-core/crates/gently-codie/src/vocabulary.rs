//! CODIE Dog Vocabulary
//!
//! Multiple ways to say the same thing - from normie to terse.
//! All parse to the same semantic meaning.
//!
//! ## Design Philosophy
//!
//! CODIE was named after a dog. The vocabulary reflects this:
//! - Simple words anyone knows (sit, stay, fetch)
//! - Can be written as natural dog stories
//! - AI understands the semantic meaning
//! - Encrypted traffic looks like dog fanfiction
//! - Even decrypted, attackers see nonsense dog stories

use std::collections::HashMap;
use lazy_static::lazy_static;

/// Semantic meaning that multiple words can map to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DogSemantic {
    // Core actions
    Entry,      // Start point (pug)
    Fetch,      // Get/retrieve (bark, fetch, get)
    Loop,       // Repeat (chase, spin, run)
    Define,     // Create function (trick, teach, howl)
    Bind,       // Assign variable (tag, name, call)
    Incomplete, // WIP marker (sniff, dig)
    Constrain,  // Set limits (fence, yard, boundary)
    Exact,      // Precise spec (sit, stay, pin)
    Immutable,  // Cannot change (bone, guard, protect)
    Flexible,   // Can change (play, blob, wiggle)
    Goal,       // Output/return (treat, reward, give)
    Save,       // Checkpoint (bury, hide, stash)

    // Control flow
    If,         // Conditional
    Else,       // Alternative
    While,      // Loop while true
    For,        // Counted loop
    Break,      // Exit loop
    Continue,   // Next iteration
    Return,     // Return value

    // Logic
    And,        // Both true
    Or,         // Either true
    Not,        // Negation
    True,       // Boolean true (wag, happy, yes)
    False,      // Boolean false (whine, sad, no)

    // Actions
    Transform,  // Change form (roll, shake, morph)
    Validate,   // Check validity (sniff, smell, check)
    Store,      // Save data (bury, hide, stash)
    Protect,    // Security (guard, watch, protect)
    Group,      // Collection (pack, litter, group)
    Call,       // Invoke (howl, bark, call)
    Wait,       // Pause/await (sit, stay, wait)
    Give,       // Hand over (shake, give, paw)

    // Meta
    Breed,      // Language type (breed, type, kind)
    Speak,      // Generate output (speak, say, tell)
}

/// A vocabulary entry with all synonyms
#[derive(Debug, Clone)]
pub struct VocabEntry {
    pub semantic: DogSemantic,
    pub canonical: &'static str,      // The "official" keyword
    pub terse: &'static str,          // Short form
    pub friendly: &'static [&'static str],  // Natural language forms
    pub story_patterns: &'static [&'static str], // Patterns for story mode
}

lazy_static! {
    /// The complete dog vocabulary
    pub static ref DOG_VOCAB: Vec<VocabEntry> = vec![
        // === Core 12 ===
        VocabEntry {
            semantic: DogSemantic::Entry,
            canonical: "pug",
            terse: "pug",
            friendly: &["pug", "start", "begin", "enter"],
            story_patterns: &["The pug", "A pug", "Once upon a time, a pug"],
        },
        VocabEntry {
            semantic: DogSemantic::Fetch,
            canonical: "bark",
            terse: "bark",
            friendly: &["bark", "fetch", "get", "retrieve", "grab"],
            story_patterns: &["barked for", "fetched", "went to get", "retrieved"],
        },
        VocabEntry {
            semantic: DogSemantic::Loop,
            canonical: "chase",
            terse: "chase",
            friendly: &["chase", "spin", "run", "loop", "repeat"],
            story_patterns: &["chased", "kept chasing", "ran around", "spun in circles"],
        },
        VocabEntry {
            semantic: DogSemantic::Define,
            canonical: "trick",
            terse: "trick",
            friendly: &["trick", "teach", "learn", "define", "howl"],
            story_patterns: &["learned a trick", "was taught to", "knows how to"],
        },
        VocabEntry {
            semantic: DogSemantic::Bind,
            canonical: "tag",
            terse: "tag",
            friendly: &["tag", "name", "call", "label", "mark"],
            story_patterns: &["was tagged", "named", "called", "marked as"],
        },
        VocabEntry {
            semantic: DogSemantic::Incomplete,
            canonical: "sniff",
            terse: "sniff",
            friendly: &["sniff", "dig", "search", "investigate", "explore"],
            story_patterns: &["sniffed around", "was digging for", "still searching"],
        },
        VocabEntry {
            semantic: DogSemantic::Constrain,
            canonical: "fence",
            terse: "fence",
            friendly: &["fence", "yard", "boundary", "limit", "contain"],
            story_patterns: &["behind the fence", "within the yard", "bounded by"],
        },
        VocabEntry {
            semantic: DogSemantic::Exact,
            canonical: "sit",
            terse: "sit",
            friendly: &["sit", "stay", "hold", "pin", "exact"],
            story_patterns: &["sat down", "stayed put", "held position", "sat exactly"],
        },
        VocabEntry {
            semantic: DogSemantic::Immutable,
            canonical: "bone",
            terse: "bone",
            friendly: &["bone", "guard", "protect", "keep", "permanent"],
            story_patterns: &["guarded the bone", "protected", "kept safe", "never let go of"],
        },
        VocabEntry {
            semantic: DogSemantic::Flexible,
            canonical: "play",
            terse: "play",
            friendly: &["play", "wiggle", "loose", "flexible", "whatever"],
            story_patterns: &["played with", "wiggled", "was flexible about"],
        },
        VocabEntry {
            semantic: DogSemantic::Goal,
            canonical: "treat",
            terse: "treat",
            friendly: &["treat", "reward", "give", "return", "output"],
            story_patterns: &["got a treat", "was rewarded with", "received"],
        },
        VocabEntry {
            semantic: DogSemantic::Save,
            canonical: "bury",
            terse: "bury",
            friendly: &["bury", "hide", "stash", "save", "anchor"],
            story_patterns: &["buried", "hid", "stashed away", "saved for later"],
        },

        // === Booleans ===
        VocabEntry {
            semantic: DogSemantic::True,
            canonical: "wag",
            terse: "wag",
            friendly: &["wag", "happy", "yes", "good", "true"],
            story_patterns: &["wagged happily", "was happy", "said yes"],
        },
        VocabEntry {
            semantic: DogSemantic::False,
            canonical: "whine",
            terse: "whine",
            friendly: &["whine", "sad", "bad", "false", "nope"],
            story_patterns: &["whined", "was sad", "said no"],
        },

        // === Control Flow ===
        VocabEntry {
            semantic: DogSemantic::If,
            canonical: "if",
            terse: "?",
            friendly: &["if", "when", "check", "maybe"],
            story_patterns: &["if", "when", "in case"],
        },
        VocabEntry {
            semantic: DogSemantic::Else,
            canonical: "else",
            terse: ":",
            friendly: &["else", "otherwise", "or"],
            story_patterns: &["otherwise", "or else", "if not"],
        },
        VocabEntry {
            semantic: DogSemantic::While,
            canonical: "while",
            terse: "while",
            friendly: &["while", "during", "as long as"],
            story_patterns: &["while", "as long as", "during"],
        },
        VocabEntry {
            semantic: DogSemantic::Break,
            canonical: "stop",
            terse: "stop",
            friendly: &["stop", "halt", "break", "enough"],
            story_patterns: &["stopped", "halted", "had enough"],
        },
        VocabEntry {
            semantic: DogSemantic::Return,
            canonical: "shake",
            terse: "→",
            friendly: &["shake", "give", "return", "hand", "paw"],
            story_patterns: &["shook hands", "gave", "returned", "offered a paw"],
        },

        // === Logic Gates ===
        VocabEntry {
            semantic: DogSemantic::And,
            canonical: "and",
            terse: "&",
            friendly: &["and", "also", "plus", "with"],
            story_patterns: &["and", "along with", "together with"],
        },
        VocabEntry {
            semantic: DogSemantic::Or,
            canonical: "or",
            terse: "|",
            friendly: &["or", "either", "maybe"],
            story_patterns: &["or", "either", "perhaps"],
        },
        VocabEntry {
            semantic: DogSemantic::Not,
            canonical: "not",
            terse: "!",
            friendly: &["not", "no", "never", "without"],
            story_patterns: &["not", "never", "without", "didn't"],
        },

        // === Actions ===
        VocabEntry {
            semantic: DogSemantic::Transform,
            canonical: "roll",
            terse: "roll",
            friendly: &["roll", "transform", "change", "morph", "shake"],
            story_patterns: &["rolled over", "transformed into", "changed to"],
        },
        VocabEntry {
            semantic: DogSemantic::Validate,
            canonical: "sniff",
            terse: "sniff",
            friendly: &["sniff", "smell", "check", "validate", "inspect"],
            story_patterns: &["sniffed", "smelled", "checked", "inspected"],
        },
        VocabEntry {
            semantic: DogSemantic::Group,
            canonical: "pack",
            terse: "pack",
            friendly: &["pack", "litter", "group", "bunch", "collection"],
            story_patterns: &["the pack", "a litter of", "grouped together"],
        },
        VocabEntry {
            semantic: DogSemantic::Wait,
            canonical: "stay",
            terse: "stay",
            friendly: &["stay", "wait", "hold", "pause"],
            story_patterns: &["stayed", "waited", "held still"],
        },

        // === Meta ===
        VocabEntry {
            semantic: DogSemantic::Breed,
            canonical: "breed",
            terse: "breed",
            friendly: &["breed", "type", "kind", "species"],
            story_patterns: &["was a", "breed of", "type of"],
        },
        VocabEntry {
            semantic: DogSemantic::Speak,
            canonical: "speak",
            terse: "speak",
            friendly: &["speak", "say", "tell", "bark out", "announce"],
            story_patterns: &["spoke", "said", "told", "announced"],
        },
    ];

    /// Word to semantic mapping for fast lookup
    pub static ref WORD_TO_SEMANTIC: HashMap<&'static str, DogSemantic> = {
        let mut map = HashMap::new();
        for entry in DOG_VOCAB.iter() {
            map.insert(entry.canonical, entry.semantic);
            map.insert(entry.terse, entry.semantic);
            for word in entry.friendly {
                map.insert(*word, entry.semantic);
            }
        }
        map
    };
}

/// Check if a word is in the dog vocabulary
pub fn is_dog_word(word: &str) -> bool {
    WORD_TO_SEMANTIC.contains_key(word.to_lowercase().as_str())
}

/// Get the semantic meaning of a word
pub fn get_semantic(word: &str) -> Option<DogSemantic> {
    WORD_TO_SEMANTIC.get(word.to_lowercase().as_str()).copied()
}

/// Get the canonical keyword for a semantic
pub fn get_canonical(semantic: DogSemantic) -> &'static str {
    DOG_VOCAB
        .iter()
        .find(|e| e.semantic == semantic)
        .map(|e| e.canonical)
        .unwrap_or("unknown")
}

/// Normalize any dog word to its canonical form
pub fn normalize(word: &str) -> Option<&'static str> {
    get_semantic(word).map(get_canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synonyms_resolve() {
        // All these should mean the same thing
        assert_eq!(get_semantic("fetch"), Some(DogSemantic::Fetch));
        assert_eq!(get_semantic("bark"), Some(DogSemantic::Fetch));
        assert_eq!(get_semantic("get"), Some(DogSemantic::Fetch));
        assert_eq!(get_semantic("retrieve"), Some(DogSemantic::Fetch));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("fetch"), Some("bark"));
        assert_eq!(normalize("get"), Some("bark"));
        assert_eq!(normalize("spin"), Some("chase"));
        assert_eq!(normalize("loop"), Some("chase"));
    }

    #[test]
    fn test_booleans() {
        assert_eq!(get_semantic("wag"), Some(DogSemantic::True));
        assert_eq!(get_semantic("happy"), Some(DogSemantic::True));
        assert_eq!(get_semantic("yes"), Some(DogSemantic::True));

        assert_eq!(get_semantic("whine"), Some(DogSemantic::False));
        assert_eq!(get_semantic("sad"), Some(DogSemantic::False));
        assert_eq!(get_semantic("nope"), Some(DogSemantic::False));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(get_semantic("BARK"), Some(DogSemantic::Fetch));
        assert_eq!(get_semantic("Bark"), Some(DogSemantic::Fetch));
        assert_eq!(get_semantic("bark"), Some(DogSemantic::Fetch));
    }
}
