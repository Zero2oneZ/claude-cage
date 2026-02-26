//! # SDF Primitives — Sources that populate the GOO field
//!
//! Each `GooSource` is a signed distance field primitive positioned in
//! 2D space. The field evaluates these and blends them with smooth_min
//! to create the unified G(x,y,t,theta).
//!
//! ## SDF Convention
//!
//! - Negative = inside the shape
//! - Zero = on the surface
//! - Positive = outside the shape
//!
//! ## Primitives
//!
//! | Primitive | Formula | Use Case |
//! |-----------|---------|----------|
//! | Sphere    | `|p| - r` | Nodes, avatars, data points |
//! | Box       | `max(|p| - half, 0) + min(max(|p.x|-h.x, |p.y|-h.y), 0)` | Panels, regions |
//! | Torus     | Cross-section of 3D torus | BS-ARTISAN integration |
//! | Line      | `|proj - p|` | Connections, edges |

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// An SDF source in the GOO field.
///
/// Each source has a position, a geometric primitive, a charge (intensity),
/// and a color for rendering. The charge modulates both visual appearance
/// and attention weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooSource {
    /// Unique identifier for this source
    pub id: String,
    /// Position in field coordinates
    pub position: Vec2,
    /// The geometric primitive
    pub primitive: SdfPrimitive,
    /// Charge (0.0 - 1.0): intensity / attention weight
    pub charge: f32,
    /// RGBA color for rendering
    pub color: [f32; 4],
}

/// SDF geometric primitives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SdfPrimitive {
    /// Circle/sphere with radius
    Sphere { radius: f32 },
    /// Axis-aligned box with half-extents
    Box { half_extents: Vec2 },
    /// Torus cross-section (2D slice of 3D torus)
    /// Major = distance from center to tube center
    /// Minor = tube radius
    Torus { major: f32, minor: f32 },
    /// Line segment between two points
    Line { start: Vec2, end: Vec2 },
}

impl GooSource {
    /// Create a new GooSource with full parameters.
    pub fn new(
        id: impl Into<String>,
        position: Vec2,
        primitive: SdfPrimitive,
        charge: f32,
        color: [f32; 4],
    ) -> Self {
        Self {
            id: id.into(),
            position,
            primitive,
            charge: charge.clamp(0.0, 1.0),
            color,
        }
    }

    /// Convenience: create a sphere source with default white color.
    pub fn sphere(id: impl Into<String>, position: Vec2, radius: f32, charge: f32) -> Self {
        Self::new(
            id,
            position,
            SdfPrimitive::Sphere { radius },
            charge,
            [1.0, 1.0, 1.0, 1.0],
        )
    }

    /// Convenience: create a box source with default white color.
    pub fn box_source(
        id: impl Into<String>,
        position: Vec2,
        half_extents: Vec2,
        charge: f32,
    ) -> Self {
        Self::new(
            id,
            position,
            SdfPrimitive::Box { half_extents },
            charge,
            [1.0, 1.0, 1.0, 1.0],
        )
    }

    /// Convenience: create a torus source with default white color.
    pub fn torus(
        id: impl Into<String>,
        position: Vec2,
        major: f32,
        minor: f32,
        charge: f32,
    ) -> Self {
        Self::new(
            id,
            position,
            SdfPrimitive::Torus { major, minor },
            charge,
            [1.0, 1.0, 1.0, 1.0],
        )
    }

    /// Convenience: create a line segment source with default white color.
    pub fn line(
        id: impl Into<String>,
        position: Vec2,
        start: Vec2,
        end: Vec2,
        charge: f32,
    ) -> Self {
        Self::new(
            id,
            position,
            SdfPrimitive::Line { start, end },
            charge,
            [1.0, 1.0, 1.0, 1.0],
        )
    }

    /// Evaluate the signed distance field at world-space point `p`.
    ///
    /// Transforms `p` into the source's local space (centered on position),
    /// then evaluates the primitive SDF.
    pub fn sdf(&self, p: Vec2) -> f32 {
        let local = p - self.position;
        match &self.primitive {
            SdfPrimitive::Sphere { radius } => sdf_sphere(local, *radius),
            SdfPrimitive::Box { half_extents } => sdf_box(local, *half_extents),
            SdfPrimitive::Torus { major, minor } => sdf_torus(local, *major, *minor),
            SdfPrimitive::Line { start, end } => sdf_line(local, *start, *end),
        }
    }
}

/// SDF for a sphere (circle in 2D): distance from origin minus radius.
fn sdf_sphere(p: Vec2, radius: f32) -> f32 {
    p.length() - radius
}

/// SDF for an axis-aligned box defined by half-extents.
///
/// Uses the standard box SDF formula:
/// ```text
/// d = |p| - half_extents
/// exterior = length(max(d, 0))
/// interior = min(max(d.x, d.y), 0)
/// result = exterior + interior
/// ```
fn sdf_box(p: Vec2, half_extents: Vec2) -> f32 {
    let d = Vec2::new(p.x.abs() - half_extents.x, p.y.abs() - half_extents.y);
    let exterior = Vec2::new(d.x.max(0.0), d.y.max(0.0)).length();
    let interior = d.x.max(d.y).min(0.0);
    exterior + interior
}

/// SDF for a torus cross-section (2D slice of a 3D torus).
///
/// The torus in 2D is two circles at distance `major` from center,
/// each with radius `minor`. We compute the distance to the nearest
/// point on the major circle, then subtract minor.
fn sdf_torus(p: Vec2, major: f32, minor: f32) -> f32 {
    let q = Vec2::new(p.length() - major, 0.0);
    q.length() - minor
}

