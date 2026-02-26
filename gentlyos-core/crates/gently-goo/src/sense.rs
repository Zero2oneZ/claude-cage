//! # Sovereignty Protection — Sense Module
//!
//! The sense module protects user sovereignty through boundary enforcement.
//! Every interaction with the GOO field is a `SenseEvent` that must pass
//! through the `SovereigntyGuard` before affecting the system.
//!
//! ## Philosophy
//!
//! The user's attention, data, and consent are sovereign. The system
//! NEVER:
//! - Initiates interaction without consent
//! - Crosses personal space boundaries
//! - Ignores withdrawal of consent
//! - Stores sense data beyond the session (unless explicitly permitted)
//!
//! ## Integration with FAFO
//!
//! Boundary violations escalate through the FAFO ladder:
//! - Warning: logged, user notified
//! - Denied: action blocked, pattern recorded
//! - Repeated violations: FAFO tarpit / poison response
//!
//! ## Event Types
//!
//! | Event | Source | Boundary Check |
//! |-------|--------|---------------|
//! | Touch | Mouse/tap | Personal space radius |
//! | Gaze | Eye tracker | Alert distance + duration |
//! | Voice | Microphone | Volume threshold + direction |
//! | Proximity | Sensor | Distance threshold |

use glam::Vec2;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A sensory event from the user or environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SenseEvent {
    /// Direct interaction (mouse click, tap, stylus)
    Touch {
        /// Position in field coordinates
        position: Vec2,
    },
    /// Gaze tracking event
    Gaze {
        /// Where the user is looking in field coordinates
        position: Vec2,
        /// How long the gaze has been sustained (seconds)
        duration: f32,
    },
    /// Voice / audio input event
    Voice {
        /// Volume level (0.0 - 1.0)
        volume: f32,
        /// Direction the voice is coming from (unit vector)
        direction: Vec2,
    },
    /// Proximity detection (e.g., sensor, camera)
    Proximity {
        /// Distance from the user to the system (field units)
        distance: f32,
    },
}

/// Boundary enforcement policy.
///
/// Defines the user's sovereignty boundaries. These are HARD limits
/// that the system must respect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryPolicy {
    /// Personal space radius — interactions closer than this trigger a check
    pub personal_space: f32,
    /// Alert distance — events within this range are flagged
    pub alert_distance: f32,
    /// Whether explicit consent is required for each interaction type
    pub consent_required: bool,
    /// Maximum gaze duration before alert (seconds)
    pub max_gaze_duration: f32,
    /// Minimum voice volume to register (threshold)
    pub min_voice_volume: f32,
    /// Maximum allowed proximity events per second (rate limit)
    pub proximity_rate_limit: f32,
    /// Whether to log all events (for audit)
    pub audit_logging: bool,
}

impl Default for BoundaryPolicy {
    fn default() -> Self {
        Self {
            personal_space: 2.0,
            alert_distance: 5.0,
            consent_required: true,
            max_gaze_duration: 10.0,
            min_voice_volume: 0.1,
            proximity_rate_limit: 10.0,
            audit_logging: true,
        }
    }
}

impl BoundaryPolicy {
    /// Create a permissive policy (minimal restrictions).
    pub fn permissive() -> Self {
        Self {
            personal_space: 0.5,
            alert_distance: 1.0,
            consent_required: false,
            max_gaze_duration: 60.0,
            min_voice_volume: 0.01,
            proximity_rate_limit: 100.0,
            audit_logging: false,
        }
    }

    /// Create a strict policy (maximum protection).
    pub fn strict() -> Self {
        Self {
            personal_space: 5.0,
            alert_distance: 10.0,
            consent_required: true,
            max_gaze_duration: 3.0,
            min_voice_volume: 0.3,
            proximity_rate_limit: 2.0,
            audit_logging: true,
        }
    }
}

/// Result of a boundary check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundaryResult {
    /// Event is allowed to proceed
    Allowed,
    /// Event is allowed but flagged as concerning
    Warning {
        reason: String,
    },
    /// Event is denied — boundary violated
    Denied {
        reason: String,
    },
}

impl BoundaryResult {
    /// Whether the event is permitted (Allowed or Warning).
    pub fn is_permitted(&self) -> bool {
        !matches!(self, BoundaryResult::Denied { .. })
    }

    /// Whether the event triggered any flag.
    pub fn is_flagged(&self) -> bool {
        !matches!(self, BoundaryResult::Allowed)
    }
}

/// A logged boundary violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationRecord {
    /// When the violation occurred
    pub timestamp: DateTime<Utc>,
    /// The event that caused it
    pub event_type: String,
    /// The boundary result
    pub result: BoundaryResult,
    /// Additional context
    pub context: String,
}

