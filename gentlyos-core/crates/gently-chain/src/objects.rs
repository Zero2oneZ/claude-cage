//! Sui object read/query — Sui's core primitive
//!
//! Everything in Sui is an object. Objects have owners (address, shared, immutable).
//! This module wraps object queries and parsing.

use serde::{Deserialize, Serialize};
use crate::types::ObjectID;

/// Sui object ownership model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectOwner {
    /// Owned by a single address
    Address(String),
    /// Shared object (anyone can use in transactions)
    Shared { initial_shared_version: u64 },
    /// Immutable (frozen forever)
    Immutable,
}

/// A Sui object with parsed metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiObject {
    /// Object ID
    pub id: ObjectID,
    /// Object version (increments on mutation)
    pub version: u64,
    /// Object digest (content hash)
    pub digest: String,
    /// Move type (e.g., "0x2::coin::Coin<0x2::sui::SUI>")
    pub type_tag: Option<String>,
    /// Owner
    pub owner: ObjectOwner,
    /// Raw BCS content (decoded by caller)
    pub content: Option<serde_json::Value>,
}

/// Query parameters for fetching objects
#[derive(Debug, Clone)]
pub struct ObjectQuery {
    /// Object ID to fetch
    pub id: ObjectID,
    /// Specific version (None = latest)
    pub version: Option<u64>,
    /// Whether to include BCS content
    pub with_content: bool,
}

impl ObjectQuery {
    pub fn by_id(id: ObjectID) -> Self {
        Self {
            id,
            version: None,
            with_content: true,
        }
    }

    pub fn at_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }
}

/// Query result for multiple objects
#[derive(Debug, Clone)]
pub struct ObjectQueryResult {
    pub objects: Vec<SuiObject>,
    pub has_more: bool,
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_query() {
        let q = ObjectQuery::by_id(ObjectID::zero()).at_version(42);
        assert_eq!(q.version, Some(42));
        assert!(q.with_content);
    }
}
