#!/usr/bin/env bash
# ============================================================
# Stage 7: Services & API Gateway
# ============================================================
# Sets up systemd services for GentlyOS components and an
# nginx reverse proxy so the 2nd PC can access everything
# through a unified gateway.
# ============================================================
set -euo pipefail

log() { echo "[stage-7] $*"; }

GENTLY_HOME=$(eval echo "~${GENTLY_USER}")

# ============================================================
# GentlyOS Web Service
# ============================================================
log "Creating GentlyOS web service..."

if command -v systemctl &>/dev/null; then
    cat > /etc/systemd/system/gentlyos-web.service <<WEBEOF
[Unit]
Description=GentlyOS Web Interface
After=network.target ollama.service
Wants=ollama.service

[Service]
Type=simple
User=${GENTLY_USER}
Group=${GENTLY_USER}
WorkingDirectory=${GENTLY_HOME}
Environment="PATH=${GENTLY_HOME}/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
ExecStart=/usr/local/bin/gently-web
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
WEBEOF

    # Enable if binary exists
    if [[ -f /usr/local/bin/gently-web ]]; then
        systemctl daemon-reload
        systemctl enable gentlyos-web
        systemctl start gentlyos-web 2>/dev/null || log "gently-web will start on next boot."
    else
        log "gently-web binary not found, service created but not started."
    fi
fi

# ============================================================
# API Gateway (nginx reverse proxy)
# ============================================================
log "Setting up API gateway on port ${API_PORT}..."

# Detect distro for nginx install
if [[ -f /etc/os-release ]]; then
    source /etc/os-release
    DISTRO="$ID"
else
    DISTRO="unknown"
fi

# Install nginx
case "$DISTRO" in
    ubuntu|debian|pop|linuxmint)
        apt-get install -y nginx ;;
    fedora|rhel|centos|rocky|almalinux)
        dnf install -y nginx ;;
    arch|manjaro)
        pacman -S --noconfirm --needed nginx ;;
    alpine)
        apk add nginx ;;
    opensuse*)
        zypper install -y nginx ;;
    *)
        log "WARNING: Could not install nginx. Install manually."
        ;;
esac

# Configure nginx as API gateway
NGINX_CONF="/etc/nginx/sites-available/gentlyos-gateway"
NGINX_ENABLED="/etc/nginx/sites-enabled/gentlyos-gateway"

# Handle distros that don't use sites-available
if [[ ! -d /etc/nginx/sites-available ]]; then
    mkdir -p /etc/nginx/sites-available /etc/nginx/sites-enabled
    # Add include to main nginx.conf if not present
    if ! grep -q "sites-enabled" /etc/nginx/nginx.conf 2>/dev/null; then
        sed -i '/http {/a\    include /etc/nginx/sites-enabled/*;' /etc/nginx/nginx.conf 2>/dev/null || true
    fi
fi

cat > "$NGINX_CONF" <<NGINXEOF
# GentlyOS Headless API Gateway
# Unified access point for all services on port ${API_PORT}

