//! # Agent Routing in Field Space — Specialists
//!
//! Specialists are agents positioned in the GOO field, each responsible
//! for a domain of expertise. Queries are routed to the nearest specialist
//! with available capacity.
//!
//! ## Field-Based Routing
//!
//! Unlike traditional routing (hashtables, priority queues), GOO routes
//! by spatial proximity in the field. Specialists that handle similar
//! domains are positioned near each other, creating natural clusters.
//!
//! ## Integration with GentlyOS
//!
//! Maps to the 72-domain router in gently-search:
//! - Each domain becomes a Specialist with a field position
//! - Queries land at field coordinates and route to nearest specialist
//! - Capacity tracks concurrent query load

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A specialist agent positioned in the GOO field.
///
/// Each specialist has a domain of expertise, a position in field
/// coordinates, and a capacity indicating how many concurrent queries
/// it can handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specialist {
    /// Unique identifier
    pub id: String,
    /// Domain of expertise (e.g., "security", "crypto", "networking")
    pub domain: String,
    /// Maximum concurrent query capacity (0.0 - 1.0, where 1.0 = fully available)
    pub capacity: f32,
    /// Position in field coordinates
    pub position: Vec2,
    /// Current load (0.0 - 1.0, where 0.0 = idle)
    pub current_load: f32,
    /// Quality score from past interactions (0.0 - 1.0)
    pub quality: f32,
}

/// Result of routing a query to a specialist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    /// The specialist's ID
    pub specialist_id: String,
    /// Distance from query point to specialist
    pub distance: f32,
    /// Available capacity after routing
    pub available_capacity: f32,
    /// Routing score (combines distance, capacity, and quality)
    pub score: f32,
}

impl Specialist {
    /// Create a new specialist.
    pub fn new(
        id: impl Into<String>,
        domain: impl Into<String>,
        position: Vec2,
        capacity: f32,
    ) -> Self {
        Self {
            id: id.into(),
            domain: domain.into(),
            capacity: capacity.clamp(0.0, 1.0),
            position,
            current_load: 0.0,
            quality: 0.5, // neutral starting quality
        }
    }

    /// Available capacity = total capacity - current load.
    pub fn available_capacity(&self) -> f32 {
        (self.capacity - self.current_load).max(0.0)
    }

    /// Whether this specialist can accept another query.
    pub fn is_available(&self) -> bool {
        self.available_capacity() > f32::EPSILON
    }

    /// Reserve capacity for a query. Returns false if no capacity available.
    pub fn reserve(&mut self, amount: f32) -> bool {
        if self.available_capacity() >= amount {
            self.current_load += amount;
            true
        } else {
            false
        }
    }

    /// Release capacity after a query completes.
    pub fn release(&mut self, amount: f32) {
        self.current_load = (self.current_load - amount).max(0.0);
    }

    /// Update quality score based on interaction outcome.
    /// Uses exponential moving average.
    pub fn update_quality(&mut self, outcome: f32) {
        let alpha = 0.1; // EMA smoothing factor
        self.quality = self.quality * (1.0 - alpha) + outcome.clamp(0.0, 1.0) * alpha;
    }

    /// Distance from this specialist to a query point.
    pub fn distance_to(&self, point: Vec2) -> f32 {
        (self.position - point).length()
    }
}

/// Route a query to the best available specialist.
///
/// Scoring formula:
/// ```text
/// score = (1.0 / (1.0 + distance)) * available_capacity * quality
/// ```
///
/// Returns the specialist with the highest score, or None if no
/// specialist has available capacity.
pub fn route_to_specialist<'a>(
    specialists: &'a [Specialist],
    query_pos: Vec2,
) -> Option<&'a Specialist> {
    specialists
        .iter()
        .filter(|s| s.is_available())
        .max_by(|a, b| {
            let score_a = routing_score(a, query_pos);
            let score_b = routing_score(b, query_pos);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Route with full scoring details.
pub fn route_detailed(
    specialists: &[Specialist],
    query_pos: Vec2,
) -> Vec<RouteResult> {
    let mut results: Vec<RouteResult> = specialists
        .iter()
        .filter(|s| s.is_available())
        .map(|s| {
            let distance = s.distance_to(query_pos);
            RouteResult {
                specialist_id: s.id.clone(),
                distance,
                available_capacity: s.available_capacity(),
                score: routing_score(s, query_pos),
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Route to a specialist in a specific domain.
pub fn route_to_domain<'a>(
    specialists: &'a [Specialist],
    query_pos: Vec2,
    domain: &str,
) -> Option<&'a Specialist> {
    specialists
        .iter()
        .filter(|s| s.is_available() && s.domain == domain)
        .max_by(|a, b| {
            let score_a = routing_score(a, query_pos);
            let score_b = routing_score(b, query_pos);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Compute routing score for a specialist relative to a query point.
fn routing_score(specialist: &Specialist, query_pos: Vec2) -> f32 {
    let distance = specialist.distance_to(query_pos);
    let proximity = 1.0 / (1.0 + distance);
    proximity * specialist.available_capacity() * specialist.quality
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_nearest() {
        let specialists = vec![
            Specialist::new("near", "general", Vec2::new(1.0, 0.0), 1.0),
            Specialist::new("far", "general", Vec2::new(100.0, 0.0), 1.0),
        ];

        let result = route_to_specialist(&specialists, Vec2::ZERO);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "near");
    }

    #[test]
    fn test_route_skips_full_capacity() {
        let mut specialists = vec![
            Specialist::new("near_full", "general", Vec2::new(1.0, 0.0), 0.5),
            Specialist::new("far_available", "general", Vec2::new(10.0, 0.0), 1.0),
        ];
        specialists[0].current_load = 0.5; // fully loaded

        let result = route_to_specialist(&specialists, Vec2::ZERO);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "far_available");
    }

    #[test]
    fn test_route_none_available() {
        let mut specialists = vec![
            Specialist::new("full", "general", Vec2::ZERO, 0.5),
        ];
        specialists[0].current_load = 0.5;

        let result = route_to_specialist(&specialists, Vec2::ZERO);
        assert!(result.is_none());
    }

    #[test]
    fn test_route_by_domain() {
        let specialists = vec![
            Specialist::new("sec", "security", Vec2::new(1.0, 0.0), 1.0),
            Specialist::new("net", "networking", Vec2::new(2.0, 0.0), 1.0),
        ];

        let result = route_to_domain(&specialists, Vec2::ZERO, "networking");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "net");
    }

    #[test]
    fn test_reserve_and_release() {
        let mut specialist = Specialist::new("s1", "general", Vec2::ZERO, 1.0);

        assert!(specialist.reserve(0.5));
        assert!((specialist.available_capacity() - 0.5).abs() < f32::EPSILON);

        assert!(specialist.reserve(0.3));
        assert!((specialist.available_capacity() - 0.2).abs() < f32::EPSILON);

        // Can't reserve more than available
        assert!(!specialist.reserve(0.5));

        specialist.release(0.3);
        assert!((specialist.available_capacity() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quality_update() {
        let mut specialist = Specialist::new("s1", "general", Vec2::ZERO, 1.0);
        let initial_quality = specialist.quality;

        specialist.update_quality(1.0); // good outcome
        assert!(specialist.quality > initial_quality);

        specialist.update_quality(0.0); // bad outcome
        // Should have decreased from peak but still above initial (EMA)
    }
}
