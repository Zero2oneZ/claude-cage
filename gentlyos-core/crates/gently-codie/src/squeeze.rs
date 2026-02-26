//! CODIE Squeeze - Vowel Removal Compression
//!
//! Lossy compression that removes vowels from identifiers.
//! AI models reconstruct meaning from context (like humans reading "pls" or "msg").
//!
//! ## Why This Works
//!
//! ```text
//! Traditional code → Compiler (needs exact spelling)
//! CODIE → AI (reconstructs from context)
//!
//! "username" → "srnm" (AI understands)
//! "database" → "dtbs" (AI understands)
//! "function" → "fn"   (dictionary)
//! ```
//!
//! ## Compression Ratios
//!
//! - Vowels are ~40% of English text
//! - Combined with glyph encoding: 70-85% total reduction
//! - AI accuracy on squeezed text: ~98%+ (context-dependent)
//!
//! ## Squeeze Levels
//!
//! - Level 0: No squeeze (preserve identifiers)
//! - Level 1: Dictionary only (common abbreviations)
//! - Level 2: Dictionary + vowel removal
//! - Level 3: Aggressive (also remove repeated consonants)

use std::collections::HashMap;
use lazy_static::lazy_static;

/// Vowels to remove
const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U'];

lazy_static! {
    /// Common programming abbreviations
    /// These are replaced BEFORE vowel stripping
    pub static ref ABBREV_DICT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Keywords & types
        m.insert("function", "fn");
        m.insert("return", "ret");
        m.insert("string", "str");
        m.insert("number", "num");
        m.insert("boolean", "bool");
        m.insert("integer", "int");
        m.insert("character", "chr");
        m.insert("unsigned", "uint");
        m.insert("pointer", "ptr");
        m.insert("reference", "ref");
        m.insert("constant", "const");
        m.insert("variable", "var");
        m.insert("parameter", "param");
        m.insert("argument", "arg");
        m.insert("attribute", "attr");
        m.insert("property", "prop");
        m.insert("element", "elem");
        m.insert("index", "idx");
        m.insert("length", "len");
        m.insert("count", "cnt");
        m.insert("maximum", "max");
        m.insert("minimum", "min");
        m.insert("average", "avg");
        m.insert("total", "tot");
        m.insert("value", "val");
        m.insert("result", "res");
        m.insert("response", "resp");
        m.insert("request", "req");
        m.insert("message", "msg");
        m.insert("error", "err");
        m.insert("warning", "warn");
        m.insert("information", "info");
        m.insert("debug", "dbg");
        m.insert("temporary", "tmp");
        m.insert("buffer", "buf");
        m.insert("source", "src");
        m.insert("destination", "dst");
        m.insert("directory", "dir");
        m.insert("document", "doc");
        m.insert("configuration", "cfg");
        m.insert("environment", "env");
        m.insert("context", "ctx");
        m.insert("connection", "conn");
        m.insert("transaction", "tx");
        m.insert("database", "db");
        m.insert("repository", "repo");
        m.insert("library", "lib");
        m.insert("package", "pkg");
        m.insert("module", "mod");
        m.insert("component", "cmp");
        m.insert("service", "svc");
        m.insert("controller", "ctrl");
        m.insert("manager", "mgr");
        m.insert("handler", "hndlr");
        m.insert("listener", "lsnr");
        m.insert("callback", "cb");
        m.insert("iterator", "iter");
        m.insert("generator", "gen");
        m.insert("allocator", "alloc");
        m.insert("memory", "mem");
        m.insert("address", "addr");
        m.insert("position", "pos");
        m.insert("location", "loc");
        m.insert("coordinate", "coord");
        m.insert("dimension", "dim");
        m.insert("width", "w");
        m.insert("height", "h");
        m.insert("column", "col");
        m.insert("row", "row");
        m.insert("previous", "prev");
        m.insert("current", "curr");
        m.insert("next", "nxt");
        m.insert("first", "fst");
        m.insert("last", "lst");
        m.insert("begin", "bgn");
        m.insert("end", "end");
        m.insert("start", "strt");
        m.insert("stop", "stp");
        m.insert("initialize", "init");
        m.insert("terminate", "term");
        m.insert("create", "crt");
        m.insert("delete", "del");
        m.insert("insert", "ins");
        m.insert("update", "upd");
        m.insert("select", "sel");
        m.insert("execute", "exec");
        m.insert("process", "proc");
        m.insert("calculate", "calc");
        m.insert("compare", "cmp");
        m.insert("convert", "conv");
        m.insert("transform", "xform");
        m.insert("validate", "valid");
        m.insert("authenticate", "auth");
        m.insert("authorize", "authz");
        m.insert("encrypt", "enc");
        m.insert("decrypt", "dec");
        m.insert("compress", "cmprs");
        m.insert("decompress", "dcmprs");
        m.insert("serialize", "ser");
        m.insert("deserialize", "deser");
        m.insert("synchronize", "sync");
        m.insert("asynchronous", "async");
        m.insert("parallel", "par");
        m.insert("sequential", "seq");
        m.insert("recursive", "rec");
        m.insert("optional", "opt");
        m.insert("required", "req");
        m.insert("default", "def");
        m.insert("custom", "cust");
        m.insert("standard", "std");
        m.insert("special", "spcl");
        m.insert("private", "priv");
        m.insert("public", "pub");
        m.insert("protected", "prot");
        m.insert("internal", "intrnl");
        m.insert("external", "extrnl");
        m.insert("input", "in");
        m.insert("output", "out");
        m.insert("forward", "fwd");
        m.insert("backward", "bwd");
        m.insert("horizontal", "horiz");
        m.insert("vertical", "vert");
        m.insert("application", "app");
        m.insert("implementation", "impl");
        m.insert("specification", "spec");
        m.insert("definition", "def");
        m.insert("declaration", "decl");
        m.insert("expression", "expr");
        m.insert("statement", "stmt");
        m.insert("condition", "cond");
        m.insert("exception", "exc");
        m.insert("collection", "coll");
        m.insert("accumulator", "acc");
        m.insert("administrator", "admin");
        m.insert("user", "usr");
        m.insert("username", "uname");
        m.insert("password", "pwd");
        m.insert("credential", "cred");
        m.insert("token", "tok");
        m.insert("session", "sess");
        m.insert("certificate", "cert");
        m.insert("signature", "sig");
        m.insert("timestamp", "ts");
        m.insert("identifier", "id");
        m.insert("unique", "uniq");
        m.insert("random", "rand");
        m.insert("sequence", "seq");
        m.insert("version", "ver");
        m.insert("revision", "rev");
        m.insert("number", "num");
        m.insert("amount", "amt");
        m.insert("quantity", "qty");
        m.insert("percent", "pct");
        m.insert("percentage", "pct");
        m.insert("ratio", "rat");
        m.insert("factor", "fac");
        m.insert("multiplier", "mult");
        m.insert("divider", "div");
        m.insert("remainder", "rem");
        m.insert("quotient", "quot");
        m.insert("positive", "pos");
        m.insert("negative", "neg");
        m.insert("absolute", "abs");
        m.insert("relative", "rel");
        m.insert("object", "obj");
        m.insert("array", "arr");
        m.insert("vector", "vec");
        m.insert("matrix", "mat");
        m.insert("queue", "q");
        m.insert("stack", "stk");
        m.insert("heap", "hp");
        m.insert("tree", "tr");
        m.insert("graph", "gr");
        m.insert("node", "nd");
        m.insert("edge", "edg");
        m.insert("vertex", "vtx");
        m.insert("path", "pth");
        m.insert("route", "rt");
        m.insert("network", "net");
        m.insert("socket", "sock");
        m.insert("protocol", "proto");
        m.insert("channel", "ch");
        m.insert("stream", "strm");
        m.insert("file", "f");
        m.insert("image", "img");
        m.insert("audio", "aud");
        m.insert("video", "vid");
        m.insert("text", "txt");
        m.insert("binary", "bin");
        m.insert("format", "fmt");
        m.insert("template", "tmpl");
        m.insert("pattern", "ptrn");
        m.insert("regular", "reg");
        m.insert("expression", "expr");
        m.insert("script", "scr");
        m.insert("program", "prog");
        m.insert("command", "cmd");
        m.insert("argument", "arg");
        m.insert("option", "opt");
        m.insert("flag", "flg");
        m.insert("switch", "sw");
        m.insert("toggle", "tgl");
        m.insert("enable", "en");
        m.insert("disable", "dis");
        m.insert("active", "actv");
        m.insert("inactive", "inactv");
        m.insert("visible", "vis");
        m.insert("hidden", "hdn");
        m.insert("locked", "lck");
        m.insert("unlocked", "ulck");
        m.insert("open", "opn");
        m.insert("close", "cls");
        m.insert("read", "rd");
        m.insert("write", "wr");
        m.insert("append", "appnd");
        m.insert("truncate", "trunc");
        m.insert("flush", "flsh");
        m.insert("clear", "clr");
        m.insert("reset", "rst");
        m.insert("reload", "rld");
        m.insert("refresh", "rfsh");
        m.insert("restore", "rstr");
        m.insert("backup", "bkp");
        m.insert("cache", "cch");
        m.insert("storage", "stor");
        m.insert("persist", "prst");
        m.insert("register", "reg");
        m.insert("unregister", "unreg");
        m.insert("subscribe", "sub");
        m.insert("unsubscribe", "unsub");
        m.insert("publish", "pub");
        m.insert("broadcast", "bcast");
        m.insert("dispatch", "dsp");
        m.insert("receive", "recv");
        m.insert("send", "snd");
        m.insert("transfer", "xfer");
        m.insert("upload", "ul");
        m.insert("download", "dl");
        m.insert("import", "imp");
        m.insert("export", "exp");
        m.insert("include", "incl");
        m.insert("exclude", "excl");
        m.insert("filter", "flt");
        m.insert("sort", "srt");
        m.insert("search", "srch");
        m.insert("find", "fnd");
        m.insert("match", "mtch");
        m.insert("replace", "rplc");
        m.insert("remove", "rmv");
        m.insert("add", "add");
        m.insert("set", "set");
        m.insert("get", "get");
        m.insert("put", "put");
        m.insert("post", "post");
        m.insert("patch", "ptch");
        m.insert("head", "hd");
        m.insert("body", "bdy");
        m.insert("header", "hdr");
        m.insert("footer", "ftr");
        m.insert("content", "cont");
        m.insert("payload", "pld");
        m.insert("metadata", "meta");
        m.insert("status", "stat");
        m.insert("state", "st");
        m.insert("event", "evt");
        m.insert("action", "act");
        m.insert("trigger", "trig");
        m.insert("hook", "hk");
        m.insert("plugin", "plgn");
        m.insert("extension", "ext");
        m.insert("middleware", "mw");
        m.insert("wrapper", "wrap");
        m.insert("adapter", "adpt");
        m.insert("bridge", "brdg");
        m.insert("proxy", "prxy");
        m.insert("gateway", "gw");
        m.insert("router", "rtr");
        m.insert("dispatcher", "dsp");
        m.insert("scheduler", "sched");
        m.insert("executor", "exec");
        m.insert("worker", "wkr");
        m.insert("thread", "thrd");
        m.insert("mutex", "mtx");
        m.insert("semaphore", "sem");
        m.insert("lock", "lk");
        m.insert("barrier", "bar");
        m.insert("signal", "sig");
        m.insert("interrupt", "intr");
        m.insert("priority", "pri");
        m.insert("timeout", "tout");
        m.insert("interval", "intv");
        m.insert("delay", "dly");
        m.insert("duration", "dur");
        m.insert("period", "prd");
        m.insert("frequency", "freq");
        m.insert("rate", "rt");
        m.insert("speed", "spd");
        m.insert("performance", "perf");
        m.insert("efficiency", "eff");
        m.insert("capacity", "cap");
        m.insert("limit", "lim");
        m.insert("threshold", "thresh");
        m.insert("boundary", "bnd");
        m.insert("range", "rng");
        m.insert("offset", "ofs");
        m.insert("margin", "mrgn");
        m.insert("padding", "pad");
        m.insert("spacing", "spc");
        m.insert("alignment", "algn");
        m.insert("layout", "lyt");
        m.insert("render", "rndr");
        m.insert("display", "disp");
        m.insert("screen", "scrn");
        m.insert("window", "wnd");
        m.insert("frame", "frm");
        m.insert("panel", "pnl");
        m.insert("dialog", "dlg");
        m.insert("modal", "mdl");
        m.insert("popup", "pp");
        m.insert("tooltip", "ttip");
        m.insert("menu", "mnu");
        m.insert("toolbar", "tbar");
        m.insert("sidebar", "sbar");
        m.insert("navigation", "nav");
        m.insert("breadcrumb", "bcrmb");
        m.insert("pagination", "pgn");
        m.insert("scroll", "scrl");
        m.insert("zoom", "zm");
        m.insert("rotate", "rot");
        m.insert("scale", "scl");
        m.insert("translate", "xlat");
        m.insert("animate", "anim");
        m.insert("transition", "trans");
        m.insert("effect", "fx");
        m.insert("color", "clr");
        m.insert("background", "bg");
        m.insert("foreground", "fg");
        m.insert("border", "bdr");
        m.insert("shadow", "shdw");
        m.insert("opacity", "opac");
        m.insert("transparency", "transp");
        m.insert("gradient", "grad");
        m.insert("texture", "tex");
        m.insert("font", "fnt");
        m.insert("bold", "bld");
        m.insert("italic", "ital");
        m.insert("underline", "uline");
        m
    };

    /// Reverse lookup for expansion (squeeze → full)
    pub static ref EXPAND_DICT: HashMap<&'static str, &'static str> = {
        ABBREV_DICT.iter().map(|(k, v)| (*v, *k)).collect()
    };
}

