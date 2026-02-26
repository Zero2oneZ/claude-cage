#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# BS-ARTISAN w=2 DEPLOYMENT
# Already in Zero2oneZ-DeathStar folder - just paste and run
# ═══════════════════════════════════════════════════════════════════════════════

set -e
echo "📍 Deploying BS-ARTISAN to: $(pwd)"

# Create directories
mkdir -p crates/bs-artisan/src

# ═══════════════════════════════════════════════════════════════════════════════
# CARGO.TOML
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/Cargo.toml << 'EOF'
[package]
name = "bs-artisan"
version = "0.1.0"
edition = "2021"
description = "Bi-State Asymptotic Refinement Through Iterative Spiral Advancement on N-dimensional Tori"
license = "MIT"
authors = ["Tom Lee <tom@gentlyos.dev>"]

[dependencies]
blake3 = "1.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# LIB.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/lib.rs << 'EOF'
//! # BS-ARTISAN
//! Bi-State Asymptotic Refinement Through Iterative Spiral Advancement on N-dimensional Tori
//! 
//! Knowledge lives on toroidal surfaces, not flat vector spaces.
//! Similarity = foam traversal, NOT cosine distance. No embedding model.
//!
//! ## Winding Levels
//! - w=1: Documentation (BTC block 933402)
//! - w=2: Core data structures (THIS)
//! - w=3: BARF retrieval
//! - w=4: Foam management
//! - w=5: GentlyOS integration
//! - w=6: Production

pub mod torus;
pub mod foam;
pub mod spline;
pub mod flux;
pub mod culling;

pub use torus::{Torus, TorusPoint};
pub use foam::{Foam, TorusBlend};
pub use spline::{Spline, SplinePoint};
pub use flux::FluxLine;
pub use culling::{Octant, FaceDirection, CullingZone};

pub type Hash = [u8; 32];

