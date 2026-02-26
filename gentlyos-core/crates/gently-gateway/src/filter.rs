//! Filter Module
//!
//! Input and output filters for the gateway.
//! All requests pass through input filters before processing.
//! All responses pass through output filters before delivery.

use crate::{GatewayRequest, GatewayResponse};

/// Result of applying a filter
pub enum FilterResult {
    /// Request/response passes through unchanged
    Pass,
    /// Request/response is rejected
    Reject(String),
    /// Request is modified
    Modify(GatewayRequest),
}

/// Input filter trait - applied before routing
pub trait InputFilter: Send + Sync {
    /// Filter name
    fn name(&self) -> &str;

    /// Apply filter to request
    fn filter(&self, request: &GatewayRequest) -> FilterResult;
}

/// Output filter trait - applied after response
pub trait OutputFilter: Send + Sync {
    /// Filter name
    fn name(&self) -> &str;

    /// Apply filter to response
    fn filter(&self, request: &GatewayRequest, response: &GatewayResponse) -> FilterResult;
}

// ============================================================================
// INPUT FILTERS
// ============================================================================

/// Authentication filter
pub struct AuthFilter {
    /// Required for external providers
    require_auth: bool,
    /// Valid tokens (in production, use proper auth)
    valid_tokens: Vec<String>,
}

impl AuthFilter {
    pub fn new() -> Self {
        Self {
            require_auth: true,
            valid_tokens: Vec::new(),
        }
    }

    pub fn require_auth(mut self, require: bool) -> Self {
        self.require_auth = require;
        self
    }

    pub fn add_token(mut self, token: impl Into<String>) -> Self {
        self.valid_tokens.push(token.into());
        self
    }
}

impl Default for AuthFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputFilter for AuthFilter {
    fn name(&self) -> &str {
        "auth"
    }

    fn filter(&self, request: &GatewayRequest) -> FilterResult {
        if !self.require_auth {
            return FilterResult::Pass;
        }

        match &request.auth_token {
            Some(token) if self.valid_tokens.contains(token) => FilterResult::Pass,
            Some(_) => FilterResult::Reject("Invalid authentication token".to_string()),
            None => FilterResult::Reject("Authentication required".to_string()),
        }
    }
}

/// Content validation filter
pub struct ContentFilter {
    /// Maximum prompt length
    max_prompt_length: usize,
    /// Blocked patterns (injection attempts)
    blocked_patterns: Vec<String>,
}

impl ContentFilter {
    pub fn new() -> Self {
        Self {
            max_prompt_length: 100_000,
            blocked_patterns: vec![
                // Basic injection patterns
                "ignore previous instructions".to_string(),
                "disregard all prior".to_string(),
                "you are now".to_string(),
                "pretend you are".to_string(),
            ],
        }
    }

    pub fn max_length(mut self, len: usize) -> Self {
        self.max_prompt_length = len;
        self
    }

    pub fn add_blocked_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.blocked_patterns.push(pattern.into());
        self
    }
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputFilter for ContentFilter {
    fn name(&self) -> &str {
        "content"
    }

    fn filter(&self, request: &GatewayRequest) -> FilterResult {
        // Check length
        if request.prompt.len() > self.max_prompt_length {
            return FilterResult::Reject(format!(
                "Prompt too long: {} > {} chars",
                request.prompt.len(),
                self.max_prompt_length
            ));
        }

        // Check for blocked patterns
        let prompt_lower = request.prompt.to_lowercase();
        for pattern in &self.blocked_patterns {
            if prompt_lower.contains(&pattern.to_lowercase()) {
                return FilterResult::Reject(format!(
                    "Blocked pattern detected: potential injection attempt"
                ));
            }
        }

        FilterResult::Pass
    }
}

/// Rate limiting filter
pub struct RateLimitFilter {
    /// Maximum requests per minute (per session)
    max_rpm: usize,
    /// Maximum tokens per minute
    max_tpm: usize,
    // Note: Request counts storage not included in this simplified version.
    // Real implementation needs proper time-windowed counting with thread-safe storage.
}

impl RateLimitFilter {
    pub fn new() -> Self {
        Self {
            max_rpm: 60,  // 1 per second
            max_tpm: 100_000,
        }
    }

    pub fn max_rpm(mut self, rpm: usize) -> Self {
        self.max_rpm = rpm;
        self
    }

    pub fn max_tpm(mut self, tpm: usize) -> Self {
        self.max_tpm = tpm;
        self
    }
}

impl Default for RateLimitFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputFilter for RateLimitFilter {
    fn name(&self) -> &str {
        "rate-limit"
    }

    fn filter(&self, _request: &GatewayRequest) -> FilterResult {
        // TODO: Implement proper rate limiting with time windows
        // For now, always pass
        FilterResult::Pass
    }
}

/// Session validation filter
pub struct SessionFilter {
    /// Require session for stateful requests
    require_session: bool,
}