/// Consent state for an interaction type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentState {
    /// Whether consent has been granted
    pub granted: bool,
    /// When consent was last updated
    pub updated_at: DateTime<Utc>,
    /// What was consented to
    pub scope: String,
    /// Whether consent can be auto-renewed or must be re-asked
    pub auto_renew: bool,
}

/// The sovereignty guard — central boundary enforcement.
///
/// All sense events flow through here before affecting the GOO field.
/// The guard maintains:
/// - Current boundary policy
/// - Consent states per interaction type
/// - Violation log
/// - Rate limiting state
pub struct SovereigntyGuard {
    /// Active boundary policy
    pub policy: BoundaryPolicy,
    /// Consent states indexed by interaction type
    consent_states: Vec<ConsentState>,
    /// Violation log (capped at max_violations)
    violations: Vec<ViolationRecord>,
    /// Maximum violations to keep in memory
    max_violations: usize,
    /// Proximity event counter for rate limiting
    proximity_count: f32,
    /// Last proximity reset time
    last_proximity_reset: f32,
    /// Total events processed
    pub events_processed: u64,
    /// Total violations recorded
    pub total_violations: u64,
}

impl SovereigntyGuard {
    /// Create a new sovereignty guard with the given policy.
    pub fn new(policy: BoundaryPolicy) -> Self {
        Self {
            policy,
            consent_states: Vec::new(),
            violations: Vec::new(),
            max_violations: 1000,
            proximity_count: 0.0,
            last_proximity_reset: 0.0,
            events_processed: 0,
            total_violations: 0,
        }
    }

    /// Check an event against the boundary policy.
    pub fn check(&mut self, event: &SenseEvent) -> BoundaryResult {
        self.events_processed += 1;
        let result = check_boundary(event, &self.policy);

        if let BoundaryResult::Denied { .. } | BoundaryResult::Warning { .. } = &result {
            if self.policy.audit_logging {
                self.log_violation(event, &result);
            }
        }

        if matches!(result, BoundaryResult::Denied { .. }) {
            self.total_violations += 1;
        }

        result
    }

    /// Check and enforce with rate limiting for proximity events.
    pub fn check_with_rate_limit(&mut self, event: &SenseEvent, current_time: f32) -> BoundaryResult {
        // Reset proximity counter every second
        if current_time - self.last_proximity_reset >= 1.0 {
            self.proximity_count = 0.0;
            self.last_proximity_reset = current_time;
        }

        if let SenseEvent::Proximity { .. } = event {
            self.proximity_count += 1.0;
            if self.proximity_count > self.policy.proximity_rate_limit {
                return BoundaryResult::Denied {
                    reason: format!(
                        "Proximity rate limit exceeded: {:.0}/{:.0} per second",
                        self.proximity_count, self.policy.proximity_rate_limit
                    ),
                };
            }
        }

        self.check(event)
    }

    /// Grant consent for a scope.
    pub fn grant_consent(&mut self, scope: impl Into<String>, auto_renew: bool) {
        let scope = scope.into();

        // Update existing or add new
        if let Some(state) = self.consent_states.iter_mut().find(|s| s.scope == scope) {
            state.granted = true;
            state.updated_at = Utc::now();
            state.auto_renew = auto_renew;
        } else {
            self.consent_states.push(ConsentState {
                granted: true,
                updated_at: Utc::now(),
                scope,
                auto_renew,
            });
        }
    }

    /// Revoke consent for a scope.
    pub fn revoke_consent(&mut self, scope: &str) {
        if let Some(state) = self.consent_states.iter_mut().find(|s| s.scope == scope) {
            state.granted = false;
            state.updated_at = Utc::now();
        }
    }

    /// Check if consent is granted for a scope.
    pub fn has_consent(&self, scope: &str) -> bool {
        self.consent_states
            .iter()
            .any(|s| s.scope == scope && s.granted)
    }

    /// Revoke ALL consents (emergency reset).
    pub fn revoke_all_consent(&mut self) {
        let now = Utc::now();
        for state in &mut self.consent_states {
            state.granted = false;
            state.updated_at = now;
        }
    }

    /// Get recent violations.
    pub fn recent_violations(&self, count: usize) -> &[ViolationRecord] {
        let start = self.violations.len().saturating_sub(count);
        &self.violations[start..]
    }

    /// Get total violation count.
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    /// Clear violation log.
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Update the boundary policy.
    pub fn set_policy(&mut self, policy: BoundaryPolicy) {
        self.policy = policy;
    }

