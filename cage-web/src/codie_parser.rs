use serde::Serialize;
use std::path::Path;

/// AST node for a CODIE instruction.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Node {
    Entry {
        name: String,
        children: Vec<Node>,
    },
    Fetch {
        target: String,
        source: String,
    },
    Bind {
        name: String,
        value: String,
    },
    Call {
        name: String,
        args: String,
        children: Vec<Node>,
    },
    Guard {
        name: String,
        children: Vec<Node>,
    },
    Rule {
        name: String,
        negated: bool,
        body: String,
        children: Vec<Node>,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Loop {
        var: String,
        collection: String,
        body: Vec<Node>,
    },
    Conditional {
        condition: String,
        action: String,
    },
    Return {
        value: String,
    },
    Checkpoint {
        name: String,
    },
    Const {
        name: String,
        value: String,
    },
    Transform {
        condition: String,
        target: String,
    },
    Comment {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub name: String,
    pub field_type: String,
}

/// A parsed CODIE program.
#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub name: String,
    pub source: String,
    pub nodes: Vec<Node>,
    pub line_count: usize,
    pub keyword_counts: std::collections::HashMap<String, usize>,
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Program {
    /// Parse a .codie source string into a Program.
    pub fn parse(name: &str, source: &str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(source);
        let nodes = parser.parse_top_level()?;

        let mut keyword_counts = std::collections::HashMap::new();
        count_keywords(&nodes, &mut keyword_counts);

        Ok(Program {
            name: name.to_string(),
            source: source.to_string(),
            nodes,
            line_count: source.lines().count(),
            keyword_counts,
        })
    }

    pub fn instruction_count(&self) -> usize {
        count_nodes(&self.nodes)
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Entry point name (first pug).
    pub fn entry_point(&self) -> Option<&str> {
        for node in &self.nodes {
            if let Node::Entry { name, .. } = node {
                return Some(name);
            }
        }
        None
    }
}

fn count_nodes(nodes: &[Node]) -> usize {
    let mut count = nodes.len();
    for node in nodes {
        match node {
            Node::Entry { children, .. }
            | Node::Guard { children, .. }
            | Node::Rule { children, .. }
            | Node::Call { children, .. }
            | Node::Loop { body: children, .. } => {
                count += count_nodes(children);
            }
            _ => {}
        }
    }
    count
}

fn count_keywords(nodes: &[Node], counts: &mut std::collections::HashMap<String, usize>) {
    for node in nodes {
        let kw = match node {
            Node::Entry { children, .. } => {
                count_keywords(children, counts);
                "pug"
            }
            Node::Fetch { .. } => "bark",
            Node::Bind { .. } => "elf",
            Node::Call { children, .. } => {
                count_keywords(children, counts);
                "cali"
            }
            Node::Guard { children, .. } => {
                count_keywords(children, counts);
                "fence"
            }
            Node::Rule { children, .. } => {
                count_keywords(children, counts);
                "bone"
            }
            Node::Struct { .. } => "blob",
            Node::Loop { body, .. } => {
                count_keywords(body, counts);
                "spin"
            }
            Node::Conditional { .. } => "?",
            Node::Return { .. } => "biz",
            Node::Checkpoint { .. } => "anchor",
            Node::Const { .. } => "pin",
            Node::Transform { .. } => "turk",
            Node::Comment { .. } => continue,
        };
        *counts.entry(kw.to_string()).or_insert(0) += 1;
    }
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Parser {
            lines: source.lines().collect(),
            pos: 0,
        }
    }

    fn parse_top_level(&mut self) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            // Skip pipe-only lines
            if trimmed == "|" {
                self.pos += 1;
                continue;
            }

            // Strip pipe-tree prefix: "+-- ", "|   +-- ", etc.
            let content = strip_pipe_prefix(trimmed);

            if let Some(node) = self.parse_line(content)? {
                nodes.push(node);
            }
            self.pos += 1;
        }

        Ok(nodes)
    }

    fn parse_line(&mut self, content: &str) -> Result<Option<Node>, ParseError> {
        let trimmed = content.trim();

        // Empty or pipe-only after stripping
        if trimmed.is_empty() || trimmed == "|" {
            return Ok(None);
        }

        // Comments: // or # or ====== separator lines
        if trimmed.starts_with("//") {
            return Ok(Some(Node::Comment { text: trimmed[2..].trim().to_string() }));
        }
        if trimmed.starts_with('#') {
            return Ok(Some(Node::Comment { text: trimmed[1..].trim().to_string() }));
        }
        if trimmed.starts_with("====") || trimmed.starts_with("----") {
            return Ok(Some(Node::Comment { text: trimmed.to_string() }));
        }
        // Closing brace variants
        if trimmed == "}" || trimmed == "})," || trimmed == "});" || trimmed == "}," {
            return Ok(None);
        }

        // Get the first word (check lowercase for case-insensitive keyword matching)
        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        let first_lower = first_word.to_lowercase();

        match first_lower.as_str() {
            "pug" => self.parse_entry(trimmed),
            "bark" => Ok(Some(self.parse_bark(trimmed))),
            "elf" => Ok(Some(self.parse_elf(trimmed))),
            "cali" => Ok(Some(self.parse_cali(trimmed))),
            "fence" => self.parse_fence(trimmed),
            "bone" => Ok(Some(self.parse_bone(trimmed))),
            // Handle standalone closing brace (consumed by brace blocks)
            "}" => Ok(None),
            "blob" => self.parse_blob(trimmed),
            "spin" => self.parse_spin(trimmed),
            "?" => Ok(Some(self.parse_conditional(trimmed))),
            "biz" => Ok(Some(self.parse_biz(trimmed))),
            "anchor" => Ok(Some(self.parse_anchor(trimmed))),
            "pin" => Ok(Some(self.parse_pin(trimmed))),
            "turk" => Ok(Some(self.parse_turk(trimmed))),
            "error" | "warn" | "return" => Ok(Some(self.parse_action(trimmed))),
            _ => {
                // Could be a continuation or unknown construct -- treat as comment
                Ok(Some(Node::Comment { text: trimmed.to_string() }))
            }
        }
    }

    fn parse_entry(&mut self, line: &str) -> Result<Option<Node>, ParseError> {
        let raw_name = line.strip_prefix("pug ").unwrap_or("").trim();

        // Check for brace block: pug NAME {
        let children = if raw_name.ends_with('{') || self.peek_next_is_brace() {
            self.collect_brace_block()
        } else {
            self.collect_children()
        };

        let name = raw_name.trim_end_matches('{').trim().to_string();

        Ok(Some(Node::Entry { name, children }))
    }

    fn parse_bark(&self, line: &str) -> Node {
        // bark target <- @source
        // bark target from source
        // bark @path/to/resource  (no assignment)
        let rest = line.strip_prefix("bark ").unwrap_or("").trim();
        if let Some((target, source)) = rest.split_once("<-") {
            Node::Fetch {
                target: target.trim().to_string(),
                source: source.trim().to_string(),
            }
        } else if let Some((target, source)) = rest.split_once(" from ") {
            Node::Fetch {
                target: target.trim().to_string(),
                source: source.trim().to_string(),
            }
        } else {
            // bark @path/to/resource  (no assignment)
            Node::Fetch {
                target: String::new(),
                source: rest.to_string(),
            }
        }
    }

    fn parse_elf(&self, line: &str) -> Node {
        // elf name <- value
        let rest = line.strip_prefix("elf ").unwrap_or("").trim();
        if let Some((name, value)) = rest.split_once("<-") {
            Node::Bind {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            }
        } else if let Some((name, value)) = rest.split_once("=") {
            // elf name = value (alternative syntax)
            Node::Bind {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            }
        } else {
            Node::Bind {
                name: rest.to_string(),
                value: String::new(),
            }
        }
    }

    fn parse_cali(&mut self, line: &str) -> Node {
        // cali FUNCTION_NAME(args)
        // cali FUNCTION_NAME(args) { body }
        // cali FUNCTION_NAME
        let rest = line.strip_prefix("cali ").unwrap_or("").trim();
        let (name, args) = if let Some(paren) = rest.find('(') {
            let n = rest[..paren].trim().to_string();
            // Find closing paren, strip everything after for args
            let after_paren = &rest[paren + 1..];
            let a = if let Some(close) = after_paren.find(')') {
                after_paren[..close].trim().to_string()
            } else {
                after_paren.trim().to_string()
            };
            (n, a)
        } else {
            (rest.trim_end_matches('{').trim().to_string(), String::new())
        };

        // Consume brace block body if present
        let children = if rest.ends_with('{') || self.peek_next_is_brace() {
            self.collect_brace_block()
        } else {
            Vec::new()
        };

        Node::Call { name, args, children }
    }

    fn parse_fence(&mut self, line: &str) -> Result<Option<Node>, ParseError> {
        let name = line
            .strip_prefix("fence")
            .unwrap_or("")
            .trim()
            .to_string();

        // Check for brace block
        let children = if name.ends_with('{') || self.peek_next_is_brace() {
            self.collect_brace_block()
        } else {
            self.collect_children()
        };

        Ok(Some(Node::Guard {
            name: name.trim_end_matches('{').trim().to_string(),
            children,
        }))
    }

    fn parse_bone(&mut self, line: &str) -> Node {
        // bone NOT: action
        // bone RULE_NAME { ... }
        // bone REQUIRES: condition
        let rest = line.strip_prefix("bone ").unwrap_or("").trim();
        let negated = rest.starts_with("NOT:");
        let body = if negated {
            rest.strip_prefix("NOT:").unwrap_or(rest).trim()
        } else {
            rest
        };

        // Check for brace block: bone NAME { ... }
        let children = if body.ends_with('{') || (body.contains('{') && !body.contains('}')) || self.peek_next_is_brace() {
            self.collect_brace_block()
        } else {
            Vec::new()
        };

        // Extract name if pattern is "NAME: body" or "NAME { body }"
        let (name, body_str) = if let Some((n, b)) = body.split_once(':') {
            if !negated {
                (n.trim().to_string(), b.trim().to_string())
            } else {
                (String::new(), body.to_string())
            }
        } else if let Some((n, _)) = body.split_once('{') {
            (n.trim().to_string(), String::new())
        } else {
            (String::new(), body.to_string())
        };

        Node::Rule {
            name,
            negated,
            body: body_str,
            children,
        }
    }

    fn parse_blob(&mut self, line: &str) -> Result<Option<Node>, ParseError> {
        // blob StructName { field: Type, ... }
        let rest = line.strip_prefix("blob ").unwrap_or("").trim();
        let name = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('{')
            .to_string();

        let mut fields = Vec::new();

        // Inline fields: blob Name { a: T, b: T }
        if let Some(brace_start) = rest.find('{') {
            if let Some(brace_end) = rest.find('}') {
                let inner = &rest[brace_start + 1..brace_end];
                for field_str in inner.split(',') {
                    if let Some((fname, ftype)) = field_str.split_once(':') {
                        fields.push(StructField {
                            name: fname.trim().to_string(),
                            field_type: ftype.trim().to_string(),
                        });
                    }
                }
            } else {
                // Multi-line block
                fields = self.collect_struct_fields();
            }
        } else {
            // Next lines have fields
            fields = self.collect_struct_fields();
        }

        Ok(Some(Node::Struct { name, fields }))
    }

    fn parse_spin(&mut self, line: &str) -> Result<Option<Node>, ParseError> {
        // spin item IN collection
        // spin LABEL
        let rest = line.strip_prefix("spin ").unwrap_or("").trim();

        let (var, collection) = if let Some((v, c)) = rest.split_once(" IN ") {
            (v.trim().to_string(), c.trim().to_string())
        } else {
            (rest.to_string(), String::new())
        };

        let body = self.collect_children();

        Ok(Some(Node::Loop {
            var,
            collection,
            body,
        }))
    }

    fn parse_conditional(&self, line: &str) -> Node {
        // ? condition -> action
        let rest = line.strip_prefix('?').unwrap_or("").trim();
        if let Some((cond, act)) = rest.split_once("->") {
            Node::Conditional {
                condition: cond.trim().to_string(),
                action: act.trim().to_string(),
            }
        } else {
            Node::Conditional {
                condition: rest.to_string(),
                action: String::new(),
            }
        }
    }

    fn parse_biz(&self, line: &str) -> Node {
        // biz -> result
        let rest = line.strip_prefix("biz").unwrap_or("").trim();
        let value = rest.strip_prefix("->").unwrap_or(rest).trim().to_string();
        Node::Return { value }
    }

    fn parse_anchor(&self, line: &str) -> Node {
        // anchor #name
        let rest = line.strip_prefix("anchor").unwrap_or("").trim();
        let name = rest.strip_prefix('#').unwrap_or(rest).trim().to_string();
        Node::Checkpoint { name }
    }

    fn parse_pin(&self, line: &str) -> Node {
        // pin NAME = value
        // pin NAME
        let rest = line.strip_prefix("pin ").unwrap_or("").trim();
        if let Some((name, value)) = rest.split_once('=') {
            Node::Const {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            }
        } else {
            Node::Const {
                name: rest.to_string(),
                value: String::new(),
            }
        }
    }

    fn parse_turk(&self, line: &str) -> Node {
        // turk if condition -> action
        let rest = line
            .strip_prefix("turk")
            .unwrap_or("")
            .trim()
            .strip_prefix("if ")
            .unwrap_or(line.strip_prefix("turk ").unwrap_or(""));
        if let Some((cond, target)) = rest.split_once("->") {
            Node::Transform {
                condition: cond.trim().to_string(),
                target: target.trim().to_string(),
            }
        } else {
            Node::Transform {
                condition: rest.trim().to_string(),
                target: String::new(),
            }
        }
    }

    fn parse_action(&self, line: &str) -> Node {
        // error "message", warn "message", return value
        Node::Comment { text: line.to_string() }
    }

    /// Collect subsequent children that are at deeper indentation or inside pipe tree.
    fn collect_children(&mut self) -> Vec<Node> {
        let mut children = Vec::new();
        let start_pos = self.pos;

        // Look at the current indent level
        let base_indent = if start_pos < self.lines.len() {
            indent_level(self.lines[start_pos])
        } else {
            return children;
        };

        while self.pos + 1 < self.lines.len() {
            let next_line = self.lines[self.pos + 1];
            let next_trimmed = next_line.trim();

            // Empty lines are OK, skip them
            if next_trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            let next_indent = indent_level(next_line);
            let next_content = strip_pipe_prefix(next_trimmed);
            let next_content_trimmed = next_content.trim();

            // If next line is at same or lower indent and is a top-level keyword, stop
            if next_indent <= base_indent && !next_trimmed.starts_with('|') && !next_trimmed.starts_with('+') {
                // Check if it's a new top-level construct
                let first_word = next_content_trimmed
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if is_top_level_keyword(first_word) || first_word.starts_with('#') {
                    break;
                }
            }

            // If it's deeper or part of pipe tree, it's a child
            if next_indent > base_indent
                || next_trimmed.starts_with('|')
                || next_trimmed.starts_with('+')
            {
                self.pos += 1;
                if let Ok(Some(node)) = self.parse_line(next_content_trimmed) {
                    children.push(node);
                }
            } else {
                break;
            }
        }

        children
    }

    fn collect_brace_block(&mut self) -> Vec<Node> {
        let mut children = Vec::new();
        let mut depth: i32 = 1;

        while self.pos + 1 < self.lines.len() && depth > 0 {
            self.pos += 1;
            let line = self.lines[self.pos].trim();

            // Skip empty lines inside blocks
            if line.is_empty() {
                continue;
            }

            // Track brace depth
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;
            depth += opens - closes;

            // If this line is just a closing brace, don't parse it
            if line == "}" {
                if depth <= 0 {
                    break;
                }
                continue;
            }

            // If depth dropped to 0, this line had the closing brace — stop after it
            if depth <= 0 {
                // Parse content before the closing brace if there is any
                let content = line.trim_end_matches('}').trim();
                if !content.is_empty() {
                    let content = strip_pipe_prefix(content);
                    if let Ok(Some(node)) = self.parse_line(content.trim()) {
                        children.push(node);
                    }
                }
                break;
            }

            let content = strip_pipe_prefix(line);
            let content_trimmed = content.trim();

            // Inside brace blocks, key: value lines are implicit Bind nodes
            // But first check if the line starts with a CODIE keyword (case-insensitive)
            let first_word = content_trimmed.split_whitespace().next().unwrap_or("");
            if !is_top_level_keyword(&first_word.to_lowercase()) {
                if let Some((key, val)) = content_trimmed.split_once(':') {
                    let key_trimmed = key.trim();
                    // Only treat as key:value if key looks like an identifier (no spaces)
                    if !key_trimmed.is_empty()
                        && !key_trimmed.contains(' ')
                    {
                        children.push(Node::Bind {
                            name: key_trimmed.to_string(),
                            value: val.trim().trim_matches('"').to_string(),
                        });
                        continue;
                    }
                }
            }

            if let Ok(Some(node)) = self.parse_line(content_trimmed) {
                children.push(node);
            }
        }

        children
    }

    fn collect_struct_fields(&mut self) -> Vec<StructField> {
        let mut fields = Vec::new();

        while self.pos + 1 < self.lines.len() {
            let next = self.lines[self.pos + 1].trim();

            if next == "}" || next.is_empty() {
                if next == "}" {
                    self.pos += 1;
                }
                break;
            }

            self.pos += 1;
            let content = strip_pipe_prefix(next);
            let content = content.trim().trim_end_matches(',');

            // Skip comments inside structs
            if content.starts_with("//") || content.starts_with('#') {
                continue;
            }

            if let Some((fname, ftype)) = content.split_once(':') {
                fields.push(StructField {
                    name: fname.trim().to_string(),
                    field_type: ftype.trim().trim_end_matches(',').to_string(),
                });
            }
        }

        fields
    }

    fn peek_next_is_brace(&self) -> bool {
        if self.pos + 1 < self.lines.len() {
            self.lines[self.pos + 1].trim().starts_with('{')
                || self.lines[self.pos + 1].trim() == "{"
        } else {
            false
        }
    }
}

