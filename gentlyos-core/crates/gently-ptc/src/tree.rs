//! Tree loading from JSON.
//!
//! Provides a universal node tree that represents the system hierarchy
//! from System level down to individual Lines.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scale level of a node in the universal tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeScale {
    System,
    Domain,
    Crate,
    Module,
    Function,
    Line,
}

impl NodeScale {
    /// Return the depth index (0 = System, 5 = Line).
    pub fn depth(&self) -> usize {
        match self {
            NodeScale::System => 0,
            NodeScale::Domain => 1,
            NodeScale::Crate => 2,
            NodeScale::Module => 3,
            NodeScale::Function => 4,
            NodeScale::Line => 5,
        }
    }
}

/// A node in the universal tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalNode {
    pub id: String,
    pub name: String,
    pub scale: NodeScale,
    pub children: Vec<String>,
    pub metadata: serde_json::Value,
}

impl UniversalNode {
    /// Check if this node is a leaf (no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// The universal tree: nodes indexed by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub nodes: HashMap<String, UniversalNode>,
    pub root_ids: Vec<String>,
}

impl Tree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_ids: Vec::new(),
        }
    }

    /// Get a node by ID.
    pub fn get(&self, id: &str) -> Option<&UniversalNode> {
        self.nodes.get(id)
    }

    /// Get all leaf node IDs.
    pub fn leaves(&self) -> Vec<String> {
        self.nodes
            .values()
            .filter(|n| n.is_leaf())
            .map(|n| n.id.clone())
            .collect()
    }

    /// Return total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Parse a `NodeScale` from a string.
fn parse_scale(s: &str) -> NodeScale {
    match s.to_lowercase().as_str() {
        "system" => NodeScale::System,
        "domain" => NodeScale::Domain,
        "crate" => NodeScale::Crate,
        "module" => NodeScale::Module,
        "function" => NodeScale::Function,
        "line" => NodeScale::Line,
        _ => NodeScale::Module, // default fallback
    }
}

/// Load a tree from a JSON value.
///
/// Expected JSON structure:
/// ```json
/// {
///   "nodes": [
///     {
///       "id": "root",
///       "name": "GentlyOS",
///       "scale": "System",
///       "children": ["security", "search"],
///       "metadata": {}
///     },
///     ...
///   ],
///   "roots": ["root"]
/// }
/// ```
pub fn load_from_json(value: &serde_json::Value) -> Result<Tree> {
    let nodes_arr = value
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("JSON must contain a 'nodes' array"))?;

    let mut nodes = HashMap::new();

    for node_val in nodes_arr {
        let id = node_val
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Each node must have a string 'id'"))?
            .to_string();

        let name = node_val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let scale_str = node_val
            .get("scale")
            .and_then(|v| v.as_str())
            .unwrap_or("Module");
        let scale = parse_scale(scale_str);

        let children: Vec<String> = node_val
            .get("children")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let metadata = node_val
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        nodes.insert(
            id.clone(),
            UniversalNode {
                id,
                name,
                scale,
                children,
                metadata,
            },
        );
    }

    let root_ids: Vec<String> = value
        .get("roots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(Tree { nodes, root_ids })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree_json() -> serde_json::Value {
        serde_json::json!({
            "nodes": [
                {
                    "id": "root",
                    "name": "GentlyOS",
                    "scale": "System",
                    "children": ["security", "search"],
                    "metadata": { "version": "1.0.0" }
                },
                {
                    "id": "security",
                    "name": "Security Domain",
                    "scale": "Domain",
                    "children": ["fafo", "berlin"],
                    "metadata": {}
                },
                {
                    "id": "search",
                    "name": "Search Domain",
                    "scale": "Domain",
                    "children": [],
                    "metadata": { "keywords": ["search", "query", "find"] }
                },
                {
                    "id": "fafo",
                    "name": "FAFO Defense",
                    "scale": "Module",
                    "children": [],
                    "metadata": { "keywords": ["threat", "attack", "defense"] }
                },
                {
                    "id": "berlin",
                    "name": "Berlin Clock",
                    "scale": "Module",
                    "children": [],
                    "metadata": { "keywords": ["crypto", "key", "rotation"] }
                }
            ],
            "roots": ["root"]
        })
    }

    #[test]
    fn test_load_tree_node_count() {
        let json = sample_tree_json();
        let tree = load_from_json(&json).expect("should parse tree");
        assert_eq!(tree.node_count(), 5);
    }

    #[test]
    fn test_tree_root_ids() {
        let json = sample_tree_json();
        let tree = load_from_json(&json).expect("should parse tree");
        assert_eq!(tree.root_ids, vec!["root".to_string()]);
    }

    #[test]
    fn test_tree_leaves() {
        let json = sample_tree_json();
        let tree = load_from_json(&json).expect("should parse tree");
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 3); // search, fafo, berlin
        assert!(leaves.contains(&"search".to_string()));
        assert!(leaves.contains(&"fafo".to_string()));
        assert!(leaves.contains(&"berlin".to_string()));
    }

    #[test]
    fn test_node_scale_depth() {
        assert_eq!(NodeScale::System.depth(), 0);
        assert_eq!(NodeScale::Line.depth(), 5);
    }

    #[test]
    fn test_get_node() {
        let json = sample_tree_json();
        let tree = load_from_json(&json).expect("should parse tree");
        let root = tree.get("root").expect("root should exist");
        assert_eq!(root.name, "GentlyOS");
        assert_eq!(root.scale, NodeScale::System);
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_missing_nodes_array() {
        let bad_json = serde_json::json!({ "roots": ["x"] });
        assert!(load_from_json(&bad_json).is_err());
    }
}
