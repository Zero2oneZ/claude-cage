//! Sui event subscription and filtering
//!
//! Events are emitted by Move functions and can be subscribed to
//! via WebSocket or queried via JSON-RPC.

use serde::{Deserialize, Serialize};
use crate::types::ObjectID;

/// Filter for querying events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Events from a specific package
    Package(ObjectID),
    /// Events of a specific Move type
    MoveEventType(String),
    /// Events from a specific module
    MoveModule {
        package: ObjectID,
        module: String,
    },
    /// Events mentioning a specific sender
    Sender(String),
    /// Events in a time range
    TimeRange {
        start_ms: u64,
        end_ms: u64,
    },
}

/// A Sui event (emitted by Move code)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiEvent {
    /// Event ID (tx_digest, event_seq)
    pub id: EventId,
    /// Package that emitted the event
    pub package_id: ObjectID,
    /// Module that emitted the event
    pub module: String,
    /// Move event type
    pub event_type: String,
    /// Parsed event data
    pub parsed_json: serde_json::Value,
    /// BCS-encoded event data
    pub bcs: Option<Vec<u8>>,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Unique event identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventId {
    /// Transaction digest
    pub tx_digest: String,
    /// Event sequence number within transaction
    pub event_seq: u64,
}

/// Paginated event query result
#[derive(Debug, Clone)]
pub struct EventPage {
    pub events: Vec<SuiEvent>,
    pub has_next: bool,
    pub cursor: Option<EventId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_serialize() {
        let filter = EventFilter::MoveEventType("0x1::reasoning::StepCreated".to_string());
        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("StepCreated"));
    }
}
