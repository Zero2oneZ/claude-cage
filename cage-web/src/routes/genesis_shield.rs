//! Genesis Shield -- `/genesis-shield`
//!
//! Security dashboard: 8 defense-in-depth layers, 4 defense rings with
//! shape-based colorblind-safe status indicators, port sovereignty translation
//! table, recent security events, and health stats.
//!
//! HTMX partials for live polling: /partials/rings, /partials/events, /partials/ports

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::middleware::Layer;
use crate::routes::{is_htmx, wrap_page};
use crate::AppState;

// ---------------------------------------------------------------
//  Data Model -- Defense Layers
// ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

impl CheckStatus {
    fn class(self) -> &'static str {
        match self {
            Self::Pass => "shield-pass",
            Self::Warn => "shield-warn",
            Self::Fail => "shield-fail",
            Self::Skip => "shield-skip",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Pass => "+",
            Self::Warn => "!",
            Self::Fail => "x",
            Self::Skip => "-",
        }
    }
}

struct SecurityLayer {
    layer_num: u8,
    name: &'static str,
    description: &'static str,
    status: CheckStatus,
    detail: &'static str,
    verification_cmd: &'static str,
}

struct BootCheck {
    order: u8,
    name: &'static str,
    description: &'static str,
    status: CheckStatus,
    elapsed_ms: u32,
}

struct HardeningTip {
    priority: &'static str,
    area: &'static str,
    recommendation: &'static str,
    current: &'static str,
}

// ---------------------------------------------------------------
//  Data Model -- Defense Rings (colorblind-safe)
// ---------------------------------------------------------------

/// Ring state uses shapes for accessibility:
///   GREEN  = circle    (healthy)
///   YELLOW = triangle  (degraded)
///   ORANGE = diamond   (warning)
///   RED    = square    (critical)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RingState {
    Green,
    Yellow,
    Orange,
    Red,
}

impl RingState {
    /// Shape-based icon -- colorblind-safe primary indicator.
    fn shape(self) -> &'static str {
        match self {
            Self::Green => "[O]",    // circle
            Self::Yellow => "[/\\]", // triangle
            Self::Orange => "[<>]",  // diamond
            Self::Red => "[#]",      // square
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Green => "HEALTHY",
            Self::Yellow => "DEGRADED",
            Self::Orange => "WARNING",
            Self::Red => "CRITICAL",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Green => "ring-green",
            Self::Yellow => "ring-yellow",
            Self::Orange => "ring-orange",
            Self::Red => "ring-red",
        }
    }
}

struct DefenseRing {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    state: RingState,
    events_total: u64,
    last_event: &'static str,
}

// ---------------------------------------------------------------
//  Data Model -- Port Sovereignty
// ---------------------------------------------------------------

/// Port trust level uses shapes for accessibility:
///   TRUSTED   = circle   (GentlyOS service)
///   UNTRUSTED = triangle (user app, shim-managed)
///   ROGUE     = square   (shim bypass -- suspicious)
#[derive(Debug, Clone, Copy)]
enum PortTrust {
    Trusted,
    Untrusted,
    Rogue,
}

impl PortTrust {
    fn shape(self) -> &'static str {
        match self {
            Self::Trusted => "[O]",   // circle
            Self::Untrusted => "[/\\]", // triangle
            Self::Rogue => "[#]",     // square
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Trusted => "TRUSTED",
            Self::Untrusted => "MANAGED",
            Self::Rogue => "ROGUE",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Self::Trusted => "port-trusted",
            Self::Untrusted => "port-managed",
            Self::Rogue => "port-rogue",
        }
    }
}

struct PortEntry {
    assigned_port: u16,
    requested_port: u16,
    service_name: &'static str,
    trust: PortTrust,
    data_in: &'static str,
    data_out: &'static str,
}

// ---------------------------------------------------------------
//  Data Model -- Security Events
// ---------------------------------------------------------------