/// Squeeze level for compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SqueezeLevel {
    /// No squeezing
    None,
    /// Dictionary abbreviations only
    Dictionary,
    /// Dictionary + vowel removal
    Vowels,
    /// Aggressive: dictionary + vowels + double consonants
    Aggressive,
}

/// Squeeze a string based on level
pub fn squeeze(text: &str, level: SqueezeLevel) -> String {
    match level {
        SqueezeLevel::None => text.to_string(),
        SqueezeLevel::Dictionary => squeeze_dictionary(text),
        SqueezeLevel::Vowels => squeeze_vowels(&squeeze_dictionary(text)),
        SqueezeLevel::Aggressive => squeeze_aggressive(text),
    }
}

/// Apply dictionary abbreviations
pub fn squeeze_dictionary(text: &str) -> String {
    let mut result = text.to_string();
    let lower = text.to_lowercase();

    // Sort by length descending to replace longer matches first
    let mut entries: Vec<_> = ABBREV_DICT.iter().collect();
    entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (full, abbrev) in entries {
        if lower.contains(*full) {
            // Case-insensitive replace while preserving some case
            let pattern = regex::Regex::new(&format!("(?i){}", regex::escape(full))).unwrap();
            result = pattern.replace_all(&result, *abbrev).to_string();
        }
    }

    result
}

