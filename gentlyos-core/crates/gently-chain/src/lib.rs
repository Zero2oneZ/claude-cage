//! GentlyOS Chain Layer — Sui/Move Economic Integration
//!
//! Move linear types replace JSON schemas. Resources are physical.
//! The compiler is the physics engine. The type system IS the schema.
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │              gently-chain (Sui/Move)              │
//! ├──────────────────────────────────────────────────┤
//! │  client.rs    │ JSON-RPC wrapper for Sui devnet  │
//! │  objects.rs   │ Object read/query (core Sui)     │
//! │  transactions │ PTB builder (programmable txs)   │
//! │  events.rs    │ Event subscription + filtering   │
//! │  types.rs     │ Move resource type mappings      │
//! │  three_kings  │ Gold/Myrrh/Frankincense metadata │
//! └──────────────────────────────────────────────────┘
//! ```

#![allow(dead_code, unused_variables, unused_imports)]

pub mod client;
pub mod objects;
pub mod transactions;
pub mod events;
pub mod types;
pub mod three_kings;
pub mod transpile;

pub use client::SuiClient;
pub use objects::{SuiObject, ObjectQuery};
pub use transactions::{PtbBuilder, TransactionResult};
pub use events::{EventFilter, SuiEvent};
pub use types::{ReasoningStep, ObjectID};
pub use three_kings::ThreeKings;
pub use transpile::{MoveModule, codie_to_move, source_to_move};
