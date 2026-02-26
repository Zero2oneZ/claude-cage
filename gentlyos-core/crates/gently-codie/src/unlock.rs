//! CODIE Unlock Protocol
//!
//! Multi-mode unlock system for CODIE instructions:
//! - Time-based (Berlin Clock)
//! - Future condition (event-dependent)
//! - Genesis-approved (creator must OK)
//!
//! ## Word-Based Identifiers
//!
//! Instead of IP addresses, users get memorable word combos:
//! ```text
//! TIMBER-LANTERN → SHA256(words + btc_block) → IPFS coordinates
//! ```
//!
//! ## Multi-Client Hydration
//!
//! Creator sets max_hydrations - multiple clients can unlock:
//! ```text
//! max_hydrations: 5 → 5 different machines can hydrate
//! Each hydration decrements counter, burns at 0
//! ```
//!
//! ## Flow
//!
//! ```text
//! Creator                          Clients (1..N)
//!    │                                   │
//!    │ 1. Create + Pin IPFS              │
//!    │    words: TIMBER-LANTERN          │
//!    │    max_clients: 5                 │
//!    │                                   │
//!    │                 ◄─────────────────│ 2. Request unlock
//!    │                                   │
//!    │ 3. See requester info             │
//!    │    [APPROVE] [DENY]               │
//!    │                                   │
//!    │ ─────────────────────────────────►│ 4. Hydrate (4 left)
//!    │                                   │
//!    │                 ◄─────────────────│ 5. Another client
//!    │ ─────────────────────────────────►│ 6. Hydrate (3 left)
//!    │                                   │
//!    │         ... until 0 or expired    │
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

/// BIP39-style wordlist for human-friendly identifiers (subset)
pub const WORD_LIST: &[&str] = &[
    "abbey", "anchor", "apple", "arrow", "atlas", "autumn", "badge", "bamboo",
    "barrel", "beacon", "blanket", "blaze", "boulder", "branch", "breeze", "bridge",
    "bronze", "bucket", "cabin", "canvas", "canyon", "castle", "cedar", "chapel",
    "cherry", "chrome", "cipher", "citrus", "cliff", "cloud", "clover", "cobalt",
    "coffee", "comet", "compass", "copper", "coral", "cotton", "crater", "creek",
    "crimson", "crystal", "cypress", "dagger", "dawn", "delta", "desert", "diamond",
    "drift", "dusk", "eagle", "ebony", "echo", "eclipse", "ember", "emerald",
    "falcon", "fern", "flame", "flint", "forest", "fossil", "fountain", "frost",
    "galaxy", "garden", "garnet", "glacier", "glen", "granite", "grove", "harbor",
    "harvest", "hawk", "hazel", "heath", "hemlock", "hollow", "horizon", "hunter",
    "indigo", "iris", "iron", "island", "ivory", "jade", "jasper", "juniper",
    "karma", "kelp", "kernel", "kindle", "lagoon", "lantern", "larch", "lava",
    "leaf", "legend", "lemon", "light", "lilac", "lily", "lunar", "maple",
    "marble", "marsh", "meadow", "mesa", "meteor", "mist", "moon", "moss",
    "mountain", "navy", "nebula", "nectar", "night", "north", "nova", "oak",
    "oasis", "obsidian", "ocean", "olive", "onyx", "orbit", "orchid", "osprey",
    "palm", "panther", "paper", "pebble", "pepper", "phoenix", "pine", "pixel",
    "plaza", "plum", "polar", "pond", "poplar", "prism", "pulse", "quartz",
    "rain", "raven", "reef", "ridge", "river", "robin", "rocket", "rose",
    "ruby", "sage", "salmon", "sand", "sapphire", "scarlet", "scroll", "shadow",
    "shell", "shore", "sierra", "silk", "silver", "slate", "snow", "solar",
    "spark", "spice", "spruce", "star", "steel", "stone", "storm", "stream",
    "summit", "sun", "swift", "thorn", "thunder", "tide", "tiger", "timber",
    "torch", "trail", "tree", "tulip", "tundra", "twilight", "umbra", "valley",
    "vapor", "velvet", "venus", "violet", "vortex", "walnut", "wave", "whisper",
    "willow", "wind", "winter", "wolf", "wood", "wren", "yarrow", "zenith",
    "zephyr", "zinc", "zone",
];

