//! CODIE Abstract Syntax Tree
//!
//! Steps 2.5, 2.6, 2.7 from BUILD_STEPS.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type annotations in CODIE
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodieType {
    Text,
    Number,
    Bool,
    Uuid,
    Hash,
    List(Box<CodieType>),
    Map(Box<CodieType>, Box<CodieType>),
    Custom(String),
    Any,
}

impl CodieType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "text" | "string" => Self::Text,
            "number" | "int" | "float" => Self::Number,
            "bool" | "boolean" => Self::Bool,
            "uuid" => Self::Uuid,
            "hash" => Self::Hash,
            "any" => Self::Any,
            _ => Self::Custom(s.to_string()),
        }
    }
}

/// Source kinds for bark operations
///
/// PTC REQUIRED for Vault access
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceKind {
    /// @database/... - Database access
    Database,
    /// @api/... - API endpoint
    Api,
    /// @storage/... - File storage
    Storage,
    /// $vault/... - Secret vault (PTC REQUIRED)
    Vault,
    /// @foam/... - BS-ARTISAN foam lookup
    Foam,
    /// @btc/... - Bitcoin blockchain
    Btc,
    /// @llm/... - LLM completion
    Llm,
    /// @network/... - Network resource
    Network,
    /// Unknown/custom source
    Custom(String),
}

impl SourceKind {
    pub fn from_path(path: &str) -> Self {
        if path.starts_with('$') {
            return Self::Vault;
        }
        let path = path.strip_prefix('@').unwrap_or(path);
        let first_part = path.split('/').next().unwrap_or("");

        match first_part {
            "database" | "db" => Self::Database,
            "api" => Self::Api,
            "storage" => Self::Storage,
            "vault" => Self::Vault,
            "foam" => Self::Foam,
            "btc" | "bitcoin" => Self::Btc,
            "llm" => Self::Llm,
            "network" => Self::Network,
            _ => Self::Custom(first_part.to_string()),
        }
    }

    /// Check if this source requires PTC (Permission To Change)
    pub fn requires_ptc(&self) -> bool {
        matches!(self, Self::Vault)
    }
}