struct SecurityEvent {
    timestamp: &'static str,
    severity: &'static str,
    severity_class: &'static str,
    source: &'static str,
    message: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset -- Layers
// ---------------------------------------------------------------

static LAYERS: &[SecurityLayer] = &[
    SecurityLayer {
        layer_num: 1,
        name: "Read-Only Root Filesystem",
        description: "Root filesystem mounted read-only. Writes restricted to tmpfs at /tmp (512M) and /run (64M).",
        status: CheckStatus::Pass,
        detail: "docker inspect shows ReadonlyRootfs: true",
        verification_cmd: "docker inspect --format '{{.HostConfig.ReadonlyRootfs}}' <container>",
    },
    SecurityLayer {
        layer_num: 2,
        name: "Capability Dropping",
        description: "ALL capabilities dropped. Only CHOWN, DAC_OVERRIDE, SETGID, SETUID re-added.",
        status: CheckStatus::Pass,
        detail: "CapDrop: ALL, CapAdd: [CHOWN, DAC_OVERRIDE, SETGID, SETUID]",
        verification_cmd: "docker inspect --format '{{.HostConfig.CapDrop}} {{.HostConfig.CapAdd}}' <container>",
    },
    SecurityLayer {
        layer_num: 3,
        name: "Seccomp Profile",
        description: "Custom seccomp profile with ~147 syscall allowlist. AF_VSOCK blocked.",
        status: CheckStatus::Pass,
        detail: "security/seccomp-default.json loaded, 147 syscalls allowed",
        verification_cmd: "docker inspect --format '{{.HostConfig.SecurityOpt}}' <container>",
    },
    SecurityLayer {
        layer_num: 4,
        name: "AppArmor Profile",
        description: "Custom AppArmor profile denying mount, ptrace, raw-network, kernel-module access.",
        status: CheckStatus::Warn,
        detail: "Profile loaded but not enforcing on all hosts (depends on kernel support)",
        verification_cmd: "sudo aa-status | grep cage",
    },
    SecurityLayer {
        layer_num: 5,
        name: "Resource Limits",
        description: "2 CPUs, 4GB memory, 512 PIDs, limited file descriptors.",
        status: CheckStatus::Pass,
        detail: "NanoCpus: 2000000000, Memory: 4294967296, PidsLimit: 512",
        verification_cmd: "docker stats --no-stream <container>",
    },
    SecurityLayer {
        layer_num: 6,
        name: "Network Filtering",
        description: "iptables rules restrict outbound to allowed_hosts only (api.anthropic.com, cdn.anthropic.com).",
        status: CheckStatus::Pass,
        detail: "sandbox_apply_network_filter() applied post-launch, 2 hosts whitelisted",
        verification_cmd: "docker exec <container> iptables -L -n",
    },
    SecurityLayer {
        layer_num: 7,
        name: "No-New-Privileges",
        description: "SecurityOpt no-new-privileges flag prevents privilege escalation via setuid/setgid binaries.",
        status: CheckStatus::Pass,
        detail: "no-new-privileges:true in SecurityOpt",
        verification_cmd: "docker inspect --format '{{.HostConfig.SecurityOpt}}' <container>",
    },
    SecurityLayer {
        layer_num: 8,
        name: "Bridge Network Isolation",
        description: "cage-filtered bridge network with inter-container communication disabled (ICC=false).",
        status: CheckStatus::Pass,
        detail: "Network cage-filtered, com.docker.network.bridge.enable_icc: false",
        verification_cmd: "docker network inspect cage-filtered",
    },
];

static BOOT_CHECKS: &[BootCheck] = &[
    BootCheck { order: 1, name: "Docker daemon reachable", description: "Verify Docker socket is accessible", status: CheckStatus::Pass, elapsed_ms: 12 },
    BootCheck { order: 2, name: "Image integrity", description: "Check container image SHA matches build manifest", status: CheckStatus::Pass, elapsed_ms: 45 },
    BootCheck { order: 3, name: "Seccomp profile loaded", description: "Validate seccomp JSON parses and loads", status: CheckStatus::Pass, elapsed_ms: 8 },
    BootCheck { order: 4, name: "AppArmor profile loaded", description: "Check AppArmor profile is loaded into kernel", status: CheckStatus::Warn, elapsed_ms: 22 },
    BootCheck { order: 5, name: "Network bridge created", description: "Verify cage-filtered network exists with ICC=false", status: CheckStatus::Pass, elapsed_ms: 15 },
    BootCheck { order: 6, name: "Tmpfs mounts verified", description: "Confirm /tmp and /run tmpfs mounts with size limits", status: CheckStatus::Pass, elapsed_ms: 5 },
    BootCheck { order: 7, name: "Non-root user active", description: "Container runs as cageuser (UID 1000), not root", status: CheckStatus::Pass, elapsed_ms: 3 },
    BootCheck { order: 8, name: "DNS resolution test", description: "Resolve api.anthropic.com from inside container", status: CheckStatus::Pass, elapsed_ms: 120 },
    BootCheck { order: 9, name: "Blocked host test", description: "Verify connection to non-whitelisted host is refused", status: CheckStatus::Pass, elapsed_ms: 2005 },
    BootCheck { order: 10, name: "Tini init check", description: "Verify tini is PID 1 for proper signal handling", status: CheckStatus::Pass, elapsed_ms: 2 },
];

static TIPS: &[HardeningTip] = &[
    HardeningTip {
        priority: "MEDIUM",
        area: "AppArmor",
        recommendation: "Ensure AppArmor profile is in enforce mode on production hosts",
        current: "Profile loaded but may be in complain mode on some kernels",
    },
    HardeningTip {
        priority: "LOW",
        area: "Seccomp",
        recommendation: "Consider further restricting syscall list based on observed usage patterns",
        current: "147 syscalls allowed -- baseline conservative allowlist",
    },
    HardeningTip {
        priority: "LOW",
        area: "DNS",
        recommendation: "Consider pinning DNS resolution to prevent DNS rebinding attacks",
        current: "Uses container-default DNS resolution",
    },
];

// ---------------------------------------------------------------
//  Static Dataset -- Defense Rings (mock data for v1.0)
// ---------------------------------------------------------------

static RINGS: &[DefenseRing] = &[
    DefenseRing {
        id: "watchdog",
        name: "Watchdog",
        description: "Deterministic anomaly detection -- process spawns, file access, privilege escalation",
        state: RingState::Green,
        events_total: 47,
        last_event: "ProcessSpawn /usr/bin/node (LOW)",
    },
    DefenseRing {
        id: "circuit-breaker",
        name: "Circuit Breaker",
        description: "Sovereignty protection -- exfiltration detection, prompt injection from APIs",
        state: RingState::Green,
        events_total: 3,
        last_event: "check_manipulation (clean)",
    },
    DefenseRing {
        id: "fafo",
        name: "FAFO Engine",
        description: "Active consequence delivery -- escalation ladder from warn to tarpit to nuke",
        state: RingState::Green,
        events_total: 0,
        last_event: "No incidents",
    },
    DefenseRing {
        id: "port-sovereignty",
        name: "Port Sovereignty",
        description: "Forced bind interception -- every port assigned sequentially, rogue detection",
        state: RingState::Yellow,
        events_total: 2,
        last_event: "Rogue port 4444 detected (nc)",
    },
];

// ---------------------------------------------------------------
//  Static Dataset -- Port Sovereignty Translation Table
// ---------------------------------------------------------------

static PORT_TABLE: &[PortEntry] = &[
    PortEntry { assigned_port: 5000, requested_port: 5000, service_name: "cage-web", trust: PortTrust::Trusted, data_in: "12.4 KB", data_out: "847.2 KB" },
    PortEntry { assigned_port: 5001, requested_port: 7335, service_name: "gently-bridge", trust: PortTrust::Trusted, data_in: "3.1 KB", data_out: "1.2 KB" },
    PortEntry { assigned_port: 5002, requested_port: 9999, service_name: "gently-fafo-tarpit", trust: PortTrust::Trusted, data_in: "0 B", data_out: "0 B" },
    PortEntry { assigned_port: 5003, requested_port: 8080, service_name: "user-app", trust: PortTrust::Untrusted, data_in: "1.8 KB", data_out: "24.6 KB" },
    PortEntry { assigned_port: 5004, requested_port: 3000, service_name: "dev-server", trust: PortTrust::Untrusted, data_in: "0.4 KB", data_out: "5.1 KB" },
];

static ROGUE_PORTS: &[PortEntry] = &[
    PortEntry { assigned_port: 4444, requested_port: 4444, service_name: "nc", trust: PortTrust::Rogue, data_in: "0 B", data_out: "0 B" },
];

// ---------------------------------------------------------------
//  Data Model -- Screen Sovereignty
// ---------------------------------------------------------------

/// Capture verdict shape indicators:
///   Authorized  = circle   (capture permitted, animation played)
///   Denied      = square   (capture blocked)
///   FAFO        = triangle (hostile attempt, FAFO triggered)
#[derive(Debug, Clone, Copy)]
enum CaptureVerdictStatus {
    Authorized,
    Denied,
    FafoTriggered,
}

impl CaptureVerdictStatus {
    fn shape(self) -> &'static str {
        match self {
            Self::Authorized => "[O]",
            Self::Denied => "[#]",
            Self::FafoTriggered => "[/\\]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Authorized => "AUTHORIZED",
            Self::Denied => "DENIED",
            Self::FafoTriggered => "FAFO",
        }
    }
    fn css_class(self) -> &'static str {
        match self {
            Self::Authorized => "capture-authorized",
            Self::Denied => "capture-denied",
            Self::FafoTriggered => "capture-fafo",
        }
    }
}

