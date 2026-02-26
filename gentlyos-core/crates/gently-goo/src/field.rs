//! # G(x, y, t, theta) — The Unified Field
//!
//! The GOO field is the single function that unifies GUI rendering,
//! attention queries, and gradient learning. At its core it evaluates
//! the smooth minimum of all SDF sources at a given point.
//!
//! ## The Math
//!
//! `smooth_min(a, b, k)` — polynomial smooth minimum:
//! ```text
//! h = max(k - |a - b|, 0) / k
//! result = min(a, b) - h * h * k * 0.25
//! ```
//!
//! The parameter `k` controls how aggressively shapes blend:
//! - k = 0 : hard min (no blending, crisp edges)
//! - k = 16 : gentle blending (subtle goo)
//! - k = 64 : extreme blending (everything merges)
//!
//! This is the dual of softmax(1/k):
//! - **Visual**: k controls SDF blobbiness
//! - **Attention**: 1/k = temperature in attention queries
//! - **Learning**: k dampens gradient magnitudes

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::source::GooSource;

/// The unified field G(x, y, t, theta).
///
/// Resolution defines the logical grid size for discretized operations
/// (rendering, gradient computation). The field itself is continuous —
/// you can sample at any (x, y) coordinate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooField {
    /// Logical resolution width (pixels when rendered)
    pub resolution_x: u32,
    /// Logical resolution height
    pub resolution_y: u32,
    /// Smooth-min softness parameter.
    /// Controls visual blobbiness, attention temperature, gradient dampening.
    /// Higher = more blending. Typical range: 8.0 - 64.0.
    pub k: f32,
    /// Current time parameter for time-varying field effects.
    pub time: f32,
}

impl GooField {
    /// Create a new GooField with given resolution and softness.
    pub fn new(resolution_x: u32, resolution_y: u32, k: f32) -> Self {
        Self {
            resolution_x,
            resolution_y,
            k,
            time: 0.0,
        }
    }

    /// Evaluate the unified field at point (x, y).
    ///
    /// Combines all source SDFs using smooth_min, then modulates by charge.
    /// Returns the blended SDF distance — negative means inside a source
    /// region, positive means outside.
    ///
    /// If no sources exist, returns f32::MAX (empty field).
    pub fn evaluate(&self, x: f32, y: f32, sources: &[GooSource]) -> f32 {
        if sources.is_empty() {
            return f32::MAX;
        }

        let p = Vec2::new(x, y);

        // Start with the first source's SDF value
        let mut combined = sources[0].sdf(p) - sources[0].charge;

        // Blend all subsequent sources using smooth_min
        for source in &sources[1..] {
            let d = source.sdf(p) - source.charge;
            combined = smooth_min(combined, d, self.k);
        }

        combined
    }

    /// Evaluate with time modulation — sources can pulse based on field time.
    ///
    /// Adds a subtle sinusoidal ripple proportional to charge and time.
    pub fn evaluate_temporal(&self, x: f32, y: f32, sources: &[GooSource]) -> f32 {
        if sources.is_empty() {
            return f32::MAX;
        }

        let p = Vec2::new(x, y);

        let mut combined = {
            let d = sources[0].sdf(p);
            let ripple = sources[0].charge * 0.1 * (self.time * 2.0).sin();
            d - sources[0].charge + ripple
        };

        for source in &sources[1..] {
            let d = source.sdf(p);
            let ripple = source.charge * 0.1 * (self.time * 2.0 + source.position.x).sin();
            let val = d - source.charge + ripple;
            combined = smooth_min(combined, val, self.k);
        }

        combined
    }

    /// Get the attention temperature derived from k.
    ///
    /// Temperature = 1/k. Lower k = higher temperature = sharper attention.
    /// Higher k = lower temperature = softer/broader attention.
    pub fn attention_temperature(&self) -> f32 {
        if self.k <= 0.0 {
            return f32::MAX;
        }
        1.0 / self.k
    }

    /// Get the gradient dampening factor derived from k.
    ///
    /// Higher k = more dampening = smoother learning.
    pub fn gradient_dampening(&self) -> f32 {
        1.0 / (1.0 + self.k * 0.01)
    }
}

