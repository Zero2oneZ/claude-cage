//! # Template / Scoring System
//!
//! Score templates evaluate GOO sources against named criteria.
//! Each template defines weighted dimensions and a threshold.
//!
//! ## Built-in Templates
//!
//! | Template | Weights | Threshold | Use |
//! |----------|---------|-----------|-----|
//! | health   | charge: 0.6, age: 0.4 | 0.5 | Source vitality |
//! | activity | interactions: 0.5, charge: 0.3, recency: 0.2 | 0.3 | Recent engagement |
//!
//! ## Integration with Inference Mining
//!
//! Score templates map to gently-inference quality scoring:
//! - Template threshold = inference USEFUL threshold (0.7)
//! - Template weights = inference step type multipliers
//! - Template evaluation = inference quality formula

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::source::GooSource;

/// A scoring template that evaluates sources against weighted criteria.
///
/// Templates are reusable: define once, apply to any set of sources.
/// The threshold determines the pass/fail boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreTemplate {
    /// Template name (e.g., "health", "activity")
    pub name: String,
    /// Weighted dimensions: dimension_name -> weight (should sum to 1.0)
    pub weights: HashMap<String, f32>,
    /// Threshold for "passing" (0.0 - 1.0)
    pub threshold: f32,
}

/// Result of scoring a single source against a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResult {
    /// Source ID
    pub source_id: String,
    /// Final score (0.0 - 1.0)
    pub score: f32,
    /// Per-dimension scores
    pub dimensions: HashMap<String, f32>,
    /// Whether the score meets the template threshold
    pub passes: bool,
}

/// Result of scoring all sources against a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScoreResult {
    /// Template used
    pub template_name: String,
    /// Per-source results
    pub results: Vec<ScoreResult>,
    /// Aggregate score (average of all source scores)
    pub aggregate: f32,
    /// Number of sources that pass the threshold
    pub passing_count: usize,
}

impl ScoreTemplate {
    /// Create a new template with given name, weights, and threshold.
    pub fn new(
        name: impl Into<String>,
        weights: HashMap<String, f32>,
        threshold: f32,
    ) -> Self {
        Self {
            name: name.into(),
            weights,
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Built-in "health" template: evaluates source vitality.
    ///
    /// Dimensions:
    /// - charge (0.6): current charge level
    /// - radius (0.4): size/presence in field (larger = healthier)
    pub fn health() -> Self {
        let mut weights = HashMap::new();
        weights.insert("charge".to_string(), 0.6);
        weights.insert("radius".to_string(), 0.4);
        Self::new("health", weights, 0.5)
    }

    /// Built-in "activity" template: evaluates recent interaction.
    ///
    /// Dimensions:
    /// - charge (0.5): recently boosted sources have higher charge
    /// - position_change (0.3): sources that moved recently score higher
    /// - color_intensity (0.2): brighter = more active
    pub fn activity() -> Self {
        let mut weights = HashMap::new();
        weights.insert("charge".to_string(), 0.5);
        weights.insert("position_change".to_string(), 0.3);
        weights.insert("color_intensity".to_string(), 0.2);
        Self::new("activity", weights, 0.3)
    }

    /// Built-in "focus" template: evaluates attention-worthiness.
    ///
    /// Maps to the inference quality formula:
    /// quality = charge * 0.3 + radius * 0.4 + color_intensity * 0.2 + position * 0.1
    pub fn focus() -> Self {
        let mut weights = HashMap::new();
        weights.insert("charge".to_string(), 0.3);
        weights.insert("radius".to_string(), 0.4);
        weights.insert("color_intensity".to_string(), 0.2);
        weights.insert("position_centrality".to_string(), 0.1);
        Self::new("focus", weights, 0.7) // Same as inference USEFUL threshold
    }
}

/// Score a list of sources against a template.
///
/// Each source is evaluated on the template's dimensions:
/// - "charge" -> source.charge
/// - "radius" -> extracted from SDF primitive (normalized to [0, 1])
/// - "color_intensity" -> average of RGB channels
/// - "position_centrality" -> 1.0 / (1.0 + distance_from_origin)
/// - "position_change" -> always 0.0 (requires historical data)
///
/// Returns the aggregate score (weighted average across all sources).
pub fn score_sources(sources: &[GooSource], template: &ScoreTemplate) -> f32 {
    if sources.is_empty() {
        return 0.0;
    }

    let results = score_sources_detailed(sources, template);
    results.aggregate
}

/// Score sources with full detail.
pub fn score_sources_detailed(sources: &[GooSource], template: &ScoreTemplate) -> BatchScoreResult {
    let mut results = Vec::with_capacity(sources.len());

    for source in sources {
        let dims = extract_dimensions(source);
        let mut score = 0.0;

        for (dim_name, weight) in &template.weights {
            let dim_value = dims.get(dim_name.as_str()).copied().unwrap_or(0.0);
            score += dim_value * weight;
        }

        let score = score.clamp(0.0, 1.0);
        let passes = score >= template.threshold;

        results.push(ScoreResult {
            source_id: source.id.clone(),
            score,
            dimensions: dims
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            passes,
        });
    }

    let aggregate = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|r| r.score).sum::<f32>() / results.len() as f32
    };

