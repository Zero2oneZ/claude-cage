#!/usr/bin/env bash
# ============================================================
# Stage 5: Ollama & Models
# ============================================================
# Installs Ollama, configures it for network access, pulls
# the requested model (default: qwen3:14b).
# ============================================================
set -euo pipefail

log() { echo "[stage-5] $*"; }

# --- Install Ollama ---
if command -v ollama &>/dev/null; then
    log "Ollama already installed: $(ollama --version 2>/dev/null || echo 'unknown version')"
else
    log "Installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

# --- Configure Ollama for remote access ---
log "Configuring Ollama to listen on ${OLLAMA_HOST}:${OLLAMA_PORT}..."

# Create systemd override for Ollama
OLLAMA_SERVICE_DIR="/etc/systemd/system/ollama.service.d"
mkdir -p "$OLLAMA_SERVICE_DIR"

cat > "${OLLAMA_SERVICE_DIR}/override.conf" <<OLLAMAEOF
[Service]
Environment="OLLAMA_HOST=${OLLAMA_HOST}:${OLLAMA_PORT}"
Environment="OLLAMA_ORIGINS=*"
Environment="OLLAMA_MODELS=/opt/gentlyos/models/ollama"
OLLAMAEOF

# Create models directory
mkdir -p /opt/gentlyos/models/ollama
chown -R "${GENTLY_USER}:${GENTLY_USER}" /opt/gentlyos/models

# If no systemd, create a wrapper script
if ! command -v systemctl &>/dev/null; then
    log "No systemd detected, creating Ollama launcher script..."
    cat > /usr/local/bin/ollama-serve <<'SERVEEOF'
#!/bin/bash
export OLLAMA_HOST="${OLLAMA_HOST:-0.0.0.0}:${OLLAMA_PORT:-11434}"
export OLLAMA_ORIGINS="*"
export OLLAMA_MODELS="/opt/gentlyos/models/ollama"
exec ollama serve
SERVEEOF
    chmod +x /usr/local/bin/ollama-serve
fi

# --- Start Ollama ---
if command -v systemctl &>/dev/null; then
    systemctl daemon-reload
    systemctl enable ollama
    systemctl restart ollama
    # Wait for Ollama to be ready
    log "Waiting for Ollama to start..."
    for i in $(seq 1 30); do
        if curl -sf "http://127.0.0.1:${OLLAMA_PORT}/api/tags" &>/dev/null; then
            log "Ollama is running."
            break
        fi
        sleep 2
    done
else
    # Start in background
    OLLAMA_HOST="${OLLAMA_HOST}:${OLLAMA_PORT}" \
    OLLAMA_ORIGINS="*" \
    OLLAMA_MODELS="/opt/gentlyos/models/ollama" \
    nohup ollama serve > /var/log/gentlyos-setup/ollama-serve.log 2>&1 &
    sleep 5
fi

# --- Verify Ollama is responding ---
if ! curl -sf "http://127.0.0.1:${OLLAMA_PORT}/api/tags" &>/dev/null; then
    log "WARNING: Ollama may not be responding yet. Model pull might fail."
    log "You can pull models manually later: ollama pull ${OLLAMA_MODEL}"
fi

# --- Pull primary model ---
log "Pulling model: ${OLLAMA_MODEL} (this may take a while)..."
ollama pull "$OLLAMA_MODEL" || {
    log "WARNING: Model pull failed. Retrying in 10 seconds..."
    sleep 10
    ollama pull "$OLLAMA_MODEL" || {
        log "ERROR: Could not pull ${OLLAMA_MODEL}."
        log "Pull manually after setup: ollama pull ${OLLAMA_MODEL}"
    }
}

# --- Pull extra models ---
if [[ -n "${OLLAMA_EXTRA_MODELS:-}" ]]; then
    for model in $OLLAMA_EXTRA_MODELS; do
        log "Pulling extra model: $model"
        ollama pull "$model" || log "WARNING: Could not pull $model"
    done
fi

# --- Show installed models ---
log "Installed models:"
ollama list 2>/dev/null || log "(could not list models)"

log "Ollama setup complete."
log "  API: http://${STATIC_IP:-localhost}:${OLLAMA_PORT}"
log "  Model: ${OLLAMA_MODEL}"
