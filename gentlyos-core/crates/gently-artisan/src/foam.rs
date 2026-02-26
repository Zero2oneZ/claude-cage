//! Foam - Multi-torus interconnected memory
//!
//! Steps 1.4, 1.6 from BUILD_STEPS.md
//! PTC REQUIRED: genesis anchor verification

use crate::torus::Torus;
use crate::coord::TorusCoordinate;
use crate::flux::FluxLine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A blend between two tori (connection/relationship)
///
/// When tori are blended, traversal can flow between them
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorusBlend {
    /// First torus ID
    pub torus_a: [u8; 32],

    /// Second torus ID
    pub torus_b: [u8; 32],

    /// Connection point on torus A
    pub point_a: TorusCoordinate,

    /// Connection point on torus B
    pub point_b: TorusCoordinate,

    /// Connection strength (0.0 - 1.0)
    /// Higher = tighter relationship
    pub strength: f64,

    /// When this blend was created (for decay tracking)
    pub created_at: u64,
}

impl TorusBlend {
    /// Create a new blend between two tori
    pub fn new(
        torus_a: [u8; 32],
        torus_b: [u8; 32],
        point_a: TorusCoordinate,
        point_b: TorusCoordinate,
        strength: f64,
    ) -> Self {
        Self {
            torus_a,
            torus_b,
            point_a,
            point_b,
            strength: strength.clamp(0.0, 1.0),
            created_at: 0,
        }
    }

    /// Create a simple blend (default coordinates)
    pub fn simple(torus_a: [u8; 32], torus_b: [u8; 32], strength: f64) -> Self {
        Self::new(
            torus_a,
            torus_b,
            TorusCoordinate::default(),
            TorusCoordinate::default(),
            strength,
        )
    }

    /// Check if blend connects a specific torus
    pub fn connects(&self, torus_id: &[u8; 32]) -> bool {
        &self.torus_a == torus_id || &self.torus_b == torus_id
    }

    /// Get the other torus in the blend
    pub fn other(&self, torus_id: &[u8; 32]) -> Option<[u8; 32]> {
        if &self.torus_a == torus_id {
            Some(self.torus_b)
        } else if &self.torus_b == torus_id {
            Some(self.torus_a)
        } else {
            None
        }
    }

    /// Check if blend is symmetric (bidirectional with equal strength)
    pub fn is_symmetric(&self) -> bool {
        true // All blends are symmetric by default
    }

    /// Decay strength over time
    pub fn decay(&mut self, factor: f64) {
        self.strength *= factor;
        self.strength = self.strength.clamp(0.0, 1.0);
    }

    /// Boost strength (usage reinforcement)
    pub fn boost(&mut self, amount: f64) {
        self.strength += amount * (1.0 - self.strength);
        self.strength = self.strength.clamp(0.0, 1.0);
    }
}

/// The Foam - interconnected collection of tori
///
/// This is the primary knowledge storage structure in BS-ARTISAN.
/// Replaces vector embeddings with topological relationships.
///
/// PTC REQUIRED: genesis anchor verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foam {
    /// All tori in this foam, keyed by ID
    pub tori: HashMap<[u8; 32], Torus>,

    /// Blends connecting tori
    pub blends: Vec<TorusBlend>,

    /// Active flux lines (in-progress traversals)
    #[serde(skip)]
    pub active_flux: Vec<FluxLine>,

    /// Genesis anchor - BTC block hash for verification
    /// PTC: Must be validated against blockchain
    pub genesis: [u8; 32],

    /// When this foam was created
    pub created_at: u64,

    /// Total tokens invested in this foam
    pub total_tokens: u64,
}

impl Foam {
    /// Create a new foam with a genesis anchor
    ///
    /// PTC: genesis should be a valid BTC block hash
    pub fn new(genesis: [u8; 32]) -> Self {
        Self {
            tori: HashMap::new(),
            blends: Vec::new(),
            active_flux: Vec::new(),
            genesis,
            created_at: 0,
            total_tokens: 0,
        }
    }

    /// Insert a torus into the foam
    pub fn insert(&mut self, torus: Torus) {
        self.tori.insert(torus.id, torus);
    }

    /// Get a torus by ID
    pub fn get(&self, id: &[u8; 32]) -> Option<&Torus> {
        self.tori.get(id)
    }

    /// Get a mutable torus by ID
    pub fn get_mut(&mut self, id: &[u8; 32]) -> Option<&mut Torus> {
        self.tori.get_mut(id)
    }

    /// Create a blend between two tori
    pub fn blend(&mut self, a: [u8; 32], b: [u8; 32], strength: f64) -> bool {
        // Verify both tori exist
        if !self.tori.contains_key(&a) || !self.tori.contains_key(&b) {
            return false;
        }

        // Check if blend already exists
        for existing in &mut self.blends {
            if (existing.torus_a == a && existing.torus_b == b)
                || (existing.torus_a == b && existing.torus_b == a)
            {
                // Update existing blend strength
                existing.boost(strength);
                return true;
            }
        }

        // Create new blend
        self.blends.push(TorusBlend::simple(a, b, strength));
        true
    }

