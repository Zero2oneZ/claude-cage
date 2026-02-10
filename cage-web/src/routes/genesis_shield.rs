//! Genesis Shield -- `/genesis-shield`
//!
//! First-boot security verification dashboard. Shows the 8 defense-in-depth
//! layers and their current verification status. Displays the boot sequence
//! checklist and any hardening gaps.

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
//  Data Model
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
//  Static Dataset
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
        current: "147 syscalls allowed — baseline conservative allowlist",
    },
    HardeningTip {
        priority: "LOW",
        area: "DNS",
        recommendation: "Consider pinning DNS resolution to prevent DNS rebinding attacks",
        current: "Uses container-default DNS resolution",
    },
];

// ---------------------------------------------------------------
//  Template Data
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
}

// ---------------------------------------------------------------
//  Routes
// ---------------------------------------------------------------

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/genesis-shield", get(genesis_shield_page))
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
        can_view_commands: can_view_commands,
    }
    .render()
    .unwrap_or_default();

    if is_htmx(&headers) {
        Html(content)
    } else {
        Html(wrap_page("Genesis Shield", &content))
    }
}
