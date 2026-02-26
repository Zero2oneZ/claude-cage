//! Three Kings — Training data provenance metadata
//!
//! Every reasoning step carries provenance via three hashes:
//! - Gold:          WHO created it (identity hash)
//! - Myrrh:         WHAT model/context produced it (preservation)
//! - Frankincense:  WHY it matters (intention hash)
//!
//! These map to the 5W dimensions in Alexandria but distilled
//! to the three that matter for on-chain provenance.

use serde::{Deserialize, Serialize};

/// Provenance metadata for training data contributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeKings {
    /// WHO created this (blake3 hash of identity)
    pub gold: Vec<u8>,
    /// WHAT model/context produced it (blake3 hash of model + context)
    pub myrrh: Vec<u8>,
    /// WHY it matters (blake3 hash of intention/purpose)
    pub frankincense: Vec<u8>,
}

impl Default for ThreeKings {
    fn default() -> Self {
        Self {
            gold: vec![0u8; 32],
            myrrh: vec![0u8; 32],
            frankincense: vec![0u8; 32],
        }
    }
}

impl ThreeKings {
    /// Create provenance from raw identity, context, and intention strings
    pub fn from_strings(identity: &str, context: &str, intention: &str) -> Self {
        Self {
            gold: blake3::hash(identity.as_bytes()).as_bytes().to_vec(),
            myrrh: blake3::hash(context.as_bytes()).as_bytes().to_vec(),
            frankincense: blake3::hash(intention.as_bytes()).as_bytes().to_vec(),
        }
    }

    /// Create from pre-computed hashes
    pub fn from_hashes(gold: &[u8], myrrh: &[u8], frankincense: &[u8]) -> Self {
        Self {
            gold: gold.to_vec(),
            myrrh: myrrh.to_vec(),
            frankincense: frankincense.to_vec(),
        }
    }

    /// Combined provenance hash (for deduplication)
    pub fn combined_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.gold);
        hasher.update(&self.myrrh);
        hasher.update(&self.frankincense);
        *hasher.finalize().as_bytes()
    }

    /// Check if provenance is empty (all zeros)
    pub fn is_empty(&self) -> bool {
        self.gold.iter().all(|&b| b == 0)
            && self.myrrh.iter().all(|&b| b == 0)
            && self.frankincense.iter().all(|&b| b == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_strings() {
        let kings = ThreeKings::from_strings("alice", "claude-3.5-sonnet", "fix auth bug");
        assert_eq!(kings.gold.len(), 32);
        assert_eq!(kings.myrrh.len(), 32);
        assert_eq!(kings.frankincense.len(), 32);
        assert!(!kings.is_empty());
    }

    #[test]
    fn test_default_is_empty() {
        let kings = ThreeKings::default();
        assert!(kings.is_empty());
    }

    #[test]
    fn test_combined_hash_deterministic() {
        let a = ThreeKings::from_strings("x", "y", "z");
        let b = ThreeKings::from_strings("x", "y", "z");
        assert_eq!(a.combined_hash(), b.combined_hash());
    }
}
