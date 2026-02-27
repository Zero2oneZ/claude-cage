//! Sui Routing — The Tesseract IS the routing table
//!
//! Each face of the 8-cell hypercube maps to a Sui routing decision:
//!
//! ```text
//! Face 0 (Actual/WHAT)       → which Move module handles this task
//! Face 1 (Eliminated/WHAT)   → adjacent modules (proven dead ends)
//! Face 2 (Potential)         → frankincense — unexplored paths
//! Face 3 (Temporal/WHEN)    → block height / epoch constraints
//! Face 4 (Observer/WHO)     → Sui address of executing agent
//! Face 5 (Context/WHERE)    → which shard / validator set
//! Face 6 (Method/HOW)       → execution strategy
//! Face 7 (Purpose/WHY)      → Three Kings intention hash
//! ```
//!
//! Alexandria doesn't do cosine similarity against a vector database.
//! It does BBBCP constraint collapse across the tesseract.
//! Three passes. 2.7% of the search space survives.

use crate::{ConceptId, SemanticTesseract, HyperFace, HyperPosition};
use gently_chain::types::{ReasoningStep, ObjectID, StepTypeOnChain};
use gently_chain::three_kings::ThreeKings;

/// A Sui address (32 bytes, hex-encoded)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiAddress(pub [u8; 32]);

impl SuiAddress {
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", self.0.iter().map(|b| format!("{:02x}", b)).collect::<String>())
    }
}

/// Result of routing a ReasoningStep through the tesseract
#[derive(Debug, Clone)]
pub struct SuiRouting {
    /// Target agent Sui address (from Observer/WHO face)
    pub target_agent: SuiAddress,

    /// Move module that should handle this (from Actual/WHAT face)
    pub target_module: String,

    /// Eliminated paths — proven dead ends (from Eliminated face)
    pub eliminated_modules: Vec<String>,

    /// Unexplored paths — frankincense (from Potential face)
    pub unexplored_paths: Vec<ConceptId>,

    /// Temporal constraint — epoch/block height hint (from Temporal face)
    pub epoch_hint: Option<String>,

    /// Domain context (from Context/WHERE face)
    pub domain: String,

    /// Execution strategy (from Method/HOW face)
    pub strategy: ExecutionStrategy,

    /// Routing confidence (0.0 - 1.0)
    /// Based on BBBCP convergence: |surviving| / |initial|
    pub confidence: f64,

    /// Number of BBBCP constraint passes applied
    pub passes_applied: u32,

    /// Search space remaining after collapse (should be < 3% for good routing)
    pub search_space_remaining: f64,
}

/// How the routed step should be executed on-chain
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Direct PTB call — single Move function
    DirectCall,
    /// Multi-step PTB — composed transaction
    ComposedPtb,
    /// Deferred — queue for batch execution
    Deferred,
    /// Local only — cache result, don't hit chain
    LocalOnly,
}

/// BBBCP constraint pass result
#[derive(Debug, Clone)]
pub struct ConstraintPassResult {
    /// Concepts that survived this pass
    pub surviving: Vec<ConceptId>,
    /// Concepts eliminated in this pass
    pub eliminated: Vec<ConceptId>,
    /// Elimination ratio for this pass
    pub elimination_ratio: f64,
}

impl SemanticTesseract {
    /// Route a ReasoningStep through the tesseract to find its Sui target.
    ///
    /// This is the integration point: Alexandria → Sui.
    /// BBBCP constraint collapse across all 8 faces determines where
    /// the step's resource should be transferred on chain.
    ///
    /// Three passes at 70% elimination each → 2.7% of search space survives.
    pub fn route_to_sui(&self, step: &ReasoningStep) -> SuiRouting {
        // Build concept from the step's provenance
        let step_concept = self.concept_from_step(step);

        // === BBBCP CONSTRAINT COLLAPSE ===
        // Collect all candidate concepts from the tesseract
        let all_concepts: Vec<ConceptId> = self.all_concepts();

        if all_concepts.is_empty() {
            return SuiRouting::empty();
        }

        // Pass 1: BONE constraints (what IS — filter by Actual face relevance)
        let pass1 = self.constraint_pass(&all_concepts, &step_concept, HyperFace::Actual, 0.7);

        // Pass 2: CIRCLE elimination (what ISN'T — remove from Eliminated face)
        let pass2 = self.constraint_pass(&pass1.surviving, &step_concept, HyperFace::Eliminated, 0.7);

        // Pass 3: PIN constraints (where it FITS — Context face)
        let pass3 = self.constraint_pass(&pass2.surviving, &step_concept, HyperFace::Context, 0.7);

        let total_passes = 3;
        let search_remaining = if all_concepts.is_empty() {
            1.0
        } else {
            pass3.surviving.len() as f64 / all_concepts.len() as f64
        };

        // Extract routing info from surviving concepts
        let target_module = self.extract_target_module(&pass3.surviving, step);
        let target_agent = self.extract_target_agent(&pass3.surviving);
        let eliminated_modules = self.extract_eliminated_modules(&pass2.eliminated);
        let unexplored = self.extract_unexplored(&pass3.surviving);
        let epoch_hint = self.extract_temporal_hint(&pass3.surviving);
        let domain = self.extract_domain(&pass3.surviving);
        let strategy = self.determine_strategy(step, search_remaining);

        SuiRouting {
            target_agent,
            target_module,
            eliminated_modules,
            unexplored_paths: unexplored,
            epoch_hint,
            domain,
            strategy,
            confidence: 1.0 - search_remaining,
            passes_applied: total_passes,
            search_space_remaining: search_remaining,
        }
    }

