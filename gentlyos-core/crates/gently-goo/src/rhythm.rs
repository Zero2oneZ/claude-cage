//! # Timing and Animation — Rhythm
//!
//! The rhythm module provides a heartbeat for the GOO field.
//! Everything in GOO pulses: sources breathe, attention oscillates,
//! and the field itself has a tempo.
//!
//! ## Parameters
//!
//! - **BPM**: beats per minute (tempo)
//! - **Phase**: current position in the beat cycle [0, 1)
//! - **Swing**: off-beat emphasis (0 = straight, 1 = full swing)
//!
//! ## Usage
//!
//! ```text
//! let mut rhythm = Rhythm::new(120.0); // 120 BPM
//! rhythm.tick(dt);                      // advance
//! let intensity = rhythm.pulse();       // 0.0 - 1.0
//! ```

use serde::{Deserialize, Serialize};

/// Rhythm controller — heartbeat of the GOO field.
///
/// Provides a continuous pulse signal based on BPM.
/// The pulse is a smooth sine wave, optionally swung.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rhythm {
    /// Beats per minute
    pub bpm: f32,
    /// Current phase in the beat cycle [0.0, 1.0)
    pub phase: f32,
    /// Swing amount [0.0, 1.0]: how much to skew the off-beat
    pub swing: f32,
    /// Total elapsed beats (monotonically increasing)
    pub total_beats: f32,
}

impl Rhythm {
    /// Create a new rhythm at the given BPM.
    pub fn new(bpm: f32) -> Self {
        Self {
            bpm: bpm.max(1.0),
            phase: 0.0,
            swing: 0.0,
            total_beats: 0.0,
        }
    }

    /// Create a rhythm with swing.
    pub fn with_swing(bpm: f32, swing: f32) -> Self {
        Self {
            bpm: bpm.max(1.0),
            phase: 0.0,
            swing: swing.clamp(0.0, 1.0),
            total_beats: 0.0,
        }
    }

    /// Advance the rhythm by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        let beats_per_second = self.bpm / 60.0;
        let beat_delta = dt * beats_per_second;

        self.total_beats += beat_delta;
        self.phase = (self.phase + beat_delta) % 1.0;
    }

    /// Get the current pulse value [0.0, 1.0].
    ///
    /// Uses a smooth sine curve:
    /// - 0.0 at phase = 0.0 (beat start)
    /// - 1.0 at phase = 0.25 (beat peak)
    /// - 0.0 at phase = 0.5 (beat trough)
    ///
    /// With swing > 0, the peak shifts later in the cycle.
    pub fn pulse(&self) -> f32 {
        let adjusted_phase = if self.swing > 0.0 {
            // Swing shifts the midpoint of the cycle
            let swing_offset = self.swing * 0.25;
            if self.phase < 0.5 {
                // First half: stretched
                self.phase / (0.5 + swing_offset) * 0.5
            } else {
                // Second half: compressed
                0.5 + (self.phase - 0.5) / (0.5 - swing_offset) * 0.5
            }
        } else {
            self.phase
        };

        // Smooth sine pulse: sin^2 for always-positive output
        let angle = adjusted_phase * std::f32::consts::TAU;
        let raw = angle.sin();
        raw * raw // sin^2 gives smooth 0-1-0 pulse
    }

    /// Whether the beat is currently "active" (pulse > 0.5).
    pub fn beat_active(&self) -> bool {
        self.pulse() > 0.5
    }

    /// Get the current beat number (integer part of total_beats).
    pub fn beat_number(&self) -> u64 {
        self.total_beats as u64
    }

    /// Reset the rhythm to beat 0.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.total_beats = 0.0;
    }

    /// Change BPM while preserving phase.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.max(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rhythm_pulse_range() {
        let mut rhythm = Rhythm::new(120.0);

        // Sample pulse at many points
        for i in 0..100 {
            rhythm.tick(0.01);
            let p = rhythm.pulse();
            assert!(p >= 0.0 && p <= 1.0, "Pulse out of range: {}", p);
        }
    }

    #[test]
    fn test_rhythm_beat_active() {
        let mut rhythm = Rhythm::new(60.0); // 1 beat per second

        // Tick forward a quarter second (should be near peak)
        rhythm.tick(0.25);
        // Phase ~ 0.25, sin^2(0.25 * TAU) = sin^2(PI/2) = 1.0
        assert!(rhythm.beat_active(), "Should be active at peak");
    }

    #[test]
    fn test_rhythm_total_beats() {
        let mut rhythm = Rhythm::new(60.0);
        rhythm.tick(1.0); // 1 second at 60 BPM = 1 beat
        assert_eq!(rhythm.beat_number(), 1);

        rhythm.tick(1.0);
        assert_eq!(rhythm.beat_number(), 2);
    }

    #[test]
    fn test_rhythm_reset() {
        let mut rhythm = Rhythm::new(120.0);
        rhythm.tick(5.0);
        rhythm.reset();
        assert_eq!(rhythm.phase, 0.0);
        assert_eq!(rhythm.total_beats, 0.0);
    }
}
