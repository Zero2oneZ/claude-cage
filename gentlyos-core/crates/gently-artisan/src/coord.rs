//! TorusCoordinate - Position on a torus surface
//!
//! Step 1.1 from BUILD_STEPS.md

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

/// A point on a torus surface using angular coordinates
///
/// - `theta` (θ): Poloidal angle [0, 2π] - represents "topic"
/// - `phi` (φ): Toroidal angle [0, 2π] - represents "abstraction level"
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TorusCoordinate {
    /// Poloidal angle - the "topic" dimension
    pub theta: f64,
    /// Toroidal angle - the "abstraction level" dimension
    pub phi: f64,
}

impl TorusCoordinate {
    /// Create a new coordinate (angles in radians)
    pub fn new(theta: f64, phi: f64) -> Self {
        Self { theta, phi }.normalize()
    }

    /// Normalize angles to [0, 2π)
    pub fn normalize(&self) -> Self {
        Self {
            theta: self.theta.rem_euclid(TAU),
            phi: self.phi.rem_euclid(TAU),
        }
    }

    /// Calculate geodesic distance on torus surface
    ///
    /// Uses the torus metric with major radius R and minor radius r
    pub fn distance(&self, other: &Self, major_radius: f64, minor_radius: f64) -> f64 {
        let d_theta = angle_diff(self.theta, other.theta);
        let d_phi = angle_diff(self.phi, other.phi);

        // Approximate geodesic distance on torus
        // ds² ≈ r²dθ² + (R + r·cos(θ))²dφ²
        let avg_theta = (self.theta + other.theta) / 2.0;
        let r = minor_radius;
        let effective_radius = major_radius + r * avg_theta.cos();

        let term1 = r * d_theta;
        let term2 = effective_radius * d_phi;

        (term1 * term1 + term2 * term2).sqrt()
    }

    /// Linear interpolation between two coordinates
    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        Self::new(
            lerp_angle(self.theta, other.theta, t),
            lerp_angle(self.phi, other.phi, t),
        )
    }

    /// Convert to Cartesian coordinates given torus radii
    ///
    /// Returns (x, y, z) in 3D space
    pub fn to_cartesian(&self, major_radius: f64, minor_radius: f64) -> (f64, f64, f64) {
        let r = minor_radius;
        let big_r = major_radius;

        let x = (big_r + r * self.theta.cos()) * self.phi.cos();
        let y = (big_r + r * self.theta.cos()) * self.phi.sin();
        let z = r * self.theta.sin();

        (x, y, z)
    }
}

impl Default for TorusCoordinate {
    fn default() -> Self {
        Self {
            theta: 0.0,
            phi: 0.0,
        }
    }
}

/// Calculate shortest angular difference (handles wraparound)
fn angle_diff(a: f64, b: f64) -> f64 {
    let diff = (b - a).rem_euclid(TAU);
    if diff > std::f64::consts::PI {
        diff - TAU
    } else {
        diff
    }
    .abs()
}

/// Interpolate between angles (shortest path)
fn lerp_angle(a: f64, b: f64, t: f64) -> f64 {
    let diff = angle_diff(a, b);
    let direction = if (b - a).rem_euclid(TAU) > std::f64::consts::PI {
        -1.0
    } else {
        1.0
    };
    (a + direction * diff * t).rem_euclid(TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let coord = TorusCoordinate::new(TAU + 1.0, -1.0);
        assert!((coord.theta - 1.0).abs() < 1e-10);
        assert!((coord.phi - (TAU - 1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_distance_same_point() {
        let coord = TorusCoordinate::new(1.0, 2.0);
        assert!((coord.distance(&coord, 10.0, 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_to_cartesian() {
        let coord = TorusCoordinate::new(0.0, 0.0);
        let (x, y, z) = coord.to_cartesian(10.0, 3.0);
        assert!((x - 13.0).abs() < 1e-10); // R + r at theta=0, phi=0
        assert!(y.abs() < 1e-10);
        assert!(z.abs() < 1e-10);
    }
}