    /// Get all tori connected to a given torus
    pub fn connected(&self, torus_id: &[u8; 32]) -> Vec<[u8; 32]> {
        self.blends
            .iter()
            .filter_map(|blend| blend.other(torus_id))
            .collect()
    }

    /// Get blends for a specific torus
    pub fn blends_for(&self, torus_id: &[u8; 32]) -> Vec<&TorusBlend> {
        self.blends
            .iter()
            .filter(|b| b.connects(torus_id))
            .collect()
    }

    /// Traverse foam from a starting torus up to max_depth
    pub fn traverse(&self, start: &[u8; 32], max_depth: usize) -> Vec<[u8; 32]> {
        let mut visited = Vec::new();
        let mut queue = vec![(*start, 0usize)];

        while let Some((current, depth)) = queue.pop() {
            if depth > max_depth || visited.contains(&current) {
                continue;
            }

            visited.push(current);

            if depth < max_depth {
                for connected in self.connected(&current) {
                    if !visited.contains(&connected) {
                        queue.push((connected, depth + 1));
                    }
                }
            }
        }

        visited
    }

    /// Count total tori
    pub fn len(&self) -> usize {
        self.tori.len()
    }

    /// Check if foam is empty
    pub fn is_empty(&self) -> bool {
        self.tori.is_empty()
    }

    /// Get all torus IDs
    pub fn torus_ids(&self) -> Vec<[u8; 32]> {
        self.tori.keys().copied().collect()
    }

    /// Decay all blend strengths (call periodically)
    pub fn decay_blends(&mut self, factor: f64) {
        for blend in &mut self.blends {
            blend.decay(factor);
        }
        // Remove dead blends
        self.blends.retain(|b| b.strength > 0.01);
    }

    /// Get foam statistics
    pub fn stats(&self) -> FoamStats {
        let total_bs: f64 = self.tori.values().map(|t| t.bs).sum();
        let avg_bs = if self.tori.is_empty() {
            1.0
        } else {
            total_bs / self.tori.len() as f64
        };

        let avg_blend_strength = if self.blends.is_empty() {
            0.0
        } else {
            self.blends.iter().map(|b| b.strength).sum::<f64>() / self.blends.len() as f64
        };

        FoamStats {
            torus_count: self.tori.len(),
            blend_count: self.blends.len(),
            avg_bs,
            avg_blend_strength,
            total_tokens: self.total_tokens,
        }
    }
}

/// Statistics about a Foam
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoamStats {
    pub torus_count: usize,
    pub blend_count: usize,
    pub avg_bs: f64,
    pub avg_blend_strength: f64,
    pub total_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_genesis() -> [u8; 32] {
        [0u8; 32] // Test genesis
    }

    #[test]
    fn test_foam_creation() {
        let foam = Foam::new(test_genesis());
        assert!(foam.is_empty());
        assert_eq!(foam.len(), 0);
    }

    #[test]
    fn test_foam_insert_and_get() {
        let mut foam = Foam::new(test_genesis());
        let torus = Torus::new("test", 5.0, 100);
        let id = torus.id;

        foam.insert(torus);

        assert_eq!(foam.len(), 1);
        assert!(foam.get(&id).is_some());
        assert_eq!(foam.get(&id).unwrap().label, "test");
    }

    #[test]
    fn test_foam_blend() {
        let mut foam = Foam::new(test_genesis());

        let t1 = Torus::new("concept_a", 5.0, 100);
        let t2 = Torus::new("concept_b", 5.0, 100);
        let id1 = t1.id;
        let id2 = t2.id;

        foam.insert(t1);
        foam.insert(t2);

        assert!(foam.blend(id1, id2, 0.8));
        assert_eq!(foam.blends.len(), 1);

        let connected = foam.connected(&id1);
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0], id2);
    }

    #[test]
    fn test_foam_traverse() {
        let mut foam = Foam::new(test_genesis());

        let t1 = Torus::new("a", 5.0, 100);
        let t2 = Torus::new("b", 5.0, 100);
        let t3 = Torus::new("c", 5.0, 100);
        let id1 = t1.id;
        let id2 = t2.id;
        let id3 = t3.id;

        foam.insert(t1);
        foam.insert(t2);
        foam.insert(t3);

        foam.blend(id1, id2, 0.5);
        foam.blend(id2, id3, 0.5);

        // Traverse from id1 with depth 2 should reach all
        let reached = foam.traverse(&id1, 2);
        assert_eq!(reached.len(), 3);

        // Traverse with depth 1 should only reach id1 and id2
        let reached = foam.traverse(&id1, 1);
        assert_eq!(reached.len(), 2);
    }
}