struct ScreenPolicyEntry {
    name: &'static str,
    description: &'static str,
    is_active: bool,
}

struct AuthorizedProcessEntry {
    process_name: &'static str,
    authority: &'static str,
}

struct CaptureHistoryEntry {
    timestamp: &'static str,
    process: &'static str,
    capture_type: &'static str,
    protocol: &'static str,
    verdict: CaptureVerdictStatus,
}

// ---------------------------------------------------------------
//  Static Dataset -- Screen Sovereignty
// ---------------------------------------------------------------

static SCREEN_POLICIES: &[ScreenPolicyEntry] = &[
    ScreenPolicyEntry { name: "DenyAll", description: "Nothing captures. Default policy.", is_active: true },
    ScreenPolicyEntry { name: "AllowGenesisOnly", description: "Only genesis-authorized processes may capture.", is_active: false },
    ScreenPolicyEntry { name: "AllowWithAnimation", description: "Allow captures but animation ALWAYS fires.", is_active: false },
    ScreenPolicyEntry { name: "Lockdown", description: "Deny ALL including genesis -- high-security mode.", is_active: false },
];

static AUTHORIZED_PROCESSES: &[AuthorizedProcessEntry] = &[
    AuthorizedProcessEntry { process_name: "gently-capture", authority: "GenesisHolder" },
    AuthorizedProcessEntry { process_name: "gently-crash-reporter", authority: "SystemService" },
];

