//! Twilio SMS Mesh Communication
//! 
//! Why SMS? Because it:
//! - Works on 2G (kid at school with bad wifi)
//! - Always gets through (no firewall issues)
//! - Instant delivery
//! - Works without app (text "make dragon" to number)
//! 
//! Architecture:
//! Phone ──SMS──► Twilio ──Webhook──► Home PC ──Result──► Twilio ──SMS──► Phone

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Router, Json,
};

// ============================================================================
// TWILIO CONFIG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub phone_number: String,  // Twilio number
    pub webhook_url: String,   // Public URL for webhooks
}

impl TwilioConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            account_sid: std::env::var("TWILIO_ACCOUNT_SID").ok()?,
            auth_token: std::env::var("TWILIO_AUTH_TOKEN").ok()?,
            phone_number: std::env::var("TWILIO_PHONE_NUMBER").ok()?,
            webhook_url: std::env::var("TWILIO_WEBHOOK_URL").ok()?,
        })
    }
}

// ============================================================================
// SMS MESSAGE TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingSms {
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "Body")]
    pub body: String,
    #[serde(rename = "MessageSid")]
    pub message_sid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsCommand {
    pub command_type: CommandType,
    pub args: Vec<String>,
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    Create,      // "make dragon" → text-to-3D
    Generate,    // "gen sword" → image generation
    Status,      // "status" → mesh status
    List,        // "list" → list creations
    Help,        // "help" → commands
    Join,        // "join 192.168.1.100" → join mesh
    Unknown,
}

impl SmsCommand {
    pub fn parse(body: &str) -> Self {
        let body = body.trim().to_lowercase();
        let parts: Vec<&str> = body.split_whitespace().collect();
        
        if parts.is_empty() {
            return Self {
                command_type: CommandType::Help,
                args: vec![],
                raw: body,
            };
        }
        
        let command_type = match parts[0] {
            "make" | "create" | "build" => CommandType::Create,
            "gen" | "generate" | "img" | "image" => CommandType::Generate,
            "status" | "stat" | "?" => CommandType::Status,
            "list" | "ls" | "show" => CommandType::List,
            "help" | "h" | "commands" => CommandType::Help,
            "join" | "connect" => CommandType::Join,
            _ => {
                // Default: treat as create command
                // "dragon" → create dragon
                return Self {
                    command_type: CommandType::Create,
                    args: parts.iter().map(|s| s.to_string()).collect(),
                    raw: body,
                };
            }
        };
        
        Self {
            command_type,
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            raw: body,
        }
    }
}

// ============================================================================
// TWILIO CLIENT
// ============================================================================

pub struct TwilioClient {
    config: TwilioConfig,
    client: reqwest::Client,
}

impl TwilioClient {
    pub fn new(config: TwilioConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
    
    /// Send an SMS
    pub async fn send_sms(&self, to: &str, body: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.config.account_sid
        );
        
        let params = [
            ("From", self.config.phone_number.as_str()),
            ("To", to),
            ("Body", body),
        ];
        
        let response = self.client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await?;
        
        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;
            Ok(json["sid"].as_str().unwrap_or("").to_string())
        } else {
            let error = response.text().await?;
            Err(format!("Twilio error: {}", error).into())
        }
    }
    
    /// Send an MMS with image
    pub async fn send_mms(
        &self, 
        to: &str, 
        body: &str,
        media_url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.config.account_sid
        );
        
        let params = [
            ("From", self.config.phone_number.as_str()),
            ("To", to),
            ("Body", body),
            ("MediaUrl", media_url),
        ];
        
        let response = self.client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await?;
        
        if response.status().is_success() {
            let json: serde_json::Value = response.json().await?;
            Ok(json["sid"].as_str().unwrap_or("").to_string())
        } else {
            let error = response.text().await?;
            Err(format!("Twilio error: {}", error).into())
        }
    }
    
    /// Validate Twilio webhook signature
    pub fn validate_signature(&self, signature: &str, url: &str, params: &HashMap<String, String>) -> bool {
        use sha2::{Sha256, Digest};
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        // Build validation string
        let mut validation_string = url.to_string();
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));
        
        for (key, value) in sorted_params {
            validation_string.push_str(key);
            validation_string.push_str(value);
        }
        
        // HMAC-SHA1 (Twilio uses SHA1)
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        
        type HmacSha1 = Hmac<Sha1>;
        
        let mut mac = HmacSha1::new_from_slice(self.config.auth_token.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(validation_string.as_bytes());
        
        let expected = STANDARD.encode(mac.finalize().into_bytes());
        expected == signature
    }
}

