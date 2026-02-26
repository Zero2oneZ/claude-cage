#!/usr/bin/env bash
# ============================================================
# Stage 3: Ethernet & Networking
# ============================================================
# Configures static IP for direct ethernet connection from
# a 2nd PC, plus firewall rules for the services.
# ============================================================
set -euo pipefail

log() { echo "[stage-3] $*"; }

# --- Skip if DHCP mode ---
if [[ "${NETWORK_MODE}" == "dhcp" ]]; then
    log "Network mode is DHCP, skipping static IP configuration."
    log "The machine will get its IP from your router."
else
    log "Configuring static IP: $STATIC_IP"

    # --- Auto-detect ethernet interface ---
    if [[ -z "${ETH_INTERFACE:-}" ]]; then
        ETH_INTERFACE=$(ip -o link show | awk -F': ' '{print $2}' | grep -E '^(eth|en|ens|eno|enp)' | head -1)
        if [[ -z "$ETH_INTERFACE" ]]; then
            log "WARNING: Could not auto-detect ethernet interface."
            log "Available interfaces:"
            ip -o link show | awk -F': ' '{print "  " $2}'
            log "Set ETH_INTERFACE in config.env and re-run --stage 3"
            exit 1
        fi
        log "Auto-detected ethernet interface: $ETH_INTERFACE"
    fi

    # --- Configure using netplan (Ubuntu 18+) ---
    if command -v netplan &>/dev/null; then
        log "Using netplan for network configuration..."
        cat > /etc/netplan/99-gentlyos-headless.yaml <<NETEOF
network:
  version: 2
  ethernets:
    ${ETH_INTERFACE}:
      addresses:
        - ${STATIC_IP}/24
      routes:
        - to: default
          via: ${STATIC_GATEWAY}
      nameservers:
        addresses: [$(echo "$STATIC_DNS" | tr ',' ', ')]
      optional: true
NETEOF
        chmod 600 /etc/netplan/99-gentlyos-headless.yaml
        netplan apply 2>/dev/null || log "Netplan apply deferred to next boot."

    # --- Configure using NetworkManager ---
    elif command -v nmcli &>/dev/null; then
        log "Using NetworkManager for network configuration..."
        CONN_NAME="gentlyos-headless"
        nmcli connection delete "$CONN_NAME" 2>/dev/null || true
        nmcli connection add \
            type ethernet \
            con-name "$CONN_NAME" \
            ifname "$ETH_INTERFACE" \
            ip4 "${STATIC_IP}/24" \
            gw4 "${STATIC_GATEWAY}"
        nmcli connection modify "$CONN_NAME" \
            ipv4.dns "$(echo "$STATIC_DNS" | tr ',' ' ')" \
            ipv4.method manual \
            connection.autoconnect yes
        nmcli connection up "$CONN_NAME" 2>/dev/null || log "Connection will activate on next boot."

    # --- Configure using /etc/network/interfaces (Debian classic) ---
    elif [[ -d /etc/network ]]; then
        log "Using /etc/network/interfaces..."
        # Don't clobber existing config, add a new stanza
        if ! grep -q "gentlyos-headless" /etc/network/interfaces 2>/dev/null; then
            cat >> /etc/network/interfaces <<IFEOF

# gentlyos-headless static ethernet
auto ${ETH_INTERFACE}
iface ${ETH_INTERFACE} inet static
    address ${STATIC_IP}
    netmask ${STATIC_NETMASK}
    gateway ${STATIC_GATEWAY}
    dns-nameservers $(echo "$STATIC_DNS" | tr ',' ' ')
IFEOF
        fi
        ifup "$ETH_INTERFACE" 2>/dev/null || log "Interface will come up on next boot."

    # --- Configure using ip command directly (fallback) ---
    else
        log "No network manager found. Using ip command directly..."
        ip addr flush dev "$ETH_INTERFACE" 2>/dev/null || true
        ip addr add "${STATIC_IP}/24" dev "$ETH_INTERFACE"
        ip link set "$ETH_INTERFACE" up
        ip route add default via "$STATIC_GATEWAY" 2>/dev/null || true

        # Make persistent via rc.local
        RC_LOCAL="/etc/rc.local"
        if [[ ! -f "$RC_LOCAL" ]]; then
            echo "#!/bin/bash" > "$RC_LOCAL"
            chmod +x "$RC_LOCAL"
        fi
        if ! grep -q "gentlyos-headless" "$RC_LOCAL" 2>/dev/null; then
            sed -i '/^exit 0$/d' "$RC_LOCAL" 2>/dev/null || true
            cat >> "$RC_LOCAL" <<RCEOF
# gentlyos-headless static IP
ip addr add ${STATIC_IP}/24 dev ${ETH_INTERFACE} 2>/dev/null || true
ip link set ${ETH_INTERFACE} up
ip route add default via ${STATIC_GATEWAY} 2>/dev/null || true
exit 0
RCEOF
        fi
    fi

    log "Static IP $STATIC_IP configured on $ETH_INTERFACE"
fi

# --- Firewall ---
if [[ "${ENABLE_FIREWALL}" == "yes" ]]; then
    log "Configuring firewall rules..."

    if command -v ufw &>/dev/null; then
        ufw --force reset
        ufw default deny incoming
        ufw default allow outgoing
        ufw allow "${SSH_PORT}/tcp" comment "SSH"
        ufw allow "${OLLAMA_PORT}/tcp" comment "Ollama API"
        ufw allow "${WEB_PORT}/tcp" comment "GentlyOS Web"
        ufw allow "${API_PORT}/tcp" comment "GentlyOS API Gateway"
        ufw --force enable
        log "UFW firewall configured and enabled."

    elif command -v firewall-cmd &>/dev/null; then
        firewall-cmd --permanent --add-port="${SSH_PORT}/tcp"
        firewall-cmd --permanent --add-port="${OLLAMA_PORT}/tcp"
        firewall-cmd --permanent --add-port="${WEB_PORT}/tcp"
        firewall-cmd --permanent --add-port="${API_PORT}/tcp"
        firewall-cmd --reload
        log "firewalld configured."

    elif command -v iptables &>/dev/null; then
        # Basic iptables rules
        iptables -F INPUT
        iptables -A INPUT -i lo -j ACCEPT
        iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
        iptables -A INPUT -p tcp --dport "$SSH_PORT" -j ACCEPT
        iptables -A INPUT -p tcp --dport "$OLLAMA_PORT" -j ACCEPT
        iptables -A INPUT -p tcp --dport "$WEB_PORT" -j ACCEPT
        iptables -A INPUT -p tcp --dport "$API_PORT" -j ACCEPT
        iptables -A INPUT -p icmp -j ACCEPT
        iptables -A INPUT -j DROP

        # Try to persist
        if command -v iptables-save &>/dev/null; then
            mkdir -p /etc/iptables
            iptables-save > /etc/iptables/rules.v4
        fi
        log "iptables firewall configured."
    else
        log "WARNING: No firewall tool found. Ports are open by default."
    fi
else
    log "Firewall setup disabled in config."
fi

log "Networking setup complete."