    /// Log a violation.
    fn log_violation(&mut self, event: &SenseEvent, result: &BoundaryResult) {
        let event_type = match event {
            SenseEvent::Touch { .. } => "touch",
            SenseEvent::Gaze { .. } => "gaze",
            SenseEvent::Voice { .. } => "voice",
            SenseEvent::Proximity { .. } => "proximity",
        };

        let context = match event {
            SenseEvent::Touch { position } => format!("at ({:.1}, {:.1})", position.x, position.y),
            SenseEvent::Gaze { position, duration } => {
                format!("at ({:.1}, {:.1}) for {:.1}s", position.x, position.y, duration)
            }
            SenseEvent::Voice { volume, direction } => {
                format!("vol={:.2} dir=({:.1}, {:.1})", volume, direction.x, direction.y)
            }
            SenseEvent::Proximity { distance } => format!("distance={:.1}", distance),
        };

        self.violations.push(ViolationRecord {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            result: result.clone(),
            context,
        });

        // Cap violation log
        if self.violations.len() > self.max_violations {
            self.violations.remove(0);
        }
    }
}

/// Check a sense event against a boundary policy.
///
/// This is the core boundary enforcement function. It evaluates each
/// event type against the policy's thresholds and returns the appropriate
/// result.
pub fn check_boundary(event: &SenseEvent, policy: &BoundaryPolicy) -> BoundaryResult {
    match event {
        SenseEvent::Touch { position } => {
            let distance = position.length(); // distance from origin

            if distance < policy.personal_space {
                BoundaryResult::Denied {
                    reason: format!(
                        "Touch inside personal space: distance {:.1} < threshold {:.1}",
                        distance, policy.personal_space
                    ),
                }
            } else if distance < policy.alert_distance {
                BoundaryResult::Warning {
                    reason: format!(
                        "Touch within alert zone: distance {:.1} < alert {:.1}",
                        distance, policy.alert_distance
                    ),
                }
            } else {
                BoundaryResult::Allowed
            }
        }

        SenseEvent::Gaze { position, duration } => {
            let distance = position.length();

            if *duration > policy.max_gaze_duration {
                BoundaryResult::Denied {
                    reason: format!(
                        "Gaze duration exceeded: {:.1}s > max {:.1}s",
                        duration, policy.max_gaze_duration
                    ),
                }
            } else if distance < policy.personal_space {
                BoundaryResult::Warning {
                    reason: format!(
                        "Gaze in personal space: distance {:.1}",
                        distance
                    ),
                }
            } else {
                BoundaryResult::Allowed
            }
        }

        SenseEvent::Voice { volume, direction: _ } => {
            if *volume < policy.min_voice_volume {
                BoundaryResult::Denied {
                    reason: format!(
                        "Voice below threshold: {:.2} < min {:.2}",
                        volume, policy.min_voice_volume
                    ),
                }
            } else {
                BoundaryResult::Allowed
            }
        }

        SenseEvent::Proximity { distance } => {
            if *distance < policy.personal_space {
                BoundaryResult::Denied {
                    reason: format!(
                        "Proximity violation: distance {:.1} < personal space {:.1}",
                        distance, policy.personal_space
                    ),
                }
            } else if *distance < policy.alert_distance {
                BoundaryResult::Warning {
                    reason: format!(
                        "Within alert distance: {:.1} < alert {:.1}",
                        distance, policy.alert_distance
                    ),
                }
            } else {
                BoundaryResult::Allowed
            }
        }
    }
}