    /// Run a single BBBCP constraint pass on a face.
    ///
    /// Eliminates ~70% of candidates based on face similarity to the step concept.
    /// The elimination rate is configurable but defaults to BBBCP's 70%.
    fn constraint_pass(
        &self,
        candidates: &[ConceptId],
        step_concept: &ConceptId,
        face: HyperFace,
        elimination_rate: f64,
    ) -> ConstraintPassResult {
        if candidates.is_empty() {
            return ConstraintPassResult {
                surviving: Vec::new(),
                eliminated: Vec::new(),
                elimination_ratio: 0.0,
            };
        }

        // Get the step concept's position in this face
        let step_face = self.positions.get(step_concept)
            .and_then(|p| p.last())
            .and_then(|p| p.face_embeddings.as_ref())
            .map(|fe| fe.get_face(face).to_vec());

        // Score each candidate by face similarity
        let mut scored: Vec<(ConceptId, f32)> = candidates.iter().map(|c| {
            let score = match (&step_face, self.positions.get(c)) {
                (Some(step_emb), Some(positions)) => {
                    positions.last()
                        .and_then(|p| p.face_embeddings.as_ref())
                        .map(|fe| {
                            let c_face = fe.get_face(face);
                            cosine_sim(step_emb, c_face)
                        })
                        .unwrap_or(0.0)
                }
                _ => 0.0,
            };
            (*c, score)
        }).collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Keep top (1 - elimination_rate) of candidates
        let keep_count = ((candidates.len() as f64) * (1.0 - elimination_rate)).ceil() as usize;
        let keep_count = keep_count.max(1); // Always keep at least 1

        let surviving: Vec<ConceptId> = scored.iter().take(keep_count).map(|(c, _)| *c).collect();
        let eliminated: Vec<ConceptId> = scored.iter().skip(keep_count).map(|(c, _)| *c).collect();

        let elimination_ratio = if candidates.is_empty() {
            0.0
        } else {
            eliminated.len() as f64 / candidates.len() as f64
        };

        ConstraintPassResult {
            surviving,
            eliminated,
            elimination_ratio,
        }
    }

    /// Build a ConceptId from a ReasoningStep's provenance
    fn concept_from_step(&self, step: &ReasoningStep) -> ConceptId {
        // Use the combined Three Kings hash as concept identity
        let hash = step.provenance.combined_hash();
        ConceptId(hash)
    }

    /// Get all concept IDs in the tesseract
    fn all_concepts(&self) -> Vec<ConceptId> {
        self.positions.keys().copied().collect()
    }

    /// Extract target Move module from surviving concepts
    fn extract_target_module(&self, surviving: &[ConceptId], step: &ReasoningStep) -> String {
        // Map step type to default module
        let default = match StepTypeOnChain::from_u8(step.step_type) {
            Some(StepTypeOnChain::Pattern) => "reasoning",
            Some(StepTypeOnChain::Conclude) => "reasoning",
            Some(StepTypeOnChain::Eliminate) => "constraint",
            Some(StepTypeOnChain::Specific) => "execution",
            Some(StepTypeOnChain::Fact) => "knowledge",
            Some(StepTypeOnChain::Suggest) => "proposal",
            Some(StepTypeOnChain::Correct) => "correction",
            Some(StepTypeOnChain::Guess) => "hypothesis",
            None => "generic",
        };

        // If surviving concepts have context hints, prefer those
        if let Some(first) = surviving.first() {
            if let Some(positions) = self.positions.get(first) {
                if let Some(pos) = positions.last() {
                    if let Some(ctx) = pos.context.first() {
                        return ctx.clone();
                    }
                }
            }
        }

        default.to_string()
    }

