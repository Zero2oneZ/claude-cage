//! Chain Integration — Winding 4+ triggers Sui submission
//!
//! BS-ARTISAN is the off-chain cache layer. Not everything needs formal
//! verification. Not every micro-task needs a Move resource.
//!
//! ```text
//! Winding 1 (RAW_IDEA)    → unverified local result
//! Winding 2 (STRUCTURED)  → locally validated
//! Winding 3 (REFINED)     → passed BBBCP constraint check
//! ─────────────────────────── CHAIN BOUNDARY ───────────────────
//! Winding 4 (TESTED)      → verified by Move execution
//! Winding 5 (DOCUMENTED)  → Alexandria topology updated
//! Winding 6 (PRODUCTION)  → SYNTH reward claimed
//! ```
//!
//! Below winding 4: local foam, no chain cost.
//! Winding 4+: chain-verified, SYNTH-eligible.
//!
//! The CullingZone maps perfectly:
//! - Inward-facing = local cache (70% compression)
//! - Outward-facing = what you publish to chain (full preservation)

use crate::torus::Torus;
use crate::foam::Foam;
use gently_chain::types::{ReasoningStep, ObjectID, StepTypeOnChain};
use gently_chain::three_kings::ThreeKings;
use gently_chain::transactions::{PtbBuilder, MoveCallArg};

/// The winding level at which results cross the chain boundary
pub const CHAIN_BOUNDARY_WINDING: u8 = 4;

/// Result of checking whether a torus should submit to chain
#[derive(Debug, Clone)]
pub struct ChainEligibility {
    /// Whether this torus has crossed the chain boundary
    pub eligible: bool,
    /// Current winding level
    pub winding: u8,
    /// Trustworthiness score (0.0 - 1.0)
    pub trustworthiness: f64,
    /// Estimated quality for ReasoningStep (mapped from BS score)
    pub estimated_quality: f64,
    /// Suggested step type based on torus characteristics
    pub suggested_step_type: StepTypeOnChain,
    /// Whether SYNTH reward can be claimed (winding 6)
    pub synth_eligible: bool,
}

/// A pending chain submission from a torus that crossed the boundary
#[derive(Debug, Clone)]
pub struct ChainSubmission {
    /// The torus being submitted
    pub torus_id: [u8; 32],
    /// The ReasoningStep to publish on-chain
    pub reasoning_step: ReasoningStep,
    /// Three Kings provenance
    pub provenance: ThreeKings,
    /// PTB commands for the submission
    pub ptb_bytes: Vec<u8>,
}

impl Torus {
    /// Check if this torus is eligible for chain submission.
    ///
    /// The chain boundary is at winding 4 (Tested).
    /// Below 4: local foam only, zero cost.
    /// 4+: chain-verified, Move resource created.
    pub fn chain_eligibility(&self) -> ChainEligibility {
        let eligible = self.winding >= CHAIN_BOUNDARY_WINDING;
        let trustworthiness = self.trustworthiness();

        // Map BS score to quality: low BS = high quality
        // quality = (1.0 - bs) scaled to fixed-point
        let estimated_quality = (1.0 - self.bs).clamp(0.0, 1.0);

        // Determine step type from torus characteristics
        let suggested_step_type = if self.winding >= 6 {
            StepTypeOnChain::Conclude  // Production = concluded
        } else if self.winding >= 5 {
            StepTypeOnChain::Pattern   // Documented = pattern recognized
        } else if self.bs < 0.3 {
            StepTypeOnChain::Fact      // Low BS = factual
        } else if self.bs < 0.5 {
            StepTypeOnChain::Specific  // Medium BS = specific detail
        } else {
            StepTypeOnChain::Suggest   // Higher BS = suggestion
        };

        let synth_eligible = self.winding >= 6 && self.bs < 0.3;

        ChainEligibility {
            eligible,
            winding: self.winding,
            trustworthiness,
            estimated_quality,
            suggested_step_type,
            synth_eligible,
        }
    }

