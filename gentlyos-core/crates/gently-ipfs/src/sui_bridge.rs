//! IPFS-Sui Bridge — Anchor CIDs on Sui
//!
//! Bulk data goes to IPFS, provenance metadata goes to Sui.
//! The bridge stores data on IPFS and records the CID as a
//! Move resource on Sui for permanent anchoring.

use crate::{IpfsClient, Result, Error};

/// Anchored content: IPFS CID + Sui object reference
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchoredContent {
    /// IPFS content identifier
    pub cid: String,
    /// Sui object ID anchoring this CID (hex string)
    pub object_id: String,
    /// Timestamp of anchoring
    pub anchored_at: chrono::DateTime<chrono::Utc>,
}

/// Bridge between IPFS storage and Sui chain anchoring
pub struct IpfsSuiBridge {
    ipfs: IpfsClient,
    // TODO: add gently_chain::SuiClient when wired
    // sui: gently_chain::SuiClient,
}

impl IpfsSuiBridge {
    /// Create a new bridge (IPFS only for now, Sui anchoring is TODO)
    pub fn new(ipfs: IpfsClient) -> Self {
        Self { ipfs }
    }

    /// Store bulk data on IPFS, anchor CID on Sui
    ///
    /// Flow:
    /// 1. Add data to IPFS → get CID
    /// 2. Pin on IPFS for persistence
    /// 3. Publish CID as Move resource on Sui (TODO)
    /// 4. Return AnchoredContent with both references
    pub async fn store_anchored(&self, data: &[u8]) -> Result<AnchoredContent> {
        // 1. Store on IPFS
        let cid = self.ipfs.add(data).await?;

        // 2. Pin
        self.ipfs.pin(&cid).await?;

        // 3. Anchor on Sui
        // TODO: wire to gently-chain SuiClient
        // let object_id = self.sui.anchor_cid(&cid).await?;
        let object_id = format!("0x{}", "0".repeat(64)); // placeholder

        Ok(AnchoredContent {
            cid,
            object_id,
            anchored_at: chrono::Utc::now(),
        })
    }

    /// Retrieve data from IPFS by CID (verifying Sui anchor exists)
    pub async fn retrieve_anchored(&self, cid: &str) -> Result<Vec<u8>> {
        // TODO: verify Sui anchor exists before fetching
        // let anchor = self.sui.get_anchor(cid).await?;

        self.ipfs.cat(cid).await
    }

    /// Check if a CID is anchored on Sui
    pub async fn is_anchored(&self, _cid: &str) -> Result<bool> {
        // TODO: query Sui for anchor object
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchored_content_serialize() {
        let content = AnchoredContent {
            cid: "QmTest123".to_string(),
            object_id: "0x0000".to_string(),
            anchored_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("QmTest123"));
    }
}