static CAPTURE_HISTORY: &[CaptureHistoryEntry] = &[
    CaptureHistoryEntry { timestamp: "2026-02-24 14:35:12", process: "gently-capture", capture_type: "Screenshot", protocol: "WlrScreencopy", verdict: CaptureVerdictStatus::Authorized },
    CaptureHistoryEntry { timestamp: "2026-02-24 14:33:08", process: "obs-studio", capture_type: "Recording", protocol: "PipeWire", verdict: CaptureVerdictStatus::Denied },
    CaptureHistoryEntry { timestamp: "2026-02-24 14:30:55", process: "gently-capture", capture_type: "Screenshot", protocol: "WlrScreencopy", verdict: CaptureVerdictStatus::Authorized },
    CaptureHistoryEntry { timestamp: "2026-02-24 14:28:20", process: "unknown-tool", capture_type: "Screenshot", protocol: "DirectFramebuffer", verdict: CaptureVerdictStatus::FafoTriggered },
    CaptureHistoryEntry { timestamp: "2026-02-24 14:25:00", process: "firefox", capture_type: "Streaming", protocol: "PipeWire", verdict: CaptureVerdictStatus::Denied },
    CaptureHistoryEntry { timestamp: "2026-02-24 14:22:30", process: "gently-crash-reporter", capture_type: "Screenshot", protocol: "WlrScreencopy", verdict: CaptureVerdictStatus::Authorized },
];

// ---------------------------------------------------------------
//  Data Model -- Active Recordings
// ---------------------------------------------------------------

/// Device icon indicators (shape-based, colorblind safe):
///   [O]  = Camera active
///   [~]  = Microphone active
///   [O~] = Camera + Mic
///   [=]  = Screen being shared
///   [*]  = Everything (full exposure)
#[derive(Debug, Clone, Copy)]
enum RecordingDeviceIcon {
    Camera,
    Microphone,
    CameraAndMic,
    Screen,
    Everything,
}

impl RecordingDeviceIcon {
    fn shape(self) -> &'static str {
        match self {
            Self::Camera => "[O]",
            Self::Microphone => "[~]",
            Self::CameraAndMic => "[O~]",
            Self::Screen => "[=]",
            Self::Everything => "[*]",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Camera => "Camera",
            Self::Microphone => "Mic",
            Self::CameraAndMic => "Cam+Mic",
            Self::Screen => "Screen",
            Self::Everything => "FULL EXPOSURE",
        }
    }
    fn css_class(self) -> &'static str {
        match self {
            Self::Camera => "rec-camera",
            Self::Microphone => "rec-mic",
            Self::CameraAndMic => "rec-cam-mic",
            Self::Screen => "rec-screen",
            Self::Everything => "rec-everything",
        }
    }
}

