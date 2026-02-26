//! # Claude Embodiment in GOO
//!
//! Claude exists as an avatar in the GOO field — a pulsing sphere
//! that responds to mood, conversation state, and rhythm.
//!
//! ## Mood System
//!
//! | Mood | Visual | Charge | Animation |
//! |------|--------|--------|-----------|
//! | Neutral | Steady glow | 0.5 | Slow pulse |
//! | Thinking | Expanding rings | 0.3-0.7 | Breathing |
//! | Speaking | Bright pulse | 0.7-1.0 | Beat-synced |
//! | Listening | Soft glow | 0.4-0.6 | Subtle ripple |
//! | Alert | Sharp flash | 0.8-1.0 | Fast strobe |
//!
//! ## Integration
//!
//! The ClaudeAvatar registers as a GooSource in the field. Its position,
//! charge, and color change based on mood. Attention queries near Claude's
//! position reveal its current state.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::source::{GooSource, SdfPrimitive};

/// Claude's mood state — determines visual presentation in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    /// Default: steady, calm presence
    Neutral,
    /// Processing: expanding/contracting, variable charge
    Thinking,
    /// Responding: bright, beat-synced pulses
    Speaking,
    /// Waiting for input: soft, receptive glow
    Listening,
    /// Something needs attention: sharp, high-charge flashes
    Alert,
}

impl Mood {
    /// Base charge for this mood (before animation modulation).
    pub fn base_charge(&self) -> f32 {
        match self {
            Mood::Neutral => 0.5,
            Mood::Thinking => 0.5,
            Mood::Speaking => 0.85,
            Mood::Listening => 0.5,
            Mood::Alert => 0.9,
        }
    }

    /// Color for this mood (RGBA).
    pub fn color(&self) -> [f32; 4] {
        match self {
            Mood::Neutral => [0.6, 0.7, 1.0, 1.0],   // soft blue
            Mood::Thinking => [0.4, 0.5, 0.9, 1.0],   // deeper blue
            Mood::Speaking => [0.8, 0.85, 1.0, 1.0],   // bright blue-white
            Mood::Listening => [0.5, 0.65, 0.9, 1.0],  // muted blue
            Mood::Alert => [1.0, 0.6, 0.3, 1.0],       // warm orange
        }
    }

    /// Animation speed multiplier for this mood.
    pub fn animation_speed(&self) -> f32 {
        match self {
            Mood::Neutral => 1.0,
            Mood::Thinking => 0.5,   // slow, deliberate
            Mood::Speaking => 2.0,   // fast, energetic
            Mood::Listening => 0.7,  // calm, attentive
            Mood::Alert => 4.0,      // rapid
        }
    }

    /// SDF radius for this mood.
    pub fn radius(&self) -> f32 {
        match self {
            Mood::Neutral => 1.5,
            Mood::Thinking => 2.0,   // larger, contemplative
            Mood::Speaking => 1.8,   // slightly expanded
            Mood::Listening => 1.3,  // slightly contracted, focused
            Mood::Alert => 2.5,      // large, attention-grabbing
        }
    }
}

/// Claude's avatar in the GOO field.
///
/// Maintains position, mood, and connection to a GooSource.
/// The avatar updates its source representation each frame based
/// on mood and animation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAvatar {
    /// Position in field coordinates
    pub position: Vec2,
    /// Current mood
    pub mood: Mood,
    /// ID of the GooSource representing Claude in the field
    pub goo_source_id: String,
    /// Animation phase (0.0 - 1.0)
    animation_phase: f32,
}