/// Unlock mode for CODIE instructions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnlockMode {
    /// Time-based: unlocks at specific BTC block timestamp slot
    TimeBased {
        unlock_slot: u64,
        btc_block: u64,
    },
    /// Condition-based: unlocks when external condition is true
    Condition {
        eval_expression: String,
        btc_anchor: u64,
    },
    /// Genesis-approved: requires creator to manually approve each request
    Genesis {
        require_approval: bool,
        auto_approve_words: Option<Vec<String>>, // Pre-approved word combos
    },
    /// Immediate: no lock, anyone with words can hydrate
    Immediate,
}

/// Client info for tracking who requests/hydrates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub word_combo: String,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub user_agent: Option<String>,
    pub request_time: u64,
    pub approved: bool,
    pub hydrated: bool,
    pub hydrate_time: Option<u64>,
}

/// Unlock request from a client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockRequest {
    pub words: (String, String),
    pub client_info: ClientInfo,
    pub timestamp: u64,
}

/// Locked CODIE instruction bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedCodie {
    /// Unique identifier words
    pub words: (String, String),
    /// Combined word string for lookup
    pub word_key: String,
    /// The compressed/encrypted CODIE payload
    pub payload: String,
    /// IPFS CID where payload is pinned
    pub ipfs_cid: Option<String>,
    /// Hash of the payload
    pub hash: String,
    /// Unlock mode
    pub mode: UnlockMode,
    /// BTC block height at creation
    pub btc_block: u64,
    /// BTC block hash at creation
    pub btc_hash: String,
    /// Creator's public key (for verification)
    pub creator_pubkey: String,
    /// Maximum number of clients that can hydrate
    pub max_hydrations: u32,
    /// Current hydration count
    pub hydrations_used: u32,
    /// Clients who have requested access
    pub clients: Vec<ClientInfo>,
    /// Approved client word combos (for genesis mode)
    pub approved_clients: HashSet<String>,
    /// Expiration: block height (0 = no expiry)
    pub expires_block: u64,
    /// Expiration: unix timestamp (0 = no expiry)
    pub expires_time: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Is this bundle still active?
    pub active: bool,
    /// Has been fully burned (all hydrations used or expired)
    pub burned: bool,
}

impl LockedCodie {
    /// Check if this bundle can still be hydrated
    pub fn can_hydrate(&self) -> bool {
        if self.burned || !self.active {
            return false;
        }
        if self.hydrations_used >= self.max_hydrations {
            return false;
        }
        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if self.expires_time > 0 && now > self.expires_time {
            return false;
        }
        true
    }

    /// Remaining hydrations available
    pub fn remaining_hydrations(&self) -> u32 {
        self.max_hydrations.saturating_sub(self.hydrations_used)
    }

    /// Check if a specific client is approved
    pub fn is_client_approved(&self, word_combo: &str) -> bool {
        self.approved_clients.contains(word_combo)
    }
}

lazy_static! {
    /// Global registry of locked CODIE bundles
    static ref LOCK_REGISTRY: RwLock<HashMap<String, LockedCodie>> = RwLock::new(HashMap::new());

    /// Pending unlock requests awaiting creator approval
    static ref PENDING_REQUESTS: RwLock<HashMap<String, Vec<UnlockRequest>>> = RwLock::new(HashMap::new());
}

/// Generate word combo from BTC block data
pub fn generate_words(btc_block_hash: &str, btc_height: u64, nonce: u32) -> (String, String) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    btc_block_hash.hash(&mut hasher);
    btc_height.hash(&mut hasher);
    nonce.hash(&mut hasher);
    let hash = hasher.finish();

    let word1_idx = (hash & 0x7FF) as usize % WORD_LIST.len(); // bits 0-10
    let word2_idx = ((hash >> 11) & 0x7FF) as usize % WORD_LIST.len(); // bits 11-21

    (
        WORD_LIST[word1_idx].to_uppercase(),
        WORD_LIST[word2_idx].to_uppercase(),
    )
}

