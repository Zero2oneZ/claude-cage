//! CODIE Wag - Question → Instruction Transformer
//!
//! `wag` is the "I don't know how" command. Users ask natural questions,
//! and wag returns CODIE instructions as the answer.
//!
//! ## How It Works
//!
//! ```text
//! User:  wag "how do i authenticate a user"
//!
//! AI:    pug AUTH
//!        bark user ← @database/users(username)
//!        sniff password matches user.hash
//!        ? valid → treat {token, user_id}
//!        ? not valid → whine "Invalid credentials"
//! ```
//!
//! ## The Loop
//!
//! 1. User doesn't know how to do something
//! 2. User asks with `wag "question"`
//! 3. AI generates CODIE instructions
//! 4. User learns by seeing their intent as code
//! 5. User can modify/use the instructions directly
//!
//! ## Intent Categories
//!
//! wag detects intent and generates appropriate templates:
//! - "how do i make/create/build X" → scaffold template
//! - "how do i get/fetch/load X" → bark template
//! - "how do i check/validate/verify X" → sniff template
//! - "how do i save/store/persist X" → bury template
//! - "how do i loop/repeat/iterate X" → chase template
//! - "how do i protect/secure X" → fence + bone template

use regex::Regex;
use lazy_static::lazy_static;

/// Intent detected from a wag question
#[derive(Debug, Clone, PartialEq)]
pub enum WagIntent {
    /// Create/build something new
    Create { thing: String },
    /// Fetch/get data
    Fetch { source: String },
    /// Validate/check something
    Validate { target: String },
    /// Store/save data
    Store { data: String },
    /// Loop/iterate
    Loop { over: String },
    /// Secure/protect
    Secure { asset: String },
    /// Connect/integrate
    Connect { service: String },
    /// Transform/convert
    Transform { from: String, to: String },
    /// Delete/remove
    Delete { target: String },
    /// Update/modify
    Update { target: String },
    /// Unknown - pass to AI
    Unknown { question: String },
}

lazy_static! {
    // More specific patterns - action word must be right after "how do i" etc.
    static ref CREATE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:make|create|build|implement|add)\s+(?:a |an |the )?(.+)"
    ).unwrap();

    static ref FETCH_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:get|fetch|load|retrieve|read)\s+(?:a |an |the )?(.+?)(?:\s+from\s+(.+))?"
    ).unwrap();

    static ref VALIDATE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:check|validate|verify|confirm|test)\s+(?:if |that |whether )?(?:a |an |the )?(.+)"
    ).unwrap();

    static ref STORE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:save|store|persist|keep|write)\s+(?:a |an |the )?(.+)"
    ).unwrap();

    static ref LOOP_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:loop|iterate|repeat|go through|process)\s+(?:over |through |each )?(?:a |an |the )?(.+)"
    ).unwrap();

    static ref SECURE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:secure|protect|guard|encrypt|hash)\s+(?:a |an |the )?(.+)"
    ).unwrap();

    static ref CONNECT_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:connect|integrate|link|hook up)\s+(?:to |with )?(?:a |an |the )?(.+)"
    ).unwrap();

    static ref DELETE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:delete|remove|drop|clear)\s+(?:a |an |the )?(.+)"
    ).unwrap();

    static ref UPDATE_PATTERN: Regex = Regex::new(
        r"(?i)how (?:do i|can i|to) (?:update|modify|change|edit)\s+(?:a |an |the )?(.+)"
    ).unwrap();
}