impl ClaudeAvatar {
    /// Create a new Claude avatar at the given position.
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            mood: Mood::Neutral,
            goo_source_id: "claude-avatar".to_string(),
            animation_phase: 0.0,
        }
    }

    /// Create with a custom source ID.
    pub fn with_id(position: Vec2, id: impl Into<String>) -> Self {
        Self {
            position,
            mood: Mood::Neutral,
            goo_source_id: id.into(),
            animation_phase: 0.0,
        }
    }

    /// Update Claude's mood. Changes the visual representation.
    pub fn update_mood(&mut self, mood: Mood) {
        self.mood = mood;
    }

    /// Convert the current avatar state to a GooSource.
    ///
    /// This creates a sphere source at Claude's position with
    /// mood-dependent radius, charge, and color.
    pub fn to_goo_source(&self) -> GooSource {
        GooSource {
            id: self.goo_source_id.clone(),
            position: self.position,
            primitive: SdfPrimitive::Sphere {
                radius: self.mood.radius(),
            },
            charge: self.mood.base_charge(),
            color: self.mood.color(),
        }
    }

    /// Compute the thinking animation value at a given time.
    ///
    /// Returns a modulation factor [0.0, 1.0] that can be applied to
    /// charge, radius, or other visual parameters.
    ///
    /// The animation is mood-dependent:
    /// - Neutral: slow sine pulse
    /// - Thinking: breathing pattern (slow in, fast out)
    /// - Speaking: sharp beat-synced pulses
    /// - Listening: subtle ripple
    /// - Alert: rapid strobing
    pub fn thinking_animation(&self, time: f32) -> f32 {
        let speed = self.mood.animation_speed();
        let t = time * speed;

        match self.mood {
            Mood::Neutral => {
                // Gentle sine pulse
                (t * std::f32::consts::TAU * 0.5).sin() * 0.5 + 0.5
            }
            Mood::Thinking => {
                // Breathing: smooth ease-in, quick ease-out
                let phase = (t * 0.5) % 1.0;
                if phase < 0.6 {
                    // Inhale (slow)
                    let p = phase / 0.6;
                    p * p // quadratic ease-in
                } else {
                    // Exhale (fast)
                    let p = (phase - 0.6) / 0.4;
                    1.0 - p * p // quadratic ease-out
                }
            }
            Mood::Speaking => {
                // Beat-synced: sharp peaks
                let phase = t % 1.0;
                let raw = (-phase * 8.0).exp(); // exponential decay
                raw.clamp(0.0, 1.0)
            }
            Mood::Listening => {
                // Subtle ripple: multiple overlapping sine waves
                let a = (t * std::f32::consts::TAU * 0.3).sin();
                let b = (t * std::f32::consts::TAU * 0.7).sin();
                (a * 0.3 + b * 0.2 + 0.5).clamp(0.0, 1.0)
            }
            Mood::Alert => {
                // Rapid strobe
                let phase = t % 1.0;
                if phase < 0.1 { 1.0 } else { 0.2 }
            }
        }
    }

    /// Get the animated charge value at the current time.
    pub fn animated_charge(&self, time: f32) -> f32 {
        let base = self.mood.base_charge();
        let anim = self.thinking_animation(time);
        let range = match self.mood {
            Mood::Neutral => 0.1,
            Mood::Thinking => 0.4,
            Mood::Speaking => 0.3,
            Mood::Listening => 0.2,
            Mood::Alert => 0.2,
        };
        (base + (anim - 0.5) * range).clamp(0.0, 1.0)
    }

    /// Move Claude's avatar to a new position.
    pub fn move_to(&mut self, position: Vec2) {
        self.position = position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mood_properties() {
        for mood in [Mood::Neutral, Mood::Thinking, Mood::Speaking, Mood::Listening, Mood::Alert] {
            let charge = mood.base_charge();
            assert!(charge >= 0.0 && charge <= 1.0, "Charge out of range for {:?}: {}", mood, charge);

            let color = mood.color();
            for c in color {
                assert!(c >= 0.0 && c <= 1.0, "Color component out of range for {:?}", mood);
            }

            assert!(mood.animation_speed() > 0.0);
            assert!(mood.radius() > 0.0);
        }
    }

    #[test]
    fn test_avatar_creation() {
        let avatar = ClaudeAvatar::new(Vec2::new(5.0, 5.0));
        assert_eq!(avatar.mood, Mood::Neutral);
        assert_eq!(avatar.goo_source_id, "claude-avatar");
    }

    #[test]
    fn test_avatar_to_source() {
        let avatar = ClaudeAvatar::new(Vec2::new(3.0, 4.0));
        let source = avatar.to_goo_source();

        assert_eq!(source.id, "claude-avatar");
        assert_eq!(source.position, Vec2::new(3.0, 4.0));
        assert_eq!(source.charge, Mood::Neutral.base_charge());
    }

    #[test]
    fn test_mood_update() {
        let mut avatar = ClaudeAvatar::new(Vec2::ZERO);
        avatar.update_mood(Mood::Thinking);
        assert_eq!(avatar.mood, Mood::Thinking);

        let source = avatar.to_goo_source();
        assert_eq!(source.charge, Mood::Thinking.base_charge());
    }

    #[test]
    fn test_thinking_animation_range() {
        let avatar = ClaudeAvatar::new(Vec2::ZERO);

        // Sample animation at many time points
        for i in 0..100 {
            let t = i as f32 * 0.1;
            let val = avatar.thinking_animation(t);
            assert!(
                val >= 0.0 && val <= 1.0,
                "Animation out of range at t={}: {}",
                t,
                val
            );
        }
    }

    #[test]
    fn test_animated_charge_range() {
        for mood in [Mood::Neutral, Mood::Thinking, Mood::Speaking, Mood::Listening, Mood::Alert] {
            let mut avatar = ClaudeAvatar::new(Vec2::ZERO);
            avatar.update_mood(mood);

            for i in 0..100 {
                let t = i as f32 * 0.1;
                let charge = avatar.animated_charge(t);
                assert!(
                    charge >= 0.0 && charge <= 1.0,
                    "Animated charge out of range for {:?} at t={}: {}",
                    mood,
                    t,
                    charge
                );
            }
        }
    }

    #[test]
    fn test_move_to() {
        let mut avatar = ClaudeAvatar::new(Vec2::ZERO);
        avatar.move_to(Vec2::new(10.0, 20.0));
        assert_eq!(avatar.position, Vec2::new(10.0, 20.0));
    }
}