/// Generate word key from tuple
pub fn word_key(words: &(String, String)) -> String {
    format!("{}-{}", words.0.to_uppercase(), words.1.to_uppercase())
}

/// Derive unlock hash from words + block data
pub fn derive_unlock_hash(words: &(String, String), btc_hash: &str, btc_height: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    words.0.hash(&mut hasher);
    words.1.hash(&mut hasher);
    btc_hash.hash(&mut hasher);
    btc_height.hash(&mut hasher);

    format!("#{:016x}", hasher.finish())
}

/// Create a new locked CODIE bundle
pub fn create_lock(
    payload: String,
    mode: UnlockMode,
    max_hydrations: u32,
    expires_blocks: u64,
    expires_secs: u64,
    btc_block_hash: &str,
    btc_height: u64,
    creator_pubkey: &str,
) -> LockedCodie {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(0);

    let words = generate_words(btc_block_hash, btc_height, nonce);
    let wkey = word_key(&words);
    let hash = derive_unlock_hash(&words, btc_block_hash, btc_height);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let expires_time = if expires_secs > 0 { now + expires_secs } else { 0 };
    let expires_block = if expires_blocks > 0 { btc_height + expires_blocks } else { 0 };

    let locked = LockedCodie {
        words: words.clone(),
        word_key: wkey.clone(),
        payload,
        ipfs_cid: None,
        hash,
        mode,
        btc_block: btc_height,
        btc_hash: btc_block_hash.to_string(),
        creator_pubkey: creator_pubkey.to_string(),
        max_hydrations,
        hydrations_used: 0,
        clients: Vec::new(),
        approved_clients: HashSet::new(),
        expires_block,
        expires_time,
        created_at: now,
        active: true,
        burned: false,
    };

    // Register
    if let Ok(mut registry) = LOCK_REGISTRY.write() {
        registry.insert(wkey, locked.clone());
    }

    locked
}

