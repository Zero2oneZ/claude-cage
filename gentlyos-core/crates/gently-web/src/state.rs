//! Application state for the web GUI

use gently_feed::LivingFeed;
use gently_search::ThoughtIndex;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;

/// User session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSession {
    pub token: String,
    pub user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub is_premium: bool,
}

impl UserSession {
    pub fn new(user_id: &str, is_premium: bool, duration_hours: i64) -> Self {
        let token = generate_token();
        let now = chrono::Utc::now();
        Self {
            token,
            user_id: user_id.to_string(),
            created_at: now,
            expires_at: now + chrono::Duration::hours(duration_hours),
            is_premium,
        }
    }

    pub fn is_valid(&self) -> bool {
        chrono::Utc::now() < self.expires_at
    }
}

/// Generate a secure random token
fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Hash a password with salt
pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

/// User credentials (stored hashed)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserCredentials {
    pub user_id: String,
    pub password_hash: String,
    pub salt: String,
    pub is_premium: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl UserCredentials {
    pub fn new(user_id: &str, password: &str, is_premium: bool) -> Self {
        use rand::Rng;
        let salt: String = (0..16).map(|_| rand::thread_rng().gen::<char>()).collect();
        let password_hash = hash_password(password, &salt);
        Self {
            user_id: user_id.to_string(),
            password_hash,
            salt,
            is_premium,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn verify(&self, password: &str) -> bool {
        let hash = hash_password(password, &self.salt);
        hash == self.password_hash
    }
}

/// Authentication state
#[derive(Clone)]
pub struct AuthState {
    /// Active sessions (token -> session)
    pub sessions: Arc<RwLock<HashMap<String, UserSession>>>,
    /// User credentials (user_id -> credentials)
    pub users: Arc<RwLock<HashMap<String, UserCredentials>>>,
    /// CSRF tokens (token -> expiry)
    pub csrf_tokens: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
    /// Secret key for signing
    pub secret_key: String,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            csrf_tokens: Arc::new(RwLock::new(HashMap::new())),
            secret_key: generate_token(),
        }
    }

    /// Create a default admin user (for development)
    pub fn with_default_admin(self) -> Self {
        let admin = UserCredentials::new("admin", "gently2026", true);
        self.users.write().unwrap().insert("admin".to_string(), admin);
        self
    }

    /// Add a user
    pub fn add_user(&self, user_id: &str, password: &str, is_premium: bool) {
        let creds = UserCredentials::new(user_id, password, is_premium);
        self.users.write().unwrap().insert(user_id.to_string(), creds);
    }

    /// Authenticate and create session
    pub fn login(&self, user_id: &str, password: &str) -> Option<UserSession> {
        let users = self.users.read().unwrap();
        if let Some(creds) = users.get(user_id) {
            if creds.verify(password) {
                let session = UserSession::new(user_id, creds.is_premium, 24);
                let token = session.token.clone();
                drop(users);
                self.sessions.write().unwrap().insert(token, session.clone());
                return Some(session);
            }
        }
        None
    }

    /// Validate a session token
    pub fn validate_token(&self, token: &str) -> Option<UserSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(token).filter(|s| s.is_valid()).cloned()
    }

    /// Logout (invalidate session)
    pub fn logout(&self, token: &str) {
        self.sessions.write().unwrap().remove(token);
    }

    /// Generate CSRF token
    pub fn generate_csrf(&self) -> String {
        let token = generate_token();
        let expiry = chrono::Utc::now() + chrono::Duration::hours(1);
        self.csrf_tokens.write().unwrap().insert(token.clone(), expiry);
        token
    }

    /// Validate CSRF token
    pub fn validate_csrf(&self, token: &str) -> bool {
        let tokens = self.csrf_tokens.read().unwrap();
        if let Some(expiry) = tokens.get(token) {
            return chrono::Utc::now() < *expiry;
        }
        false
    }

    /// Clean up expired sessions and CSRF tokens
    pub fn cleanup_expired(&self) {
        let now = chrono::Utc::now();

        // Clean sessions
        self.sessions.write().unwrap().retain(|_, s| s.expires_at > now);

        // Clean CSRF tokens
        self.csrf_tokens.write().unwrap().retain(|_, e| *e > now);
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Living feed with charge/decay items
    pub feed: Arc<RwLock<LivingFeed>>,
    /// Thought index for search
    pub index: Arc<RwLock<ThoughtIndex>>,
    /// Alexandria enabled flag
    pub alexandria_enabled: bool,
    /// Current chat history
    pub chat_history: Arc<RwLock<Vec<ChatMessage>>>,
    /// Security events
    pub security_events: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Server start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Authentication state
    pub auth: AuthState,
    /// Data directory for persistence
    pub data_dir: PathBuf,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".gently")
            .join("web");

        // Create data directory if it doesn't exist
        let _ = std::fs::create_dir_all(&data_dir);

        Self {
            feed: Arc::new(RwLock::new(LivingFeed::new())),
            index: Arc::new(RwLock::new(ThoughtIndex::new())),
            alexandria_enabled: false,
            chat_history: Arc::new(RwLock::new(Vec::new())),
            security_events: Arc::new(RwLock::new(Vec::new())),
            started_at: chrono::Utc::now(),
            auth: AuthState::new().with_default_admin(),
            data_dir,
        }
    }

    /// Load state from disk
    pub fn load() -> Self {
        let state = Self::new();

        // Try to load feed
        if let Ok(storage) = gently_feed::FeedStorage::default_location() {
            if let Ok(feed) = storage.load() {
                *state.feed.write().unwrap() = feed;
            }
        }

        // Try to load thought index
        let index_path = ThoughtIndex::default_path();
        if let Ok(index) = ThoughtIndex::load(&index_path) {
            *state.index.write().unwrap() = index;
        }

        // Load chat history
        if let Err(e) = state.load_chat_history() {
            tracing::warn!("Failed to load chat history: {}", e);
        }

        // Load users (or use defaults if none exist)
        if let Err(e) = state.load_users() {
            tracing::warn!("Failed to load users: {}", e);
        }

        state
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> i64 {
        (chrono::Utc::now() - self.started_at).num_seconds()
    }

    /// Get chat history file path
    fn chat_history_path(&self) -> PathBuf {
        self.data_dir.join("chat_history.json")
    }

    /// Save chat history to disk
    pub fn save_chat_history(&self) -> Result<(), std::io::Error> {
        let history = self.chat_history.read().unwrap();
        let json = serde_json::to_string_pretty(&*history)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(self.chat_history_path(), json)
    }

    /// Load chat history from disk
    pub fn load_chat_history(&self) -> Result<(), std::io::Error> {
        let path = self.chat_history_path();
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(&path)?;
        let history: Vec<ChatMessage> = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        *self.chat_history.write().unwrap() = history;
        Ok(())
    }

    /// Add a chat message and persist
    pub fn add_chat_message(&self, msg: ChatMessage) {
        {
            let mut history = self.chat_history.write().unwrap();
            history.push(msg);
        }
        // Best-effort persistence
        let _ = self.save_chat_history();
    }

    /// Clear chat history and persist
    pub fn clear_chat_history(&self) {
        {
            let mut history = self.chat_history.write().unwrap();
            history.clear();
        }
        let _ = self.save_chat_history();
    }

    /// Get users file path
    fn users_path(&self) -> PathBuf {
        self.data_dir.join("users.json")
    }

    /// Save users to disk
    pub fn save_users(&self) -> Result<(), std::io::Error> {
        let users = self.auth.users.read().unwrap();
        let json = serde_json::to_string_pretty(&*users)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(self.users_path(), json)
    }

    /// Load users from disk
    pub fn load_users(&self) -> Result<(), std::io::Error> {
        let path = self.users_path();
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(&path)?;
        let users: HashMap<String, UserCredentials> = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        *self.auth.users.write().unwrap() = users;
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// A chat message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub id: uuid::Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tokens_used: Option<u32>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: "user".to_string(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
            tokens_used: None,
        }
    }

    pub fn assistant(content: impl Into<String>, tokens: Option<u32>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: "assistant".to_string(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
            tokens_used: tokens,
        }
    }
}