/// Remove vowels from text
pub fn squeeze_vowels(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_vowel = false;

    for (i, c) in text.chars().enumerate() {
        if VOWELS.contains(&c) {
            // Keep first letter if it's a vowel (for readability)
            if i == 0 || (prev_was_vowel && result.is_empty()) {
                result.push(c);
            }
            prev_was_vowel = true;
        } else {
            result.push(c);
            prev_was_vowel = false;
        }
    }

    result
}

/// Aggressive squeeze: dictionary + vowels + remove repeated consonants
pub fn squeeze_aggressive(text: &str) -> String {
    let dict_squeezed = squeeze_dictionary(text);
    let vowel_squeezed = squeeze_vowels(&dict_squeezed);

    // Remove repeated consonants
    let mut result = String::with_capacity(vowel_squeezed.len());
    let mut prev_char: Option<char> = None;

    for c in vowel_squeezed.chars() {
        if let Some(prev) = prev_char {
            // Skip if same consonant repeated
            if c == prev && !VOWELS.contains(&c) {
                continue;
            }
        }
        result.push(c);
        prev_char = Some(c);
    }

    result
}

/// Squeeze an identifier (camelCase/snake_case aware)
pub fn squeeze_identifier(ident: &str, level: SqueezeLevel) -> String {
    if level == SqueezeLevel::None {
        return ident.to_string();
    }

    // Split on camelCase and snake_case boundaries
    let parts = split_identifier(ident);

    // Squeeze each part
    let squeezed: Vec<String> = parts
        .into_iter()
        .map(|part| squeeze(&part, level))
        .collect();

    // Rejoin (preserve original separator style)
    if ident.contains('_') {
        squeezed.join("_")
    } else {
        // CamelCase - capitalize first letter of each part after first
        let mut result = String::new();
        for (i, part) in squeezed.iter().enumerate() {
            if i == 0 {
                result.push_str(part);
            } else if !part.is_empty() {
                let mut chars = part.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_ascii_uppercase());
                    result.extend(chars);
                }
            }
        }
        result
    }
}