// ============================================================================
// SMS MESH STATE
// ============================================================================

#[derive(Debug, Clone)]
pub struct SmsMeshState {
    pub twilio: Arc<TwilioClient>,
    pub pending_tasks: Arc<RwLock<HashMap<String, PendingTask>>>,  // phone -> task
    pub user_sessions: Arc<RwLock<HashMap<String, UserSession>>>,  // phone -> session
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTask {
    pub task_id: String,
    pub command: SmsCommand,
    pub from_phone: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub phone: String,
    pub creations: Vec<String>,
    pub last_active: u64,
    pub mesh_address: Option<String>,
}

// ============================================================================
// WEBHOOK HANDLERS
// ============================================================================

/// Handle incoming SMS webhook from Twilio
pub async fn handle_incoming_sms(
    State(state): State<SmsMeshState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse incoming SMS
    let sms = IncomingSms {
        from: params.get("From").cloned().unwrap_or_default(),
        to: params.get("To").cloned().unwrap_or_default(),
        body: params.get("Body").cloned().unwrap_or_default(),
        message_sid: params.get("MessageSid").cloned().unwrap_or_default(),
    };
    
    println!("[SMS] From: {} | Body: {}", sms.from, sms.body);
    
    // Parse command
    let command = SmsCommand::parse(&sms.body);
    
    // Get or create user session
    let mut sessions = state.user_sessions.write().await;
    let session = sessions.entry(sms.from.clone()).or_insert_with(|| UserSession {
        phone: sms.from.clone(),
        creations: vec![],
        last_active: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        mesh_address: None,
    });
    session.last_active = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Handle command
    let response = match command.command_type {
        CommandType::Create => {
            let prompt = command.args.join(" ");
            if prompt.is_empty() {
                "🎮 What do you want to create? Text: make [thing]\nExample: make a dragon".to_string()
            } else {
                // Queue task
                let task = PendingTask {
                    task_id: format!("sms_{}", uuid::Uuid::new_v4()),
                    command: command.clone(),
                    from_phone: sms.from.clone(),
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                state.pending_tasks.write().await.insert(sms.from.clone(), task);
                
                format!("✨ Creating: {}\n\nThis may take a minute. We'll text you when it's ready!", prompt)
            }
        }
        
        CommandType::Generate => {
            let prompt = command.args.join(" ");
            if prompt.is_empty() {
                "🎨 What image? Text: gen [description]\nExample: gen sunset over mountains".to_string()
            } else {
                format!("🎨 Generating: {}\n\nWe'll send the image when ready!", prompt)
            }
        }
        
        CommandType::Status => {
            format!(
                "🛰️ ORBIT Status\n\n\
                📱 Your creations: {}\n\
                🖥️ Mesh: {}\n\
                ⚡ Status: Online",
                session.creations.len(),
                session.mesh_address.as_deref().unwrap_or("Not connected")
            )
        }
        
        CommandType::List => {
            if session.creations.is_empty() {
                "📦 No creations yet!\n\nText: make [thing]\nExample: make a sword".to_string()
            } else {
                let list = session.creations.iter()
                    .take(5)
                    .enumerate()
                    .map(|(i, c)| format!("{}. {}", i + 1, c))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("📦 Your creations:\n{}", list)
            }
        }
        
        CommandType::Join => {
            if let Some(addr) = command.args.first() {
                session.mesh_address = Some(addr.clone());
                format!("🛰️ Joined mesh at: {}\n\nYour phone is now connected!", addr)
            } else {
                "🔗 Text: join [address]\nExample: join 192.168.1.100:9999".to_string()
            }
        }
        
        CommandType::Help | CommandType::Unknown => {
            "🎮 ORBIT Commands:\n\n\
            make [thing] - Create 3D\n\
            gen [desc] - Generate image\n\
            status - Mesh status\n\
            list - Your creations\n\
            join [addr] - Join mesh\n\n\
            Example: make a dragon".to_string()
        }
    };
    
    // Return TwiML response
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
    <Message>{}</Message>
</Response>"#,
        response
    )
}

/// Callback when task completes - send result via SMS
pub async fn send_task_result(
    state: &SmsMeshState,
    phone: &str,
    result: TaskResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message = match result {
        TaskResult::Text3D { prompt, preview_url } => {
            // Send MMS with preview
            if let Some(url) = preview_url {
                state.twilio.send_mms(
                    phone,
                    &format!("✨ Created: {}\n\nOpen Game Forge to use it!", prompt),
                    &url,
                ).await?;
            } else {
                state.twilio.send_sms(
                    phone,
                    &format!("✨ Created: {}\n\nOpen Game Forge to use it!", prompt),
                ).await?;
            }
            
            // Update session
            if let Some(session) = state.user_sessions.write().await.get_mut(phone) {
                session.creations.push(prompt);
            }
            
            return Ok(());
        }
        
        TaskResult::Image { prompt, image_url } => {
            state.twilio.send_mms(
                phone,
                &format!("🎨 Generated: {}", prompt),
                &image_url,
            ).await?;
            return Ok(());
        }
        
        TaskResult::Error { message } => {
            format!("❌ Error: {}\n\nTry again?", message)
        }
    };
    
    state.twilio.send_sms(phone, &message).await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    Text3D {
        prompt: String,
        preview_url: Option<String>,
    },
    Image {
        prompt: String,
        image_url: String,
    },
    Error {
        message: String,
    },
}

// ============================================================================
// WEBHOOK SERVER
// ============================================================================

pub fn create_sms_router(state: SmsMeshState) -> Router {
    Router::new()
        .route("/sms/incoming", post(handle_incoming_sms))
        .route("/sms/status", get(sms_status))
        .with_state(state)
}

async fn sms_status(State(state): State<SmsMeshState>) -> impl IntoResponse {
    let sessions = state.user_sessions.read().await;
    let pending = state.pending_tasks.read().await;
    
    Json(serde_json::json!({
        "active_users": sessions.len(),
        "pending_tasks": pending.len(),
        "status": "online"
    }))
}

// ============================================================================
// SMS MESH INTEGRATION
// ============================================================================

/// Integrate SMS mesh with ORBIT server
pub struct SmsMeshIntegration {
    pub state: SmsMeshState,
    pub task_sender: tokio::sync::mpsc::Sender<(String, SmsCommand)>,
}

impl SmsMeshIntegration {
    pub fn new(config: TwilioConfig) -> (Self, tokio::sync::mpsc::Receiver<(String, SmsCommand)>) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        let state = SmsMeshState {
            twilio: Arc::new(TwilioClient::new(config)),
            pending_tasks: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
        };
        