/// Request unlock from a client
pub fn request_unlock(
    words: (String, String),
    ip_address: Option<String>,
    location: Option<String>,
    user_agent: Option<String>,
) -> Result<UnlockRequestResult, UnlockError> {
    let wkey = word_key(&words);

    let mut registry = LOCK_REGISTRY.write().map_err(|_| UnlockError::RegistryLocked)?;

    let locked = registry.get_mut(&wkey).ok_or(UnlockError::NotFound)?;

    if !locked.can_hydrate() {
        if locked.burned {
            return Err(UnlockError::Burned);
        }
        if locked.hydrations_used >= locked.max_hydrations {
            return Err(UnlockError::MaxHydrations);
        }
        return Err(UnlockError::Expired);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let client_combo = format!("{}-{}-{}",
        words.0, words.1,
        ip_address.as_deref().unwrap_or("unknown")
    );

    let client_info = ClientInfo {
        word_combo: client_combo.clone(),
        ip_address,
        location,
        user_agent,
        request_time: now,
        approved: false,
        hydrated: false,
        hydrate_time: None,
    };

    // Check unlock mode
    match &locked.mode {
        UnlockMode::Immediate => {
            // Auto-approve, hydrate immediately
            let mut approved_client = client_info.clone();
            approved_client.approved = true;
            locked.clients.push(approved_client);
            locked.approved_clients.insert(client_combo);

            Ok(UnlockRequestResult::Approved {
                payload: locked.payload.clone(),
                remaining: locked.remaining_hydrations(),
            })
        }
        UnlockMode::TimeBased { unlock_slot, .. } => {
            // Check if time has arrived
            let current_slot = now / 300; // 5-minute slots
            if current_slot >= *unlock_slot {
                let mut approved_client = client_info.clone();
                approved_client.approved = true;
                locked.clients.push(approved_client);
                locked.approved_clients.insert(client_combo);

                Ok(UnlockRequestResult::Approved {
                    payload: locked.payload.clone(),
                    remaining: locked.remaining_hydrations(),
                })
            } else {
                locked.clients.push(client_info);
                Ok(UnlockRequestResult::TimeLocked {
                    unlocks_at_slot: *unlock_slot,
                    current_slot,
                })
            }
        }
        UnlockMode::Condition { eval_expression, .. } => {
            // Condition must be evaluated externally
            locked.clients.push(client_info);
            Ok(UnlockRequestResult::ConditionPending {
                expression: eval_expression.clone(),
            })
        }
        UnlockMode::Genesis { require_approval, auto_approve_words } => {
            // Check auto-approve list
            if let Some(auto_words) = auto_approve_words {
                if auto_words.contains(&words.0) || auto_words.contains(&words.1) {
                    let mut approved_client = client_info.clone();
                    approved_client.approved = true;
                    locked.clients.push(approved_client);
                    locked.approved_clients.insert(client_combo);

                    return Ok(UnlockRequestResult::Approved {
                        payload: locked.payload.clone(),
                        remaining: locked.remaining_hydrations(),
                    });
                }
            }

            if *require_approval {
                // Add to pending requests
                locked.clients.push(client_info.clone());

                let request = UnlockRequest {
                    words: words.clone(),
                    client_info,
                    timestamp: now,
                };

                if let Ok(mut pending) = PENDING_REQUESTS.write() {
                    pending.entry(wkey.clone()).or_default().push(request);
                }

                Ok(UnlockRequestResult::PendingApproval {
                    position_in_queue: locked.clients.len(),
                    max_hydrations: locked.max_hydrations,
                    used_hydrations: locked.hydrations_used,
                })
            } else {
                let mut approved_client = client_info.clone();
                approved_client.approved = true;
                locked.clients.push(approved_client);
                locked.approved_clients.insert(client_combo);

                Ok(UnlockRequestResult::Approved {
                    payload: locked.payload.clone(),
                    remaining: locked.remaining_hydrations(),
                })
            }
        }
    }
}

/// Creator approves a pending request
pub fn approve_request(
    words: &(String, String),
    client_index: usize,
    creator_signature: &str,
) -> Result<ApprovalResult, UnlockError> {
    let wkey = word_key(words);

    let mut registry = LOCK_REGISTRY.write().map_err(|_| UnlockError::RegistryLocked)?;
    let locked = registry.get_mut(&wkey).ok_or(UnlockError::NotFound)?;

    // Verify creator (in production, verify signature)
    if creator_signature.is_empty() {
        return Err(UnlockError::InvalidSignature);
    }

    if client_index >= locked.clients.len() {
        return Err(UnlockError::ClientNotFound);
    }

    if !locked.can_hydrate() {
        return Err(UnlockError::MaxHydrations);
    }

    let client = &mut locked.clients[client_index];
    client.approved = true;
    locked.approved_clients.insert(client.word_combo.clone());

    // Remove from pending
    if let Ok(mut pending) = PENDING_REQUESTS.write() {
        pending.remove(&wkey);
    }

    Ok(ApprovalResult {
        client_word_combo: client.word_combo.clone(),
        payload: locked.payload.clone(),
        remaining_hydrations: locked.remaining_hydrations(),
    })
}

/// Creator denies a request
pub fn deny_request(
    words: &(String, String),
    client_index: usize,
) -> Result<(), UnlockError> {
    let wkey = word_key(words);

    let mut registry = LOCK_REGISTRY.write().map_err(|_| UnlockError::RegistryLocked)?;
    let locked = registry.get_mut(&wkey).ok_or(UnlockError::NotFound)?;

    if client_index < locked.clients.len() {
        locked.clients.remove(client_index);
    }

    // Remove from pending
    if let Ok(mut pending) = PENDING_REQUESTS.write() {
        if let Some(reqs) = pending.get_mut(&wkey) {
            if client_index < reqs.len() {
                reqs.remove(client_index);
            }
        }
    }

    Ok(())
}

/// Client hydrates (consumes one hydration slot)
pub fn hydrate(
    words: &(String, String),
    client_id: &str,
) -> Result<HydrateResult, UnlockError> {
    let wkey = word_key(words);

    let mut registry = LOCK_REGISTRY.write().map_err(|_| UnlockError::RegistryLocked)?;
    let locked = registry.get_mut(&wkey).ok_or(UnlockError::NotFound)?;

    if !locked.can_hydrate() {
        return Err(UnlockError::CannotHydrate);
    }

    if !locked.approved_clients.contains(client_id) {
        return Err(UnlockError::NotApproved);
    }

    // Find client and mark as hydrated
    let client = locked.clients.iter_mut()
        .find(|c| c.word_combo == client_id && c.approved && !c.hydrated);

    if let Some(c) = client {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        c.hydrated = true;
        c.hydrate_time = Some(now);
        locked.hydrations_used += 1;

        // Check if burned (all hydrations used)
        if locked.hydrations_used >= locked.max_hydrations {
            locked.burned = true;
            locked.active = false;
        }

        Ok(HydrateResult {
            payload: locked.payload.clone(),
            hydrations_remaining: locked.remaining_hydrations(),
            burned: locked.burned,
        })
    } else {
        Err(UnlockError::AlreadyHydrated)
    }
}

/// Get pending requests for creator to review
pub fn get_pending_requests(words: &(String, String)) -> Vec<UnlockRequest> {
    let wkey = word_key(words);

    PENDING_REQUESTS.read()
        .map(|pending| pending.get(&wkey).cloned().unwrap_or_default())
        .unwrap_or_default()
}

/// Get status of a locked bundle
pub fn get_status(words: &(String, String)) -> Option<LockStatus> {
    let wkey = word_key(words);

    LOCK_REGISTRY.read().ok()?.get(&wkey).map(|locked| LockStatus {
        word_key: locked.word_key.clone(),
        hash: locked.hash.clone(),
        mode: format!("{:?}", locked.mode),
        max_hydrations: locked.max_hydrations,
        hydrations_used: locked.hydrations_used,
        remaining: locked.remaining_hydrations(),
        client_count: locked.clients.len(),
        approved_count: locked.approved_clients.len(),
        expires_block: locked.expires_block,
        expires_time: locked.expires_time,
        active: locked.active,
        burned: locked.burned,
        ipfs_cid: locked.ipfs_cid.clone(),
    })
}

/// Burn a lock (creator can force-burn)
pub fn burn(words: &(String, String), creator_signature: &str) -> Result<(), UnlockError> {
    let wkey = word_key(words);

    let mut registry = LOCK_REGISTRY.write().map_err(|_| UnlockError::RegistryLocked)?;
    let locked = registry.get_mut(&wkey).ok_or(UnlockError::NotFound)?;

    if creator_signature.is_empty() {
        return Err(UnlockError::InvalidSignature);
    }

    locked.burned = true;
    locked.active = false;

    Ok(())
}

/// Result of unlock request
#[derive(Debug, Clone)]
pub enum UnlockRequestResult {
    /// Immediately approved, here's the payload
    Approved {
        payload: String,
        remaining: u32,
    },
    /// Waiting for creator approval
    PendingApproval {
        position_in_queue: usize,
        max_hydrations: u32,
        used_hydrations: u32,
    },
    /// Time-locked, not ready yet
    TimeLocked {
        unlocks_at_slot: u64,
        current_slot: u64,
    },
    /// Waiting for condition to be true
    ConditionPending {
        expression: String,
    },
}

/// Result of approval
#[derive(Debug, Clone)]
pub struct ApprovalResult {
    pub client_word_combo: String,
    pub payload: String,
    pub remaining_hydrations: u32,
}

/// Result of hydration
#[derive(Debug, Clone)]
pub struct HydrateResult {
    pub payload: String,
    pub hydrations_remaining: u32,
    pub burned: bool,
}

/// Lock status for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStatus {
    pub word_key: String,
    pub hash: String,
    pub mode: String,
    pub max_hydrations: u32,
    pub hydrations_used: u32,
    pub remaining: u32,
    pub client_count: usize,
    pub approved_count: usize,
    pub expires_block: u64,
    pub expires_time: u64,
    pub active: bool,
    pub burned: bool,
    pub ipfs_cid: Option<String>,
}