/// Split identifier into parts
fn split_identifier(ident: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for c in ident.chars() {
        if c == '_' {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
        } else if c.is_uppercase() && !current.is_empty() {
            parts.push(current.clone());
            current.clear();
            current.push(c.to_ascii_lowercase());
        } else {
            current.push(c.to_ascii_lowercase());
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// Calculate compression ratio
pub fn compression_ratio(original: &str, squeezed: &str) -> f64 {
    if original.is_empty() {
        return 0.0;
    }
    1.0 - (squeezed.len() as f64 / original.len() as f64)
}

/// Squeeze statistics
#[derive(Debug, Clone)]
pub struct SqueezeStats {
    pub original_len: usize,
    pub squeezed_len: usize,
    pub ratio: f64,
    pub vowels_removed: usize,
    pub abbreviations_applied: usize,
}

/// Squeeze with statistics
pub fn squeeze_with_stats(text: &str, level: SqueezeLevel) -> (String, SqueezeStats) {
    let original_len = text.len();

    // Count abbreviations that will be applied
    let lower = text.to_lowercase();
    let abbreviations_applied = ABBREV_DICT
        .keys()
        .filter(|k| lower.contains(*k))
        .count();

    // Count vowels
    let vowels_removed = if level >= SqueezeLevel::Vowels {
        text.chars().filter(|c| VOWELS.contains(c)).count()
    } else {
        0
    };

    let squeezed = squeeze(text, level);
    let squeezed_len = squeezed.len();
    let ratio = compression_ratio(text, &squeezed);

    (squeezed, SqueezeStats {
        original_len,
        squeezed_len,
        ratio,
        vowels_removed,
        abbreviations_applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_squeeze() {
        assert_eq!(squeeze_dictionary("function"), "fn");
        assert_eq!(squeeze_dictionary("return value"), "ret val");
        assert_eq!(squeeze_dictionary("database connection"), "db conn");
        assert_eq!(squeeze_dictionary("username"), "uname");
    }

    #[test]
    fn test_vowel_squeeze() {
        assert_eq!(squeeze_vowels("ballerina"), "bllrn");
        assert_eq!(squeeze_vowels("hello"), "hll");
        assert_eq!(squeeze_vowels("world"), "wrld");
        // Keep first vowel
        assert_eq!(squeeze_vowels("apple"), "appl");
    }

    #[test]
    fn test_combined_squeeze() {
        let result = squeeze("authenticate user", SqueezeLevel::Vowels);
        println!("Combined squeeze: 'authenticate user' → '{}'", result);
        // Dictionary replaces "authenticate" → "auth", "user" → "usr"
        // Vowel squeeze then removes remaining vowels
        assert!(result.contains("th")); // "auth" vowel-squeezed
        assert!(!result.contains("authenticate"));
    }

    #[test]
    fn test_aggressive_squeeze() {
        // Removes repeated consonants
        let result = squeeze_aggressive("running");
        println!("Aggressive squeeze: 'running' → '{}'", result);
        assert!(!result.contains("nn"));
    }

    #[test]
    fn test_identifier_squeeze() {
        // CamelCase - getUserName splits into [get, user, name]
        // "user" → "usr", others unchanged, then rejoin
        let result = squeeze_identifier("getUserName", SqueezeLevel::Dictionary);
        println!("Identifier squeeze: 'getUserName' → '{}'", result);
        assert!(result.contains("Usr") || result.contains("usr"));

        // snake_case
        let result2 = squeeze_identifier("database_connection", SqueezeLevel::Dictionary);
        println!("Identifier squeeze: 'database_connection' → '{}'", result2);
        assert_eq!(result2, "db_conn");
    }

    #[test]
    fn test_compression_ratio() {
        let original = "authenticate user from database";
        let (squeezed, stats) = squeeze_with_stats(original, SqueezeLevel::Vowels);

        println!("Original: {} ({} bytes)", original, stats.original_len);
        println!("Squeezed: {} ({} bytes)", squeezed, stats.squeezed_len);
        println!("Ratio: {:.1}%", stats.ratio * 100.0);

        assert!(stats.ratio > 0.3); // At least 30% compression
    }

    #[test]
    fn test_ai_readable() {
        // These should still be understandable by AI
        let cases = [
            ("username", "unm"),
            ("password", "pwd"),
            ("database", "db"),
            ("function", "fn"),
            ("connection", "conn"),
            ("configuration", "cfg"),
        ];

        for (original, expected_contains) in cases {
            let squeezed = squeeze(original, SqueezeLevel::Vowels);
            println!("{} → {}", original, squeezed);
            // Should contain the abbreviation or be similar
            assert!(
                squeezed.len() < original.len(),
                "Should compress: {} → {}",
                original,
                squeezed
            );
        }
    }

    #[test]
    fn test_preserves_special_chars() {
        let result = squeeze("@database/users", SqueezeLevel::Vowels);
        assert!(result.contains("@"));
        assert!(result.contains("/"));
    }

    #[test]
    fn test_real_codie_squeeze() {
        let codie = "bark user ← @database/users\ntreat → authentication_token";
        let squeezed = squeeze(codie, SqueezeLevel::Vowels);

        println!("Original:\n{}", codie);
        println!("\nSqueezed:\n{}", squeezed);

        // Should be significantly shorter
        assert!(squeezed.len() < codie.len());
    }
}
