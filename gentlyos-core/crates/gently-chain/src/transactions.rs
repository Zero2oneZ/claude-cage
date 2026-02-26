//! Programmable Transaction Block (PTB) builder
//!
//! Sui transactions are programmable — multiple Move calls composed
//! in a single atomic transaction. This is the builder.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::types::ObjectID;

/// A Move call argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoveCallArg {
    /// Pure value (BCS-encoded)
    Pure(Vec<u8>),
    /// Object reference
    Object(ObjectID),
    /// Result from a previous command in the PTB
    Result(u16),
}

/// A single command in a PTB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PtbCommand {
    /// Call a Move function
    MoveCall {
        package: ObjectID,
        module: String,
        function: String,
        type_args: Vec<String>,
        args: Vec<MoveCallArg>,
    },
    /// Transfer an object to an address
    TransferObjects {
        objects: Vec<MoveCallArg>,
        recipient: String,
    },
    /// Split a coin
    SplitCoins {
        coin: MoveCallArg,
        amounts: Vec<MoveCallArg>,
    },
    /// Merge coins
    MergeCoins {
        target: MoveCallArg,
        sources: Vec<MoveCallArg>,
    },
}

/// Builder for Programmable Transaction Blocks
pub struct PtbBuilder {
    commands: Vec<PtbCommand>,
    gas_budget: u64,
    sender: Option<String>,
}

impl PtbBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            gas_budget: 10_000_000, // 0.01 SUI default
            sender: None,
        }
    }

    /// Set gas budget (in MIST, 1 SUI = 1_000_000_000 MIST)
    pub fn gas_budget(mut self, budget: u64) -> Self {
        self.gas_budget = budget;
        self
    }

    /// Set sender address
    pub fn sender(mut self, addr: &str) -> Self {
        self.sender = Some(addr.to_string());
        self
    }

    /// Add a Move function call
    pub fn move_call(
        mut self,
        package: ObjectID,
        module: &str,
        function: &str,
        type_args: Vec<String>,
        args: Vec<MoveCallArg>,
    ) -> Self {
        self.commands.push(PtbCommand::MoveCall {
            package,
            module: module.to_string(),
            function: function.to_string(),
            type_args,
            args,
        });
        self
    }

    /// Add a transfer command
    pub fn transfer(mut self, objects: Vec<MoveCallArg>, recipient: &str) -> Self {
        self.commands.push(PtbCommand::TransferObjects {
            objects,
            recipient: recipient.to_string(),
        });
        self
    }

    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Build the PTB (returns serialized form)
    // TODO: actual BCS serialization when sui-types is available
    pub fn build(self) -> Result<Vec<u8>> {
        if self.commands.is_empty() {
            anyhow::bail!("PTB has no commands");
        }
        // Placeholder: return JSON encoding until BCS is wired
        Ok(serde_json::to_vec(&self.commands)?)
    }
}

impl Default for PtbBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a submitted transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    /// Transaction digest
    pub digest: String,
    /// Created object IDs
    pub created: Vec<ObjectID>,
    /// Mutated object IDs
    pub mutated: Vec<ObjectID>,
    /// Gas used (in MIST)
    pub gas_used: u64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptb_builder() {
        let ptb = PtbBuilder::new()
            .gas_budget(5_000_000)
            .move_call(
                ObjectID::zero(),
                "reasoning",
                "create_step",
                vec![],
                vec![MoveCallArg::Pure(vec![42])],
            );
        assert_eq!(ptb.command_count(), 1);
    }

    #[test]
    fn test_empty_ptb_fails() {
        let result = PtbBuilder::new().build();
        assert!(result.is_err());
    }
}