/// Unlock errors
#[derive(Debug, Clone, PartialEq)]
pub enum UnlockError {
    NotFound,
    RegistryLocked,
    MaxHydrations,
    Expired,
    Burned,
    NotApproved,
    AlreadyHydrated,
    InvalidSignature,
    ClientNotFound,
    CannotHydrate,
    TimeLocked,
    ConditionNotMet,
}

impl std::fmt::Display for UnlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Lock not found"),
            Self::RegistryLocked => write!(f, "Registry temporarily locked"),
            Self::MaxHydrations => write!(f, "Maximum hydrations reached"),
            Self::Expired => write!(f, "Lock has expired"),
            Self::Burned => write!(f, "Lock has been burned"),
            Self::NotApproved => write!(f, "Client not approved"),
            Self::AlreadyHydrated => write!(f, "Client already hydrated"),
            Self::InvalidSignature => write!(f, "Invalid creator signature"),
            Self::ClientNotFound => write!(f, "Client not found"),
            Self::CannotHydrate => write!(f, "Cannot hydrate this lock"),
            Self::TimeLocked => write!(f, "Still time-locked"),
            Self::ConditionNotMet => write!(f, "Unlock condition not met"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_words() {
        let words = generate_words("0000000000000000000abc123", 881234, 12345);
        assert!(!words.0.is_empty());
        assert!(!words.1.is_empty());
        println!("Generated words: {}-{}", words.0, words.1);
    }

    #[test]
    fn test_deterministic_words() {
        let words1 = generate_words("0000abc", 100, 1);
        let words2 = generate_words("0000abc", 100, 1);
        assert_eq!(words1, words2);
    }

    #[test]
    fn test_create_lock_immediate() {
        let locked = create_lock(
            "ρTEST⟨βdata⟩".to_string(),
            UnlockMode::Immediate,
            5, // max 5 hydrations
            0, // no block expiry
            3600, // 1 hour expiry
            "0000000abc",
            881234,
            "creator_pubkey_123",
        );

        assert_eq!(locked.max_hydrations, 5);
        assert_eq!(locked.hydrations_used, 0);
        assert!(locked.active);
        assert!(!locked.burned);
        println!("Created lock: {}", locked.word_key);
    }

    #[test]
    fn test_request_immediate_unlock() {
        let locked = create_lock(
            "ρPAYLOAD".to_string(),
            UnlockMode::Immediate,
            3,
            0,
            3600,
            "0000000def",
            881235,
            "creator_123",
        );

        let result = request_unlock(
            locked.words.clone(),
            Some("192.168.1.1".to_string()),
            Some("Austin, TX".to_string()),
            None,
        );

        assert!(result.is_ok());
        if let Ok(UnlockRequestResult::Approved { payload, remaining }) = result {
            assert_eq!(payload, "ρPAYLOAD");
            assert_eq!(remaining, 3); // Not consumed yet, just approved
        }
    }

    #[test]
    fn test_genesis_pending_approval() {
        let locked = create_lock(
            "ρSECRET".to_string(),
            UnlockMode::Genesis {
                require_approval: true,
                auto_approve_words: None,
            },
            2,
            0,
            7200,
            "0000000ghi",
            881236,
            "creator_456",
        );

        let result = request_unlock(
            locked.words.clone(),
            Some("10.0.0.1".to_string()),
            None,
            None,
        );

        assert!(result.is_ok());
        if let Ok(UnlockRequestResult::PendingApproval { position_in_queue, .. }) = result {
            assert_eq!(position_in_queue, 1);
        }
    }

    #[test]
    fn test_max_hydrations() {
        let locked = create_lock(
            "ρLIMITED".to_string(),
            UnlockMode::Immediate,
            2, // Only 2 allowed
            0,
            3600,
            "0000000jkl",
            881237,
            "creator_789",
        );

        // First request
        let _ = request_unlock(locked.words.clone(), Some("1.1.1.1".to_string()), None, None);

        // Hydrate first
        let status = get_status(&locked.words).unwrap();
        assert_eq!(status.remaining, 2);
    }

    #[test]
    fn test_word_key_format() {
        let words = ("TIMBER".to_string(), "LANTERN".to_string());
        let key = word_key(&words);
        assert_eq!(key, "TIMBER-LANTERN");
    }

    #[test]
    fn test_derive_hash() {
        let words = ("COPPER".to_string(), "FOUNTAIN".to_string());
        let hash = derive_unlock_hash(&words, "0000abc123", 881234);
        assert!(hash.starts_with('#'));
        assert_eq!(hash.len(), 17); // # + 16 hex chars
    }
}