    /// Prepare a chain submission for this torus.
    ///
    /// Returns None if winding < 4 (not chain-eligible).
    /// The submission includes a ReasoningStep resource and PTB commands.
    pub fn prepare_submission(
        &self,
        identity: &str,
        context: &str,
        intention: &str,
        package_id: ObjectID,
    ) -> Option<ChainSubmission> {
        let eligibility = self.chain_eligibility();
        if !eligibility.eligible {
            return None;
        }

        let provenance = ThreeKings::from_strings(identity, context, intention);
        let quality_fixed = (eligibility.estimated_quality * 1_000_000.0) as u64;

        let reasoning_step = ReasoningStep {
            id: ObjectID::zero(), // Assigned by Sui on publish
            quality: quality_fixed,
            step_type: eligibility.suggested_step_type as u8,
            provenance: provenance.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        // Build PTB for publishing the ReasoningStep
        let ptb = PtbBuilder::new()
            .gas_budget(10_000_000) // 0.01 SUI
            .move_call(
                package_id,
                "reasoning",
                "create_step",
                vec![],
                vec![
                    MoveCallArg::Pure(quality_fixed.to_le_bytes().to_vec()),
                    MoveCallArg::Pure(vec![eligibility.suggested_step_type as u8]),
                    MoveCallArg::Pure(provenance.gold.clone()),
                    MoveCallArg::Pure(provenance.myrrh.clone()),
                    MoveCallArg::Pure(provenance.frankincense.clone()),
                ],
            );

        let ptb_bytes = ptb.build().unwrap_or_default();

        Some(ChainSubmission {
            torus_id: self.id,
            reasoning_step,
            provenance,
            ptb_bytes,
        })
    }

    /// Promote this torus: refine winding AND check chain boundary.
    ///
    /// Returns Some(ChainEligibility) if the torus just crossed the chain
    /// boundary (went from winding 3 → 4), None otherwise.
    pub fn promote(&mut self) -> Option<ChainEligibility> {
        let was_below = self.winding < CHAIN_BOUNDARY_WINDING;
        if !self.refine() {
            return None; // Already at max winding
        }

        if was_below && self.winding >= CHAIN_BOUNDARY_WINDING {
            // Just crossed the boundary!
            Some(self.chain_eligibility())
        } else {
            None
        }
    }
}

impl Foam {
    /// Check all tori for chain eligibility, return those ready for submission.
    pub fn chain_eligible_tori(&self) -> Vec<(&Torus, ChainEligibility)> {
        self.tori.values()
            .map(|t| (t, t.chain_eligibility()))
            .filter(|(_, e)| e.eligible)
            .collect()
    }

    /// Get tori that are SYNTH-eligible (winding 6, low BS)
    pub fn synth_eligible_tori(&self) -> Vec<&Torus> {
        self.tori.values()
            .filter(|t| {
                let e = t.chain_eligibility();
                e.synth_eligible
            })
            .collect()
    }

    /// Count tori by chain status
    pub fn chain_stats(&self) -> ChainStats {
        let mut local = 0;
        let mut chain_eligible = 0;
        let mut synth_eligible = 0;

        for torus in self.tori.values() {
            let e = torus.chain_eligibility();
            if e.synth_eligible {
                synth_eligible += 1;
            }
            if e.eligible {
                chain_eligible += 1;
            } else {
                local += 1;
            }
        }

        ChainStats {
            local_only: local,
            chain_eligible,
            synth_eligible,
            total: self.tori.len(),
        }
    }