        (Self { state, task_sender: tx }, rx)
    }
    
    /// Start the SMS webhook server
    pub async fn start_webhook_server(&self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = create_sms_router(self.state.clone());
        
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        println!("[SMS] Webhook server listening on port {}", port);
        
        axum::serve(listener, app).await?;
        Ok(())
    }
    
    /// Process pending tasks and send to mesh
    pub async fn process_pending_tasks(&self) {
        loop {
            let tasks: Vec<_> = {
                let pending = self.state.pending_tasks.read().await;
                pending.values().cloned().collect()
            };
            
            for task in tasks {
                // Send to mesh processing
                if self.task_sender.send((task.from_phone.clone(), task.command.clone())).await.is_ok() {
                    // Remove from pending
                    self.state.pending_tasks.write().await.remove(&task.from_phone);
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

// ============================================================================
// EXAMPLE USAGE
// ============================================================================

/*
Usage:

1. Set environment variables:
   TWILIO_ACCOUNT_SID=ACxxxxx
   TWILIO_AUTH_TOKEN=xxxxxx
   TWILIO_PHONE_NUMBER=+1234567890
   TWILIO_WEBHOOK_URL=https://your-server.com/sms/incoming

2. Configure Twilio webhook to point to your server

3. User texts: "make a dragon"
   → Twilio forwards to webhook
   → ORBIT queues task
   → Home PC generates 3D model
   → ORBIT sends MMS with preview

SMS Commands:
- "make dragon" → Creates 3D dragon
- "gen sunset" → Generates image
- "status" → Shows mesh status
- "help" → Shows commands

Works on 2G! No app needed!
*/

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_parsing() {
        let cmd = SmsCommand::parse("make a dragon");
        assert_eq!(cmd.command_type, CommandType::Create);
        assert_eq!(cmd.args, vec!["a", "dragon"]);
        
        let cmd = SmsCommand::parse("dragon");
        assert_eq!(cmd.command_type, CommandType::Create);
        assert_eq!(cmd.args, vec!["dragon"]);
        
        let cmd = SmsCommand::parse("status");
        assert_eq!(cmd.command_type, CommandType::Status);
        
        let cmd = SmsCommand::parse("help");
        assert_eq!(cmd.command_type, CommandType::Help);
    }
}