/// Overlay type for display.
#[derive(Debug, Clone, Copy)]
enum OverlayTypeDisplay {
    FullBorder,
    WindowBorder,
    Badge,
}

impl OverlayTypeDisplay {
    fn label(self) -> &'static str {
        match self {
            Self::FullBorder => "Full Border",
            Self::WindowBorder => "Window Border",
            Self::Badge => "Badge",
        }
    }
}

struct ActiveRecordingEntry {
    device: RecordingDeviceIcon,
    overlay: OverlayTypeDisplay,
    process_name: &'static str,
    pid: u32,
    duration: &'static str,
    acknowledged: bool,
}

struct RecordingHistoryEntry {
    timestamp: &'static str,
    device: RecordingDeviceIcon,
    process_name: &'static str,
    action: &'static str,
    duration: &'static str,
}

// ---------------------------------------------------------------
//  Static Dataset -- Active Recordings (mock: one active Zoom call)
// ---------------------------------------------------------------

static ACTIVE_RECORDINGS: &[ActiveRecordingEntry] = &[
    ActiveRecordingEntry {
        device: RecordingDeviceIcon::CameraAndMic,
        overlay: OverlayTypeDisplay::WindowBorder,
        process_name: "zoom",
        pid: 8421,
        duration: "00:14:32",
        acknowledged: true,
    },
];

static RECORDING_HISTORY: &[RecordingHistoryEntry] = &[
    RecordingHistoryEntry { timestamp: "2026-02-24 14:20:00", device: RecordingDeviceIcon::CameraAndMic, process_name: "zoom", action: "started", duration: "ongoing" },
    RecordingHistoryEntry { timestamp: "2026-02-24 13:55:00", device: RecordingDeviceIcon::Microphone, process_name: "discord", action: "stopped", duration: "01:22:15" },
    RecordingHistoryEntry { timestamp: "2026-02-24 12:32:00", device: RecordingDeviceIcon::Microphone, process_name: "discord", action: "started", duration: "" },
    RecordingHistoryEntry { timestamp: "2026-02-24 11:45:00", device: RecordingDeviceIcon::Camera, process_name: "obs-studio", action: "stopped", duration: "00:03:20" },
    RecordingHistoryEntry { timestamp: "2026-02-24 11:41:40", device: RecordingDeviceIcon::Camera, process_name: "obs-studio", action: "started", duration: "" },
    RecordingHistoryEntry { timestamp: "2026-02-24 10:00:00", device: RecordingDeviceIcon::Everything, process_name: "obs-studio", action: "escalation", duration: "EMERGENCY FLASH" },
    RecordingHistoryEntry { timestamp: "2026-02-24 09:58:00", device: RecordingDeviceIcon::Screen, process_name: "obs-studio", action: "started", duration: "" },
    RecordingHistoryEntry { timestamp: "2026-02-24 09:55:00", device: RecordingDeviceIcon::CameraAndMic, process_name: "obs-studio", action: "started", duration: "" },
];

// ---------------------------------------------------------------
//  Static Dataset -- Recent Security Events
// ---------------------------------------------------------------

static RECENT_EVENTS: &[SecurityEvent] = &[
    SecurityEvent { timestamp: "2026-02-24 14:23:01", severity: "HIGH", severity_class: "event-high", source: "Watchdog", message: "Rogue port 4444 opened by process 'nc' -- shim bypass" },
    SecurityEvent { timestamp: "2026-02-24 14:22:58", severity: "LOW", severity_class: "event-low", source: "Watchdog", message: "ProcessSpawn /usr/bin/node pid=1234 parent=1" },
    SecurityEvent { timestamp: "2026-02-24 14:22:45", severity: "INFO", severity_class: "event-info", source: "PortAlloc", message: "Allocated port 5004 for dev-server (requested 3000)" },
    SecurityEvent { timestamp: "2026-02-24 14:22:30", severity: "INFO", severity_class: "event-info", source: "PortAlloc", message: "Allocated port 5003 for user-app (requested 8080)" },
    SecurityEvent { timestamp: "2026-02-24 14:22:10", severity: "LOW", severity_class: "event-low", source: "Circuit", message: "check_manipulation on API response -- clean" },
    SecurityEvent { timestamp: "2026-02-24 14:21:55", severity: "INFO", severity_class: "event-info", source: "Boot", message: "GentlySecurityRuntime initialized -- 10 subsystems armed" },
];

