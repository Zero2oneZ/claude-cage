#!/usr/bin/env bash
set -euo pipefail
#
# find-machine.sh — Discover the GPU-3090 machine on the local network.
#
# Scans 192.168.1.0/24 using multiple discovery methods, then verifies SSH
# connectivity. Saves the discovered IP to .target-ip for use by other scripts
# and the Makefile.
#
# Discovery methods (in order of preference):
#   1. arp-scan --localnet
#   2. nmap -sn 192.168.1.0/24
#   3. Fallback: ping sweep + arp table
#

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_IP_FILE="${PROJECT_DIR}/.target-ip"
TARGET_HOSTNAME="gpu-3090"
TARGET_USER="zero20nez"
# Direct ethernet connection — no router, static IPs
# Your machine: 10.0.0.1/30, 3090: 10.0.0.2/30
NETWORK="10.0.0.0/30"
TARGET_STATIC_IP="10.0.0.2"
GATEWAY="10.0.0.1"
LOCAL_INTERFACE="enp2s0"
SSH_KEY="${PROJECT_DIR}/keys/3090-headless"
SSH_TIMEOUT=2
SSH_OPTS=(-o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
info()  { printf '\033[1;34m[INFO]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m  %s\n' "$*"; }
ok()    { printf '\033[1;32m[ OK ]\033[0m  %s\n' "$*"; }
err()   { printf '\033[1;31m[ERR ]\033[0m  %s\n' "$*" >&2; }

# Collect unique IPs from all discovery methods
declare -A DISCOVERED_IPS

# ---------------------------------------------------------------------------
# Configure local ethernet interface for direct connection
# ---------------------------------------------------------------------------
setup_local_interface() {
    info "Setting up local ethernet interface ${LOCAL_INTERFACE} for direct connection..."

    # Check if interface exists
    if ! ip link show "${LOCAL_INTERFACE}" &>/dev/null; then
        err "Interface ${LOCAL_INTERFACE} not found!"
        err "Available interfaces:"
        ip link show | grep -E "^[0-9]+:" | awk '{print "  " $2}' | tr -d ':'
        exit 1
    fi

    # Check if cable is connected
    local carrier
    carrier="$(cat /sys/class/net/${LOCAL_INTERFACE}/carrier 2>/dev/null || echo 0)"
    if [[ "${carrier}" != "1" ]]; then
        warn "No cable detected on ${LOCAL_INTERFACE}!"
        warn "Please connect the ethernet cable between this machine and the 3090."
        echo ""
        read -r -p "Press Enter once the cable is connected..."

        # Bring interface up to detect cable
        sudo ip link set "${LOCAL_INTERFACE}" up 2>/dev/null || true
        sleep 2

        carrier="$(cat /sys/class/net/${LOCAL_INTERFACE}/carrier 2>/dev/null || echo 0)"
        if [[ "${carrier}" != "1" ]]; then
            err "Still no cable detected. Check the physical connection."
            exit 1
        fi
    fi
    ok "Cable detected on ${LOCAL_INTERFACE}"

    # Configure the IP address
    local current_ip
    current_ip="$(ip addr show "${LOCAL_INTERFACE}" 2>/dev/null | grep -oP 'inet \K[\d.]+' || true)"

    if [[ "${current_ip}" == "${GATEWAY}" ]]; then
        ok "Interface already configured with ${GATEWAY}/30"
    else
        info "Configuring ${LOCAL_INTERFACE} with IP ${GATEWAY}/30..."
        sudo ip addr flush dev "${LOCAL_INTERFACE}" 2>/dev/null || true
        sudo ip addr add "${GATEWAY}/30" dev "${LOCAL_INTERFACE}" 2>/dev/null
        sudo ip link set "${LOCAL_INTERFACE}" up
        ok "Interface configured: ${LOCAL_INTERFACE} = ${GATEWAY}/30"
    fi

    # Enable IP forwarding so the 3090 can reach the internet through this machine
    info "Enabling IP forwarding for internet access..."
    sudo sysctl -w net.ipv4.ip_forward=1 &>/dev/null || true

    # Set up NAT so 3090 can reach the internet via your WiFi
    local wifi_if="wlp3s0"
    if ip link show "${wifi_if}" &>/dev/null; then
        info "Setting up NAT from ${LOCAL_INTERFACE} to ${wifi_if}..."
        sudo iptables -t nat -C POSTROUTING -o "${wifi_if}" -j MASQUERADE 2>/dev/null || \
            sudo iptables -t nat -A POSTROUTING -o "${wifi_if}" -j MASQUERADE
        sudo iptables -C FORWARD -i "${LOCAL_INTERFACE}" -o "${wifi_if}" -j ACCEPT 2>/dev/null || \
            sudo iptables -A FORWARD -i "${LOCAL_INTERFACE}" -o "${wifi_if}" -j ACCEPT
        sudo iptables -C FORWARD -i "${wifi_if}" -o "${LOCAL_INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
            sudo iptables -A FORWARD -i "${wifi_if}" -o "${LOCAL_INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT
        ok "NAT configured — 3090 can reach internet via your WiFi"
    else
        warn "WiFi interface ${wifi_if} not found — 3090 won't have internet access"
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Discovery Method 0: Listen for UDP beacon (fastest — machine broadcasts its IP)
# The autoinstall sets up install-beacon.service which broadcasts
# "GPU3090-READY:<ip>" on UDP port 9999 every 5 seconds.
# ---------------------------------------------------------------------------
discover_beacon() {
    info "Listening for GPU-3090 beacon on UDP port 9999 (15 second timeout)..."
    local beacon_ip=""

    # Try socat first, fall back to netcat
    if command -v socat &>/dev/null; then
        beacon_ip="$(timeout 15 socat -u UDP4-RECV:9999,reuseaddr,broadcast STDOUT 2>/dev/null | grep -oP 'GPU3090-READY:\K[\d.]+' | head -1 || true)"
    elif command -v nc &>/dev/null; then
        beacon_ip="$(timeout 15 nc -u -l -p 9999 2>/dev/null | grep -oP 'GPU3090-READY:\K[\d.]+' | head -1 || true)"
    else
        warn "Neither socat nor nc available — skipping beacon listener."
        return 0
    fi

    if [[ -n "${beacon_ip}" ]]; then
        ok "Beacon received! GPU-3090 is at ${beacon_ip}"
        DISCOVERED_IPS["${beacon_ip}"]=1
        BEACON_IP="${beacon_ip}"
    else
        info "No beacon received (machine may still be installing, or beacon not running)."
    fi
}

BEACON_IP=""

# ---------------------------------------------------------------------------
# Discovery Method 1: arp-scan
# ---------------------------------------------------------------------------
discover_arpscan() {
    if ! command -v arp-scan &>/dev/null; then
        warn "arp-scan not found, skipping."
        return 0
    fi
    info "Scanning with arp-scan..."
    local output
    output="$(sudo arp-scan --localnet 2>/dev/null || true)"
    while IFS= read -r line; do
        local ip
        ip="$(echo "${line}" | grep -oP '^\d+\.\d+\.\d+\.\d+' || true)"
        if [[ -n "${ip}" && "${ip}" != "${GATEWAY}" ]]; then
            DISCOVERED_IPS["${ip}"]=1
        fi
    done <<< "${output}"
}

# ---------------------------------------------------------------------------
# Discovery Method 2: nmap
# ---------------------------------------------------------------------------
discover_nmap() {
    if ! command -v nmap &>/dev/null; then
        warn "nmap not found, skipping."
        return 0
    fi
    info "Scanning with nmap -sn ${NETWORK}..."
    local output
    output="$(nmap -sn "${NETWORK}" 2>/dev/null || true)"
    while IFS= read -r line; do
        local ip
        ip="$(echo "${line}" | grep -oP '\d+\.\d+\.\d+\.\d+' || true)"
        if [[ -n "${ip}" && "${ip}" != "${GATEWAY}" ]]; then
            DISCOVERED_IPS["${ip}"]=1
        fi
    done <<< "${output}"
}

# ---------------------------------------------------------------------------
# Discovery Method 3: Fallback ping sweep + arp table
# ---------------------------------------------------------------------------
discover_ping_sweep() {
    info "Running ping sweep on ${NETWORK} (fallback method)..."

    # Extract the base network (e.g., 192.168.1)
    local base
    base="$(echo "${NETWORK}" | cut -d'.' -f1-3)"

    # Ping sweep — send one ping to each host, in parallel
    for i in $(seq 1 254); do
        ping -c 1 -W 1 "${base}.${i}" &>/dev/null &
    done

    # Wait for all pings to complete (with a timeout)
    info "Waiting for ping sweep to complete..."
    sleep 5
    # Kill any remaining pings
    jobs -p | xargs -r kill 2>/dev/null || true
    wait 2>/dev/null || true

    # Now read the ARP table
    info "Reading ARP table..."
    local arp_output
    arp_output="$(arp -n 2>/dev/null || ip neigh show 2>/dev/null || true)"
    while IFS= read -r line; do
        local ip
        ip="$(echo "${line}" | grep -oP '^\d+\.\d+\.\d+\.\d+|(?<=\s)\d+\.\d+\.\d+\.\d+' | head -1 || true)"
        if [[ -n "${ip}" && "${ip}" != "${GATEWAY}" && "${ip}" =~ ^192\.168\.1\. ]]; then
            # Only include entries that resolved (not incomplete)
            if ! echo "${line}" | grep -q "incomplete"; then
                DISCOVERED_IPS["${ip}"]=1
            fi
        fi
    done <<< "${arp_output}"
}

# ---------------------------------------------------------------------------
# Probe a host via SSH to check hostname
# ---------------------------------------------------------------------------
probe_ssh() {
    local ip="$1"
    local hostname

    # Determine SSH key arguments
    local key_args=()
    if [[ -f "${SSH_KEY}" ]]; then
        key_args=(-i "${SSH_KEY}")
    fi

    hostname="$(ssh "${SSH_OPTS[@]}" "${key_args[@]}" "${TARGET_USER}@${ip}" hostname 2>/dev/null || true)"
    echo "${hostname}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo ""
    info "GPU-3090 Direct Ethernet Connection"
    info "Target: ${TARGET_STATIC_IP} (static IP configured in autoinstall)"
    echo ""

    # -----------------------------------------------------------------------
    # Step 1: Configure local ethernet interface
    # -----------------------------------------------------------------------
    setup_local_interface

    # -----------------------------------------------------------------------
    # Step 2: Direct connection — we know the IP, just wait for it to respond
    # -----------------------------------------------------------------------
    info "Waiting for 3090 to come online at ${TARGET_STATIC_IP}..."
    echo ""

    # First try the beacon (fastest feedback)
    discover_beacon

    # If no beacon, the machine might still be installing or beacon not running yet
    # Just try pinging the known IP directly
    if [[ -z "${BEACON_IP}" ]]; then
        info "No beacon received. Trying direct ping to ${TARGET_STATIC_IP}..."
        local attempts=0
        local max_attempts=60  # 5 minutes max wait
        while (( attempts < max_attempts )); do
            if ping -c 1 -W 1 "${TARGET_STATIC_IP}" &>/dev/null; then
                ok "Host ${TARGET_STATIC_IP} is responding to ping!"
                DISCOVERED_IPS["${TARGET_STATIC_IP}"]=1
                break
            fi
            ((attempts++))
            printf "\r  Waiting... %d/%d (Ctrl+C to cancel)" "${attempts}" "${max_attempts}"
            sleep 5
        done
        echo ""

        if (( attempts >= max_attempts )); then
            err "Timeout waiting for ${TARGET_STATIC_IP} to respond."
            err "Possible causes:"
            err "  - 3090 is still installing Ubuntu"
            err "  - 3090 didn't boot from USB"
            err "  - Ethernet cable not connected"
            err "  - Wrong interface configured"
            exit 1
        fi
    else
        DISCOVERED_IPS["${TARGET_STATIC_IP}"]=1
    fi

    # -----------------------------------------------------------------------
    # Step 3: Wait for SSH to become available
    # -----------------------------------------------------------------------
    info "Waiting for SSH to become available on ${TARGET_STATIC_IP}..."

    local ssh_attempts=0
    local ssh_max=60  # 5 more minutes for SSH to come up
    while (( ssh_attempts < ssh_max )); do
        local hostname
        hostname="$(probe_ssh "${TARGET_STATIC_IP}")"
        if [[ -n "${hostname}" ]]; then
            ok "SSH is up! Hostname: ${hostname}"
            break
        fi
        ((ssh_attempts++))
        printf "\r  Waiting for SSH... %d/%d" "${ssh_attempts}" "${ssh_max}"
        sleep 5
    done
    echo ""

    if (( ssh_attempts >= ssh_max )); then
        err "Timeout waiting for SSH on ${TARGET_STATIC_IP}."
        err "The machine responds to ping but SSH is not available."
        err "Possible causes:"
        err "  - SSH server not installed or not started"
        err "  - Firewall blocking port 22"
        err "  - Still in early boot phase"
        exit 1
    fi

    # -----------------------------------------------------------------------
    # Step 4: Save and test connectivity
    # -----------------------------------------------------------------------
    save_target_ip "${TARGET_STATIC_IP}"
    test_full_connectivity "${TARGET_STATIC_IP}"
}

# ---------------------------------------------------------------------------
# Save the target IP to .target-ip
# ---------------------------------------------------------------------------
save_target_ip() {
    local ip="$1"
    echo "${ip}" > "${TARGET_IP_FILE}"
    ok "Target IP saved to ${TARGET_IP_FILE}"
}

# ---------------------------------------------------------------------------
# Test full SSH connectivity
# ---------------------------------------------------------------------------
test_full_connectivity() {
    local ip="$1"
    echo ""
    info "Testing full SSH connectivity to ${TARGET_USER}@${ip}..."

    local key_args=()
    if [[ -f "${SSH_KEY}" ]]; then
        key_args=(-i "${SSH_KEY}")
    fi

    # Run a suite of quick checks
    local ok_count=0
    local tests=0

    # Test 1: Basic SSH connection
    ((tests++))
    if ssh "${SSH_OPTS[@]}" "${key_args[@]}" "${TARGET_USER}@${ip}" "echo 'SSH connection OK'" 2>/dev/null; then
        ((ok_count++))
    else
        warn "  Basic SSH connection failed."
    fi

    # Test 2: Check sudo access
    ((tests++))
    if ssh "${SSH_OPTS[@]}" "${key_args[@]}" "${TARGET_USER}@${ip}" "sudo -n true" 2>/dev/null; then
        ok "  Passwordless sudo: available"
        ((ok_count++))
    else
        warn "  Passwordless sudo: not available (may need password)"
    fi

    # Test 3: Check nvidia-smi
    ((tests++))
    if ssh "${SSH_OPTS[@]}" "${key_args[@]}" "${TARGET_USER}@${ip}" "nvidia-smi --query-gpu=name --format=csv,noheader" 2>/dev/null; then
        ok "  NVIDIA GPU detected"
        ((ok_count++))
    else
        warn "  nvidia-smi not available (GPU drivers may not be installed yet)"
    fi

    echo ""
    ok "Connectivity test complete: ${ok_count}/${tests} checks passed."
    echo ""
    info "You can now run:"
    info "  make provision   — Install ML stack on the 3090"
    info "  make dropbear    — Configure LUKS remote unlock"
    info "  make verify      — Run system health check"
}

main "$@"
