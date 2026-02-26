//! Fire-and-forget storage for PTC events and artifacts.
//!
//! The `PtcStorage` trait allows pluggable persistence backends.
//! `NullStorage` is a no-op implementation for testing and dry runs.

use crate::phase::Phase;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A PTC event to be persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtcEvent {
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Which phase the event belongs to
    pub phase: Phase,
    /// Event type identifier (e.g., "intent_received", "task_completed")
    pub event_type: String,
    /// Arbitrary event data
    pub data: serde_json::Value,
}

/// Async trait for storing PTC events and artifacts.
#[async_trait]
pub trait PtcStorage: Send + Sync {
    /// Store an event record. Fire-and-forget — errors are logged, not fatal.
    async fn store_event(&self, event: &PtcEvent) -> Result<()>;

    /// Store a binary artifact by ID. Fire-and-forget.
    async fn store_artifact(&self, id: Uuid, data: &[u8]) -> Result<()>;
}

/// A no-op storage backend that discards everything.
///
/// Useful for testing, dry runs, and when persistence is not needed.
pub struct NullStorage;

#[async_trait]
impl PtcStorage for NullStorage {
    async fn store_event(&self, _event: &PtcEvent) -> Result<()> {
        Ok(())
    }

    async fn store_artifact(&self, _id: Uuid, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}