    /// Spawn a new torus from a chain-verified result.
    ///
    /// When a Move execution completes, the result flows back as a new torus
    /// that auto-blends with related foam. Starts at winding 4 (Tested)
    /// because it was chain-verified.
    pub fn spawn_from_chain_result(
        &mut self,
        label: &str,
        tokens: u64,
        major_radius: f64,
        parent_id: Option<[u8; 32]>,
    ) -> [u8; 32] {
        let mut torus = Torus::new(label, major_radius, tokens);
        torus.winding = CHAIN_BOUNDARY_WINDING; // Chain-verified = Tested
        torus.bs = 0.3; // Chain validation reduces BS
        torus.parent = parent_id;

        let id = torus.id;
        self.total_tokens += tokens;
        self.insert(torus);

        // Auto-blend with parent if provided
        if let Some(parent) = parent_id {
            self.blend(id, parent, 0.9);
        }

        id
    }
}

/// Statistics about foam's chain integration status
#[derive(Debug, Clone)]
pub struct ChainStats {
    /// Tori below winding 4 (local cache only)
    pub local_only: usize,
    /// Tori at winding 4+ (chain-verified)
    pub chain_eligible: usize,
    /// Tori at winding 6 with low BS (SYNTH rewards)
    pub synth_eligible: usize,
    /// Total tori in foam
    pub total: usize,
}

impl ChainStats {
    /// Ratio of chain-verified to total
    pub fn chain_ratio(&self) -> f64 {
        if self.total == 0 { 0.0 } else { self.chain_eligible as f64 / self.total as f64 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_boundary() {
        let mut torus = Torus::new("test_concept", 10.0, 500);

        // Winding 1 — not eligible
        assert!(!torus.chain_eligibility().eligible);

        // Wind up to 3 — still not eligible
        torus.winding = 3;
        assert!(!torus.chain_eligibility().eligible);

        // Wind to 4 — crosses boundary
        torus.winding = 4;
        assert!(torus.chain_eligibility().eligible);
    }

    #[test]
    fn test_promote_detects_boundary_crossing() {
        let mut torus = Torus::new("crossing", 10.0, 500);
        torus.winding = 3;

        // Promote from 3 → 4 should return Some
        let crossed = torus.promote();
        assert!(crossed.is_some());
        assert_eq!(torus.winding, 4);

        // Promote from 4 → 5 should return None (already past boundary)
        let crossed = torus.promote();
        assert!(crossed.is_none());
    }

    #[test]
    fn test_synth_eligibility() {
        let mut torus = Torus::new("production_ready", 10.0, 1000);
        torus.winding = 6;
        torus.bs = 0.2;

        let e = torus.chain_eligibility();
        assert!(e.synth_eligible);
        assert!(e.eligible);
    }

    #[test]
    fn test_synth_not_eligible_high_bs() {
        let mut torus = Torus::new("dubious", 10.0, 100);
        torus.winding = 6;
        torus.bs = 0.5; // Too much BS

        assert!(!torus.chain_eligibility().synth_eligible);
    }

    #[test]
    fn test_prepare_submission() {
        let mut torus = Torus::new("ready", 10.0, 500);
        torus.winding = 4;
        torus.bs = 0.3;

        let sub = torus.prepare_submission(
            "alice",
            "claude-opus",
            "test inference",
            ObjectID::zero(),
        );

        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.torus_id, torus.id);
        assert!(sub.reasoning_step.quality > 0);
        assert!(!sub.provenance.is_empty());
    }

    #[test]
    fn test_prepare_submission_below_boundary() {
        let torus = Torus::new("not_ready", 10.0, 100);
        // winding = 1, below boundary

        let sub = torus.prepare_submission(
            "alice", "claude-opus", "test", ObjectID::zero(),
        );
        assert!(sub.is_none());
    }

    #[test]
    fn test_foam_chain_stats() {
        let mut foam = Foam::new([0u8; 32]);

        // Add tori at various winding levels
        let mut t1 = Torus::new("local", 5.0, 100);
        t1.winding = 2;

        let mut t2 = Torus::new("chain", 5.0, 200);
        t2.winding = 4;
        t2.bs = 0.4;

        let mut t3 = Torus::new("production", 5.0, 500);
        t3.winding = 6;
        t3.bs = 0.2;

        foam.insert(t1);
        foam.insert(t2);
        foam.insert(t3);

        let stats = foam.chain_stats();
        assert_eq!(stats.local_only, 1);
        assert_eq!(stats.chain_eligible, 2);
        assert_eq!(stats.synth_eligible, 1);
        assert_eq!(stats.total, 3);
    }

    #[test]
    fn test_spawn_from_chain_result() {
        let mut foam = Foam::new([0u8; 32]);
        let parent = Torus::new("parent", 10.0, 500);
        let parent_id = parent.id;
        foam.insert(parent);

        let child_id = foam.spawn_from_chain_result("child", 200, 8.0, Some(parent_id));

        let child = foam.get(&child_id).unwrap();
        assert_eq!(child.winding, 4); // Chain-verified
        assert_eq!(child.bs, 0.3);    // Reduced BS
        assert_eq!(child.parent, Some(parent_id));

        // Should be blended with parent
        let connected = foam.connected(&child_id);
        assert!(connected.contains(&parent_id));
    }

    #[test]
    fn test_quality_mapping() {
        let mut torus = Torus::new("quality_test", 10.0, 500);
        torus.winding = 5;

        // High BS = low quality
        torus.bs = 0.9;
        assert!(torus.chain_eligibility().estimated_quality < 0.2);

        // Low BS = high quality
        torus.bs = 0.1;
        assert!(torus.chain_eligibility().estimated_quality > 0.8);
    }
}