// ---------------------------------------------------------------
//  Template View Models
// ---------------------------------------------------------------

struct LayerView {
    layer_num: u8,
    name: String,
    description: String,
    status_class: String,
    status_label: String,
    status_icon: String,
    detail: String,
    verification_cmd: String,
    can_view_cmd: bool,
}

struct BootCheckView {
    order: u8,
    name: String,
    description: String,
    status_class: String,
    status_label: String,
    status_icon: String,
    elapsed_ms: u32,
}

struct TipView {
    priority: String,
    area: String,
    recommendation: String,
    current: String,
}

struct RingView {
    id: String,
    name: String,
    description: String,
    state_shape: String,
    state_label: String,
    state_class: String,
    events_total: u64,
    last_event: String,
}

#[derive(Clone)]
struct PortView {
    assigned_port: u16,
    requested_port: u16,
    service_name: String,
    trust_shape: String,
    trust_label: String,
    trust_class: String,
    data_in: String,
    data_out: String,
}

struct EventView {
    timestamp: String,
    severity: String,
    severity_class: String,
    source: String,
    message: String,
}

struct ActiveRecordingView {
    device_shape: String,
    device_label: String,
    device_class: String,
    overlay_label: String,
    process_name: String,
    pid: u32,
    duration: String,
    acknowledged: bool,
    ack_label: String,
}

struct RecordingHistoryView {
    timestamp: String,
    device_shape: String,
    device_label: String,
    device_class: String,
    process_name: String,
    action: String,
    duration: String,
}

struct ScreenPolicyView {
    name: String,
    description: String,
    is_active: bool,
    active_shape: String,
}

struct AuthorizedProcessView {
    process_name: String,
    authority: String,
}

struct CaptureHistoryView {
    timestamp: String,
    process: String,
    capture_type: String,
    protocol: String,
    verdict_shape: String,
    verdict_label: String,
    verdict_class: String,
}

// ---------------------------------------------------------------
//  Templates
// ---------------------------------------------------------------

#[derive(Template)]
#[template(path = "genesis_shield.html")]
struct GenesisShieldTemplate {
    layer_label: String,
    layer_badge: String,
    layers: Vec<LayerView>,
    boot_checks: Vec<BootCheckView>,
    tips: Vec<TipView>,
    pass_count: usize,
    warn_count: usize,
    fail_count: usize,
    total_layers: usize,
    boot_total_ms: u32,
    can_rerun: bool,
    can_view_commands: bool,
    rings: Vec<RingView>,
    ports: Vec<PortView>,
    rogue_ports: Vec<PortView>,
    events: Vec<EventView>,
    total_ports: usize,
    total_rogue: usize,
    screen_policies: Vec<ScreenPolicyView>,
    authorized_processes: Vec<AuthorizedProcessView>,
    capture_history: Vec<CaptureHistoryView>,
    active_captures: String,
    total_captures: usize,
    total_denied: usize,
    total_fafo: usize,
    active_recordings: Vec<ActiveRecordingView>,
    recording_history: Vec<RecordingHistoryView>,
    is_anything_live: bool,
}

#[derive(Template)]
#[template(path = "partials/screen_capture.html")]
struct ScreenCapturePartial {
    screen_policies: Vec<ScreenPolicyView>,
    authorized_processes: Vec<AuthorizedProcessView>,
    capture_history: Vec<CaptureHistoryView>,
    active_captures: String,
    total_captures: usize,
    total_denied: usize,
    total_fafo: usize,
    active_recordings: Vec<ActiveRecordingView>,
    recording_history: Vec<RecordingHistoryView>,
    is_anything_live: bool,
}

#[derive(Template)]
#[template(path = "partials/rings.html")]
struct RingsPartial {
    rings: Vec<RingView>,
}

#[derive(Template)]
#[template(path = "partials/events.html")]
struct EventsPartial {
    events: Vec<EventView>,
}

#[derive(Template)]
#[template(path = "partials/ports.html")]
struct PortsPartial {
    ports: Vec<PortView>,
    rogue_ports: Vec<PortView>,
    total_ports: usize,
    total_rogue: usize,
}

// ---------------------------------------------------------------
//  Data builders
// ---------------------------------------------------------------

fn build_rings() -> Vec<RingView> {
    RINGS
        .iter()
        .map(|r| RingView {
            id: r.id.to_string(),
            name: r.name.to_string(),
            description: r.description.to_string(),
            state_shape: r.state.shape().to_string(),
            state_label: r.state.label().to_string(),
            state_class: r.state.css_class().to_string(),
            events_total: r.events_total,
            last_event: r.last_event.to_string(),
        })
        .collect()
}

