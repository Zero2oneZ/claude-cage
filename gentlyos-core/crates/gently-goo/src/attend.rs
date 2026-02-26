//! # Attention as Field Query
//!
//! Attention in GOO is not a separate mechanism — it IS the field.
//! Querying attention at a point is the same as evaluating the SDF
//! and ranking sources by proximity.
//!
//! ## The Key Insight
//!
//! Temperature in attention = 1/k in smooth_min.
//!
//! - Low temperature (high k): broad, soft attention — everything blends
//! - High temperature (low k): sharp, focused attention — winner-take-all
//!
//! The same parameter that makes shapes blobby in the GUI makes
//! attention soft in the query. ONE parameter, ONE behavior.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::field::GooField;
use crate::source::GooSource;

/// An attention query over the GOO field.
///
/// Focus: where to look (field coordinates)
/// Radius: how far to consider (sources outside radius are ignored)
/// Temperature: sharpness of attention (mapped to 1/k in smooth_min)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionQuery {
    /// Center of attention in field coordinates
    pub focus: Vec2,
    /// Maximum radius to consider (field units)
    pub radius: f32,
    /// Temperature: controls attention sharpness.
    /// Low temperature = sharp focus (winner-take-all)
    /// High temperature = soft/distributed attention
    /// Mapped to k via: k = 1/temperature
    pub temperature: f32,
}

impl AttentionQuery {
    /// Create a new attention query.
    pub fn new(focus: Vec2, radius: f32, temperature: f32) -> Self {
        Self {
            focus,
            radius,
            temperature: temperature.max(f32::EPSILON), // prevent division by zero
        }
    }

    /// Create a sharp (focused) attention query.
    /// High temperature → steep exponential decay → winner-take-all.
    pub fn sharp(focus: Vec2, radius: f32) -> Self {
        Self::new(focus, radius, 100.0)
    }

    /// Create a broad (distributed) attention query.
    /// Low temperature → gentle exponential decay → distributed attention.
    pub fn broad(focus: Vec2, radius: f32) -> Self {
        Self::new(focus, radius, 0.01)
    }

    /// Derive the smooth_min k parameter from temperature.
    ///
    /// k = 1/temperature. This is the dual relationship:
    /// - Visual blobbiness (k) = attention softness (1/temperature)
    pub fn derived_k(&self) -> f32 {
        1.0 / self.temperature
    }
}

/// Query attention over the field — returns source IDs ranked by relevance.
///
/// For each source within the attention radius:
/// 1. Compute SDF distance from focus point
/// 2. Subtract charge (charged sources attract more attention)
/// 3. Convert to attention weight using softmax-like formula
/// 4. Sort by weight descending
///
/// Returns pairs of (source_id, attention_weight) where weight is in [0, 1].
pub fn query_attention(
    field: &GooField,
    sources: &[GooSource],
    query: &AttentionQuery,
) -> Vec<(String, f32)> {
    if sources.is_empty() {
        return Vec::new();
    }

    let k = query.derived_k();

    // Compute raw scores for sources within radius
    let mut scored: Vec<(String, f32)> = Vec::new();

    for source in sources {
        let distance = source.sdf(query.focus);

        // Skip sources outside attention radius
        if distance > query.radius {
            continue;
        }

        // Raw score: closer = higher, charge boosts
        // Using exponential weighting with temperature
        let raw = (-distance * query.temperature).exp() * (0.5 + source.charge * 0.5);
        scored.push((source.id.clone(), raw));
    }

    if scored.is_empty() {
        return Vec::new();
    }

    // Normalize to [0, 1] (softmax-like)
    let total: f32 = scored.iter().map(|(_, s)| s).sum();
    if total > f32::EPSILON {
        for entry in &mut scored {
            entry.1 /= total;
        }
    }

    // Sort by weight descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored
}

