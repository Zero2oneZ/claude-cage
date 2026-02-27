//! # gently-artisan
//!
//! BS-ARTISAN: Toroidal knowledge storage system.
//!
//! Replaces vector embeddings with topological geometry:
//! - `Torus`: Knowledge stored on torus surface (θ=topic, φ=abstraction)
//! - `Foam`: Multi-torus interconnected memory
//! - `FluxLine`: Traversal paths through major/minor loops
//! - `BARF`: Bark And Retrieve Foam - XOR-based retrieval
//!
//! Formula: `r = tokens / 2π` (context length determines radius)

pub mod coord;
pub mod torus;
pub mod foam;
pub mod flux;
pub mod barf;
pub mod winding;
pub mod chain;

pub use coord::TorusCoordinate;
pub use torus::{Torus, TorusPoint};
pub use foam::{Foam, TorusBlend};
pub use flux::FluxLine;
pub use barf::{BarfQuery, BarfResult};
pub use winding::WindingLevel;
pub use chain::{ChainEligibility, ChainSubmission, ChainStats, CHAIN_BOUNDARY_WINDING};

use std::f64::consts::TAU; // 2π

/// Convert token count to minor radius
/// Formula from spec: r = tokens / 2π
#[inline]
pub fn tokens_to_radius(tokens: u64) -> f64 {
    tokens as f64 / TAU
}

/// Convert minor radius back to approximate token count
#[inline]
pub fn radius_to_tokens(radius: f64) -> u64 {
    (radius * TAU) as u64
}