/// Strip pipe-tree prefixes: "+-- ", "|   +-- ", "|       +-- ", etc.
fn strip_pipe_prefix(line: &str) -> &str {
    let mut s = line;

    // Strip leading | and whitespace repeatedly
    loop {
        let trimmed = s.trim_start();
        if trimmed.starts_with('|') {
            s = &trimmed[1..];
        } else if trimmed.starts_with("+--") {
            s = trimmed[3..].trim_start();
            break;
        } else {
            s = trimmed;
            break;
        }
    }

    s
}

fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn is_top_level_keyword(word: &str) -> bool {
    matches!(
        word,
        "pug" | "bark" | "elf" | "spin" | "cali" | "turk" | "fence" | "pin" | "bone" | "blob"
            | "biz" | "anchor" | "?"
    )
}

/// Load and parse all .codie files from a directory.
pub fn load_all(dir: &Path) -> Vec<Program> {
    let mut programs = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read CODIE directory {}: {e}", dir.display());
            return programs;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("codie") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match std::fs::read_to_string(&path) {
                Ok(source) => match Program::parse(&name, &source) {
                    Ok(program) => {
                        eprintln!(
                            "  Parsed {}: {} lines, {} instructions, entry={}",
                            name,
                            program.line_count,
                            program.instruction_count(),
                            program.entry_point().unwrap_or("(none)")
                        );
                        programs.push(program);
                    }
                    Err(e) => {
                        eprintln!("  WARN: Failed to parse {}: {e}", path.display());
                    }
                },
                Err(e) => {
                    eprintln!("  WARN: Failed to read {}: {e}", path.display());
                }
            }
        }
    }

    programs.sort_by(|a, b| a.name.cmp(&b.name));
    eprintln!("Loaded {} CODIE programs", programs.len());
    programs
}
