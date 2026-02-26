//! GentlyOS-specific Move resource type mappings
//!
//! These Rust types mirror the on-chain Move structs. Move's linear type system
//! means resources can't be copied or dropped — the compiler enforces physics.

use serde::{Deserialize, Serialize};
use crate::three_kings::ThreeKings;

/// Sui object ID (32 bytes, hex-encoded on wire)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectID(pub [u8; 32]);

impl ObjectID {
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn from_hex(hex: &str) -> anyhow::Result<Self> {
        let hex = hex.strip_prefix("0x").unwrap_or(hex);
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            anyhow::bail!("ObjectID must be 32 bytes, got {}", bytes.len());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl Default for ObjectID {
    fn default() -> Self {
        Self::zero()
    }
}

/// On-chain Move resource: a scored reasoning step
///
/// ```move
/// struct ReasoningStep has key, store {
///     id: UID,
///     quality: u64,
///     step_type: u8,
///     provenance: ThreeKings,
///     timestamp: u64,
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Sui object ID (assigned on publish)
    pub id: ObjectID,
    /// Fixed-point quality score (quality * 1_000_000)
    pub quality: u64,
    /// Step type enum as u8 (Pattern=0, Conclude=1, etc.)
    pub step_type: u8,
    /// Three Kings provenance metadata
    pub provenance: ThreeKings,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
}

impl Default for ReasoningStep {
    fn default() -> Self {
        Self {
            id: ObjectID::zero(),
            quality: 0,
            step_type: 0,
            provenance: ThreeKings::default(),
            timestamp: 0,
        }
    }
}

/// Step type discriminant for on-chain encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StepTypeOnChain {
    Pattern = 0,
    Conclude = 1,
    Eliminate = 2,
    Specific = 3,
    Fact = 4,
    Suggest = 5,
    Correct = 6,
    Guess = 7,
}

impl StepTypeOnChain {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Pattern),
            1 => Some(Self::Conclude),
            2 => Some(Self::Eliminate),
            3 => Some(Self::Specific),
            4 => Some(Self::Fact),
            5 => Some(Self::Suggest),
            6 => Some(Self::Correct),
            7 => Some(Self::Guess),
            _ => None,
        }
    }
}

/// Anchored content: IPFS CID stored with Sui object reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchoredContent {
    /// IPFS content identifier
    pub cid: String,
    /// Sui object ID anchoring this CID
    pub object_id: ObjectID,
}

fn hex_decode(hex: &str) -> anyhow::Result<Vec<u8>> {
    Ok(hex::decode(hex.strip_prefix("0x").unwrap_or(hex))?)
}

// Re-export hex for internal use
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(hex: &str) -> Result<Vec<u8>, DecodeError> {
        if hex.len() % 2 != 0 {
            return Err(DecodeError);
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| DecodeError))
            .collect()
    }

    #[derive(Debug)]
    pub struct DecodeError;
    impl std::fmt::Display for DecodeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "hex decode error")
        }
    }
    impl std::error::Error for DecodeError {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_roundtrip() {
        let id = ObjectID([42u8; 32]);
        let hex = id.to_hex();
        let parsed = ObjectID::from_hex(&hex).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_step_type_roundtrip() {
        for v in 0..=7u8 {
            assert!(StepTypeOnChain::from_u8(v).is_some());
        }
        assert!(StepTypeOnChain::from_u8(8).is_none());
    }
}
