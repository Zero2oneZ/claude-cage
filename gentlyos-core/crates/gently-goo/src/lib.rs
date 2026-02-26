#![allow(dead_code, unused_variables, unused_imports)]
//! # gently-goo — GOO Unified Field Dashboard
//!
//! GOO = GUI + Attention + Learning in ONE object.
//!
//! The central insight: `smooth_min(k)` is the dual of `softmax(1/k)`.
//! The same parameter `k` controls:
//! - **Visual blobbiness** — SDF blending between shapes
//! - **Attention softness** — query temperature over sources
//! - **Learning smoothness** — gradient dampening across the field
//!
//! Everything is G(x, y, t, theta). One function renders the GUI, routes
//! attention, and drives learning. No separation between "display" and
//! "computation" — the field IS the interface.
//!
//! ## Architecture
//!
//! ```text
//! GooEngine
//! ├── GooField          # G(x,y,t,theta) — the unified field
//! ├── Vec<GooSource>    # SDF primitives that populate the field
//! ├── RenderConfig      # Pixel output from field samples
//! ├── AttentionQuery    # Field queries = attention mechanism
//! ├── GradientStep      # Learning = gradient descent in field space
//! ├── ScoreTemplate     # Template scoring over sources
//! ├── CascadeChain      # ML model integration pipeline
//! ├── Rhythm            # Timing / animation heartbeat
//! ├── Specialist        # Agent routing in field coordinates
//! ├── SovereigntyGuard  # Boundary / consent protection
//! └── ClaudeAvatar      # Claude's embodiment in GOO
//! ```

pub mod field;
pub mod source;
pub mod render;
pub mod attend;
pub mod learn;
pub mod score;
pub mod cascade;
pub mod rhythm;
pub mod specialist;
pub mod sense;
pub mod claude;

// Re-export key types for ergonomic usage
pub use field::GooField;
pub use source::{GooSource, SdfPrimitive};
pub use render::RenderConfig;
pub use attend::AttentionQuery;
pub use learn::GradientStep;
pub use score::ScoreTemplate;
pub use cascade::{CascadeModel, CascadeChain, IdentityModel};
pub use rhythm::Rhythm;
pub use specialist::Specialist;
pub use sense::{SovereigntyGuard, SenseEvent, BoundaryPolicy, BoundaryResult};
pub use claude::{ClaudeAvatar, Mood};

use glam::Vec2;

/// The GOO Engine — single entry point for the unified field system.
///
/// Everything flows through here: rendering, attention, learning, animation.
/// The engine holds the field, the sources that populate it, and the current
/// time step. Each `tick()` advances the entire system.
pub struct GooEngine {
    /// The unified field G(x,y,t,theta)
    pub field: GooField,
    /// All SDF sources populating the field
    pub sources: Vec<GooSource>,
    /// Current simulation time (seconds)
    pub time: f32,
    /// Render configuration
    pub render_config: RenderConfig,
    /// Animation rhythm
    pub rhythm: Rhythm,
    /// Sovereignty guard for boundary protection
    pub sovereignty: SovereigntyGuard,
    /// Claude's avatar in the field
    pub claude_avatar: Option<ClaudeAvatar>,
}

impl GooEngine {
    /// Create a new GOO engine with default parameters.
    ///
    /// Field resolution defaults to 256x256, k=32.0 for smooth blending.
    /// Rhythm defaults to 60 BPM (one pulse per second).
    pub fn new() -> Self {
        Self {
            field: GooField::new(256, 256, 32.0),
            sources: Vec::new(),
            time: 0.0,
            render_config: RenderConfig::default(),
            rhythm: Rhythm::new(60.0),
            sovereignty: SovereigntyGuard::new(BoundaryPolicy::default()),
            claude_avatar: None,
        }
    }

    /// Advance the engine by `dt` seconds.
    ///
    /// Updates:
    /// 1. Global time
    /// 2. Field time parameter
    /// 3. Rhythm phase
    /// 4. Source charges (decay toward 0 if not refreshed)
    /// 5. Claude avatar animation (if present)
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        self.field.time = self.time;
        self.rhythm.tick(dt);

        // Gentle charge decay on all sources — keeps the field alive
        let decay_rate = 0.01 * dt;
        for source in &mut self.sources {
            source.charge = (source.charge - decay_rate).max(0.0);
        }