fn build_ports(entries: &[PortEntry]) -> Vec<PortView> {
    entries
        .iter()
        .map(|p| PortView {
            assigned_port: p.assigned_port,
            requested_port: p.requested_port,
            service_name: p.service_name.to_string(),
            trust_shape: p.trust.shape().to_string(),
            trust_label: p.trust.label().to_string(),
            trust_class: p.trust.css_class().to_string(),
            data_in: p.data_in.to_string(),
            data_out: p.data_out.to_string(),
        })
        .collect()
}

fn build_events() -> Vec<EventView> {
    RECENT_EVENTS
        .iter()
        .map(|e| EventView {
            timestamp: e.timestamp.to_string(),
            severity: e.severity.to_string(),
            severity_class: e.severity_class.to_string(),
            source: e.source.to_string(),
            message: e.message.to_string(),
        })
        .collect()
}

fn build_active_recordings() -> Vec<ActiveRecordingView> {
    ACTIVE_RECORDINGS
        .iter()
        .map(|r| ActiveRecordingView {
            device_shape: r.device.shape().to_string(),
            device_label: r.device.label().to_string(),
            device_class: r.device.css_class().to_string(),
            overlay_label: r.overlay.label().to_string(),
            process_name: r.process_name.to_string(),
            pid: r.pid,
            duration: r.duration.to_string(),
            acknowledged: r.acknowledged,
            ack_label: if r.acknowledged { "Yes" } else { "No" }.to_string(),
        })
        .collect()
}

fn build_recording_history() -> Vec<RecordingHistoryView> {
    RECORDING_HISTORY
        .iter()
        .map(|r| RecordingHistoryView {
            timestamp: r.timestamp.to_string(),
            device_shape: r.device.shape().to_string(),
            device_label: r.device.label().to_string(),
            device_class: r.device.css_class().to_string(),
            process_name: r.process_name.to_string(),
            action: r.action.to_string(),
            duration: r.duration.to_string(),
        })
        .collect()
}

fn build_screen_policies() -> Vec<ScreenPolicyView> {
    SCREEN_POLICIES
        .iter()
        .map(|p| ScreenPolicyView {
            name: p.name.to_string(),
            description: p.description.to_string(),
            is_active: p.is_active,
            active_shape: if p.is_active { "[*]" } else { "[ ]" }.to_string(),
        })
        .collect()
}

fn build_authorized_processes() -> Vec<AuthorizedProcessView> {
    AUTHORIZED_PROCESSES
        .iter()
        .map(|p| AuthorizedProcessView {
            process_name: p.process_name.to_string(),
            authority: p.authority.to_string(),
        })
        .collect()
}