/// Parse a wag question into intent
pub fn parse_intent(question: &str) -> WagIntent {
    let q = question.trim().trim_matches('"').trim_matches('\'');

    // Try patterns - most common first
    if let Some(caps) = CREATE_PATTERN.captures(q) {
        return WagIntent::Create {
            thing: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = FETCH_PATTERN.captures(q) {
        return WagIntent::Fetch {
            source: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = VALIDATE_PATTERN.captures(q) {
        return WagIntent::Validate {
            target: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = STORE_PATTERN.captures(q) {
        return WagIntent::Store {
            data: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = UPDATE_PATTERN.captures(q) {
        return WagIntent::Update {
            target: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = DELETE_PATTERN.captures(q) {
        return WagIntent::Delete {
            target: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = LOOP_PATTERN.captures(q) {
        return WagIntent::Loop {
            over: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = CONNECT_PATTERN.captures(q) {
        return WagIntent::Connect {
            service: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    if let Some(caps) = SECURE_PATTERN.captures(q) {
        return WagIntent::Secure {
            asset: caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
    }

    WagIntent::Unknown {
        question: q.to_string(),
    }
}

/// Generate CODIE template from intent
pub fn generate_template(intent: &WagIntent) -> String {
    match intent {
        WagIntent::Create { thing } => generate_create_template(thing),
        WagIntent::Fetch { source } => generate_fetch_template(source),
        WagIntent::Validate { target } => generate_validate_template(target),
        WagIntent::Store { data } => generate_store_template(data),
        WagIntent::Loop { over } => generate_loop_template(over),
        WagIntent::Secure { asset } => generate_secure_template(asset),
        WagIntent::Connect { service } => generate_connect_template(service),
        WagIntent::Delete { target } => generate_delete_template(target),
        WagIntent::Update { target } => generate_update_template(target),
        WagIntent::Transform { from, to } => generate_transform_template(from, to),
        WagIntent::Unknown { question } => generate_unknown_template(question),
    }
}

fn to_identifier(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_uppercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn generate_create_template(thing: &str) -> String {
    let name = to_identifier(thing);
    format!(r#"pug {name}

fence
├── bone NOT: crash on invalid input
└── bone NOT: lose data unexpectedly

sniff "{thing}" requirements
├── What data does it need?
├── What actions can user take?
└── What should it return?

trick create_{name}(input)
├── sniff input is valid
│   └── ? not valid → whine "Invalid input"
├── tag result ← process(input)
└── treat → result

trick get_{name}(id)
└── bark data ← @storage/{name}s/{{id}}

trick update_{name}(id, changes)
├── bark existing ← @storage/{name}s/{{id}}
├── tag updated ← existing + changes
└── bury updated @storage/{name}s/{{id}}

trick delete_{name}(id)
└── bury nothing @storage/{name}s/{{id}}

treat → {{create_{name}, get_{name}, update_{name}, delete_{name}}}
"#)
}

fn generate_fetch_template(source: &str) -> String {
    let name = to_identifier(source);
    format!(r#"pug FETCH_{name}

bark {source} ← @database/{source}
├── ? not found → whine "{source} not found"
├── ? error → whine "Failed to fetch {source}"
└── ? found → wag

treat → {source}
"#)
}

fn generate_validate_template(target: &str) -> String {
    let name = to_identifier(target);
    format!(r#"pug VALIDATE_{name}

sniff {target}
├── ? empty → whine "{target} cannot be empty"
├── ? invalid format → whine "{target} has invalid format"
├── ? too long → whine "{target} exceeds maximum length"
└── ? all checks pass → wag

treat → {{valid: wag, errors: pack}}
"#)
}

fn generate_store_template(data: &str) -> String {
    let name = to_identifier(data);
    format!(r#"pug STORE_{name}

fence
├── bone NOT: store without validation
└── bone NOT: overwrite without confirmation

sniff {data} is valid
├── ? not valid → whine "Cannot store invalid {data}"

bury {data} @storage/{data}
├── ? success → wag
└── ? error → whine "Failed to store {data}"

treat → {{stored: wag, id: generated_id}}
"#)
}

fn generate_loop_template(over: &str) -> String {
    let name = to_identifier(over);
    format!(r#"pug PROCESS_{name}

bark {over} ← @source/{over}

chase item IN {over}
├── sniff item is valid
│   └── ? not valid → skip
├── tag processed ← transform(item)
└── pack results ← results + processed

treat → results
"#)
}

fn generate_secure_template(asset: &str) -> String {
    let name = to_identifier(asset);
    format!(r#"pug SECURE_{name}

fence
├── bone NOT: store {asset} in plaintext
├── bone NOT: log {asset} values
├── bone NOT: expose {asset} in errors
└── bone NOT: cache {asset} unencrypted

trick encrypt_{name}(value)
├── tag salt ← generate_salt()
├── tag hash ← hash(value + salt)
└── bury hash @vault/{asset}

trick verify_{name}(input, stored_hash)
├── tag input_hash ← hash(input + stored_salt)
├── sniff input_hash matches stored_hash
│   └── ? matches → wag
│   └── ? no match → whine
└── treat → {{verified: result}}

treat → {{encrypt_{name}, verify_{name}}}
"#)
}

fn generate_connect_template(service: &str) -> String {
    let name = to_identifier(service);
    format!(r#"pug CONNECT_{name}

fence
├── bone NOT: hardcode credentials
├── bone NOT: skip SSL verification
└── bone NOT: ignore connection errors

bark config ← $vault/{service}_credentials

trick connect()
├── tag connection ← @api/{service}/connect(config)
├── sniff connection is alive
│   └── ? not alive → whine "Failed to connect to {service}"
└── treat → connection

trick disconnect()
└── tag connection ← closed

trick call_{name}(endpoint, data)
├── sniff connection is alive
│   └── ? not alive → connect()
├── bark response ← @api/{service}/{{endpoint}}(data)
└── treat → response

treat → {{connect, disconnect, call_{name}}}
"#)
}

fn generate_delete_template(target: &str) -> String {
    let name = to_identifier(target);
    format!(r#"pug DELETE_{name}

fence
├── bone NOT: delete without confirmation
└── bone NOT: hard delete (use soft delete)

sniff {target} exists
├── ? not exists → whine "{target} not found"

sniff user confirmed deletion
├── ? not confirmed → whine "Deletion cancelled"

tag {target}.deleted ← wag
tag {target}.deleted_at ← now()

bury {target} @storage/{target}

treat → {{deleted: wag, id: {target}.id}}
"#)
}

fn generate_update_template(target: &str) -> String {
    let name = to_identifier(target);
    format!(r#"pug UPDATE_{name}

bark existing_{target} ← @storage/{target}/{{id}}
├── ? not found → whine "{target} not found"

sniff changes are valid
├── ? not valid → whine "Invalid changes"

tag updated_{target} ← existing_{target} + changes
tag updated_{target}.updated_at ← now()

bury updated_{target} @storage/{target}/{{id}}

treat → updated_{target}
"#)
}

fn generate_transform_template(from: &str, to: &str) -> String {
    format!(r#"pug TRANSFORM_{}_TO_{}

bark source ← {from}

roll source → {to}
├── map fields appropriately
├── convert types as needed
└── validate output format

treat → {to}
"#, to_identifier(from), to_identifier(to))
}

fn generate_unknown_template(question: &str) -> String {
    format!(r#"pug ANSWER

# Question: {question}
#
# I need more context to generate specific instructions.
# Try asking with action words like:
#
#   wag "how do i CREATE a user login"
#   wag "how do i FETCH user data"
#   wag "how do i VALIDATE an email"
#   wag "how do i STORE a document"
#   wag "how do i SECURE passwords"
#   wag "how do i CONNECT to an API"
#
# Or describe what you're trying to build!

sniff "{question}"
└── What are you trying to accomplish?

treat → instructions
"#)
}

/// Main wag function - question in, CODIE out
pub fn wag(question: &str) -> String {
    let intent = parse_intent(question);
    generate_template(&intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_intent() {
        let intent = parse_intent("how do i make a todo app");
        assert!(matches!(intent, WagIntent::Create { .. }));

        if let WagIntent::Create { thing } = intent {
            assert!(thing.contains("todo"));
        }
    }

    #[test]
    fn test_fetch_intent() {
        let intent = parse_intent("how do i get user data");
        assert!(matches!(intent, WagIntent::Fetch { .. }));
    }

    #[test]
    fn test_validate_intent() {
        let intent = parse_intent("how do i check if email is valid");
        assert!(matches!(intent, WagIntent::Validate { .. }));
    }

    #[test]
    fn test_secure_intent() {
        let intent = parse_intent("how do i protect passwords");
        assert!(matches!(intent, WagIntent::Secure { .. }));
    }

    #[test]
    fn test_generate_create() {
        let template = wag("how do i make a todo app");
        assert!(template.contains("pug"));
        assert!(template.contains("fence"));
        assert!(template.contains("trick"));
        assert!(template.contains("treat"));
    }

    #[test]
    fn test_generate_secure() {
        let template = wag("how do i secure user passwords");
        assert!(template.contains("bone NOT: store"));
        assert!(template.contains("encrypt"));
        assert!(template.contains("vault"));
    }

    #[test]
    fn test_unknown_gives_help() {
        let template = wag("asdfghjkl");
        assert!(template.contains("more context"));
        assert!(template.contains("CREATE"));
    }
}
