//! Torus and TorusPoint - Core geometry primitives
//!
//! Steps 1.2, 1.3 from BUILD_STEPS.md
//! PTC REQUIRED: blake3 hash generation for Torus.id and TorusPoint.content_hash

use crate::coord::TorusCoordinate;
use crate::tokens_to_radius;
use serde::{Deserialize, Serialize};

/// A knowledge torus in the Foam
///
/// Represents a concept/topic as a toroidal surface where:
/// - `major_radius` (R) = scope/importance of the concept
/// - `minor_radius` (r) = tokens_spent / 2π (investment in understanding)
/// - `winding` = refinement level (1-6)
/// - `bs` = substance score (0-1, lower = more substance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torus {
    /// Unique identifier - blake3 hash of label + creation context
    /// PTC: Hash generation
    pub id: [u8; 32],

    /// Human-readable label for this torus
    pub label: String,

    /// Major radius (R) - scope/importance
    pub major_radius: f64,

    /// Minor radius (r) - derived from tokens spent (r = tokens / 2π)
    pub minor_radius: f64,

    /// Winding/refinement level (1-6)
    /// 1=RawIdea, 2=Structured, 3=Refined, 4=Tested, 5=Documented, 6=Production
    pub winding: u8,

    /// Bullshit score (0-1, lower = more substance)
    /// Derived from: validation passes, citation count, test coverage
    pub bs: f64,

    /// Parent torus (for hierarchical concepts)
    pub parent: Option<[u8; 32]>,

    /// Creation timestamp (BTC block height for anchoring)
    pub created_at: u64,
}

impl Torus {
    /// Create a new torus with the given parameters
    ///
    /// PTC: Uses blake3 for id generation
    pub fn new(label: &str, major_radius: f64, tokens_spent: u64) -> Self {
        let minor_radius = tokens_to_radius(tokens_spent);

        // PTC: Hash generation for id
        let id = Self::compute_id(label, major_radius);

        Self {
            id,
            label: label.to_string(),
            major_radius,
            minor_radius,
            winding: 1, // Start as RawIdea
            bs: 1.0,    // Start with maximum BS (unvalidated)
            parent: None,
            created_at: 0, // Should be set to BTC block height
        }
    }

    /// Compute torus ID from label and context
    ///
    /// PTC: blake3 hash
    fn compute_id(label: &str, major_radius: f64) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(label.as_bytes());
        hasher.update(&major_radius.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Update minor radius based on additional tokens spent
    pub fn add_tokens(&mut self, tokens: u64) {
        let current_tokens = crate::radius_to_tokens(self.minor_radius);
        self.minor_radius = tokens_to_radius(current_tokens + tokens);
    }

    /// Increase winding level (refinement)
    pub fn refine(&mut self) -> bool {
        if self.winding < 6 {
            self.winding += 1;
            true
        } else {
            false
        }
    }

    /// Update BS score based on validation
    pub fn validate(&mut self, validation_score: f64) {
        // BS decreases as validation increases
        // New BS = weighted average of old and validation
        self.bs = self.bs * 0.7 + (1.0 - validation_score) * 0.3;
        self.bs = self.bs.clamp(0.0, 1.0);
    }

    /// Convert a coordinate on this torus to Cartesian space
    pub fn to_cartesian(&self, coord: &TorusCoordinate) -> (f64, f64, f64) {
        coord.to_cartesian(self.major_radius, self.minor_radius)
    }

    /// Calculate geodesic distance between two points on this torus
    pub fn distance(&self, a: &TorusCoordinate, b: &TorusCoordinate) -> f64 {
        a.distance(b, self.major_radius, self.minor_radius)
    }

    /// Get trustworthiness score (inverse of BS, scaled by winding)
    pub fn trustworthiness(&self) -> f64 {
        let bs_factor = 1.0 - self.bs;
        let winding_factor = self.winding as f64 / 6.0;
        bs_factor * 0.7 + winding_factor * 0.3
    }
}

/// A point on a specific torus, referencing content
///
/// PTC REQUIRED: content_hash uses blake3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorusPoint {
    /// Which torus this point is on
    pub torus_id: [u8; 32],

    /// Position on the torus surface
    pub coord: TorusCoordinate,

    /// Hash of the content at this point
    /// PTC: blake3 hash of content
    pub content_hash: [u8; 32],

    /// Optional metadata
    pub metadata: Option<String>,
}

impl TorusPoint {
    /// Create a new point on a torus
    ///
    /// PTC: Uses blake3 for content_hash
    pub fn new(torus_id: [u8; 32], coord: TorusCoordinate, content: &[u8]) -> Self {
        // PTC: Hash generation for content
        let content_hash = *blake3::hash(content).as_bytes();

        Self {
            torus_id,
            coord,
            content_hash,
            metadata: None,
        }
    }

    /// Create with pre-computed hash (for references)
    pub fn from_hash(torus_id: [u8; 32], coord: TorusCoordinate, content_hash: [u8; 32]) -> Self {
        Self {
            torus_id,
            coord,
            content_hash,
            metadata: None,
        }
    }

    /// Add metadata to this point
    pub fn with_metadata(mut self, metadata: &str) -> Self {
        self.metadata = Some(metadata.to_string());
        self
    }

    /// Convert to Cartesian coordinates using the given torus
    pub fn to_cartesian(&self, torus: &Torus) -> (f64, f64, f64) {
        torus.to_cartesian(&self.coord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_torus_creation() {
        let torus = Torus::new("test_concept", 10.0, 100);
        assert_eq!(torus.label, "test_concept");
        assert_eq!(torus.major_radius, 10.0);
        assert!(torus.minor_radius > 0.0);
        assert_eq!(torus.winding, 1);
        assert_eq!(torus.bs, 1.0);
    }

    #[test]
    fn test_torus_refinement() {
        let mut torus = Torus::new("test", 5.0, 50);
        assert!(torus.refine()); // 1 -> 2
        assert!(torus.refine()); // 2 -> 3
        assert_eq!(torus.winding, 3);

        // Refine to max
        torus.winding = 6;
        assert!(!torus.refine()); // Can't go past 6
    }

    #[test]
    fn test_torus_validation() {
        let mut torus = Torus::new("test", 5.0, 50);
        assert_eq!(torus.bs, 1.0);

        torus.validate(1.0); // Perfect validation
        assert!(torus.bs < 1.0);

        torus.validate(1.0);
        torus.validate(1.0);
        assert!(torus.bs < 0.5); // BS should decrease with validation
    }

    #[test]
    fn test_torus_point() {
        let torus = Torus::new("test", 5.0, 50);
        let coord = TorusCoordinate::new(1.0, 2.0);
        let point = TorusPoint::new(torus.id, coord, b"test content");

        assert_eq!(point.torus_id, torus.id);
        assert_eq!(point.coord.theta, coord.theta);
    }
}