/// Classify an event's severity for FAFO integration.
///
/// Returns a severity level 0-4 matching FAFO escalation:
/// - 0: No issue (Allowed)
/// - 1: Minor concern (Warning)
/// - 2: Boundary breach (Denied)
/// - 3: Repeated/persistent violation
/// - 4: Critical sovereignty breach
pub fn classify_severity(result: &BoundaryResult, prior_violations: u64) -> u8 {
    match result {
        BoundaryResult::Allowed => 0,
        BoundaryResult::Warning { .. } => {
            if prior_violations > 5 {
                2 // escalated due to history
            } else {
                1
            }
        }
        BoundaryResult::Denied { .. } => {
            if prior_violations > 20 {
                4 // critical — persistent attacker
            } else if prior_violations > 10 {
                3 // repeated — escalate
            } else {
                2 // standard denial
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_allowed() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Touch {
            position: Vec2::new(10.0, 10.0),
        };
        let result = check_boundary(&event, &policy);
        assert_eq!(result, BoundaryResult::Allowed);
    }

    #[test]
    fn test_touch_denied_personal_space() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Touch {
            position: Vec2::new(0.5, 0.5), // distance ~0.7, inside personal_space=2.0
        };
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Denied { .. }));
    }

    #[test]
    fn test_touch_warning_alert_zone() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Touch {
            position: Vec2::new(3.0, 0.0), // distance=3.0, between personal(2.0) and alert(5.0)
        };
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Warning { .. }));
    }

    #[test]
    fn test_gaze_duration_exceeded() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Gaze {
            position: Vec2::new(10.0, 10.0),
            duration: 15.0, // exceeds max_gaze_duration=10.0
        };
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Denied { .. }));
    }

    #[test]
    fn test_voice_below_threshold() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Voice {
            volume: 0.01, // below min_voice_volume=0.1
            direction: Vec2::X,
        };
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Denied { .. }));
    }

    #[test]
    fn test_voice_allowed() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Voice {
            volume: 0.5,
            direction: Vec2::X,
        };
        let result = check_boundary(&event, &policy);
        assert_eq!(result, BoundaryResult::Allowed);
    }

    #[test]
    fn test_proximity_denied() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Proximity { distance: 1.0 }; // inside personal_space=2.0
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Denied { .. }));
    }

    #[test]
    fn test_proximity_warning() {
        let policy = BoundaryPolicy::default();
        let event = SenseEvent::Proximity { distance: 3.0 }; // between personal(2.0) and alert(5.0)
        let result = check_boundary(&event, &policy);
        assert!(matches!(result, BoundaryResult::Warning { .. }));
    }

    #[test]
    fn test_sovereignty_guard_consent() {
        let mut guard = SovereigntyGuard::new(BoundaryPolicy::default());

        assert!(!guard.has_consent("gaze_tracking"));

        guard.grant_consent("gaze_tracking", false);
        assert!(guard.has_consent("gaze_tracking"));

        guard.revoke_consent("gaze_tracking");
        assert!(!guard.has_consent("gaze_tracking"));
    }

    #[test]
    fn test_sovereignty_guard_revoke_all() {
        let mut guard = SovereigntyGuard::new(BoundaryPolicy::default());
        guard.grant_consent("gaze", false);
        guard.grant_consent("voice", true);
        guard.grant_consent("touch", false);

        guard.revoke_all_consent();

        assert!(!guard.has_consent("gaze"));
        assert!(!guard.has_consent("voice"));
        assert!(!guard.has_consent("touch"));
    }

    #[test]
    fn test_sovereignty_guard_violation_logging() {
        let mut guard = SovereigntyGuard::new(BoundaryPolicy::default());

        let event = SenseEvent::Touch {
            position: Vec2::new(0.5, 0.5),
        };
        let result = guard.check(&event);

        assert!(matches!(result, BoundaryResult::Denied { .. }));
        assert_eq!(guard.violation_count(), 1);
        assert_eq!(guard.total_violations, 1);
    }

    #[test]
    fn test_severity_classification() {
        assert_eq!(classify_severity(&BoundaryResult::Allowed, 0), 0);

        assert_eq!(
            classify_severity(&BoundaryResult::Warning { reason: "test".into() }, 0),
            1
        );

        assert_eq!(
            classify_severity(&BoundaryResult::Denied { reason: "test".into() }, 0),
            2
        );

        // Escalated by history
        assert_eq!(
            classify_severity(&BoundaryResult::Denied { reason: "test".into() }, 15),
            3
        );
        assert_eq!(
            classify_severity(&BoundaryResult::Denied { reason: "test".into() }, 25),
            4
        );
    }

    #[test]
    fn test_rate_limiting() {
        let policy = BoundaryPolicy {
            proximity_rate_limit: 2.0,
            ..BoundaryPolicy::default()
        };
        let mut guard = SovereigntyGuard::new(policy);

        let event = SenseEvent::Proximity { distance: 10.0 }; // far enough to not trigger distance check

        // First two should pass
        let r1 = guard.check_with_rate_limit(&event, 0.0);
        assert!(r1.is_permitted());
        let r2 = guard.check_with_rate_limit(&event, 0.1);
        assert!(r2.is_permitted());

        // Third should be rate-limited
        let r3 = guard.check_with_rate_limit(&event, 0.2);
        assert!(matches!(r3, BoundaryResult::Denied { .. }));

        // After 1 second reset, should pass again
        let r4 = guard.check_with_rate_limit(&event, 1.5);
        assert!(r4.is_permitted());
    }

    #[test]
    fn test_boundary_result_methods() {
        assert!(BoundaryResult::Allowed.is_permitted());
        assert!(!BoundaryResult::Allowed.is_flagged());

        let warning = BoundaryResult::Warning { reason: "x".into() };
        assert!(warning.is_permitted());
        assert!(warning.is_flagged());

        let denied = BoundaryResult::Denied { reason: "x".into() };
        assert!(!denied.is_permitted());
        assert!(denied.is_flagged());
    }

    #[test]
    fn test_strict_policy() {
        let policy = BoundaryPolicy::strict();
        assert!(policy.personal_space > BoundaryPolicy::default().personal_space);
        assert!(policy.consent_required);
    }

    #[test]
    fn test_permissive_policy() {
        let policy = BoundaryPolicy::permissive();
        assert!(policy.personal_space < BoundaryPolicy::default().personal_space);
        assert!(!policy.consent_required);
    }
}
