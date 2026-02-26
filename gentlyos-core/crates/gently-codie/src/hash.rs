//! CODIE Hash-Addressable Encoding
//!
//! Provides content-addressable hashing for compressed CODIE.
//! Enables passing entire apps as hash strings for instant hydration.
//!
//! ## Hash Types
//!
//! ```text
//! Short Hash:   #c7f3a2b1          (8 hex chars, fast lookup)
//! Full Hash:    #c7f3a2b1e4d5f6... (64 hex chars, SHA-256)
//! Compact Hash: ©ρβμ...            (Base-glyph encoding)
//! ```
//!
//! ## PTC Integration
//!
//! Hashes integrate with Permission To Change (PTC) for:
//! - Vault access discrimination ($vault → requires PTC)
//! - Hash verification (#hash → BTC anchor check)
//! - Source integrity (@source → content hash match)
//!
//! ## SVG/HTMX Embedding
//!
//! ```svg
//! <svg data-codie="#c7f3a2b1" data-hydrate="auto">
//!   <!-- UI elements defined by CODIE -->
//! </svg>
//! ```
//!
//! The hash resolves to compressed CODIE which hydrates into:
//! - Interactive elements (HTMX attributes)
//! - Visual structure (SVG paths)
//! - Data bindings (reactive updates)

use std::collections::HashMap;
use std::sync::RwLock;
use lazy_static::lazy_static;

/// A CODIE content hash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodieHash {
    /// Full SHA-256 hash bytes
    bytes: [u8; 32],
}

impl CodieHash {
    /// Create hash from compressed CODIE
    pub fn from_compressed(compressed: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Use a simple hash for now (would use SHA-256 in production)
        let mut hasher = DefaultHasher::new();
        compressed.hash(&mut hasher);
        let hash64 = hasher.finish();

        // Expand to 32 bytes by hashing multiple times
        let mut bytes = [0u8; 32];
        for i in 0..4 {
            let mut h = DefaultHasher::new();
            (hash64, i).hash(&mut h);
            let chunk = h.finish().to_le_bytes();
            bytes[i*8..(i+1)*8].copy_from_slice(&chunk);
        }

        Self { bytes }
    }

    /// Short hash (8 hex chars) for quick lookup
    pub fn short(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3])
    }

    /// Full hash (64 hex chars)
    pub fn full(&self) -> String {
        let hex: String = self.bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        format!("#{}", hex)
    }

    /// Compact hash using glyph encoding (shorter than hex)
    pub fn compact(&self) -> String {
        // Use base-64-like encoding with CODIE-friendly chars
        const ALPHABET: &[char] = &[
            'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ',
            'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο', 'π',
            'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω',
            'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ',
            'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ', 'Ο', 'Π',
            'Ρ', 'Σ', 'Τ', 'Υ', 'Φ', 'Χ', 'Ψ', 'Ω',
            '0', '1', '2', '3', '4', '5', '6', '7',
            '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ];

        let mut result = String::from("©");
        // Encode first 12 bytes (gives ~16 chars)
        for &byte in &self.bytes[..12] {
            let idx = (byte as usize) % ALPHABET.len();
            result.push(ALPHABET[idx]);
        }
        result
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Create from hex string (with or without # prefix)
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() < 8 {
            return None;
        }

        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            if i >= 32 {
                break;
            }
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(chunk).ok()?,
                16
            ) {
                bytes[i] = byte;
            }
        }

        Some(Self { bytes })
    }
}

impl std::fmt::Display for CodieHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short())
    }
}

