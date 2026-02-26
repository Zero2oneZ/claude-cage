#!/usr/bin/env bash
# ============================================================
# Stage 1: System Base Packages
# ============================================================
# Detects distro, installs core dependencies, updates system.
# ============================================================
set -euo pipefail

log() { echo "[stage-1] $*"; }

# --- Detect distro ---
detect_distro() {
    if [[ -f /etc/os-release ]]; then
        # shellcheck source=/dev/null
        source /etc/os-release
        echo "$ID"
    elif [[ -f /etc/alpine-release ]]; then
        echo "alpine"
    elif command -v apt-get &>/dev/null; then
        echo "debian"
    elif command -v dnf &>/dev/null; then
        echo "fedora"
    elif command -v pacman &>/dev/null; then
        echo "arch"
    else
        echo "unknown"
    fi
}

DISTRO=$(detect_distro)
log "Detected distro: $DISTRO"

# --- Update and install base packages ---
case "$DISTRO" in
    ubuntu|debian|pop|linuxmint|kali)
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -y
        apt-get upgrade -y
        apt-get install -y \
            build-essential \
            curl wget git \
            openssh-server \
            ufw \
            net-tools iproute2 \
            htop tmux screen \
            pkg-config libssl-dev \
            ca-certificates \
            gnupg lsb-release \
            jq unzip \
            systemd \
            lsof \
            sqlite3 libsqlite3-dev
        ;;
    fedora|rhel|centos|rocky|almalinux)
        dnf update -y
        dnf groupinstall -y "Development Tools"
        dnf install -y \
            curl wget git \
            openssh-server \
            firewalld \
            net-tools iproute \
            htop tmux screen \
            openssl-devel pkg-config \
            ca-certificates \
            jq unzip \
            sqlite sqlite-devel
        ;;
    arch|manjaro|endeavouros)
        pacman -Syu --noconfirm
        pacman -S --noconfirm --needed \
            base-devel \
            curl wget git \
            openssh \
            ufw \
            net-tools iproute2 \
            htop tmux screen \
            openssl pkg-config \
            jq unzip \
            sqlite
        ;;
    alpine)
        apk update
        apk upgrade
        apk add \
            build-base \
            curl wget git \
            openssh \
            iptables \
            net-tools iproute2 \
            htop tmux screen \
            openssl-dev pkgconf \
            jq unzip \
            sqlite sqlite-dev \
            bash
        ;;
    opensuse*|sles)
        zypper refresh
        zypper update -y
        zypper install -y \
            -t pattern devel_basis \
            curl wget git \
            openssh \
            firewalld \
            net-tools iproute2 \
            htop tmux screen \
            libopenssl-devel pkg-config \
            jq unzip \
            sqlite3 sqlite3-devel
        ;;
    *)
        log "WARNING: Unknown distro '$DISTRO'. Attempting generic install."
        log "You may need to install packages manually."
        if command -v apt-get &>/dev/null; then
            apt-get update -y
            apt-get install -y build-essential curl wget git openssh-server ufw net-tools jq
        fi
        ;;
esac

# --- Install Rust toolchain if not present ---
if ! command -v rustc &>/dev/null; then
    log "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # Source for current script; profile is set for user in stage 2
    source "$HOME/.cargo/env" 2>/dev/null || true
else
    log "Rust already installed: $(rustc --version)"
fi

# --- Ensure cargo is in PATH for subsequent stages ---
if [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
fi

log "System base packages installed successfully."