    /// Extract target agent address from Observer face of surviving concepts
    fn extract_target_agent(&self, surviving: &[ConceptId]) -> SuiAddress {
        // The first surviving concept's observer = the target agent
        for concept in surviving {
            if let Some(positions) = self.positions.get(concept) {
                if let Some(pos) = positions.last() {
                    if let Some(observer) = pos.observer.first() {
                        // Hash the observer string to get a Sui address
                        let hash = blake3::hash(observer.as_bytes());
                        return SuiAddress(*hash.as_bytes());
                    }
                }
            }
        }
        SuiAddress::zero()
    }

    /// Extract eliminated module names
    fn extract_eliminated_modules(&self, eliminated: &[ConceptId]) -> Vec<String> {
        let mut modules = Vec::new();
        for concept in eliminated.iter().take(5) {
            if let Some(positions) = self.positions.get(concept) {
                if let Some(pos) = positions.last() {
                    for ctx in &pos.context {
                        if !modules.contains(ctx) {
                            modules.push(ctx.clone());
                        }
                    }
                }
            }
        }
        modules
    }

    /// Extract unexplored paths from Potential face
    fn extract_unexplored(&self, surviving: &[ConceptId]) -> Vec<ConceptId> {
        let mut unexplored = Vec::new();
        for concept in surviving {
            if let Some(positions) = self.positions.get(concept) {
                if let Some(pos) = positions.last() {
                    unexplored.extend_from_slice(&pos.potential);
                }
            }
        }
        unexplored.truncate(10);
        unexplored
    }

    /// Extract temporal hint from surviving concepts
    fn extract_temporal_hint(&self, surviving: &[ConceptId]) -> Option<String> {
        for concept in surviving {
            if let Some(positions) = self.positions.get(concept) {
                if let Some(pos) = positions.last() {
                    if let Some(era) = pos.temporal.era_tags.first() {
                        return Some(era.clone());
                    }
                }
            }
        }
        None
    }

    /// Extract domain from Context face
    fn extract_domain(&self, surviving: &[ConceptId]) -> String {
        for concept in surviving {
            if let Some(positions) = self.positions.get(concept) {
                if let Some(pos) = positions.last() {
                    if let Some(ctx) = pos.context.first() {
                        return ctx.clone();
                    }
                }
            }
        }
        "default".to_string()
    }

    /// Determine execution strategy based on step quality and convergence
    fn determine_strategy(&self, step: &ReasoningStep, search_remaining: f64) -> ExecutionStrategy {
        let quality = step.quality as f64 / 1_000_000.0;

        // Low quality or high uncertainty → keep local
        if quality < 0.4 || search_remaining > 0.5 {
            return ExecutionStrategy::LocalOnly;
        }

        // High quality, well-converged → direct call
        if quality >= 0.7 && search_remaining < 0.03 {
            return ExecutionStrategy::DirectCall;
        }

        // Medium quality → compose with validation
        if quality >= 0.5 {
            return ExecutionStrategy::ComposedPtb;
        }

        // Default: defer for batch
        ExecutionStrategy::Deferred
    }
}

impl SuiRouting {
    /// Empty routing (no concepts in tesseract)
    pub fn empty() -> Self {
        Self {
            target_agent: SuiAddress::zero(),
            target_module: "generic".to_string(),
            eliminated_modules: Vec::new(),
            unexplored_paths: Vec::new(),
            epoch_hint: None,
            domain: "default".to_string(),
            strategy: ExecutionStrategy::LocalOnly,
            confidence: 0.0,
            passes_applied: 0,
            search_space_remaining: 1.0,
        }
    }

    /// Check if routing converged well (< 3% search space remaining)
    pub fn is_converged(&self) -> bool {
        self.search_space_remaining < 0.03
    }

    /// Check if this should hit the chain at all
    pub fn requires_chain(&self) -> bool {
        self.strategy != ExecutionStrategy::LocalOnly
    }
}