        // Update Claude avatar if present
        if let Some(ref mut avatar) = self.claude_avatar {
            let pulse = self.rhythm.pulse();
            // Claude's source charge follows the rhythm
            if let Some(src) = self.sources.iter_mut().find(|s| s.id == avatar.goo_source_id) {
                src.charge = 0.5 + 0.3 * pulse;
            }
        }
    }

    /// Sample the unified field at a point.
    ///
    /// Returns the combined SDF value at (x, y) — negative means inside
    /// a source, positive means outside. The smooth_min blending creates
    /// the characteristic "goo" effect where nearby sources merge.
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        self.field.evaluate(x, y, &self.sources)
    }

    /// Add an SDF source to the field.
    ///
    /// The source immediately begins contributing to G(x,y,t,theta).
    /// Its charge determines visual intensity and attention weight.
    pub fn add_source(&mut self, source: GooSource) {
        self.sources.push(source);
    }

    /// Remove an SDF source by ID.
    ///
    /// Returns true if a source was found and removed.
    pub fn remove_source(&mut self, id: &str) -> bool {
        let before = self.sources.len();
        self.sources.retain(|s| s.id != id);
        self.sources.len() < before
    }

    /// Render the current field state to a pixel buffer.
    ///
    /// Returns RGBA f32 values for each pixel in row-major order.
    pub fn render(&self) -> Vec<[f32; 4]> {
        render::render_field(&self.field, &self.sources, &self.render_config)
    }

    /// Query attention over the field — returns source IDs ranked by
    /// proximity to the focus point.
    pub fn query_attention(&self, query: &AttentionQuery) -> Vec<(String, f32)> {
        attend::query_attention(&self.field, &self.sources, query)
    }

    /// Compute gradient steps for all sources toward a target position.
    pub fn compute_gradient(&self, target: Vec2) -> Vec<GradientStep> {
        learn::compute_gradient(&self.field, &self.sources, target)
    }

    /// Apply computed gradient steps to move sources.
    pub fn apply_gradient(&mut self, steps: &[GradientStep], learning_rate: f32) {
        for step in steps {
            if let Some(source) = self.sources.iter_mut().find(|s| s.id == step.source_id) {
                source.position += step.direction * step.magnitude * learning_rate;
            }
        }
    }

    /// Embed Claude as an avatar in the field.
    pub fn embed_claude(&mut self, position: Vec2) {
        let avatar = ClaudeAvatar::new(position);
        let goo_source = avatar.to_goo_source();
        self.sources.push(goo_source);
        self.claude_avatar = Some(avatar);
    }

    /// Get the number of active sources (charge > 0).
    pub fn active_source_count(&self) -> usize {
        self.sources.iter().filter(|s| s.charge > 0.0).count()
    }

    /// Get total field energy — sum of all source charges.
    pub fn total_energy(&self) -> f32 {
        self.sources.iter().map(|s| s.charge).sum()
    }
}

impl Default for GooEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_lifecycle() {
        let mut engine = GooEngine::new();
        assert_eq!(engine.sources.len(), 0);
        assert_eq!(engine.time, 0.0);

        let source = GooSource::sphere("test", Vec2::ZERO, 1.0, 1.0);
        engine.add_source(source);
        assert_eq!(engine.sources.len(), 1);

        engine.tick(0.016); // ~60fps frame
        assert!(engine.time > 0.0);

        let removed = engine.remove_source("test");
        assert!(removed);
        assert_eq!(engine.sources.len(), 0);

        let not_found = engine.remove_source("nonexistent");
        assert!(!not_found);
    }

    #[test]
    fn test_engine_sampling() {
        let mut engine = GooEngine::new();
        engine.add_source(GooSource::sphere("s1", Vec2::ZERO, 2.0, 1.0));

        // Inside the sphere — should be negative
        let val = engine.sample(0.0, 0.0);
        assert!(val < 0.0, "Center of sphere should be negative SDF, got {}", val);

        // Far outside — should be positive
        let val = engine.sample(100.0, 100.0);
        assert!(val > 0.0, "Far from sphere should be positive SDF, got {}", val);
    }

    #[test]
    fn test_embed_claude() {
        let mut engine = GooEngine::new();
        engine.embed_claude(Vec2::new(5.0, 5.0));
        assert!(engine.claude_avatar.is_some());
        assert_eq!(engine.sources.len(), 1);
        assert_eq!(engine.sources[0].id, "claude-avatar");
    }
}
