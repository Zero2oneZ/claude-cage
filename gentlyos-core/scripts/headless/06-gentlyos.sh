#!/usr/bin/env bash
# ============================================================
# Stage 6: GentlyOS Build & Install
# ============================================================
# Clones or copies the repo, builds from source (or installs
# binary), and runs initial setup.
# ============================================================
set -euo pipefail

log() { echo "[stage-6] $*"; }

# Ensure cargo is available
if [[ -f "/root/.cargo/env" ]]; then
    source "/root/.cargo/env"
fi
GENTLY_HOME=$(eval echo "~${GENTLY_USER}")
if [[ -f "${GENTLY_HOME}/.cargo/env" ]]; then
    source "${GENTLY_HOME}/.cargo/env"
fi

mkdir -p /opt/gentlyos

# --- Get the source ---
if [[ "${GENTLYOS_INSTALL_MODE}" == "source" ]]; then
    log "Setting up GentlyOS from source..."

    if [[ -d "${GENTLYOS_REPO_PATH}" && -f "${GENTLYOS_REPO_PATH}/Cargo.toml" ]]; then
        log "Repo already exists at ${GENTLYOS_REPO_PATH}, pulling latest..."
        cd "$GENTLYOS_REPO_PATH"
        git pull origin master 2>/dev/null || git pull 2>/dev/null || log "Pull skipped (no remote or offline)"
    elif [[ -d "$SCRIPT_DIR/../../Cargo.toml" ]] || [[ -f "$SCRIPT_DIR/../../Cargo.toml" ]]; then
        # We're running from within the repo - copy it
        log "Copying repo from $SCRIPT_DIR/../.."
        REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
        if [[ "$REPO_ROOT" != "$GENTLYOS_REPO_PATH" ]]; then
            cp -a "$REPO_ROOT" "$GENTLYOS_REPO_PATH"
        else
            log "Already at repo path."
        fi
    else
        log "Cloning GentlyOS repository..."
        git clone https://github.com/Zero2oneZ/GentlyOS-Rusted-Metal.git "$GENTLYOS_REPO_PATH"
    fi

    cd "$GENTLYOS_REPO_PATH"

    # --- Build ---
    log "Building GentlyOS (release mode)..."
    cargo build --release -p gently-cli 2>&1 | tail -5

    # Build web interface
    if cargo build --release -p gently-web 2>&1 | tail -5; then
        log "gently-web built successfully."
    else
        log "WARNING: gently-web build failed (non-critical)."
    fi

    # --- Install binaries ---
    log "Installing binaries..."
    install -m 755 target/release/gently /usr/local/bin/gently 2>/dev/null || \
        cp target/release/gently /usr/local/bin/gently

    if [[ -f target/release/gently-web ]]; then
        install -m 755 target/release/gently-web /usr/local/bin/gently-web 2>/dev/null || \
            cp target/release/gently-web /usr/local/bin/gently-web
    fi

else
    # Binary install
    log "Installing GentlyOS from binary..."
    if [[ -f "$SCRIPT_DIR/../../web/install.sh" ]]; then
        bash "$SCRIPT_DIR/../../web/install.sh" --skip-setup
    else
        curl -fsSL https://gentlyos.com/install.sh | bash -s -- --skip-setup
    fi
fi

# --- Run initial setup for the gently user ---
log "Running GentlyOS setup for user '${GENTLY_USER}'..."
su - "$GENTLY_USER" -c "gently setup --force" 2>/dev/null || {
    log "Auto-setup skipped (may need manual gently setup)."
    # Create minimal directory structure
    su - "$GENTLY_USER" -c "mkdir -p ~/.gently/{alexandria,brain,feed,models,vault}"
}

# --- Set permissions ---
chown -R "${GENTLY_USER}:${GENTLY_USER}" /opt/gentlyos
chown -R "${GENTLY_USER}:${GENTLY_USER}" "${GENTLY_HOME}/.gently" 2>/dev/null || true

log "GentlyOS installed successfully."
log "  Binary: $(which gently 2>/dev/null || echo '/usr/local/bin/gently')"
log "  Data:   ${GENTLY_HOME}/.gently/"
