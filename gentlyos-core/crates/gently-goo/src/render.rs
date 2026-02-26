//! # Pixel Rendering from Field
//!
//! Converts the continuous GOO field into a discrete pixel buffer.
//! Each pixel samples G(x,y,t,theta) and maps the SDF distance to
//! an RGBA color value.
//!
//! ## Color Mapping
//!
//! - Negative SDF (inside): blend source colors, alpha = 1.0
//! - Zero SDF (surface): full intensity edge
//! - Positive SDF (outside): fade to transparent based on threshold
//!
//! The threshold parameter controls the "glow" falloff around shapes.

use serde::{Deserialize, Serialize};

use crate::field::GooField;
use crate::source::GooSource;

/// Configuration for rendering the field to pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    /// Output width in pixels
    pub width: u32,
    /// Output height in pixels
    pub height: u32,
    /// Scale factor: field units per pixel
    pub scale: f32,
    /// SDF threshold for visibility. Pixels with SDF > threshold are fully transparent.
    pub threshold: f32,
    /// Background color (RGBA)
    pub background: [f32; 4],
    /// Whether to render the glow/falloff region
    pub glow_enabled: bool,
    /// Glow intensity multiplier
    pub glow_intensity: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            scale: 1.0,
            threshold: 5.0,
            background: [0.0, 0.0, 0.0, 0.0],
            glow_enabled: true,
            glow_intensity: 0.5,
        }
    }
}

impl RenderConfig {
    /// Create a render config with specific dimensions.
    pub fn with_size(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }
}

/// Render the GOO field to an RGBA pixel buffer.
///
/// Returns a Vec of [f32; 4] RGBA values, one per pixel, in row-major order.
/// The field is centered on (0, 0) and scaled by `config.scale`.
///
/// Each pixel:
/// 1. Maps to field coordinates: `field_x = (px - width/2) * scale`
/// 2. Evaluates the field: `sdf = field.evaluate(field_x, field_y, sources)`
/// 3. Maps SDF to color:
///    - sdf < 0: inside — find dominant source color, full alpha
///    - 0 <= sdf < threshold: surface/glow — interpolated alpha
///    - sdf >= threshold: outside — background color
pub fn render_field(
    field: &GooField,
    sources: &[GooSource],
    config: &RenderConfig,
) -> Vec<[f32; 4]> {
    let pixel_count = (config.width * config.height) as usize;
    let mut buffer = vec![config.background; pixel_count];

    if sources.is_empty() {
        return buffer;
    }

    let half_w = config.width as f32 * 0.5;
    let half_h = config.height as f32 * 0.5;

    for py in 0..config.height {
        for px in 0..config.width {
            let field_x = (px as f32 - half_w) * config.scale;
            let field_y = (py as f32 - half_h) * config.scale;

            let sdf = field.evaluate(field_x, field_y, sources);
            let idx = (py * config.width + px) as usize;

            if sdf < 0.0 {
                // Inside: find the nearest source for color
                let color = dominant_source_color(field_x, field_y, sources);
                buffer[idx] = [color[0], color[1], color[2], 1.0];
            } else if sdf < config.threshold && config.glow_enabled {
                // Glow region: interpolate alpha
                let t = 1.0 - (sdf / config.threshold);
                let alpha = t * t * config.glow_intensity; // quadratic falloff
                let color = dominant_source_color(field_x, field_y, sources);
                buffer[idx] = [color[0], color[1], color[2], alpha];
            }
            // else: keep background
        }
    }

    buffer
}

/// Find the dominant source color at a point.
///
/// Returns the color of the source with the smallest SDF value
/// (nearest/most inside). Weighted by charge for priority.
fn dominant_source_color(x: f32, y: f32, sources: &[GooSource]) -> [f32; 4] {
    let p = glam::Vec2::new(x, y);
    let mut best_color = [1.0, 1.0, 1.0, 1.0];
    let mut best_distance = f32::MAX;

    for source in sources {
        let d = source.sdf(p) - source.charge; // charge biases toward this source
        if d < best_distance {
            best_distance = d;
            best_color = source.color;
        }
    }

    best_color
}

/// Render to a flat u8 RGBA buffer (for image output).
///
/// Converts f32 [0.0, 1.0] to u8 [0, 255].
pub fn render_field_u8(
    field: &GooField,
    sources: &[GooSource],
    config: &RenderConfig,
) -> Vec<u8> {
    let float_buffer = render_field(field, sources, config);
    float_buffer
        .iter()
        .flat_map(|rgba| {
            [
                (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[2].clamp(0.0, 1.0) * 255.0) as u8,
                (rgba[3].clamp(0.0, 1.0) * 255.0) as u8,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn test_render_empty_field() {
        let field = GooField::new(256, 256, 32.0);
        let config = RenderConfig::with_size(16, 16);
        let buffer = render_field(&field, &[], &config);
        assert_eq!(buffer.len(), 256); // 16 * 16
        // All pixels should be background
        for pixel in &buffer {
            assert_eq!(*pixel, config.background);
        }
    }

    #[test]
    fn test_render_with_source() {
        let field = GooField::new(64, 64, 16.0);
        let sources = vec![GooSource::sphere("s1", Vec2::ZERO, 8.0, 1.0)];
        let config = RenderConfig::with_size(64, 64);
        let buffer = render_field(&field, &sources, &config);

        // Center pixel (32, 32) maps to field (0, 0) — inside sphere
        let center_idx = (32 * 64 + 32) as usize;
        assert_eq!(buffer[center_idx][3], 1.0, "Center pixel should have full alpha");

        // Corner pixel (0, 0) maps to field (-32, -32) — far outside
        let corner_idx = 0;
        assert!(
            buffer[corner_idx][3] < 1.0,
            "Corner pixel should not have full alpha"
        );
    }
}