/// SDF for a line segment from `start` to `end`.
///
/// Projects point onto the line segment and returns distance
/// to the nearest point on the segment.
fn sdf_line(p: Vec2, start: Vec2, end: Vec2) -> f32 {
    let pa = p - start;
    let ba = end - start;
    let ba_len_sq = ba.length_squared();

    if ba_len_sq < f32::EPSILON {
        // Degenerate line (start == end), treat as point
        return pa.length();
    }

    let t = (pa.dot(ba) / ba_len_sq).clamp(0.0, 1.0);
    let nearest = start + ba * t;
    (p - nearest).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_sdf() {
        let source = GooSource::sphere("s", Vec2::ZERO, 2.0, 1.0);

        // Center: distance = -2.0 (inside)
        assert!((source.sdf(Vec2::ZERO) - (-2.0)).abs() < f32::EPSILON);

        // On surface: distance = 0.0
        assert!((source.sdf(Vec2::new(2.0, 0.0))).abs() < f32::EPSILON);

        // Outside: distance = 1.0
        assert!((source.sdf(Vec2::new(3.0, 0.0)) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sphere_sdf_offset() {
        let source = GooSource::sphere("s", Vec2::new(5.0, 5.0), 1.0, 1.0);

        // At source center: distance = -1.0
        assert!((source.sdf(Vec2::new(5.0, 5.0)) - (-1.0)).abs() < f32::EPSILON);

        // 1 unit away from center on x-axis: on surface = 0.0
        assert!((source.sdf(Vec2::new(6.0, 5.0))).abs() < f32::EPSILON);
    }

    #[test]
    fn test_box_sdf() {
        let source = GooSource::box_source("b", Vec2::ZERO, Vec2::new(2.0, 1.0), 1.0);

        // Center: inside the box
        let center = source.sdf(Vec2::ZERO);
        assert!(center < 0.0, "Center of box should be negative, got {}", center);
        // Interior distance should be -min(half_extents) = -1.0
        assert!((center - (-1.0)).abs() < f32::EPSILON);

        // On face along x-axis: distance = 0
        let on_face = source.sdf(Vec2::new(2.0, 0.0));
        assert!(on_face.abs() < f32::EPSILON, "On face should be ~0, got {}", on_face);

        // Outside along x-axis
        let outside = source.sdf(Vec2::new(3.0, 0.0));
        assert!((outside - 1.0).abs() < f32::EPSILON, "1 unit outside should be 1.0, got {}", outside);

        // Corner: distance to nearest corner (2,1) from (3,2)
        // d = (|3|-2, |2|-1) = (1, 1), length = sqrt(2)
        let corner = source.sdf(Vec2::new(3.0, 2.0));
        assert!(
            (corner - std::f32::consts::SQRT_2).abs() < 0.001,
            "Corner distance should be sqrt(2), got {}",
            corner
        );
    }

    #[test]
    fn test_torus_sdf() {
        let source = GooSource::torus("t", Vec2::ZERO, 5.0, 1.0, 1.0);

        // On the major circle (at distance 5 from origin): should be -1.0 (inside tube)
        let on_major = source.sdf(Vec2::new(5.0, 0.0));
        assert!(
            (on_major - (-1.0)).abs() < 0.01,
            "On major circle should be -minor, got {}",
            on_major
        );

        // At origin: distance to major circle is 5, minus minor = 4
        let at_origin = source.sdf(Vec2::ZERO);
        assert!(
            (at_origin - 4.0).abs() < 0.01,
            "At origin should be major-minor, got {}",
            at_origin
        );

        // On the outer edge: distance 6 from origin, major=5, so tube distance = 1-1 = 0
        let outer_edge = source.sdf(Vec2::new(6.0, 0.0));
        assert!(
            outer_edge.abs() < 0.01,
            "On outer edge should be ~0, got {}",
            outer_edge
        );
    }

    #[test]
    fn test_line_sdf() {
        let source = GooSource::line(
            "l",
            Vec2::ZERO,
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            1.0,
        );

        // Point directly above midpoint of line: distance = 3.0
        let above = source.sdf(Vec2::new(2.0, 3.0));
        assert!(
            (above - 3.0).abs() < 0.01,
            "3 units above line should be 3.0, got {}",
            above
        );

        // Point on the line: distance = 0.0
        let on_line = source.sdf(Vec2::new(2.0, 0.0));
        assert!(on_line.abs() < f32::EPSILON, "On line should be 0, got {}", on_line);

        // Point past the end: distance to endpoint (4,0) from (6,0) = 2.0
        let past_end = source.sdf(Vec2::new(6.0, 0.0));
        assert!(
            (past_end - 2.0).abs() < 0.01,
            "Past end should be distance to endpoint, got {}",
            past_end
        );
    }

    #[test]
    fn test_line_degenerate() {
        // Degenerate line (point)
        let source = GooSource::line("l", Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, 1.0);
        let d = source.sdf(Vec2::new(3.0, 4.0));
        assert!((d - 5.0).abs() < 0.01, "Degenerate line should act as point SDF");
    }

    #[test]
    fn test_charge_clamping() {
        let source = GooSource::sphere("s", Vec2::ZERO, 1.0, 2.5);
        assert_eq!(source.charge, 1.0, "Charge should be clamped to 1.0");

        let source2 = GooSource::sphere("s2", Vec2::ZERO, 1.0, -0.5);
        assert_eq!(source2.charge, 0.0, "Charge should be clamped to 0.0");
    }
}
