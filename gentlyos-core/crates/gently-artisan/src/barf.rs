//! BARF - Bark And Retrieve Foam
//!
//! Steps 1.10, 1.11, 1.12 from BUILD_STEPS.md
//! PTC REQUIRED: Hash derivation from text

use crate::foam::Foam;
use serde::{Deserialize, Serialize};

/// A BARF query for retrieving from Foam
///
/// Uses XOR distance + topological path boosting
///
/// PTC REQUIRED: Hash derivation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarfQuery {
    /// Query hash (blake3 of query text)
    /// PTC: Hash derivation from text
    pub query_hash: [u8; 32],

    /// Maximum results to return
    pub max_results: usize,

    /// Whether to boost connected tori (multiply distance by 0.5)
    pub boost_connected: bool,

    /// Optional: filter by minimum winding level
    pub min_winding: Option<u8>,

    /// Optional: filter by maximum BS score
    pub max_bs: Option<f64>,
}

impl BarfQuery {
    /// Create a query from text
    ///
    /// PTC: Uses blake3 hash
    pub fn from_text(text: &str) -> Self {
        let query_hash = *blake3::hash(text.as_bytes()).as_bytes();
        Self {
            query_hash,
            max_results: 10,
            boost_connected: true,
            min_winding: None,
            max_bs: None,
        }
    }

    /// Create a query from an existing hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self {
            query_hash: hash,
            max_results: 10,
            boost_connected: true,
            min_winding: None,
            max_bs: None,
        }
    }

    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Disable connected boosting
    pub fn without_boost(mut self) -> Self {
        self.boost_connected = false;
        self
    }

    /// Filter by minimum winding level
    pub fn with_min_winding(mut self, level: u8) -> Self {
        self.min_winding = Some(level);
        self
    }

    /// Filter by maximum BS score
    pub fn with_max_bs(mut self, bs: f64) -> Self {
        self.max_bs = Some(bs);
        self
    }
}

/// Result from a BARF query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarfResult {
    /// The torus ID that matched
    pub torus_id: [u8; 32],

    /// XOR-based distance (lower = closer match)
    pub distance: f64,

    /// Path from query to this result (for explainability)
    pub path: Vec<[u8; 32]>,

    /// Torus label (for convenience)
    pub label: String,

    /// Torus trustworthiness score
    pub trustworthiness: f64,
}

impl BarfResult {
    pub fn new(torus_id: [u8; 32], distance: f64, label: String, trustworthiness: f64) -> Self {
        Self {
            torus_id,
            distance,
            path: vec![torus_id],
            label,
            trustworthiness,
        }
    }

    pub fn with_path(mut self, path: Vec<[u8; 32]>) -> Self {
        self.path = path;
        self
    }
}

/// Calculate XOR distance between two hashes
///
/// Returns a normalized distance (0.0 = identical, 1.0 = maximally different)
fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> f64 {
    let mut diff_bits = 0u32;
    for i in 0..32 {
        diff_bits += (a[i] ^ b[i]).count_ones();
    }
    // Normalize to 0.0 - 1.0 (max 256 bits different)
    diff_bits as f64 / 256.0
}

impl Foam {
    /// BARF - Bark And Retrieve Foam
    ///
    /// Retrieval using XOR distance with topological boosting
    pub fn barf(&self, query: &BarfQuery) -> Vec<BarfResult> {
        let mut results: Vec<BarfResult> = Vec::new();

        // Calculate distance to all tori
        for (id, torus) in &self.tori {
            // Apply filters
            if let Some(min_wind) = query.min_winding {
                if torus.winding < min_wind {
                    continue;
                }
            }
            if let Some(max_bs) = query.max_bs {
                if torus.bs > max_bs {
                    continue;
                }
            }

            let mut distance = xor_distance(&query.query_hash, id);

            // Boost connected tori (if enabled)
            if query.boost_connected {
                let connected_count = self.connected(id).len();
                if connected_count > 0 {
                    // More connections = stronger boost
                    let boost = 0.5_f64.powi((connected_count as i32).min(3));
                    distance *= boost;
                }
            }

            results.push(BarfResult::new(
                *id,
                distance,
                torus.label.clone(),
                torus.trustworthiness(),
            ));
        }

        // Sort by distance (ascending)
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        // Limit results
        results.truncate(query.max_results);

        results
    }

    /// BARF with path tracking (shows how results connect)
    pub fn barf_with_paths(&self, query: &BarfQuery) -> Vec<BarfResult> {
        let mut results = self.barf(query);

        // Get the best result's ID first (to avoid borrow issues)
        let best_id = results.first().map(|r| r.torus_id);

        // For top results, trace paths back
        for result in &mut results {
            let mut path = vec![result.torus_id];

            // Simple path: just show direct connections to query-closest torus
            if let Some(first_id) = best_id {
                if result.torus_id != first_id {
                    // Check if connected to best result
                    let connected = self.connected(&result.torus_id);
                    if connected.contains(&first_id) {
                        path.insert(0, first_id);
                    }
                }
            }

            result.path = path;
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::torus::Torus;

    fn test_foam() -> Foam {
        let mut foam = Foam::new([0u8; 32]);

        let mut t1 = Torus::new("rust_programming", 10.0, 500);
        t1.winding = 4;
        t1.bs = 0.3;

        let mut t2 = Torus::new("rust_memory", 8.0, 300);
        t2.winding = 3;
        t2.bs = 0.4;

        let mut t3 = Torus::new("python_programming", 10.0, 400);
        t3.winding = 2;
        t3.bs = 0.6;

        let id1 = t1.id;
        let id2 = t2.id;

        foam.insert(t1);
        foam.insert(t2);
        foam.insert(t3);

        // Connect rust concepts
        foam.blend(id1, id2, 0.8);

        foam
    }

    #[test]
    fn test_barf_basic() {
        let foam = test_foam();
        let query = BarfQuery::from_text("rust programming language");

        let results = foam.barf(&query);

        assert!(!results.is_empty());
        assert!(results.len() <= 10);

        // Results should be sorted by distance
        for i in 1..results.len() {
            assert!(results[i].distance >= results[i - 1].distance);
        }
    }

    #[test]
    fn test_barf_filtering() {
        let foam = test_foam();

        // Filter by winding
        let query = BarfQuery::from_text("programming").with_min_winding(4);
        let results = foam.barf(&query);

        for r in &results {
            assert!(foam.get(&r.torus_id).unwrap().winding >= 4);
        }

        // Filter by BS
        let query = BarfQuery::from_text("programming").with_max_bs(0.5);
        let results = foam.barf(&query);

        for r in &results {
            assert!(foam.get(&r.torus_id).unwrap().bs <= 0.5);
        }
    }

    #[test]
    fn test_xor_distance() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        assert_eq!(xor_distance(&a, &b), 0.0);

        let c = [0xFFu8; 32];
        assert_eq!(xor_distance(&a, &c), 1.0);
    }
}