/// Get the single most-attended source at a focus point.
///
/// Convenience function that returns the top result from query_attention.
pub fn top_attention(
    field: &GooField,
    sources: &[GooSource],
    focus: Vec2,
    radius: f32,
) -> Option<(String, f32)> {
    let query = AttentionQuery::new(focus, radius, 0.1);
    let results = query_attention(field, sources, &query);
    results.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closest_source_gets_highest_attention() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("near", Vec2::new(1.0, 0.0), 1.0, 0.5),
            GooSource::sphere("far", Vec2::new(10.0, 0.0), 1.0, 0.5),
        ];

        let query = AttentionQuery::new(Vec2::ZERO, 100.0, 0.1);
        let results = query_attention(&field, &sources, &query);

        assert!(!results.is_empty(), "Should have attention results");
        assert_eq!(results[0].0, "near", "Nearest source should have highest attention");
        assert!(
            results[0].1 > results[1].1,
            "Near ({}) should score higher than far ({})",
            results[0].1,
            results[1].1
        );
    }

    #[test]
    fn test_charge_boosts_attention() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("low_charge", Vec2::new(5.0, 0.0), 1.0, 0.1),
            GooSource::sphere("high_charge", Vec2::new(5.0, 1.0), 1.0, 1.0),
        ];

        // Both at similar distance, but high_charge has more charge
        let query = AttentionQuery::new(Vec2::ZERO, 100.0, 0.1);
        let results = query_attention(&field, &sources, &query);

        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].0, "high_charge",
            "Higher charge should win when distances are similar"
        );
    }

    #[test]
    fn test_attention_radius_filtering() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("close", Vec2::new(2.0, 0.0), 1.0, 0.5),
            GooSource::sphere("far_away", Vec2::new(100.0, 0.0), 1.0, 0.5),
        ];

        // Small radius should only catch the close source
        let query = AttentionQuery::new(Vec2::ZERO, 5.0, 0.1);
        let results = query_attention(&field, &sources, &query);

        assert_eq!(results.len(), 1, "Only close source should be in radius");
        assert_eq!(results[0].0, "close");
    }

    #[test]
    fn test_attention_empty_sources() {
        let field = GooField::new(256, 256, 32.0);
        let query = AttentionQuery::new(Vec2::ZERO, 100.0, 0.1);
        let results = query_attention(&field, &[], &query);
        assert!(results.is_empty());
    }

    #[test]
    fn test_attention_weights_sum_to_one() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("a", Vec2::new(1.0, 0.0), 1.0, 0.5),
            GooSource::sphere("b", Vec2::new(3.0, 0.0), 1.0, 0.5),
            GooSource::sphere("c", Vec2::new(5.0, 0.0), 1.0, 0.5),
        ];

        let query = AttentionQuery::new(Vec2::ZERO, 100.0, 0.1);
        let results = query_attention(&field, &sources, &query);

        let total: f32 = results.iter().map(|(_, w)| w).sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "Attention weights should sum to ~1.0, got {}",
            total
        );
    }

    #[test]
    fn test_sharp_vs_broad_attention() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("near", Vec2::new(1.0, 0.0), 1.0, 0.5),
            GooSource::sphere("mid", Vec2::new(3.0, 0.0), 1.0, 0.5),
        ];

        // Sharp attention: winner should dominate
        let sharp = AttentionQuery::sharp(Vec2::ZERO, 100.0);
        let sharp_results = query_attention(&field, &sources, &sharp);

        // Broad attention: more distributed
        let broad = AttentionQuery::broad(Vec2::ZERO, 100.0);
        let broad_results = query_attention(&field, &sources, &broad);

        // Sharp should give more weight to the nearest
        let sharp_spread = sharp_results[0].1 - sharp_results[1].1;
        let broad_spread = broad_results[0].1 - broad_results[1].1;

        assert!(
            sharp_spread > broad_spread,
            "Sharp attention should have more spread ({}) than broad ({})",
            sharp_spread,
            broad_spread
        );
    }
}