/// Literal values in CODIE
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodieLiteral {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// The CODIE Abstract Syntax Tree
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodieAst {
    /// Program entry point: pug NAME
    Program {
        name: String,
        hash: Option<String>,
        body: Vec<CodieAst>,
    },

    /// Fetch operation: bark target ← source
    Fetch {
        target: String,
        source: String,
        source_kind: SourceKind,
        options: HashMap<String, CodieAst>,
        error_handlers: Vec<ErrorHandler>,
    },

    /// Loop: spin item IN collection
    Loop {
        iterator: String,
        collection: Box<CodieAst>,
        body: Vec<CodieAst>,
    },

    /// While loop: spin WHILE condition
    WhileLoop {
        condition: Box<CodieAst>,
        body: Vec<CodieAst>,
    },

    /// Forever loop: spin FOREVER
    ForeverLoop { body: Vec<CodieAst> },

    /// Times loop: spin N TIMES
    TimesLoop {
        count: u64,
        body: Vec<CodieAst>,
    },

    /// Function definition: cali NAME
    Function {
        name: String,
        params: Vec<(String, Option<CodieType>)>,
        body: Vec<CodieAst>,
        returns: Option<Box<CodieAst>>,
    },

    /// Variable binding: elf name ← value
    Variable {
        name: String,
        type_hint: Option<CodieType>,
        value: Box<CodieAst>,
    },

    /// Incomplete marker: turk or turk(#hash)
    Incomplete {
        hash: Option<String>,
        comment: Option<String>,
    },

    /// Constraint block: fence
    Constraint { rules: Vec<CodieAst> },

    /// Specification block: pin
    Specification {
        name: Option<String>,
        fields: Vec<(String, CodieAst)>,
    },

    /// Immutable rule: bone NOT: rule
    Immutable { rule: String },

    /// Flexible block: blob
    Flexible {
        name: Option<String>,
        body: Vec<CodieAst>,
    },

    /// Goal/return: biz → result
    Goal {
        expression: Box<CodieAst>,
        anchor_hash: Option<String>,
    },

    /// Checkpoint: anchor #hash
    Checkpoint { hash: String },

    /// Conditional: ? condition → action
    Conditional {
        condition: Box<CodieAst>,
        then_branch: Box<CodieAst>,
    },

    /// Return expression: → value
    Return { value: Box<CodieAst> },

    /// Break statement
    Break,

    /// Source reference: @path or $path
    Source {
        kind: SourceKind,
        path: String,
        args: Vec<CodieAst>,
    },

    /// Literal value
    Literal(CodieLiteral),

    /// Identifier reference
    Identifier(String),

    /// Binary operation
    BinaryOp {
        left: Box<CodieAst>,
        op: String,
        right: Box<CodieAst>,
    },

    /// Function call
    Call {
        function: String,
        args: Vec<CodieAst>,
    },

    /// Object/map literal: {key: value, ...}
    Object { fields: Vec<(String, CodieAst)> },

    /// List literal: [item, ...]
    List { items: Vec<CodieAst> },

    /// Property access: obj.prop
    Property {
        object: Box<CodieAst>,
        property: String,
    },

    /// Comment (for preservation)
    Comment(String),

    /// Empty node
    Empty,
}

impl CodieAst {
    /// Check if this node requires PTC review
    pub fn requires_ptc(&self) -> bool {
        match self {
            CodieAst::Fetch { source_kind, .. } => source_kind.requires_ptc(),
            CodieAst::Source { kind, .. } => kind.requires_ptc(),
            CodieAst::Program { body, .. } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::Function { body, .. } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::Loop { body, .. } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::WhileLoop { body, .. } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::ForeverLoop { body } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::TimesLoop { body, .. } => body.iter().any(|n| n.requires_ptc()),
            CodieAst::Constraint { rules } => rules.iter().any(|n| n.requires_ptc()),
            CodieAst::Flexible { body, .. } => body.iter().any(|n| n.requires_ptc()),
            _ => false,
        }
    }

    /// Get all vault references in this AST
    pub fn vault_references(&self) -> Vec<String> {
        let mut refs = Vec::new();
        self.collect_vault_refs(&mut refs);
        refs
    }

    fn collect_vault_refs(&self, refs: &mut Vec<String>) {
        match self {
            CodieAst::Fetch {
                source,
                source_kind: SourceKind::Vault,
                ..
            } => {
                refs.push(source.clone());
            }
            CodieAst::Source {
                kind: SourceKind::Vault,
                path,
                ..
            } => {
                refs.push(path.clone());
            }
            CodieAst::Program { body, .. }
            | CodieAst::Function { body, .. }
            | CodieAst::Loop { body, .. }
            | CodieAst::WhileLoop { body, .. }
            | CodieAst::ForeverLoop { body }
            | CodieAst::TimesLoop { body, .. }
            | CodieAst::Constraint { rules: body }
            | CodieAst::Flexible { body, .. } => {
                for node in body {
                    node.collect_vault_refs(refs);
                }
            }
            _ => {}
        }
    }
}

/// Error handler for fetch operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorHandler {
    pub condition: String,
    pub action: CodieAst,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_kind_parsing() {
        assert_eq!(SourceKind::from_path("@database/users"), SourceKind::Database);
        assert_eq!(SourceKind::from_path("@api/endpoint"), SourceKind::Api);
        assert_eq!(SourceKind::from_path("$vault/secret"), SourceKind::Vault);
        assert_eq!(SourceKind::from_path("@foam/hash"), SourceKind::Foam);
    }

    #[test]
    fn test_ptc_detection() {
        let vault_source = CodieAst::Source {
            kind: SourceKind::Vault,
            path: "secret_key".to_string(),
            args: vec![],
        };
        assert!(vault_source.requires_ptc());

        let db_source = CodieAst::Source {
            kind: SourceKind::Database,
            path: "users".to_string(),
            args: vec![],
        };
        assert!(!db_source.requires_ptc());
    }
}