/// Polynomial smooth minimum — the heart of GOO blending.
///
/// Blends two SDF values smoothly. Parameter `k` controls blend radius:
/// - k = 0: hard minimum (no blending)
/// - k > 0: smooth transition in a band of width k
///
/// ```text
/// h = max(k - |a - b|, 0) / k
/// result = min(a, b) - h * h * k * 0.25
/// ```
///
/// This is equivalent to softmax(1/k) in the attention interpretation.
pub fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// Hard minimum — no blending, for comparison.
pub fn hard_min(a: f32, b: f32) -> f32 {
    a.min(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{GooSource, SdfPrimitive};

    #[test]
    fn test_smooth_min_basics() {
        // With k=0, should behave like hard min
        assert_eq!(smooth_min(3.0, 5.0, 0.0), 3.0);
        assert_eq!(smooth_min(5.0, 3.0, 0.0), 3.0);

        // With k>0, result should be <= hard min (smooth min dips below)
        let sm = smooth_min(3.0, 5.0, 32.0);
        assert!(sm <= 3.0, "smooth_min should be <= hard min, got {}", sm);

        // When values are far apart, smooth_min approaches hard min
        let sm_far = smooth_min(0.0, 1000.0, 32.0);
        assert!((sm_far - 0.0).abs() < 0.01, "far values should approximate hard min");
    }

    #[test]
    fn test_smooth_min_symmetric() {
        let k = 16.0;
        let result_ab = smooth_min(2.0, 4.0, k);
        let result_ba = smooth_min(4.0, 2.0, k);
        assert!(
            (result_ab - result_ba).abs() < f32::EPSILON,
            "smooth_min should be symmetric"
        );
    }

    #[test]
    fn test_field_evaluate_single_sphere() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![GooSource::sphere("s1", Vec2::ZERO, 2.0, 1.0)];

        // At center of sphere with radius 2.0, charge 1.0:
        // sdf(0,0) = 0.0 - 2.0 = -2.0, then subtract charge: -2.0 - 1.0 = -3.0
        let val = field.evaluate(0.0, 0.0, &sources);
        assert!(val < 0.0, "Inside sphere should be negative, got {}", val);

        // At distance 2.0 (on the surface), sdf = 0.0, minus charge = -1.0
        let val_surface = field.evaluate(2.0, 0.0, &sources);
        assert!(val_surface < 0.0, "On surface with charge should still be negative");

        // Far away (distance 100), sdf ~ 98.0, minus charge = 97.0
        let val_far = field.evaluate(100.0, 0.0, &sources);
        assert!(val_far > 0.0, "Far from sphere should be positive");
    }

    #[test]
    fn test_field_evaluate_two_spheres_blending() {
        let field = GooField::new(256, 256, 32.0);
        let sources = vec![
            GooSource::sphere("s1", Vec2::new(-2.0, 0.0), 1.5, 1.0),
            GooSource::sphere("s2", Vec2::new(2.0, 0.0), 1.5, 1.0),
        ];

        // Midpoint between the two spheres — smooth_min should blend
        let val_mid = field.evaluate(0.0, 0.0, &sources);

        // Each sphere at midpoint: distance = 2.0, sdf = 2.0 - 1.5 = 0.5, minus charge = -0.5
        // smooth_min of (-0.5, -0.5, 32) should be <= -0.5
        assert!(
            val_mid <= -0.5,
            "Blended midpoint should be <= individual SDF, got {}",
            val_mid
        );
    }

    #[test]
    fn test_field_empty() {
        let field = GooField::new(256, 256, 32.0);
        let val = field.evaluate(0.0, 0.0, &[]);
        assert_eq!(val, f32::MAX, "Empty field should return f32::MAX");
    }

    #[test]
    fn test_attention_temperature() {
        let field = GooField::new(256, 256, 32.0);
        let temp = field.attention_temperature();
        assert!((temp - 1.0 / 32.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_gradient_dampening() {
        let field_soft = GooField::new(256, 256, 64.0);
        let field_hard = GooField::new(256, 256, 8.0);
        assert!(
            field_soft.gradient_dampening() < field_hard.gradient_dampening(),
            "Higher k should produce more dampening (lower factor)"
        );
    }
}