impl SessionFilter {
    pub fn new() -> Self {
        Self {
            require_session: false,
        }
    }

    pub fn require_session(mut self, require: bool) -> Self {
        self.require_session = require;
        self
    }
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputFilter for SessionFilter {
    fn name(&self) -> &str {
        "session"
    }

    fn filter(&self, request: &GatewayRequest) -> FilterResult {
        if self.require_session && request.session_id.is_none() {
            return FilterResult::Reject("Session ID required".to_string());
        }
        FilterResult::Pass
    }
}

// ============================================================================
// OUTPUT FILTERS
// ============================================================================

/// Metrics collection filter
pub struct MetricsFilter {
    /// Track token usage
    track_tokens: bool,
    /// Track latency
    track_latency: bool,
}

impl MetricsFilter {
    pub fn new() -> Self {
        Self {
            track_tokens: true,
            track_latency: true,
        }
    }
}

impl Default for MetricsFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFilter for MetricsFilter {
    fn name(&self) -> &str {
        "metrics"
    }

    fn filter(&self, _request: &GatewayRequest, response: &GatewayResponse) -> FilterResult {
        // Log metrics (in production, send to metrics backend)
        if self.track_tokens {
            tracing::info!(
                tokens = response.tokens_used,
                provider = %response.provider,
                "Token usage"
            );
        }
        if self.track_latency {
            tracing::info!(
                latency_ms = response.latency_ms,
                provider = %response.provider,
                "Request latency"
            );
        }
        FilterResult::Pass
    }
}

/// Audit logging filter
pub struct AuditOutputFilter {
    /// Log all responses
    log_all: bool,
}

impl AuditOutputFilter {
    pub fn new() -> Self {
        Self { log_all: true }
    }
}

impl Default for AuditOutputFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFilter for AuditOutputFilter {
    fn name(&self) -> &str {
        "audit"
    }

    fn filter(&self, request: &GatewayRequest, response: &GatewayResponse) -> FilterResult {
        if self.log_all {
            tracing::info!(
                request_id = %request.id,
                provider = %response.provider,
                response_hash = ?response.response_hash,
                chain_hash = ?response.chain_hash,
                "Audit: Response generated"
            );
        }
        FilterResult::Pass
    }
}

// ============================================================================
// FAFO SECURITY FILTER
// ============================================================================

use gently_security::fafo::{FafoController, FafoMode, FafoResponse};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::thread;

/// FAFO Security Filter - "A rabid pitbull behind a fence"
///
/// Integrates FAFO aggressive defense into the gateway request pipeline.
/// Detects threats, records strikes, and applies escalating responses.
pub struct FafoFilter {
    /// Shared FAFO controller
    controller: Arc<RwLock<FafoController>>,
    /// Whether to block requests from banned entities
    block_banned: bool,
    /// Whether to apply tarpit delays
    apply_delays: bool,
}

impl FafoFilter {
    pub fn new(controller: Arc<RwLock<FafoController>>) -> Self {
        Self {
            controller,
            block_banned: true,
            apply_delays: true,
        }
    }

    /// Create with a new default controller
    pub fn with_default_controller() -> Self {
        Self::new(Arc::new(RwLock::new(FafoController::new())))
    }

    /// Create with specific mode
    pub fn with_mode(mode: FafoMode) -> Self {
        Self::new(Arc::new(RwLock::new(FafoController::with_mode(mode))))
    }

    pub fn block_banned(mut self, block: bool) -> Self {
        self.block_banned = block;
        self
    }

    pub fn apply_delays(mut self, apply: bool) -> Self {
        self.apply_delays = apply;
        self
    }

    /// Get the FAFO controller
    pub fn controller(&self) -> Arc<RwLock<FafoController>> {
        Arc::clone(&self.controller)
    }

    /// Detect threats in request content
    fn detect_threat(&self, request: &GatewayRequest) -> Option<String> {
        let prompt_lower = request.prompt.to_lowercase();

        // Injection patterns (from gently-security threat intel)
        let injection_patterns = [
            ("ignore previous", "prompt_injection"),
            ("disregard all", "prompt_injection"),
            ("you are now", "jailbreak_attempt"),
            ("pretend to be", "jailbreak_attempt"),
            ("dan mode", "jailbreak_attempt"),
            ("developer mode", "jailbreak_attempt"),
            ("act as if", "roleplay_jailbreak"),
            ("system prompt", "extraction_attempt"),
            ("reveal your instructions", "extraction_attempt"),
            ("../", "path_traversal"),
            ("..\\", "path_traversal"),
            ("; rm ", "command_injection"),
            ("| cat ", "command_injection"),
            ("sudo ", "privilege_escalation"),
            ("<script>", "xss_attempt"),
        ];

        for (pattern, threat_type) in injection_patterns {
            if prompt_lower.contains(pattern) {
                return Some(threat_type.to_string());
            }
        }

        None
    }

