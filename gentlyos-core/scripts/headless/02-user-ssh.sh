#!/usr/bin/env bash
# ============================================================
# Stage 2: User Account & SSH Setup
# ============================================================
# Creates the gently user, configures SSH for remote access.
# ============================================================
set -euo pipefail

log() { echo "[stage-2] $*"; }

# --- Create user ---
if id "$GENTLY_USER" &>/dev/null; then
    log "User '$GENTLY_USER' already exists."
else
    log "Creating user '$GENTLY_USER'..."
    useradd -m -s /bin/bash -G sudo,adm "$GENTLY_USER" 2>/dev/null || \
    useradd -m -s /bin/bash "$GENTLY_USER"

    # Add to sudo/wheel group depending on distro
    if getent group sudo &>/dev/null; then
        usermod -aG sudo "$GENTLY_USER" 2>/dev/null || true
    fi
    if getent group wheel &>/dev/null; then
        usermod -aG wheel "$GENTLY_USER" 2>/dev/null || true
    fi
fi

# --- Set password ---
if [[ "${GENTLY_PASSWORD_PROMPT:-no}" == "yes" ]]; then
    log "Set password for '$GENTLY_USER' interactively:"
    passwd "$GENTLY_USER"
else
    log "Setting password for '$GENTLY_USER' from config..."
    echo "${GENTLY_USER}:${GENTLY_PASSWORD}" | chpasswd
fi

# --- SSH server setup ---
log "Configuring SSH server..."

SSHD_CONFIG="/etc/ssh/sshd_config"

# Backup original
if [[ -f "$SSHD_CONFIG" && ! -f "${SSHD_CONFIG}.bak.gentlyos" ]]; then
    cp "$SSHD_CONFIG" "${SSHD_CONFIG}.bak.gentlyos"
fi

# Write a clean sshd_config drop-in to avoid clobbering existing config
DROPIN_DIR="/etc/ssh/sshd_config.d"
mkdir -p "$DROPIN_DIR"

cat > "$DROPIN_DIR/gentlyos-headless.conf" <<SSHEOF
# GentlyOS headless setup
Port ${SSH_PORT}
PermitRootLogin no
PasswordAuthentication ${SSH_ALLOW_PASSWORD}
PubkeyAuthentication yes
X11Forwarding no
MaxAuthTries 5
ClientAliveInterval 120
ClientAliveCountMax 3
SSHEOF

# Ensure Include directive exists in main config for drop-ins
if ! grep -q "^Include.*sshd_config.d" "$SSHD_CONFIG" 2>/dev/null; then
    echo "Include /etc/ssh/sshd_config.d/*.conf" >> "$SSHD_CONFIG"
fi

# --- SSH authorized key (optional) ---
if [[ -n "${SSH_AUTHORIZED_KEY:-}" ]]; then
    log "Installing SSH authorized key..."
    GENTLY_HOME=$(eval echo "~${GENTLY_USER}")
    SSH_DIR="${GENTLY_HOME}/.ssh"
    mkdir -p "$SSH_DIR"
    echo "$SSH_AUTHORIZED_KEY" >> "$SSH_DIR/authorized_keys"
    chmod 700 "$SSH_DIR"
    chmod 600 "$SSH_DIR/authorized_keys"
    chown -R "${GENTLY_USER}:${GENTLY_USER}" "$SSH_DIR"
    log "Authorized key installed. You can disable password auth after verifying key login."
fi

# --- Start/enable SSH ---
if command -v systemctl &>/dev/null; then
    systemctl enable sshd 2>/dev/null || systemctl enable ssh 2>/dev/null || true
    systemctl restart sshd 2>/dev/null || systemctl restart ssh 2>/dev/null || true
elif command -v rc-service &>/dev/null; then
    rc-update add sshd default 2>/dev/null || true
    rc-service sshd restart 2>/dev/null || true
elif command -v service &>/dev/null; then
    service ssh restart 2>/dev/null || service sshd restart 2>/dev/null || true
fi

# --- Setup Rust for the gently user ---
GENTLY_HOME=$(eval echo "~${GENTLY_USER}")
if [[ ! -d "${GENTLY_HOME}/.cargo" ]]; then
    log "Installing Rust for user '$GENTLY_USER'..."
    su - "$GENTLY_USER" -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable'
fi

# --- Shell profile for gently user ---
cat > "${GENTLY_HOME}/.bash_profile_gentlyos" <<'PROFILEEOF'
# GentlyOS environment
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/gentlyos/venv/bin:$PATH"
export OLLAMA_HOST="0.0.0.0"

# Activate Python venv if it exists
if [[ -f /opt/gentlyos/venv/bin/activate ]]; then
    source /opt/gentlyos/venv/bin/activate
fi
PROFILEEOF

# Source it from .bashrc if not already
if ! grep -q "bash_profile_gentlyos" "${GENTLY_HOME}/.bashrc" 2>/dev/null; then
    echo 'source ~/.bash_profile_gentlyos 2>/dev/null || true' >> "${GENTLY_HOME}/.bashrc"
fi

chown -R "${GENTLY_USER}:${GENTLY_USER}" "${GENTLY_HOME}/.bash_profile_gentlyos"

log "User '$GENTLY_USER' configured with SSH access on port $SSH_PORT."
