//! Query Pipeline — The complete data flow
//!
//! ```text
//! QUERY ARRIVES
//!      │
//!      ▼
//! BARF RETRIEVAL ──── HIT (winding ≥ 4) ──→ Return cached
//!      │                                     Cost: zero
//!      │ MISS
//!      ▼
//! ALEXANDRIA ROUTE ── 3 BBBCP passes
//!      │               2.7% survives
//!      ▼
//! CODIE→MOVE ──────── Transpile to PTB
//!      │
//!      ▼
//! SUI EXECUTES ────── Resource in → resource out
//!      │
//!      ▼
//! RESULT FLOWS BACK ─ New torus spawns in foam
//!      │               Alexandria topology updates
//!      ▼
//! SYNTH REWARD ────── If quality ≥ 0.7, Three Kings stored
//! ```

use anyhow::Result;
use crate::types::{ReasoningStep, ObjectID};
use crate::three_kings::ThreeKings;

/// Outcome of a pipeline query
#[derive(Debug, Clone)]
pub enum PipelineResult {
    /// Cache hit from BARF — no chain interaction needed
    CacheHit {
        /// The cached torus label
        label: String,
        /// Torus ID that matched
        torus_id: [u8; 32],
        /// BARF distance score (lower = better match)
        distance: f64,
        /// Trustworthiness of the cached result
        trustworthiness: f64,
        /// Winding level of the cached torus
        winding: u8,
    },

    /// Cache miss — routed through Alexandria and submitted to chain
    ChainExecuted {
        /// The ReasoningStep published on-chain
        step: ReasoningStep,
        /// Target module that handled the execution
        target_module: String,
        /// Routing confidence from BBBCP collapse
        routing_confidence: f64,
        /// New torus ID spawned from result
        new_torus_id: Option<[u8; 32]>,
        /// Whether SYNTH reward was earned
        synth_rewarded: bool,
    },

    /// Routed but kept local (quality too low or convergence too poor)
    LocalOnly {
        /// Reason for staying local
        reason: String,
        /// Estimated quality
        quality: f64,
    },
}

impl PipelineResult {
    /// Whether this result hit the chain
    pub fn hit_chain(&self) -> bool {
        matches!(self, PipelineResult::ChainExecuted { .. })
    }

    /// Whether this was a cache hit
    pub fn was_cached(&self) -> bool {
        matches!(self, PipelineResult::CacheHit { .. })
    }

    /// Whether SYNTH was rewarded
    pub fn synth_rewarded(&self) -> bool {
        match self {
            PipelineResult::ChainExecuted { synth_rewarded, .. } => *synth_rewarded,
            _ => false,
        }
    }
}

/// Configuration for the pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Minimum winding level for BARF cache hits to be trusted
    pub min_cache_winding: u8,

    /// Maximum BARF distance for a hit to be accepted
    pub max_cache_distance: f64,

    /// Minimum quality threshold for chain submission
    pub min_chain_quality: f64,

    /// Minimum routing confidence for chain submission
    pub min_routing_confidence: f64,

    /// Quality threshold for SYNTH rewards
    pub synth_quality_threshold: f64,

    /// Number of BBBCP constraint passes
    pub bbbcp_passes: u32,

    /// Elimination rate per BBBCP pass
    pub elimination_rate: f64,

    /// Gas budget for Sui transactions (in MIST)
    pub gas_budget: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            min_cache_winding: 4,
            max_cache_distance: 0.3,
            min_chain_quality: 0.4,
            min_routing_confidence: 0.5,
            synth_quality_threshold: 0.7,
            bbbcp_passes: 3,
            elimination_rate: 0.7,
            gas_budget: 10_000_000, // 0.01 SUI
        }
    }
}

impl PipelineConfig {
    /// Calculate expected search space remaining after BBBCP passes
    ///
    /// Formula: |surviving| = |Ω| × (1 - elimination_rate)^passes
    pub fn expected_search_remaining(&self) -> f64 {
        (1.0 - self.elimination_rate).powi(self.bbbcp_passes as i32)
    }

    /// Validate config values
    pub fn validate(&self) -> Result<()> {
        if self.elimination_rate <= 0.0 || self.elimination_rate >= 1.0 {
            anyhow::bail!("elimination_rate must be between 0 and 1, got {}", self.elimination_rate);
        }
        if self.synth_quality_threshold < 0.0 || self.synth_quality_threshold > 1.0 {
            anyhow::bail!("synth_quality_threshold must be between 0 and 1");
        }
        Ok(())
    }
}

/// Pipeline statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Total queries processed
    pub total_queries: u64,
    /// Cache hits (BARF)
    pub cache_hits: u64,
    /// Chain executions
    pub chain_executions: u64,
    /// Local-only results
    pub local_only: u64,
    /// SYNTH rewards earned
    pub synth_rewards: u64,
    /// Total gas spent (in MIST)
    pub total_gas_spent: u64,
}

impl PipelineStats {
    /// Cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        if self.total_queries == 0 { 0.0 }
        else { self.cache_hits as f64 / self.total_queries as f64 }
    }

    /// Chain execution ratio
    pub fn chain_ratio(&self) -> f64 {
        if self.total_queries == 0 { 0.0 }
        else { self.chain_executions as f64 / self.total_queries as f64 }
    }

    /// Average gas per chain execution
    pub fn avg_gas_per_execution(&self) -> f64 {
        if self.chain_executions == 0 { 0.0 }
        else { self.total_gas_spent as f64 / self.chain_executions as f64 }
    }

    /// Record a cache hit
    pub fn record_cache_hit(&mut self) {
        self.total_queries += 1;
        self.cache_hits += 1;
    }

    /// Record a chain execution
    pub fn record_chain_execution(&mut self, gas: u64, synth: bool) {
        self.total_queries += 1;
        self.chain_executions += 1;
        self.total_gas_spent += gas;
        if synth { self.synth_rewards += 1; }
    }

    /// Record a local-only result
    pub fn record_local_only(&mut self) {
        self.total_queries += 1;
        self.local_only += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convergence_formula() {
        let config = PipelineConfig::default();
        let remaining = config.expected_search_remaining();
        // 3 passes at 70% → (0.3)^3 = 0.027 = 2.7%
        assert!((remaining - 0.027).abs() < 0.001);
    }

    #[test]
    fn test_config_validation() {
        let mut config = PipelineConfig::default();
        assert!(config.validate().is_ok());

        config.elimination_rate = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pipeline_stats() {
        let mut stats = PipelineStats::default();

        stats.record_cache_hit();
        stats.record_cache_hit();
        stats.record_chain_execution(5_000_000, true);
        stats.record_local_only();

        assert_eq!(stats.total_queries, 4);
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.chain_executions, 1);
        assert_eq!(stats.synth_rewards, 1);
        assert_eq!(stats.cache_hit_ratio(), 0.5);
    }

    #[test]
    fn test_pipeline_result_predicates() {
        let hit = PipelineResult::CacheHit {
            label: "test".to_string(),
            torus_id: [0u8; 32],
            distance: 0.1,
            trustworthiness: 0.9,
            winding: 5,
        };
        assert!(hit.was_cached());
        assert!(!hit.hit_chain());
        assert!(!hit.synth_rewarded());

        let chain = PipelineResult::ChainExecuted {
            step: ReasoningStep::default(),
            target_module: "reasoning".to_string(),
            routing_confidence: 0.95,
            new_torus_id: Some([1u8; 32]),
            synth_rewarded: true,
        };
        assert!(!chain.was_cached());
        assert!(chain.hit_chain());
        assert!(chain.synth_rewarded());
    }
}
