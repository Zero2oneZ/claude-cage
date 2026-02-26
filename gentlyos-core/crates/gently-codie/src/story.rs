//! CODIE Story Mode Parser
//!
//! Converts natural language dog stories into CODIE instructions.
//! This enables steganographic encoding where real instructions
//! look like innocent dog fanfiction.
//!
//! ## Example
//!
//! ```text
//! Input (story):
//!   "The pug wandered into the database yard.
//!    He barked for a user named 'admin'.
//!    If the bone wasn't valid, he whined sadly.
//!    Good boy! Here's your treat: a fresh token."
//!
//! Output (CODIE):
//!   pug @database
//!   bark user ← "admin"
//!   ? not valid → whine
//!   treat → token
//! ```

use crate::vocabulary::{get_semantic, DogSemantic};
use regex::Regex;
use lazy_static::lazy_static;

/// A parsed story fragment
#[derive(Debug, Clone)]
pub struct StoryFragment {
    pub semantic: DogSemantic,
    pub subject: Option<String>,
    pub object: Option<String>,
    pub modifier: Option<String>,
    pub raw: String,
}

/// Story parser for converting dog stories to CODIE
pub struct StoryParser {
    fragments: Vec<StoryFragment>,
}

lazy_static! {
    // Pattern: "The pug [verbed] [object]"
    static ref PUG_PATTERN: Regex = Regex::new(
        r"(?i)(?:the |a )?pug\s+(\w+)\s*(?:into |to |for |at )?\s*(?:the )?(\w+)?"
    ).unwrap();

    // Pattern: "[subject] barked/fetched for [object]"
    static ref FETCH_PATTERN: Regex = Regex::new(
        r"(?i)(?:he |she |it |the pug )?(barked|fetched|got|retrieved|grabbed)\s+(?:for |a |the )?(\w+)"
    ).unwrap();

    // Pattern: "if [condition]" or "when [condition]"
    static ref IF_PATTERN: Regex = Regex::new(
        r"(?i)(?:if|when|in case)\s+(?:the )?(.+?)(?:,|then|→)"
    ).unwrap();

    // Pattern: "treat/reward: [value]" or "got a treat: [value]"
    static ref TREAT_PATTERN: Regex = Regex::new(
        r"(?i)(?:treat|reward|got a treat|here'?s? (?:your |a )?treat)[:\s]+(\w+)"
    ).unwrap();

    // Pattern: "sat/stayed [condition]"
    static ref SIT_PATTERN: Regex = Regex::new(
        r"(?i)(?:sat|stayed|waited)\s+(?:until |for |while )?(.+)"
    ).unwrap();

    // Pattern: "bone NOT: [rule]" or "guarded the bone: [rule]"
    static ref BONE_PATTERN: Regex = Regex::new(
        r"(?i)(?:bone\s+not|never\s+let|guarded)[:\s]+(.+)"
    ).unwrap();

    // Pattern: "wagged/happy" = true, "whined/sad" = false
    static ref BOOL_PATTERN: Regex = Regex::new(
        r"(?i)(wagged|happy|happily|yes|good)|(whined|sad|sadly|no|bad)"
    ).unwrap();

    // Pattern: "chased [target] [count] times" or "kept chasing"
    static ref CHASE_PATTERN: Regex = Regex::new(
        r"(?i)(?:chased|kept chasing|ran around|spun)\s*(?:the )?(\w+)?\s*(?:(\d+)\s*times?)?"
    ).unwrap();

    // Pattern: "buried [object] by/at [location]"
    static ref BURY_PATTERN: Regex = Regex::new(
        r"(?i)(?:buried|hid|stashed)\s+(?:the )?(\w+)\s+(?:by|at|in)\s+(?:the )?(\w+)"
    ).unwrap();
}

impl StoryParser {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    /// Parse a dog story into semantic fragments
    pub fn parse(&mut self, story: &str) -> Vec<StoryFragment> {
        self.fragments.clear();

        // Split into sentences
        let sentences: Vec<&str> = story
            .split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
            .filter(|s| !s.trim().is_empty())
            .collect();

        for sentence in sentences {
            self.parse_sentence(sentence.trim());
        }

        self.fragments.clone()
    }

    fn parse_sentence(&mut self, sentence: &str) {
        // Try each pattern
        if let Some(frag) = self.try_pug_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_fetch_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_if_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_treat_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_sit_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_bone_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_chase_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }
        if let Some(frag) = self.try_bury_pattern(sentence) {
            self.fragments.push(frag);
            return;
        }