    let passing_count = results.iter().filter(|r| r.passes).count();

    BatchScoreResult {
        template_name: template.name.clone(),
        results,
        aggregate,
        passing_count,
    }
}

/// Extract dimension values from a GooSource.
///
/// Returns a map of dimension_name -> value (all in [0, 1]).
fn extract_dimensions(source: &GooSource) -> HashMap<&str, f32> {
    let mut dims = HashMap::new();

    // Charge: directly from source
    dims.insert("charge", source.charge);

    // Radius: normalized — assume "typical" radius is 10.0
    let radius = match &source.primitive {
        crate::source::SdfPrimitive::Sphere { radius } => *radius,
        crate::source::SdfPrimitive::Box { half_extents } => half_extents.length(),
        crate::source::SdfPrimitive::Torus { major, minor } => major + minor,
        crate::source::SdfPrimitive::Line { start, end } => (*end - *start).length() * 0.5,
    };
    dims.insert("radius", (radius / 10.0).clamp(0.0, 1.0));

    // Color intensity: average of RGB
    let intensity = (source.color[0] + source.color[1] + source.color[2]) / 3.0;
    dims.insert("color_intensity", intensity.clamp(0.0, 1.0));

    // Position centrality: closer to origin = higher
    let dist = source.position.length();
    dims.insert("position_centrality", 1.0 / (1.0 + dist * 0.1));

    // Position change: requires history, always 0 for now
    dims.insert("position_change", 0.0);

    dims
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn test_health_template() {
        let template = ScoreTemplate::health();
        let sources = vec![
            GooSource::sphere("healthy", Vec2::ZERO, 5.0, 1.0),
            GooSource::sphere("weak", Vec2::ZERO, 0.5, 0.1),
        ];

        let result = score_sources_detailed(&sources, &template);
        assert_eq!(result.results.len(), 2);

        // Healthy source should pass
        assert!(result.results[0].passes, "Healthy source should pass");
        // Weak source should have lower score
        assert!(
            result.results[0].score > result.results[1].score,
            "Healthy should score higher than weak"
        );
    }

    #[test]
    fn test_score_empty() {
        let template = ScoreTemplate::health();
        let score = score_sources(&[], &template);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_focus_template_threshold() {
        let template = ScoreTemplate::focus();
        assert_eq!(template.threshold, 0.7, "Focus threshold should match inference USEFUL");
    }

    #[test]
    fn test_aggregate_score() {
        let template = ScoreTemplate::health();
        let sources = vec![
            GooSource::sphere("a", Vec2::ZERO, 5.0, 0.8),
            GooSource::sphere("b", Vec2::ZERO, 5.0, 0.8),
        ];

        let result = score_sources_detailed(&sources, &template);
        let manual_avg = result.results.iter().map(|r| r.score).sum::<f32>() / 2.0;
        assert!(
            (result.aggregate - manual_avg).abs() < f32::EPSILON,
            "Aggregate should be average of scores"
        );
    }
}
