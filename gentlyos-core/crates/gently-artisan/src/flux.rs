//! FluxLine - Traversal paths through major/minor loops
//!
//! Step 1.5 from BUILD_STEPS.md

use crate::coord::TorusCoordinate;
use crate::torus::Torus;
use crate::tokens_to_radius;
use serde::{Deserialize, Serialize};

/// A FluxLine represents active traversal/accumulation on a torus
///
/// When tokens accumulate past a threshold, the flux "breaks"
/// into a new torus (concept specialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluxLine {
    /// Origin torus where this flux started
    pub origin_torus: [u8; 32],

    /// Current accumulated length (in tokens)
    pub current_length: u64,

    /// Threshold for breaking into new torus
    pub threshold: u64,

    /// Direction of flux on the torus
    pub direction: TorusCoordinate,

    /// Label prefix for new tori when breaking
    pub label_prefix: String,

    /// Whether this flux is still active
    pub active: bool,
}

impl FluxLine {
    /// Create a new flux line from an origin torus
    pub fn new(origin: [u8; 32], threshold: u64, label_prefix: &str) -> Self {
        Self {
            origin_torus: origin,
            current_length: 0,
            threshold,
            direction: TorusCoordinate::default(),
            label_prefix: label_prefix.to_string(),
            active: true,
        }
    }

    /// Create with specific direction
    pub fn with_direction(mut self, direction: TorusCoordinate) -> Self {
        self.direction = direction;
        self
    }

    /// Accumulate tokens into this flux
    pub fn accumulate(&mut self, tokens: u64) {
        if self.active {
            self.current_length += tokens;
        }
    }

    /// Check if flux should break into new torus
    pub fn should_break(&self) -> bool {
        self.active && self.current_length >= self.threshold
    }

    /// Get the current radius this flux represents
    pub fn to_radius(&self) -> f64 {
        tokens_to_radius(self.current_length)
    }

    /// Break flux into a new torus
    ///
    /// Returns the new torus and resets the flux
    pub fn break_into_torus(&mut self, suffix: &str, major_radius: f64) -> Option<Torus> {
        if !self.should_break() {
            return None;
        }

        let label = format!("{}_{}", self.label_prefix, suffix);
        let torus = Torus::new(&label, major_radius, self.current_length);

        // Reset flux
        self.current_length = 0;
        self.active = false;

        Some(torus)
    }

    /// Get progress toward threshold (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        if self.threshold == 0 {
            1.0
        } else {
            (self.current_length as f64 / self.threshold as f64).min(1.0)
        }
    }

    /// Deactivate this flux
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Check if flux is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Multiple flux lines emanating from a single point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluxBundle {
    pub origin: [u8; 32],
    pub lines: Vec<FluxLine>,
}

impl FluxBundle {
    pub fn new(origin: [u8; 32]) -> Self {
        Self {
            origin,
            lines: Vec::new(),
        }
    }

    pub fn add_line(&mut self, threshold: u64, label: &str, direction: TorusCoordinate) {
        let line = FluxLine::new(self.origin, threshold, label).with_direction(direction);
        self.lines.push(line);
    }

    pub fn accumulate_all(&mut self, tokens: u64) {
        for line in &mut self.lines {
            line.accumulate(tokens);
        }
    }

    pub fn ready_to_break(&self) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.should_break())
            .map(|(i, _)| i)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flux_accumulation() {
        let mut flux = FluxLine::new([0u8; 32], 100, "test");

        flux.accumulate(30);
        assert_eq!(flux.current_length, 30);
        assert!(!flux.should_break());

        flux.accumulate(80);
        assert_eq!(flux.current_length, 110);
        assert!(flux.should_break());
    }

    #[test]
    fn test_flux_break() {
        let mut flux = FluxLine::new([0u8; 32], 50, "concept");
        flux.accumulate(60);

        let torus = flux.break_into_torus("alpha", 5.0);
        assert!(torus.is_some());

        let t = torus.unwrap();
        assert_eq!(t.label, "concept_alpha");
        assert!(!flux.is_active());
        assert_eq!(flux.current_length, 0);
    }

    #[test]
    fn test_flux_progress() {
        let mut flux = FluxLine::new([0u8; 32], 100, "test");

        assert_eq!(flux.progress(), 0.0);

        flux.accumulate(50);
        assert!((flux.progress() - 0.5).abs() < 0.01);

        flux.accumulate(100);
        assert_eq!(flux.progress(), 1.0); // Capped at 1.0
    }
}
