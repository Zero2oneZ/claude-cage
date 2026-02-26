//! Sui JSON-RPC client wrapper
//!
//! Thin client over Sui's JSON-RPC API. When sui-sdk is available as a
//! crates.io dependency, this will delegate to it. For now, raw JSON-RPC.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::{ObjectID, ReasoningStep, AnchoredContent};

/// Sui network endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiNetwork {
    Devnet,
    Testnet,
    Mainnet,
}

impl SuiNetwork {
    pub fn rpc_url(&self) -> &'static str {
        match self {
            SuiNetwork::Devnet => "https://fullnode.devnet.sui.io:443",
            SuiNetwork::Testnet => "https://fullnode.testnet.sui.io:443",
            SuiNetwork::Mainnet => "https://fullnode.mainnet.sui.io:443",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SuiNetwork::Devnet => "devnet",
            SuiNetwork::Testnet => "testnet",
            SuiNetwork::Mainnet => "mainnet",
        }
    }
}

/// Sui JSON-RPC client
pub struct SuiClient {
    network: SuiNetwork,
    rpc_url: String,
    http: reqwest::Client,
    /// Package ID where GentlyOS Move modules are published
    package_id: Option<ObjectID>,
}

impl SuiClient {
    /// Create a new client for a Sui network
    pub fn new(network: SuiNetwork) -> Self {
        Self {
            rpc_url: network.rpc_url().to_string(),
            network,
            http: reqwest::Client::new(),
            package_id: None,
        }
    }

    /// Create with custom RPC URL
    pub fn with_url(url: &str) -> Self {
        Self {
            rpc_url: url.to_string(),
            network: SuiNetwork::Devnet,
            http: reqwest::Client::new(),
            package_id: None,
        }
    }

    /// Set the GentlyOS package ID
    pub fn set_package(&mut self, id: ObjectID) {
        self.package_id = Some(id);
    }

    /// Get current network
    pub fn network(&self) -> SuiNetwork {
        self.network
    }

    /// Raw JSON-RPC call
    async fn rpc_call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self.http
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }

        let result = json.get("result")
            .ok_or_else(|| anyhow::anyhow!("Missing result in RPC response"))?;

        Ok(serde_json::from_value(result.clone())?)
    }

    /// Publish a reasoning step as a Move resource
    // TODO: implement actual PTB construction when sui-sdk is available
    pub async fn publish_reasoning_step(&self, _step: ReasoningStep) -> Result<ObjectID> {
        // Placeholder — will build a Programmable Transaction Block that calls
        // gentlyos::reasoning::create_step(quality, step_type, gold, myrrh, frankincense)
        Ok(ObjectID::zero())
    }

    /// Anchor an IPFS CID on Sui
    // TODO: implement actual PTB construction
    pub async fn anchor_cid(&self, cid: &str) -> Result<ObjectID> {
        // Placeholder — will call gentlyos::ipfs::anchor(cid_bytes)
        let _ = cid;
        Ok(ObjectID::zero())
    }

    /// Check if connected (ping the node)
    pub async fn is_connected(&self) -> bool {
        // TODO: call sui_getLatestCheckpointSequenceNumber
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_urls() {
        assert!(SuiNetwork::Devnet.rpc_url().contains("devnet"));
        assert!(SuiNetwork::Testnet.rpc_url().contains("testnet"));
        assert!(SuiNetwork::Mainnet.rpc_url().contains("mainnet"));
    }

    #[test]
    fn test_client_creation() {
        let client = SuiClient::new(SuiNetwork::Devnet);
        assert_eq!(client.network(), SuiNetwork::Devnet);
    }
}
