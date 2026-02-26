//! # ML Model Integration — Cascade Chain
//!
//! The cascade module provides a trait-based interface for chaining
//! ML models in the GOO pipeline. Models transform field data
//! (SDF samples, attention scores, gradient vectors) through a
//! sequence of processing steps.
//!
//! ## Architecture
//!
//! ```text
//! Input → Model A → Model B → Model C → Output
//!         (embed)   (transform) (classify)
//! ```
//!
//! The `CascadeModel` trait defines the interface. `CascadeChain`
//! chains multiple models sequentially. `IdentityModel` is the
//! default pass-through for when no ML is needed.

use serde::{Deserialize, Serialize};

/// Trait for ML models in the cascade pipeline.
///
/// Each model takes a float vector and produces a float vector.
/// The input/output dimensions can differ (for embedding, projection, etc.).
pub trait CascadeModel: Send + Sync {
    /// Transform input data through this model.
    fn predict(&self, input: &[f32]) -> Vec<f32>;

    /// Human-readable model name.
    fn name(&self) -> &str;

    /// Expected input dimension (0 = any).
    fn input_dim(&self) -> usize {
        0
    }

    /// Expected output dimension (0 = same as input).
    fn output_dim(&self) -> usize {
        0
    }
}

/// Identity model — returns input unchanged.
///
/// Used as the default when no ML processing is needed.
/// Also useful as a placeholder in cascade chains during development.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityModel {
    label: String,
}

impl IdentityModel {
    pub fn new() -> Self {
        Self {
            label: "identity".to_string(),
        }
    }

    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl Default for IdentityModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeModel for IdentityModel {
    fn predict(&self, input: &[f32]) -> Vec<f32> {
        input.to_vec()
    }

    fn name(&self) -> &str {
        &self.label
    }
}

/// A chain of cascade models applied sequentially.
///
/// Output of model N becomes input to model N+1.
/// An empty chain behaves like IdentityModel.
pub struct CascadeChain {
    models: Vec<Box<dyn CascadeModel>>,
    label: String,
}

impl CascadeChain {
    /// Create an empty cascade chain.
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            label: "cascade".to_string(),
        }
    }

    /// Create a cascade chain with a descriptive label.
    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            models: Vec::new(),
            label: label.into(),
        }
    }

    /// Add a model to the end of the chain.
    pub fn push(&mut self, model: Box<dyn CascadeModel>) {
        self.models.push(model);
    }

    /// Add a model to the chain (builder pattern).
    pub fn with(mut self, model: Box<dyn CascadeModel>) -> Self {
        self.models.push(model);
        self
    }

    /// Number of models in the chain.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Get model names in order.
    pub fn model_names(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.name()).collect()
    }
}

impl Default for CascadeChain {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeModel for CascadeChain {
    fn predict(&self, input: &[f32]) -> Vec<f32> {
        let mut data = input.to_vec();
        for model in &self.models {
            data = model.predict(&data);
        }
        data
    }

    fn name(&self) -> &str {
        &self.label
    }
}

/// A scaling model — multiplies all inputs by a constant factor.
///
/// Useful for normalization or amplitude adjustment in the cascade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleModel {
    factor: f32,
}

impl ScaleModel {
    pub fn new(factor: f32) -> Self {
        Self { factor }
    }
}

impl CascadeModel for ScaleModel {
    fn predict(&self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|x| x * self.factor).collect()
    }

    fn name(&self) -> &str {
        "scale"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_model() {
        let model = IdentityModel::new();
        let input = vec![1.0, 2.0, 3.0];
        let output = model.predict(&input);
        assert_eq!(input, output);
    }

    #[test]
    fn test_cascade_chain_empty() {
        let chain = CascadeChain::new();
        assert!(chain.is_empty());
        let input = vec![1.0, 2.0];
        let output = chain.predict(&input);
        assert_eq!(input, output);
    }

    #[test]
    fn test_cascade_chain_with_models() {
        let chain = CascadeChain::new()
            .with(Box::new(ScaleModel::new(2.0)))
            .with(Box::new(ScaleModel::new(3.0)));

        assert_eq!(chain.len(), 2);

        let input = vec![1.0, 2.0, 3.0];
        let output = chain.predict(&input);

        // 1.0 * 2.0 * 3.0 = 6.0, etc.
        assert_eq!(output, vec![6.0, 12.0, 18.0]);
    }

    #[test]
    fn test_model_names() {
        let chain = CascadeChain::new()
            .with(Box::new(IdentityModel::with_label("embed")))
            .with(Box::new(ScaleModel::new(1.0)));

        let names = chain.model_names();
        assert_eq!(names, vec!["embed", "scale"]);
    }
}
