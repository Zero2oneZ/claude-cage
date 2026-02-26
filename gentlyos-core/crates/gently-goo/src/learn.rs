//! # Gradient Learning in Field Space
//!
//! Learning in GOO is gradient descent over the SDF field. Each source
//! can be "pulled" toward a target position by computing the direction
//! and magnitude of the gradient.
//!
//! ## The Connection
//!
//! The field's k parameter (smooth_min softness) also controls learning:
//! - High k = smooth gradients = stable but slow learning
//! - Low k = sharp gradients = fast but potentially unstable
//!
//! This is the same parameter that controls visual blobbiness and
//! attention softness. ONE parameter, THREE behaviors.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::field::GooField;
use crate::source::GooSource;

/// A single gradient step for one source.
///
/// Describes the direction and magnitude a source should move
/// to reduce the field's distance to a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStep {
    /// ID of the source this step applies to
    pub source_id: String,
    /// Direction to move (normalized)
    pub direction: Vec2,
    /// Step magnitude — how far to move (before learning rate)
    pub magnitude: f32,
}

/// Compute gradient steps for all sources toward a target position.
///
/// For each source:
/// 1. Compute vector from source position to target
/// 2. Normalize to get direction
/// 3. Scale magnitude by source charge (more charged = more responsive)
/// 4. Apply field dampening (derived from k)
///
/// Returns a GradientStep for each source. Apply these with a learning rate
/// to move sources toward the target.
pub fn compute_gradient(
    field: &GooField,
    sources: &[GooSource],
    target: Vec2,
) -> Vec<GradientStep> {
    let dampening = field.gradient_dampening();
    let mut steps = Vec::with_capacity(sources.len());

    for source in sources {
        let to_target = target - source.position;
        let distance = to_target.length();

        if distance < f32::EPSILON {
            // Already at target — zero step
            steps.push(GradientStep {
                source_id: source.id.clone(),
                direction: Vec2::ZERO,
                magnitude: 0.0,
            });
            continue;
        }

        let direction = to_target / distance; // normalize
        let magnitude = source.charge * dampening * distance;

        steps.push(GradientStep {
            source_id: source.id.clone(),
            direction,
            magnitude,
        });
    }

    steps
}

/// Compute gradient for a single source toward a target.
///
/// Convenience function when you only need to move one source.
pub fn compute_single_gradient(
    field: &GooField,
    source: &GooSource,
    target: Vec2,
) -> GradientStep {
    let dampening = field.gradient_dampening();
    let to_target = target - source.position;
    let distance = to_target.length();

    if distance < f32::EPSILON {
        return GradientStep {
            source_id: source.id.clone(),
            direction: Vec2::ZERO,
            magnitude: 0.0,
        };
    }

    GradientStep {
        source_id: source.id.clone(),
        direction: to_target / distance,
        magnitude: source.charge * dampening * distance,
    }
}

/// Apply a list of gradient steps to sources with a given learning rate.
///
/// Modifies source positions in-place. Returns the total displacement.
pub fn apply_steps(sources: &mut [GooSource], steps: &[GradientStep], learning_rate: f32) -> f32 {
    let mut total_displacement = 0.0;

    for step in steps {
        if let Some(source) = sources.iter_mut().find(|s| s.id == step.source_id) {
            let displacement = step.direction * step.magnitude * learning_rate;
            source.position += displacement;
            total_displacement += displacement.length();
        }
    }

    total_displacement
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_toward_target() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![GooSource::sphere("s1", Vec2::ZERO, 1.0, 1.0)];
        let target = Vec2::new(10.0, 0.0);

        let steps = compute_gradient(&field, &sources, target);
        assert_eq!(steps.len(), 1);

        // Direction should point toward target (positive x)
        assert!(steps[0].direction.x > 0.0, "Should point toward target");
        assert!(steps[0].magnitude > 0.0, "Should have positive magnitude");
    }

    #[test]
    fn test_gradient_at_target() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![GooSource::sphere("s1", Vec2::new(5.0, 5.0), 1.0, 1.0)];
        let target = Vec2::new(5.0, 5.0);

        let steps = compute_gradient(&field, &sources, target);
        assert_eq!(steps[0].magnitude, 0.0, "At target, magnitude should be 0");
    }

    #[test]
    fn test_charge_affects_magnitude() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("low", Vec2::ZERO, 1.0, 0.1),
            GooSource::sphere("high", Vec2::ZERO, 1.0, 1.0),
        ];
        let target = Vec2::new(10.0, 0.0);

        let steps = compute_gradient(&field, &sources, target);
        assert!(
            steps[1].magnitude > steps[0].magnitude,
            "Higher charge should produce larger magnitude"
        );
    }

    #[test]
    fn test_apply_steps() {
        let field = GooField::new(256, 256, 32.0);
        let mut sources = vec![GooSource::sphere("s1", Vec2::ZERO, 1.0, 1.0)];
        let target = Vec2::new(10.0, 0.0);

        let steps = compute_gradient(&field, &sources, target);
        let displacement = apply_steps(&mut sources, &steps, 0.1);

        assert!(displacement > 0.0, "Should have moved");
        assert!(sources[0].position.x > 0.0, "Should have moved toward target");
    }
}