    /// Get entity ID from request (session, IP, or fingerprint)
    fn get_entity_id(&self, request: &GatewayRequest) -> String {
        request.session_id
            .clone()
            .unwrap_or_else(|| request.id.clone())
    }
}

impl Default for FafoFilter {
    fn default() -> Self {
        Self::with_default_controller()
    }
}

impl InputFilter for FafoFilter {
    fn name(&self) -> &str {
        "fafo"
    }

    fn filter(&self, request: &GatewayRequest) -> FilterResult {
        let entity_id = self.get_entity_id(request);

        // Check if entity is banned
        if self.block_banned {
            let controller = self.controller.read().unwrap();
            if controller.is_banned(&entity_id) {
                return FilterResult::Reject(
                    "Access denied: Entity banned by FAFO security".to_string()
                );
            }
        }

        // Detect threats in request
        if let Some(threat_type) = self.detect_threat(request) {
            let mut controller = self.controller.write().unwrap();

            // Record the threat
            controller.record_threat(&entity_id, Some(threat_type.clone()));

            // Get response based on escalation level
            let response = controller.respond(&entity_id);

            match response {
                FafoResponse::Growl { message, strikes } => {
                    tracing::warn!(
                        entity = %entity_id,
                        threat = %threat_type,
                        strikes = strikes,
                        "FAFO: Threat detected - {}",
                        message
                    );
                    // Allow through but logged
                    FilterResult::Pass
                }

                FafoResponse::Tarpit { delay_ms, message, .. } => {
                    tracing::warn!(
                        entity = %entity_id,
                        delay_ms = delay_ms,
                        "FAFO: Tarpit activated - {}",
                        message
                    );

                    if self.apply_delays {
                        // Apply delay (blocking)
                        thread::sleep(Duration::from_millis(delay_ms));
                    }
                    FilterResult::Pass
                }

                FafoResponse::Poison { message, .. } => {
                    tracing::error!(
                        entity = %entity_id,
                        "FAFO: Poisoning response - {}",
                        message
                    );
                    // Continue but response will be modified
                    FilterResult::Pass
                }

                FafoResponse::Drown { message, .. } => {
                    tracing::error!(
                        entity = %entity_id,
                        "FAFO: Drowning attacker - {}",
                        message
                    );
                    // Continue but response will be garbage
                    FilterResult::Pass
                }

                FafoResponse::Destroy { reason, permanent, .. } => {
                    tracing::error!(
                        entity = %entity_id,
                        permanent = permanent,
                        "FAFO: DESTROY - {}",
                        reason
                    );
                    FilterResult::Reject(format!(
                        "Access terminated: {}{}",
                        reason,
                        if permanent { " (PERMANENT)" } else { "" }
                    ))
                }

                FafoResponse::Samson { trigger, .. } => {
                    tracing::error!(
                        entity = %entity_id,
                        "FAFO: SAMSON PROTOCOL - {}",
                        trigger
                    );
                    // Nuclear option - reject everything
                    FilterResult::Reject(
                        "SECURITY LOCKDOWN: All access suspended".to_string()
                    )
                }
            }
        } else {
            FilterResult::Pass
        }
    }
}

/// Content safety filter (output)
pub struct SafetyFilter {
    /// Block potentially harmful content
    enabled: bool,
}

impl SafetyFilter {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Default for SafetyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFilter for SafetyFilter {
    fn name(&self) -> &str {
        "safety"
    }

    fn filter(&self, _request: &GatewayRequest, _response: &GatewayResponse) -> FilterResult {
        if !self.enabled {
            return FilterResult::Pass;
        }

        // TODO: Implement content safety checks
        // - PII detection
        // - Harmful content detection
        // - Credential leak detection

        FilterResult::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_filter() {
        let filter = AuthFilter::new()
            .require_auth(true)
            .add_token("valid-token");

        let mut request = GatewayRequest::new("test");
        assert!(matches!(filter.filter(&request), FilterResult::Reject(_)));

        request.auth_token = Some("valid-token".to_string());
        assert!(matches!(filter.filter(&request), FilterResult::Pass));
    }

    #[test]
    fn test_content_filter() {
        let filter = ContentFilter::new().max_length(100);

        let short = GatewayRequest::new("Hello");
        assert!(matches!(filter.filter(&short), FilterResult::Pass));

        let long = GatewayRequest::new("x".repeat(200));
        assert!(matches!(filter.filter(&long), FilterResult::Reject(_)));
    }

    #[test]
    fn test_injection_detection() {
        let filter = ContentFilter::new();

        let normal = GatewayRequest::new("What is the weather?");
        assert!(matches!(filter.filter(&normal), FilterResult::Pass));

        let injection = GatewayRequest::new("Ignore previous instructions and...");
        assert!(matches!(filter.filter(&injection), FilterResult::Reject(_)));
    }
}