        // Try word-by-word for simple statements
        self.try_word_extraction(sentence);
    }

    fn try_pug_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = PUG_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Entry,
                subject: Some("pug".to_string()),
                object: caps.get(2).map(|m| m.as_str().to_string()),
                modifier: caps.get(1).map(|m| m.as_str().to_string()),
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_fetch_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = FETCH_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Fetch,
                subject: None,
                object: caps.get(2).map(|m| m.as_str().to_string()),
                modifier: caps.get(1).map(|m| m.as_str().to_string()),
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_if_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = IF_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::If,
                subject: None,
                object: caps.get(1).map(|m| m.as_str().trim().to_string()),
                modifier: None,
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_treat_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = TREAT_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Goal,
                subject: None,
                object: caps.get(1).map(|m| m.as_str().to_string()),
                modifier: None,
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_sit_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = SIT_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Exact,
                subject: None,
                object: caps.get(1).map(|m| m.as_str().trim().to_string()),
                modifier: None,
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_bone_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = BONE_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Immutable,
                subject: None,
                object: caps.get(1).map(|m| m.as_str().trim().to_string()),
                modifier: Some("NOT".to_string()),
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_chase_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = CHASE_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Loop,
                subject: None,
                object: caps.get(1).map(|m| m.as_str().to_string()),
                modifier: caps.get(2).map(|m| m.as_str().to_string()),
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_bury_pattern(&self, sentence: &str) -> Option<StoryFragment> {
        if let Some(caps) = BURY_PATTERN.captures(sentence) {
            Some(StoryFragment {
                semantic: DogSemantic::Save,
                subject: caps.get(1).map(|m| m.as_str().to_string()),
                object: caps.get(2).map(|m| m.as_str().to_string()),
                modifier: None,
                raw: sentence.to_string(),
            })
        } else {
            None
        }
    }

    fn try_word_extraction(&mut self, sentence: &str) {
        // Extract individual semantic words
        for word in sentence.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(semantic) = get_semantic(clean) {
                self.fragments.push(StoryFragment {
                    semantic,
                    subject: None,
                    object: None,
                    modifier: None,
                    raw: word.to_string(),
                });
            }
        }
    }

    /// Convert fragments to CODIE source
    pub fn to_codie(&self) -> String {
        let mut output = String::new();

        for frag in &self.fragments {
            let line = match frag.semantic {
                DogSemantic::Entry => {
                    format!("pug {}", frag.object.as_deref().unwrap_or("PROGRAM"))
                }
                DogSemantic::Fetch => {
                    format!(
                        "bark {} ← @source",
                        frag.object.as_deref().unwrap_or("data")
                    )
                }
                DogSemantic::If => {
                    format!("? {} →", frag.object.as_deref().unwrap_or("condition"))
                }
                DogSemantic::Goal => {
                    format!("treat → {}", frag.object.as_deref().unwrap_or("result"))
                }
                DogSemantic::Exact => {
                    format!("sit {}", frag.object.as_deref().unwrap_or(""))
                }
                DogSemantic::Immutable => {
                    format!(
                        "bone NOT: {}",
                        frag.object.as_deref().unwrap_or("rule")
                    )
                }
                DogSemantic::Loop => {
                    if let Some(count) = &frag.modifier {
                        format!("chase {} TIMES", count)
                    } else {
                        format!(
                            "chase {}",
                            frag.object.as_deref().unwrap_or("FOREVER")
                        )
                    }
                }
                DogSemantic::Save => {
                    format!(
                        "bury {} @{}",
                        frag.subject.as_deref().unwrap_or("data"),
                        frag.object.as_deref().unwrap_or("location")
                    )
                }
                DogSemantic::True => "wag".to_string(),
                DogSemantic::False => "whine".to_string(),
                _ => continue,
            };
            output.push_str(&line);
            output.push('\n');
        }

        output
    }
}

impl Default for StoryParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a dog story to CODIE
pub fn story_to_codie(story: &str) -> String {
    let mut parser = StoryParser::new();
    parser.parse(story);
    parser.to_codie()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_story() {
        let story = "The pug wandered into the database. He barked for a user.";
        let mut parser = StoryParser::new();
        let frags = parser.parse(story);

        assert!(frags.iter().any(|f| f.semantic == DogSemantic::Entry));
        assert!(frags.iter().any(|f| f.semantic == DogSemantic::Fetch));
    }

    #[test]
    fn test_treat_pattern() {
        let story = "Good boy! Here's your treat: token";
        let mut parser = StoryParser::new();
        let frags = parser.parse(story);

        let treat = frags.iter().find(|f| f.semantic == DogSemantic::Goal);
        assert!(treat.is_some());
        assert_eq!(treat.unwrap().object, Some("token".to_string()));
    }

    #[test]
    fn test_bone_pattern() {
        let story = "Bone NOT: store passwords in plaintext";
        let mut parser = StoryParser::new();
        let frags = parser.parse(story);

        let bone = frags.iter().find(|f| f.semantic == DogSemantic::Immutable);
        assert!(bone.is_some());
    }

    #[test]
    fn test_full_conversion() {
        let story = r#"
            The pug wandered into the authentication yard.
            He barked for a user named admin.
            If the bone wasn't valid, he whined.
            Good boy! Here's your treat: token.
        "#;

        let codie = story_to_codie(story);
        assert!(codie.contains("pug"));
        assert!(codie.contains("bark"));
        assert!(codie.contains("treat"));
    }
}
