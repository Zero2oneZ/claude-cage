#!/usr/bin/env bash
set -euo pipefail
#
# setup-local-network.sh — Configure your machine for direct ethernet to the 3090
#
# This sets up:
#   - Static IP 10.0.0.1/30 on your ethernet port (enp2s0)
#   - IP forwarding so the 3090 can reach the internet
#   - NAT masquerading via your WiFi connection
#
# After running this, the 3090 at 10.0.0.2 can:
#   - Communicate with your machine
#   - Reach the internet (via your WiFi)
#   - Download packages, models, etc.
#

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
LOCAL_INTERFACE="enp2s0"       # Your ethernet port
WIFI_INTERFACE="wlp3s0"        # Your WiFi interface
LOCAL_IP="10.0.0.1"            # IP for your machine
REMOTE_IP="10.0.0.2"           # IP for the 3090
NETMASK="30"                   # /30 = just 2 usable IPs

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
info()  { printf '\033[1;34m[INFO]\033[0m  %s\n' "$*"; }
ok()    { printf '\033[1;32m[ OK ]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m  %s\n' "$*"; }
err()   { printf '\033[1;31m[ERR ]\033[0m  %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo ""
    echo "========================================"
    echo "  Direct Ethernet Network Setup"
    echo "========================================"
    echo ""
    echo "  Your machine:  ${LOCAL_IP}/${NETMASK} on ${LOCAL_INTERFACE}"
    echo "  3090 machine:  ${REMOTE_IP}/${NETMASK}"
    echo ""

    # Check for root
    if [[ $EUID -ne 0 ]]; then
        err "This script must be run as root (use sudo)."
        exit 1
    fi

    # Check interface exists
    if ! ip link show "${LOCAL_INTERFACE}" &>/dev/null; then
        err "Interface ${LOCAL_INTERFACE} not found!"
        err "Available interfaces:"
        ip link show | grep -E "^[0-9]+:" | awk '{print "  " $2}' | tr -d ':'
        exit 1
    fi

    # Bring interface up
    info "Bringing up ${LOCAL_INTERFACE}..."
    ip link set "${LOCAL_INTERFACE}" up

    # Check for cable
    sleep 1
    local carrier
    carrier="$(cat /sys/class/net/${LOCAL_INTERFACE}/carrier 2>/dev/null || echo 0)"
    if [[ "${carrier}" != "1" ]]; then
        warn "No cable detected on ${LOCAL_INTERFACE}."
        warn "Connect the ethernet cable between this machine and the 3090."
    else
        ok "Cable detected on ${LOCAL_INTERFACE}"
    fi

    # Configure IP
    info "Configuring ${LOCAL_INTERFACE} with ${LOCAL_IP}/${NETMASK}..."
    ip addr flush dev "${LOCAL_INTERFACE}" 2>/dev/null || true
    ip addr add "${LOCAL_IP}/${NETMASK}" dev "${LOCAL_INTERFACE}"
    ok "IP configured"

    # Enable IP forwarding
    info "Enabling IP forwarding..."
    sysctl -w net.ipv4.ip_forward=1 >/dev/null
    # Make it persistent
    if ! grep -q "^net.ipv4.ip_forward=1" /etc/sysctl.conf 2>/dev/null; then
        echo "net.ipv4.ip_forward=1" >> /etc/sysctl.conf
    fi
    ok "IP forwarding enabled"

    # Set up NAT if WiFi is available
    if ip link show "${WIFI_INTERFACE}" &>/dev/null; then
        info "Setting up NAT masquerading (${LOCAL_INTERFACE} -> ${WIFI_INTERFACE})..."

        # MASQUERADE outbound traffic
        iptables -t nat -C POSTROUTING -o "${WIFI_INTERFACE}" -j MASQUERADE 2>/dev/null || \
            iptables -t nat -A POSTROUTING -o "${WIFI_INTERFACE}" -j MASQUERADE

        # Allow forwarding from ethernet to WiFi
        iptables -C FORWARD -i "${LOCAL_INTERFACE}" -o "${WIFI_INTERFACE}" -j ACCEPT 2>/dev/null || \
            iptables -A FORWARD -i "${LOCAL_INTERFACE}" -o "${WIFI_INTERFACE}" -j ACCEPT

        # Allow established connections back
        iptables -C FORWARD -i "${WIFI_INTERFACE}" -o "${LOCAL_INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
            iptables -A FORWARD -i "${WIFI_INTERFACE}" -o "${LOCAL_INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT

        ok "NAT configured — 3090 can reach internet via your WiFi"
    else
        warn "WiFi interface ${WIFI_INTERFACE} not found."
        warn "3090 will not have internet access (local connection only)."
    fi

    echo ""
    echo "========================================"
    echo "  Setup Complete"
    echo "========================================"
    echo ""
    echo "  Your machine:  ${LOCAL_IP}"
    echo "  3090 target:   ${REMOTE_IP}"
    echo ""
    echo "  To test connectivity after 3090 boots:"
    echo "    ping ${REMOTE_IP}"
    echo "    ssh zero20nez@${REMOTE_IP}"
    echo ""
    echo "  Or run: make find"
    echo ""
}

main "$@"