fn build_capture_history() -> Vec<CaptureHistoryView> {
    CAPTURE_HISTORY
        .iter()
        .map(|e| CaptureHistoryView {
            timestamp: e.timestamp.to_string(),
            process: e.process.to_string(),
            capture_type: e.capture_type.to_string(),
            protocol: e.protocol.to_string(),
            verdict_shape: e.verdict.shape().to_string(),
            verdict_label: e.verdict.label().to_string(),
            verdict_class: e.verdict.css_class().to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/genesis-shield", get(genesis_shield_page))
        .route("/partials/rings", get(rings_partial))
        .route("/partials/events", get(events_partial))
        .route("/partials/ports", get(ports_partial))
        .route("/partials/screen-capture", get(screen_capture_partial))
}

async fn genesis_shield_page(
    headers: HeaderMap,
    ext: axum::extract::Request,
) -> impl IntoResponse {
    let layer = ext
        .extensions()
        .get::<Layer>()
        .copied()
        .unwrap_or(Layer::User);

    let can_view_commands = layer.has_access(Layer::OsAdmin);

    let layers: Vec<LayerView> = LAYERS
        .iter()
        .map(|l| LayerView {
            layer_num: l.layer_num,
            name: l.name.to_string(),
            description: l.description.to_string(),
            status_class: l.status.class().to_string(),
            status_label: l.status.label().to_string(),
            status_icon: l.status.icon().to_string(),
            detail: l.detail.to_string(),
            verification_cmd: l.verification_cmd.to_string(),
            can_view_cmd: can_view_commands,
        })
        .collect();

    let boot_checks: Vec<BootCheckView> = BOOT_CHECKS
        .iter()
        .map(|b| BootCheckView {
            order: b.order,
            name: b.name.to_string(),
            description: b.description.to_string(),
            status_class: b.status.class().to_string(),
            status_label: b.status.label().to_string(),
            status_icon: b.status.icon().to_string(),
            elapsed_ms: b.elapsed_ms,
        })
        .collect();

    let tips: Vec<TipView> = TIPS
        .iter()
        .map(|t| TipView {
            priority: t.priority.to_string(),
            area: t.area.to_string(),
            recommendation: t.recommendation.to_string(),
            current: t.current.to_string(),
        })
        .collect();

    let pass_count = LAYERS.iter().filter(|l| l.status == CheckStatus::Pass).count();
    let warn_count = LAYERS.iter().filter(|l| l.status == CheckStatus::Warn).count();
    let fail_count = LAYERS.iter().filter(|l| l.status == CheckStatus::Fail).count();
    let boot_total_ms: u32 = BOOT_CHECKS.iter().map(|b| b.elapsed_ms).sum();

    let rings = build_rings();
    let ports = build_ports(PORT_TABLE);
    let rogue_ports = build_ports(ROGUE_PORTS);
    let events = build_events();
    let screen_policies = build_screen_policies();
    let authorized_processes = build_authorized_processes();
    let capture_history = build_capture_history();

    let total_captures = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::Authorized))
        .count();
    let total_denied = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::Denied))
        .count();
    let total_fafo = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::FafoTriggered))
        .count();

    let content = GenesisShieldTemplate {
        layer_label: layer.label().to_string(),
        layer_badge: layer.badge_class().to_string(),
        layers,
        boot_checks,
        tips,
        pass_count,
        warn_count,
        fail_count,
        total_layers: LAYERS.len(),
        boot_total_ms,
        can_rerun: layer.has_access(Layer::OsAdmin),
        can_view_commands,
        rings,
        ports,
        rogue_ports: rogue_ports.clone(),
        events,
        total_ports: PORT_TABLE.len(),
        total_rogue: ROGUE_PORTS.len(),
        screen_policies,
        authorized_processes,
        capture_history,
        active_captures: if ACTIVE_RECORDINGS.is_empty() {
            "No active recordings. All devices idle.".to_string()
        } else {
            format!("{} active", ACTIVE_RECORDINGS.len())
        },
        total_captures,
        total_denied,
        total_fafo,
        active_recordings: build_active_recordings(),
        recording_history: build_recording_history(),
        is_anything_live: !ACTIVE_RECORDINGS.is_empty(),
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Genesis Shield", &content))
    }
}

async fn rings_partial(_headers: HeaderMap) -> impl IntoResponse {
    let rings = build_rings();
    Html(
        RingsPartial { rings }
            .render()
            .unwrap_or_default(),
    )
}

async fn events_partial(_headers: HeaderMap) -> impl IntoResponse {
    let events = build_events();
    Html(
        EventsPartial { events }
            .render()
            .unwrap_or_default(),
    )
}

async fn ports_partial(_headers: HeaderMap) -> impl IntoResponse {
    let ports = build_ports(PORT_TABLE);
    let rogue_ports = build_ports(ROGUE_PORTS);
    Html(
        PortsPartial {
            ports,
            rogue_ports: rogue_ports.clone(),
            total_ports: PORT_TABLE.len(),
            total_rogue: ROGUE_PORTS.len(),
        }
        .render()
        .unwrap_or_default(),
    )
}

async fn screen_capture_partial(_headers: HeaderMap) -> impl IntoResponse {
    let total_captures = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::Authorized))
        .count();
    let total_denied = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::Denied))
        .count();
    let total_fafo = CAPTURE_HISTORY
        .iter()
        .filter(|e| matches!(e.verdict, CaptureVerdictStatus::FafoTriggered))
        .count();

    Html(
        ScreenCapturePartial {
            screen_policies: build_screen_policies(),
            authorized_processes: build_authorized_processes(),
            capture_history: build_capture_history(),
            active_captures: if ACTIVE_RECORDINGS.is_empty() {
                "No active recordings. All devices idle.".to_string()
            } else {
                format!("{} active", ACTIVE_RECORDINGS.len())
            },
            total_captures,
            total_denied,
            total_fafo,
            active_recordings: build_active_recordings(),
            recording_history: build_recording_history(),
            is_anything_live: !ACTIVE_RECORDINGS.is_empty(),
        }
        .render()
        .unwrap_or_default(),
    )
}