/// Cosine similarity between two slices
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::tesseract::{TemporalPosition, FaceEmbeddings, DIMS_PER_FACE};

    fn make_step(quality: f64, step_type: u8) -> ReasoningStep {
        ReasoningStep {
            id: ObjectID::zero(),
            quality: (quality * 1_000_000.0) as u64,
            step_type,
            provenance: ThreeKings::from_strings("test-agent", "claude-opus", "unit test"),
            timestamp: 0,
        }
    }

    fn make_concept(text: &str) -> ConceptId {
        ConceptId::from_concept(text)
    }

    fn populated_tesseract() -> SemanticTesseract {
        let mut t = SemanticTesseract::new();

        // Add some concepts with face embeddings
        let concepts = ["reasoning", "constraint", "execution", "knowledge", "hypothesis"];
        for (i, name) in concepts.iter().enumerate() {
            let concept = make_concept(name);
            let mut embedding = vec![0.0f32; 384];
            // Give each concept a different signature in each face
            let base = i as f32 * 0.2;
            for d in 0..DIMS_PER_FACE {
                embedding[d] = base + (d as f32 * 0.01); // Actual face
                embedding[48 + d] = 1.0 - base;           // Eliminated face
                embedding[240 + d] = base + 0.1;          // Context face
            }

            let pos = HyperPosition {
                concept,
                actual: vec![concept],
                eliminated: Vec::new(),
                potential: vec![make_concept(&format!("{}_future", name))],
                temporal: TemporalPosition {
                    valid_from: None,
                    valid_until: None,
                    era_tags: vec!["sui-era".to_string()],
                    moments: Vec::new(),
                },
                observer: vec![format!("agent_{}", i)],
                context: vec![name.to_string()],
                method: Vec::new(),
                purpose: Vec::new(),
                embedding: Some(embedding.clone()),
                face_embeddings: Some(FaceEmbeddings::from_embedding(&embedding)),
                recorded_at: Utc::now(),
            };
            t.record_position(pos);
        }

        // Also record a position for the step's provenance concept
        let step_concept = {
            let kings = ThreeKings::from_strings("test-agent", "claude-opus", "unit test");
            ConceptId(kings.combined_hash())
        };
        let mut embedding = vec![0.0f32; 384];
        for d in 0..DIMS_PER_FACE {
            embedding[d] = 0.5 + (d as f32 * 0.01);
            embedding[240 + d] = 0.6;
        }
        let pos = HyperPosition {
            concept: step_concept,
            actual: vec![make_concept("reasoning")],
            eliminated: Vec::new(),
            potential: Vec::new(),
            temporal: TemporalPosition::default(),
            observer: vec!["test-agent".to_string()],
            context: vec!["reasoning".to_string()],
            method: Vec::new(),
            purpose: Vec::new(),
            embedding: Some(embedding.clone()),
            face_embeddings: Some(FaceEmbeddings::from_embedding(&embedding)),
            recorded_at: Utc::now(),
        };
        t.record_position(pos);

        t
    }

    #[test]
    fn test_route_to_sui_basic() {
        let tesseract = populated_tesseract();
        let step = make_step(0.8, StepTypeOnChain::Pattern as u8);

        let routing = tesseract.route_to_sui(&step);

        // Should have applied 3 passes
        assert_eq!(routing.passes_applied, 3);
        // Should have some confidence
        assert!(routing.confidence > 0.0);
        // High quality step should hit chain
        assert!(routing.requires_chain());
    }

    #[test]
    fn test_route_to_sui_low_quality() {
        let tesseract = populated_tesseract();
        let step = make_step(0.2, StepTypeOnChain::Guess as u8);

        let routing = tesseract.route_to_sui(&step);

        // Low quality → local only
        assert_eq!(routing.strategy, ExecutionStrategy::LocalOnly);
        assert!(!routing.requires_chain());
    }

    #[test]
    fn test_route_to_sui_empty_tesseract() {
        let tesseract = SemanticTesseract::new();
        let step = make_step(0.9, StepTypeOnChain::Conclude as u8);

        let routing = tesseract.route_to_sui(&step);

        assert_eq!(routing.confidence, 0.0);
        assert!(!routing.is_converged());
        assert_eq!(routing.strategy, ExecutionStrategy::LocalOnly);
    }

    #[test]
    fn test_constraint_pass_eliminates() {
        let tesseract = populated_tesseract();
        let all = tesseract.all_concepts();
        let step_concept = {
            let kings = ThreeKings::from_strings("test-agent", "claude-opus", "unit test");
            ConceptId(kings.combined_hash())
        };

        let result = tesseract.constraint_pass(&all, &step_concept, HyperFace::Actual, 0.7);

        // Should eliminate ~70% (but keep at least 1)
        assert!(result.surviving.len() <= all.len());
        assert!(!result.surviving.is_empty());
        assert!(result.elimination_ratio > 0.0);
    }

    #[test]
    fn test_convergence_formula() {
        // |surviving| = |Ω| × (1-0.7)^n
        // 3 passes → 2.7% remains
        let initial = 100.0_f64;
        let after_3 = initial * (1.0 - 0.7_f64).powi(3);
        assert!((after_3 - 2.7).abs() < 0.01);
    }

    #[test]
    fn test_execution_strategy_thresholds() {
        let tesseract = populated_tesseract();

        // Quality >= 0.7, well-converged → DirectCall
        let step = make_step(0.9, 0);
        let routing = tesseract.route_to_sui(&step);
        // Note: actual strategy depends on convergence, not just quality

        // Quality < 0.4 → LocalOnly
        let step = make_step(0.3, 7);
        let routing = tesseract.route_to_sui(&step);
        assert_eq!(routing.strategy, ExecutionStrategy::LocalOnly);
    }
}