/// A security event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityEvent {
    pub id: uuid::Uuid,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SecurityEvent {
    pub fn new(event_type: &str, severity: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let salt = "test_salt";
        let password = "my_password";
        let hash1 = hash_password(password, salt);
        let hash2 = hash_password(password, salt);

        // Same password + salt = same hash
        assert_eq!(hash1, hash2);

        // Different password = different hash
        let hash3 = hash_password("other_password", salt);
        assert_ne!(hash1, hash3);

        // Different salt = different hash
        let hash4 = hash_password(password, "other_salt");
        assert_ne!(hash1, hash4);
    }

    #[test]
    fn test_user_credentials() {
        let creds = UserCredentials::new("testuser", "testpass", false);

        assert_eq!(creds.user_id, "testuser");
        assert!(!creds.is_premium);
        assert!(creds.verify("testpass"));
        assert!(!creds.verify("wrongpass"));
    }

    #[test]
    fn test_user_session_validity() {
        let session = UserSession::new("user1", true, 24);

        assert!(session.is_valid());
        assert!(session.is_premium);
        assert!(!session.token.is_empty());
    }

    #[test]
    fn test_auth_login_logout() {
        let auth = AuthState::new().with_default_admin();

        // Login with correct password
        let session = auth.login("admin", "gently2026");
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.user_id, "admin");
        assert!(session.is_premium);

        // Validate token
        let validated = auth.validate_token(&session.token);
        assert!(validated.is_some());

        // Logout
        auth.logout(&session.token);
        let validated = auth.validate_token(&session.token);
        assert!(validated.is_none());
    }

    #[test]
    fn test_auth_wrong_password() {
        let auth = AuthState::new().with_default_admin();

        // Login with wrong password
        let session = auth.login("admin", "wrongpassword");
        assert!(session.is_none());
    }

    #[test]
    fn test_auth_nonexistent_user() {
        let auth = AuthState::new().with_default_admin();

        // Login with nonexistent user
        let session = auth.login("nobody", "anypassword");
        assert!(session.is_none());
    }

    #[test]
    fn test_csrf_tokens() {
        let auth = AuthState::new();

        // Generate CSRF token
        let token = auth.generate_csrf();
        assert!(!token.is_empty());

        // Validate token
        assert!(auth.validate_csrf(&token));

        // Invalid token
        assert!(!auth.validate_csrf("invalid_token"));
    }

    #[test]
    fn test_add_user() {
        let auth = AuthState::new();

        auth.add_user("newuser", "newpass", true);

        let session = auth.login("newuser", "newpass");
        assert!(session.is_some());
        assert!(session.unwrap().is_premium);
    }

    #[test]
    fn test_chat_message() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");
        assert!(user_msg.tokens_used.is_none());

        let assistant_msg = ChatMessage::assistant("Hi there", Some(42));
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there");
        assert_eq!(assistant_msg.tokens_used, Some(42));
    }

    #[test]
    fn test_security_event() {
        let event = SecurityEvent::new("login", "info", "User logged in");
        assert_eq!(event.event_type, "login");
        assert_eq!(event.severity, "info");
        assert_eq!(event.message, "User logged in");
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert_eq!(state.uptime_secs(), 0);
        assert!(state.chat_history.read().unwrap().is_empty());
        assert!(state.security_events.read().unwrap().is_empty());
    }

    #[test]
    fn test_chat_history_add_and_clear() {
        let state = AppState::new();

        // Add messages
        state.add_chat_message(ChatMessage::user("Hello"));
        state.add_chat_message(ChatMessage::assistant("Hi", None));

        assert_eq!(state.chat_history.read().unwrap().len(), 2);

        // Clear
        state.clear_chat_history();
        assert!(state.chat_history.read().unwrap().is_empty());
    }

    #[test]
    fn test_chat_persistence() {
        // Create state with temp directory
        let temp_dir = std::env::temp_dir().join(format!("gently_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create first state and save
        {
            let state = AppState {
                feed: std::sync::Arc::new(std::sync::RwLock::new(gently_feed::LivingFeed::new())),
                index: std::sync::Arc::new(std::sync::RwLock::new(ThoughtIndex::new())),
                alexandria_enabled: false,
                chat_history: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
                security_events: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
                started_at: chrono::Utc::now(),
                auth: AuthState::new(),
                data_dir: temp_dir.clone(),
            };

            // Add and save
            state.add_chat_message(ChatMessage::user("Test message"));
            state.save_chat_history().unwrap();
        }

        // Create second state and load
        {
            let state = AppState {
                feed: std::sync::Arc::new(std::sync::RwLock::new(gently_feed::LivingFeed::new())),
                index: std::sync::Arc::new(std::sync::RwLock::new(ThoughtIndex::new())),
                alexandria_enabled: false,
                chat_history: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
                security_events: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
                started_at: chrono::Utc::now(),
                auth: AuthState::new(),
                data_dir: temp_dir.clone(),
            };

            // Load and verify
            state.load_chat_history().unwrap();
            let history = state.chat_history.read().unwrap();
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].content, "Test message");
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