lazy_static! {
    /// Global hash registry for content-addressable lookup
    static ref HASH_REGISTRY: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

/// Register compressed CODIE by hash
pub fn register(compressed: &str) -> CodieHash {
    let hash = CodieHash::from_compressed(compressed);
    let short = hash.short();

    if let Ok(mut registry) = HASH_REGISTRY.write() {
        registry.insert(short.clone(), compressed.to_string());
        registry.insert(hash.full(), compressed.to_string());
    }

    hash
}

/// Lookup compressed CODIE by hash
pub fn lookup(hash: &str) -> Option<String> {
    let normalized = if hash.starts_with('#') {
        hash.to_string()
    } else if hash.starts_with('©') {
        // Compact hash - need to search registry
        return lookup_compact(hash);
    } else {
        format!("#{}", hash)
    };

    HASH_REGISTRY.read().ok()?.get(&normalized).cloned()
}

/// Lookup by compact hash
fn lookup_compact(compact: &str) -> Option<String> {
    // For now, search all registered hashes
    // In production, this would use a proper index
    let registry = HASH_REGISTRY.read().ok()?;
    for (key, value) in registry.iter() {
        let hash = CodieHash::from_compressed(value);
        if hash.compact() == compact {
            return Some(value.clone());
        }
    }
    None
}

/// Hash and store CODIE, return all hash formats
pub fn store(compressed: &str) -> HashBundle {
    let hash = register(compressed);
    HashBundle {
        short: hash.short(),
        full: hash.full(),
        compact: hash.compact(),
        size: compressed.len(),
    }
}

/// Bundle of all hash formats for a stored CODIE
#[derive(Debug, Clone)]
pub struct HashBundle {
    pub short: String,
    pub full: String,
    pub compact: String,
    pub size: usize,
}

impl HashBundle {
    /// Generate SVG data attribute
    pub fn svg_attr(&self) -> String {
        format!("data-codie=\"{}\" data-size=\"{}\"", self.short, self.size)
    }

    /// Generate HTMX attribute for auto-hydration
    pub fn htmx_attr(&self) -> String {
        format!(
            "hx-get=\"/codie/hydrate{}\" hx-trigger=\"load\" hx-swap=\"innerHTML\"",
            self.short
        )
    }

    /// Generate complete element for SVG embedding
    pub fn svg_element(&self, tag: &str) -> String {
        format!(
            "<{} {} {} class=\"codie-hydrate\"></{}>",
            tag, self.svg_attr(), self.htmx_attr(), tag
        )
    }
}

/// PTC Discriminator for hash-based access control
#[derive(Debug, Clone, PartialEq)]
pub enum PtcLevel {
    /// Public - anyone can access
    Public,
    /// Registered - requires known hash
    Registered,
    /// Vault - requires PTC approval
    Vault,
    /// Anchor - requires BTC block verification
    Anchor,
}

/// Check PTC level from reference prefix
pub fn ptc_level(reference: &str) -> PtcLevel {
    match reference.chars().next() {
        Some('@') => PtcLevel::Public,     // @source - public data
        Some('#') => PtcLevel::Registered, // #hash - registered content
        Some('$') => PtcLevel::Vault,      // $vault - secure storage
        Some('₿') => PtcLevel::Anchor,     // ₿block - BTC anchored
        _ => PtcLevel::Public,
    }
}

/// Validate hash reference integrity
pub fn verify_reference(reference: &str) -> bool {
    let level = ptc_level(reference);

    match level {
        PtcLevel::Public => true, // Always accessible
        PtcLevel::Registered => lookup(reference).is_some(),
        PtcLevel::Vault => {
            // Would check PTC system in production
            // For now, verify it's registered
            lookup(&reference.replace('$', "#")).is_some()
        }
        PtcLevel::Anchor => {
            // Would verify BTC anchor in production
            false // Requires external verification
        }
    }
}

/// Encode compressed CODIE for URL/data URI
pub fn encode_url(compressed: &str) -> String {
    // Use percent encoding for URL safety
    let mut encoded = String::new();
    for c in compressed.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            encoded.push(c);
        } else {
            // Encode as UTF-8 bytes
            for byte in c.to_string().as_bytes() {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Decode URL-encoded compressed CODIE
pub fn decode_url(encoded: &str) -> String {
    let mut decoded = Vec::new();
    let mut chars = encoded.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Parse hex bytes
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                decoded.push(byte);
            }
        } else {
            decoded.extend(c.to_string().as_bytes());
        }
    }

    String::from_utf8_lossy(&decoded).to_string()
}

/// Generate a data URI containing compressed CODIE
pub fn data_uri(compressed: &str) -> String {
    let encoded = encode_url(compressed);
    format!("data:text/codie;charset=utf-8,{}", encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let compressed = "ρTEST⟨βdata⟩";
        let hash = CodieHash::from_compressed(compressed);

        assert!(hash.short().starts_with('#'));
        assert_eq!(hash.short().len(), 9); // # + 8 hex chars
        assert_eq!(hash.full().len(), 65); // # + 64 hex chars
        assert!(hash.compact().starts_with('©'));
    }

    #[test]
    fn test_deterministic_hash() {
        let compressed = "ρTEST⟨βdata⟩";
        let hash1 = CodieHash::from_compressed(compressed);
        let hash2 = CodieHash::from_compressed(compressed);

        assert_eq!(hash1.short(), hash2.short());
        assert_eq!(hash1.full(), hash2.full());
    }

    #[test]
    fn test_different_inputs_different_hashes() {
        let hash1 = CodieHash::from_compressed("ρTEST1");
        let hash2 = CodieHash::from_compressed("ρTEST2");

        assert_ne!(hash1.short(), hash2.short());
    }

    #[test]
    fn test_register_and_lookup() {
        let compressed = "ρUNIQUE_TEST⟨βspecial_data⟩";
        let hash = register(compressed);

        let retrieved = lookup(&hash.short());
        assert_eq!(retrieved, Some(compressed.to_string()));
    }

    #[test]
    fn test_store_bundle() {
        let compressed = "ρBUNDLE_TEST";
        let bundle = store(compressed);

        assert!(bundle.short.starts_with('#'));
        assert!(bundle.compact.starts_with('©'));
        assert_eq!(bundle.size, compressed.len());
    }

    #[test]
    fn test_ptc_levels() {
        assert_eq!(ptc_level("@database/users"), PtcLevel::Public);
        assert_eq!(ptc_level("#c7f3a2b1"), PtcLevel::Registered);
        assert_eq!(ptc_level("$vault/secrets"), PtcLevel::Vault);
        assert_eq!(ptc_level("₿block/123"), PtcLevel::Anchor);
    }

    #[test]
    fn test_url_encoding() {
        let compressed = "ρTEST⟨βdata←@source⟩";
        let encoded = encode_url(compressed);
        let decoded = decode_url(&encoded);

        assert_eq!(compressed, decoded);
    }

    #[test]
    fn test_svg_attr() {
        let bundle = store("ρSVG_TEST");
        let attr = bundle.svg_attr();

        assert!(attr.contains("data-codie="));
        assert!(attr.contains("data-size="));
    }

    #[test]
    fn test_htmx_attr() {
        let bundle = store("ρHTMX_TEST");
        let attr = bundle.htmx_attr();

        assert!(attr.contains("hx-get="));
        assert!(attr.contains("hx-trigger=\"load\""));
    }
}