server {
    listen ${API_PORT};
    server_name _;

    # --- Status endpoint ---
    location /status {
        default_type application/json;
        return 200 '{"status":"ok","services":{"ollama":"http://localhost:${OLLAMA_PORT}","web":"http://localhost:${WEB_PORT}"},"node":"gentlyos-headless"}';
    }

    # --- Ollama API passthrough ---
    # Use from 2nd PC: curl http://${STATIC_IP:-HOST}:${API_PORT}/ollama/api/generate ...
    location /ollama/ {
        proxy_pass http://127.0.0.1:${OLLAMA_PORT}/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_read_timeout 600s;
        proxy_send_timeout 600s;
        proxy_buffering off;
        # Streaming support for LLM responses
        chunked_transfer_encoding on;
    }

    # --- GentlyOS Web UI ---
    location /gently/ {
        proxy_pass http://127.0.0.1:${WEB_PORT}/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    # --- Direct Ollama /api paths (convenience) ---
    location /api/ {
        proxy_pass http://127.0.0.1:${OLLAMA_PORT}/api/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_read_timeout 600s;
        proxy_buffering off;
        chunked_transfer_encoding on;
    }

    # --- Default: GentlyOS Web ---
    location / {
        proxy_pass http://127.0.0.1:${WEB_PORT}/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
NGINXEOF

# Enable the site
ln -sf "$NGINX_CONF" "$NGINX_ENABLED" 2>/dev/null || true

# Remove default site if it conflicts
rm -f /etc/nginx/sites-enabled/default 2>/dev/null || true

# Test and start nginx
if command -v nginx &>/dev/null; then
    nginx -t 2>&1 && log "Nginx config valid." || log "WARNING: Nginx config test failed."
    if command -v systemctl &>/dev/null; then
        systemctl enable nginx
        systemctl restart nginx 2>/dev/null || systemctl start nginx 2>/dev/null || true
    elif command -v rc-service &>/dev/null; then
        rc-update add nginx default
        rc-service nginx restart 2>/dev/null || true
    fi
fi

# ============================================================
# Connection helper script for the 2nd PC
# ============================================================
log "Creating connection helper script..."

cat > /opt/gentlyos/connect-from-remote.sh <<'REMOTEEOF'
#!/usr/bin/env bash
# ============================================================
# Run this on your 2ND PC to connect to the headless node.
# Copy this file to your other machine and execute it.
# ============================================================
set -euo pipefail

HEADLESS_IP="${1:-192.168.1.100}"
API_PORT="${2:-9090}"
SSH_PORT="${3:-22}"

echo "============================================"
echo "  GentlyOS Headless Node Connection Test"
echo "============================================"
echo ""
echo "Target: $HEADLESS_IP"
echo ""

# Test SSH
echo -n "[SSH]    Port $SSH_PORT ... "
if nc -z -w3 "$HEADLESS_IP" "$SSH_PORT" 2>/dev/null; then
    echo "OK"
else
    echo "FAIL (is SSH running?)"
fi

# Test Ollama
echo -n "[Ollama] Port 11434 ... "
if curl -sf "http://${HEADLESS_IP}:11434/api/tags" &>/dev/null; then
    echo "OK"
    echo "         Models: $(curl -sf "http://${HEADLESS_IP}:11434/api/tags" | python3 -c "import sys,json; [print('           -', m['name']) for m in json.load(sys.stdin).get('models',[])]" 2>/dev/null || echo "(could not list)")"
else
    echo "FAIL"
fi

# Test Gateway
echo -n "[Gateway] Port $API_PORT ... "
if curl -sf "http://${HEADLESS_IP}:${API_PORT}/status" &>/dev/null; then
    echo "OK"
else
    echo "FAIL"
fi

echo ""
echo "============================================"
echo "  Quick Start Commands"
echo "============================================"
echo ""
echo "# SSH into the headless node:"
echo "  ssh gently@${HEADLESS_IP} -p ${SSH_PORT}"
echo ""
echo "# Chat with the model via API:"
echo "  curl http://${HEADLESS_IP}:${API_PORT}/api/chat -d '{"
echo "    \"model\": \"qwen3:14b\","
echo "    \"messages\": [{\"role\": \"user\", \"content\": \"Hello\"}]"
echo "  }'"
echo ""
echo "# Use with Open WebUI, VS Code Continue, etc:"
echo "  Ollama URL: http://${HEADLESS_IP}:11434"
echo ""
echo "# Open GentlyOS Web UI:"
echo "  http://${HEADLESS_IP}:${API_PORT}/"
echo ""
REMOTEEOF

chmod +x /opt/gentlyos/connect-from-remote.sh
chown "${GENTLY_USER}:${GENTLY_USER}" /opt/gentlyos/connect-from-remote.sh

# Also copy to user home for easy access
cp /opt/gentlyos/connect-from-remote.sh "${GENTLY_HOME}/connect-from-remote.sh"
chown "${GENTLY_USER}:${GENTLY_USER}" "${GENTLY_HOME}/connect-from-remote.sh"

# ============================================================
# Summary
# ============================================================
log ""
log "================================================================"
log "  All services configured!"
log "================================================================"
log ""
log "  Ollama API:        http://${STATIC_IP:-0.0.0.0}:${OLLAMA_PORT}"
log "  GentlyOS Web:      http://${STATIC_IP:-0.0.0.0}:${WEB_PORT}"
log "  API Gateway:       http://${STATIC_IP:-0.0.0.0}:${API_PORT}"
log "  SSH:               ssh ${GENTLY_USER}@${STATIC_IP:-0.0.0.0} -p ${SSH_PORT}"
log ""
log "  Gateway routes:"
log "    /              → GentlyOS Web UI"
log "    /api/*         → Ollama API"
log "    /ollama/*      → Ollama API"
log "    /gently/*      → GentlyOS Web"
log "    /status        → Health check"
log ""
log "  Helper script: ~/connect-from-remote.sh"
log "================================================================"