pub fn hash(data: &[u8]) -> Hash {
    *blake3::hash(data).as_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BSCoefficient(pub f64);

impl BSCoefficient {
    pub fn new(value: f64) -> Self { Self(value.clamp(0.0, 1.0)) }
    pub fn substance_ratio(&self) -> f64 { 1.0 - self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Winding(pub u8);

impl Winding {
    pub const RAW_IDEA: Self = Self(1);
    pub const STRUCTURED: Self = Self(2);
    pub const REFINED: Self = Self(3);
    pub const TESTED: Self = Self(4);
    pub const DOCUMENTED: Self = Self(5);
    pub const PRODUCTION: Self = Self(6);
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# TORUS.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/torus.rs << 'EOF'
use crate::{Hash, BSCoefficient, Winding, hash};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TorusPoint {
    pub theta: f64,
    pub phi: f64,
}

impl TorusPoint {
    pub fn new(theta: f64, phi: f64) -> Self {
        Self {
            theta: theta % std::f64::consts::TAU,
            phi: phi % std::f64::consts::TAU,
        }
    }
    
    pub fn to_cartesian(&self, major_r: f64, minor_r: f64) -> (f64, f64, f64) {
        let x = (major_r + minor_r * self.phi.cos()) * self.theta.cos();
        let y = (major_r + minor_r * self.phi.cos()) * self.theta.sin();
        let z = minor_r * self.phi.sin();
        (x, y, z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torus {
    pub id: Hash,
    pub label: String,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub winding: Winding,
    pub bs: BSCoefficient,
    pub parent: Option<Hash>,
    pub created_at: u64,
    pub touched_at: u64,
}

impl Torus {
    pub fn new(label: &str, major_radius: f64) -> Self {
        let id = hash(label.as_bytes());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        Self {
            id, label: label.to_string(), major_radius,
            minor_radius: 0.1, winding: Winding::RAW_IDEA,
            bs: BSCoefficient::new(0.5), parent: None,
            created_at: now, touched_at: now,
        }
    }
    
    pub fn from_flux(parent: Hash, tokens_spent: f64, label: &str) -> Self {
        let minor_radius = tokens_spent / std::f64::consts::TAU;
        let mut t = Self::new(label, minor_radius * 2.0);
        t.minor_radius = minor_radius;
        t.parent = Some(parent);
        t
    }
    
    pub fn surface_area(&self) -> f64 {
        4.0 * std::f64::consts::PI.powi(2) * self.major_radius * self.minor_radius
    }
    
    pub fn distance(&self, a: &TorusPoint, b: &TorusPoint) -> f64 {
        let dtheta = (a.theta - b.theta).abs().min(std::f64::consts::TAU - (a.theta - b.theta).abs());
        let dphi = (a.phi - b.phi).abs().min(std::f64::consts::TAU - (a.phi - b.phi).abs());
        ((self.major_radius * dtheta).powi(2) + (self.minor_radius * dphi).powi(2)).sqrt()
    }
    
    pub fn touch(&mut self) {
        self.touched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    }
    
    pub fn advance_winding(&mut self) {
        if self.winding.0 < 6 { self.winding = Winding(self.winding.0 + 1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_torus() {
        let t = Torus::new("test", 1.0);
        assert_eq!(t.winding, Winding::RAW_IDEA);
        assert!(t.surface_area() > 0.0);
    }
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# FOAM.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/foam.rs << 'EOF'
use crate::{Hash, Torus, TorusPoint, FluxLine, hash};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorusBlend {
    pub torus_a: Hash,
    pub torus_b: Hash,
    pub point_a: TorusPoint,
    pub point_b: TorusPoint,
    pub strength: f64,
}

impl TorusBlend {
    pub fn new(a: Hash, b: Hash, pa: TorusPoint, pb: TorusPoint) -> Self {
        Self { torus_a: a, torus_b: b, point_a: pa, point_b: pb, strength: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Foam {
    pub tori: HashMap<Hash, Torus>,
    pub blends: Vec<TorusBlend>,
    pub active_flux: Vec<FluxLine>,
    pub genesis: Hash,
}

impl Foam {
    pub fn new(genesis_data: &[u8]) -> Self {
        Self { tori: HashMap::new(), blends: Vec::new(), active_flux: Vec::new(), genesis: hash(genesis_data) }
    }
    
    pub fn add_torus(&mut self, t: Torus) { self.tori.insert(t.id, t); }
    
    pub fn blend(&mut self, a: Hash, b: Hash, pa: TorusPoint, pb: TorusPoint) {
        if self.tori.contains_key(&a) && self.tori.contains_key(&b) {
            self.blends.push(TorusBlend::new(a, b, pa, pb));
        }
    }
    
    pub fn connected(&self, id: &Hash) -> Vec<Hash> {
        self.blends.iter().filter_map(|b| {
            if &b.torus_a == id { Some(b.torus_b) }
            else if &b.torus_b == id { Some(b.torus_a) }
            else { None }
        }).collect()
    }
    
    /// BARF: Bidirectional Asymptotic Refinement Fetch
    pub fn barf(&self, query: &Hash, max: usize) -> Vec<(Hash, f64)> {
        let mut d: Vec<_> = self.tori.keys().map(|id| {
            let xor: u32 = id.iter().zip(query).map(|(a,b)| (a^b).count_ones()).sum();
            (*id, xor as f64)
        }).collect();
        d.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
        
        if let Some((nearest,_)) = d.first() {
            for c in self.connected(nearest) {
                if let Some(p) = d.iter().position(|(h,_)| h==&c) { d[p].1 *= 0.5; }
            }
        }
        d.sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());
        d.truncate(max);
        d
    }
    
    pub fn traverse(&self, start: &Hash, target: &Hash) -> Option<(Vec<Hash>, f64)> {
        if !self.tori.contains_key(start) || !self.tori.contains_key(target) { return None; }
        if start == target { return Some((vec![*start], 0.0)); }
        
        let mut visited: HashMap<Hash, (Hash, f64)> = HashMap::new();
        let mut queue = vec![(*start, 0.0)];
        visited.insert(*start, (*start, 0.0));
        
        while let Some((cur, dist)) = queue.pop() {
            if &cur == target {
                let mut path = vec![cur];
                let mut n = cur;
                while &n != start { let (p,_) = visited[&n]; path.push(p); n = p; }
                path.reverse();
                return Some((path, dist));
            }
            for nb in self.connected(&cur) {
                if !visited.contains_key(&nb) {
                    visited.insert(nb, (cur, dist+1.0));
                    queue.push((nb, dist+1.0));
                }
            }
        }
        None
    }
    
    pub fn start_flux(&mut self, id: Hash, thresh: f64) {
        if self.tori.contains_key(&id) { self.active_flux.push(FluxLine::new(id, thresh)); }
    }
    
    pub fn accumulate_flux(&mut self, tokens: u64) {
        for f in &mut self.active_flux { f.accumulate(tokens); }
    }
    
    pub fn process_flux_breaks(&mut self) -> Vec<Hash> {
        let mut new_ids = Vec::new();
        let mut broken = Vec::new();
        
        for (i, f) in self.active_flux.iter().enumerate() {
            if f.should_break() {
                let t = Torus::from_flux(f.origin_torus, f.current_length, &format!("flux_{}", i));
                new_ids.push(t.id);
                self.blend(f.origin_torus, t.id, f.origin_point, TorusPoint::new(0.0, 0.0));
                self.add_torus(t);
                broken.push(i);
            }
        }
        for i in broken.into_iter().rev() { self.active_flux.remove(i); }
        new_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_traverse() {
        let mut f = Foam::new(b"gen");
        let t1 = Torus::new("a", 1.0); let t2 = Torus::new("b", 1.0); let t3 = Torus::new("c", 1.0);
        let (id1, id2, id3) = (t1.id, t2.id, t3.id);
        f.add_torus(t1); f.add_torus(t2); f.add_torus(t3);
        f.blend(id1, id2, TorusPoint::new(0.0,0.0), TorusPoint::new(0.0,0.0));
        f.blend(id2, id3, TorusPoint::new(0.0,0.0), TorusPoint::new(0.0,0.0));
        let r = f.traverse(&id1, &id3);
        assert!(r.is_some());
        assert_eq!(r.unwrap().1, 2.0);
    }
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# SPLINE.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/spline.rs << 'EOF'
use crate::{TorusPoint, BSCoefficient, Winding};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplinePoint {
    pub point: TorusPoint,
    pub dwell_time: f64,
    pub content_hash: [u8; 32],
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spline {
    pub points: Vec<SplinePoint>,
    pub winding: Winding,
    pub bs: BSCoefficient,
}

impl Default for Spline { fn default() -> Self { Self::new() } }

impl Spline {
    pub fn new() -> Self {
        Self { points: Vec::new(), winding: Winding::RAW_IDEA, bs: BSCoefficient::new(0.5) }
    }
    
    pub fn add_point(&mut self, pt: TorusPoint, dwell: f64, content: &[u8]) {
        let ch = *blake3::hash(content).as_bytes();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        self.points.push(SplinePoint { point: pt, dwell_time: dwell, content_hash: ch, timestamp: ts });
        
        if self.points.len() >= 2 {
            let prev = &self.points[self.points.len()-2];
            let curr = &self.points[self.points.len()-1];
            if prev.point.theta > 5.0 && curr.point.theta < 1.0 {
                self.winding = Winding(self.winding.0.saturating_add(1).min(6));
            }
        }
    }
    
    pub fn total_attention(&self) -> f64 { self.points.iter().map(|p| p.dwell_time).sum() }
    
    pub fn span(&self, idx: usize, radius: usize) -> &[SplinePoint] {
        let s = idx.saturating_sub(radius);
        let e = (idx + radius + 1).min(self.points.len());
        &self.points[s..e]
    }
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# FLUX.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/flux.rs << 'EOF'
use crate::{Hash, TorusPoint};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluxLine {
    pub origin_torus: Hash,
    pub origin_point: TorusPoint,
    pub current_length: f64,
    pub threshold: f64,
    pub context_hash: Hash,
}

impl FluxLine {
    pub fn new(origin: Hash, threshold: f64) -> Self {
        Self { origin_torus: origin, origin_point: TorusPoint::new(0.0,0.0),
               current_length: 0.0, threshold, context_hash: [0u8;32] }
    }
    
    pub fn accumulate(&mut self, tokens: u64) { self.current_length += tokens as f64; }
    pub fn should_break(&self) -> bool { self.current_length >= self.threshold }
    pub fn to_radius(&self) -> f64 { self.current_length / std::f64::consts::TAU }
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# CULLING.RS
# ═══════════════════════════════════════════════════════════════════════════════
cat > crates/bs-artisan/src/culling.rs << 'EOF'
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Octant { PPP, PPN, PNP, PNN, NPP, NPN, NNP, NNN }

impl Octant {
    pub fn from_coords(x: f64, y: f64, z: f64) -> Self {
        match (x >= 0.0, y >= 0.0, z >= 0.0) {
            (true,true,true) => Self::PPP, (true,true,false) => Self::PPN,
            (true,false,true) => Self::PNP, (true,false,false) => Self::PNN,
            (false,true,true) => Self::NPP, (false,true,false) => Self::NPN,
            (false,false,true) => Self::NNP, (false,false,false) => Self::NNN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaceDirection { Inward, Outward }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CullingZone { pub octant: Octant, pub direction: FaceDirection }

impl CullingZone {
    pub fn from_point_normal(p: (f64,f64,f64), n: (f64,f64,f64)) -> Self {
        let oct = Octant::from_coords(p.0, p.1, p.2);
        let dot = p.0*n.0 + p.1*n.1 + p.2*n.2;
        let dir = if dot < 0.0 { FaceDirection::Inward } else { FaceDirection::Outward };
        Self { octant: oct, direction: dir }
    }
    
    pub fn should_compress(&self) -> bool { matches!(self.direction, FaceDirection::Inward) }
    pub fn compression_ratio(&self) -> f64 {
        match self.direction { FaceDirection::Inward => 0.3, FaceDirection::Outward => 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_culling() {
        let z = CullingZone::from_point_normal((1.0,1.0,1.0), (-1.0,-1.0,-1.0));
        assert!(z.should_compress());
    }
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# GIT
# ═══════════════════════════════════════════════════════════════════════════════
echo "📦 Staging..."
git add -A

echo "📝 Committing..."
git commit -m "feat: BS-ARTISAN w=2 - toroidal knowledge topology

- Torus, TorusPoint (knowledge containers)
- Foam, TorusBlend (multi-torus topology)
- BARF retrieval (no embeddings, foam traversal)
- Spline (reasoning paths with winding)
- FluxLine (tokens → new torus spawning)
- CullingZone (face-normal compression)

Proof: de7b79b446e31bd487bc479eee1942ae116e07c60881a094f0fe3f9da3e13b2a
BTC: 933402"

echo "🚀 Pushing..."
git push

echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "✅ DONE - BS-ARTISAN w=2 deployed to Zero2oneZ-DeathStar"
echo ""
echo "Files created:"
ls -la crates/bs-artisan/src/
echo ""
echo "On Dell: git pull && cargo build -p bs-artisan && cargo test -p bs-artisan"
echo "═══════════════════════════════════════════════════════════════════════════════"
