#!/usr/bin/env bash
set -euo pipefail
#
# pxe-server.sh — PXE Boot Server for headless Ubuntu installation
#
# This turns your machine into a PXE server that serves the Ubuntu installer
# to the 3090 over the direct ethernet connection. No USB needed.
#
# How it works:
#   1. Runs a DHCP server (dnsmasq) that offers PXE boot to the 3090
#   2. Runs a TFTP server with the Ubuntu netboot files
#   3. Runs an HTTP server with the full ISO and autoinstall config
#   4. When the 3090 powers on with network boot enabled, it:
#      - Gets IP from our DHCP
#      - Downloads bootloader via TFTP
#      - Boots Ubuntu installer via HTTP
#      - Runs autoinstall automatically
#
# Requirements: dnsmasq, wget, xorriso or 7z
#
# Usage:
#   sudo ./scripts/pxe-server.sh start    # Start the PXE server
#   sudo ./scripts/pxe-server.sh stop     # Stop the PXE server
#   sudo ./scripts/pxe-server.sh status   # Check if running
#

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_DIR="${PROJECT_DIR}/config"

# Network settings (direct ethernet connection)
INTERFACE="enp2s0"
SERVER_IP="10.0.0.1"
CLIENT_IP="10.0.0.2"
NETMASK="255.255.255.252"

# PXE server directories
PXE_ROOT="/srv/pxe"
TFTP_ROOT="${PXE_ROOT}/tftp"
HTTP_ROOT="${PXE_ROOT}/http"
LOG_DIR="${PXE_ROOT}/log"

# Ports
HTTP_PORT="8080"
TFTP_PORT="69"

# Ubuntu ISO
ISO_NAME="ubuntu-24.04.1-live-server-amd64.iso"
ISO_URL="https://releases.ubuntu.com/24.04.1/${ISO_NAME}"
ISO_PATH="${PROJECT_DIR}/${ISO_NAME}"

# PID files
DNSMASQ_PID="/var/run/pxe-dnsmasq.pid"
HTTP_PID="/var/run/pxe-http.pid"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}    $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}      $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}    $*"; }
err()     { echo -e "${RED}[ERROR]${NC}   $*" >&2; }
step()    { echo -e "\n${CYAN}${BOLD}▶ $*${NC}"; }

check_root() {
    if [[ $EUID -ne 0 ]]; then
        err "This script must be run as root (use sudo)."
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Install dependencies
# ---------------------------------------------------------------------------
install_deps() {
    step "Checking dependencies"

    local missing=()

    command -v dnsmasq &>/dev/null || missing+=("dnsmasq")
    command -v python3 &>/dev/null || missing+=("python3")
    command -v wget &>/dev/null || missing+=("wget")
    (command -v xorriso &>/dev/null || command -v 7z &>/dev/null) || missing+=("p7zip-full")

    if [[ ${#missing[@]} -gt 0 ]]; then
        info "Installing: ${missing[*]}"
        apt-get update -qq
        apt-get install -y "${missing[@]}"
    fi

    ok "Dependencies ready"
}

# ---------------------------------------------------------------------------
# Download and extract Ubuntu ISO
# ---------------------------------------------------------------------------
prepare_iso() {
    step "Preparing Ubuntu ISO"

    # Download if needed
    if [[ ! -f "${ISO_PATH}" ]]; then
        info "Downloading Ubuntu 24.04.1 Server ISO..."
        wget --show-progress -O "${ISO_PATH}" "${ISO_URL}"
    else
        ok "ISO already exists: ${ISO_PATH}"
    fi

    # Create directories
    mkdir -p "${TFTP_ROOT}" "${HTTP_ROOT}" "${LOG_DIR}"

    # Extract ISO to HTTP root (for casper/filesystem access)
    if [[ ! -d "${HTTP_ROOT}/casper" ]]; then
        info "Extracting ISO to ${HTTP_ROOT}..."
        if command -v xorriso &>/dev/null; then
            xorriso -osirrox on -indev "${ISO_PATH}" -extract / "${HTTP_ROOT}" 2>/dev/null
        else
            7z x -o"${HTTP_ROOT}" "${ISO_PATH}" -y >/dev/null
        fi
        chmod -R a+r "${HTTP_ROOT}"
        ok "ISO extracted"
    else
        ok "ISO already extracted"
    fi

    # Copy autoinstall config
    info "Setting up autoinstall configuration..."
    mkdir -p "${HTTP_ROOT}/server"
    cp "${CONFIG_DIR}/autoinstall.yaml" "${HTTP_ROOT}/server/user-data"
    cp "${CONFIG_DIR}/meta-data" "${HTTP_ROOT}/server/meta-data"

    # Copy forensics
    if [[ -d "${PROJECT_DIR}/forensics" ]]; then
        cp -r "${PROJECT_DIR}/forensics" "${HTTP_ROOT}/"
    fi

    # Copy scripts
    mkdir -p "${HTTP_ROOT}/scripts"
    cp "${PROJECT_DIR}/scripts/post-install.sh" "${HTTP_ROOT}/scripts/" 2>/dev/null || true

    ok "Autoinstall config ready"
}

# ---------------------------------------------------------------------------
# Set up TFTP boot files
# ---------------------------------------------------------------------------
setup_tftp() {
    step "Setting up TFTP boot files"

    # For UEFI PXE boot, we need:
    # - bootx64.efi (GRUB EFI bootloader)
    # - grub/grub.cfg (GRUB config pointing to HTTP server)

    mkdir -p "${TFTP_ROOT}/grub"

    # Extract GRUB EFI from the ISO
    if [[ ! -f "${TFTP_ROOT}/bootx64.efi" ]]; then
        info "Extracting GRUB EFI bootloader..."

        # Try to find it in the extracted ISO
        local grub_efi=""
        for path in \
            "${HTTP_ROOT}/EFI/BOOT/BOOTX64.EFI" \
            "${HTTP_ROOT}/EFI/BOOT/bootx64.efi" \
            "${HTTP_ROOT}/boot/grub/x86_64-efi/grub.efi"; do
            if [[ -f "${path}" ]]; then
                grub_efi="${path}"
                break
            fi
        done

        if [[ -n "${grub_efi}" ]]; then
            cp "${grub_efi}" "${TFTP_ROOT}/bootx64.efi"
            ok "GRUB EFI copied from ISO"
        else
            # Download grub-efi-amd64-signed package and extract
            warn "GRUB EFI not found in ISO, downloading..."
            apt-get download grub-efi-amd64-signed 2>/dev/null
            dpkg-deb -x grub-efi-amd64-signed*.deb /tmp/grub-extract
            cp /tmp/grub-extract/usr/lib/grub/x86_64-efi-signed/grubnetx64.efi.signed "${TFTP_ROOT}/bootx64.efi"
            rm -rf /tmp/grub-extract grub-efi-amd64-signed*.deb
            ok "GRUB EFI downloaded"
        fi
    fi

    # Create GRUB config for PXE boot
    info "Creating GRUB PXE config..."
    cat > "${TFTP_ROOT}/grub/grub.cfg" <<GRUBCFG
set timeout=1
set default=0

menuentry "Ubuntu 24.04 Autoinstall (PXE)" {
    linux (http,${SERVER_IP}:${HTTP_PORT})/casper/vmlinuz \\
        ip=dhcp \\
        url=http://${SERVER_IP}:${HTTP_PORT}/ubuntu-24.04.1-live-server-amd64.iso \\
        autoinstall ds=nocloud-net\\;s=http://${SERVER_IP}:${HTTP_PORT}/server/ \\
        console=ttyS0,115200n8 \\
        ---
    initrd (http,${SERVER_IP}:${HTTP_PORT})/casper/initrd
}
GRUBCFG

    ok "TFTP boot files ready"
}

# ---------------------------------------------------------------------------
# Configure network interface
# ---------------------------------------------------------------------------
setup_network() {
    step "Configuring network interface ${INTERFACE}"

    # Check interface exists
    if ! ip link show "${INTERFACE}" &>/dev/null; then
        err "Interface ${INTERFACE} not found!"
        exit 1
    fi

    # Bring up and configure
    ip link set "${INTERFACE}" up
    ip addr flush dev "${INTERFACE}" 2>/dev/null || true
    ip addr add "${SERVER_IP}/30" dev "${INTERFACE}"

    # Enable IP forwarding for internet access
    sysctl -w net.ipv4.ip_forward=1 >/dev/null

    # NAT via WiFi
    local wifi="wlp3s0"
    if ip link show "${wifi}" &>/dev/null; then
        iptables -t nat -C POSTROUTING -o "${wifi}" -j MASQUERADE 2>/dev/null || \
            iptables -t nat -A POSTROUTING -o "${wifi}" -j MASQUERADE
        iptables -C FORWARD -i "${INTERFACE}" -o "${wifi}" -j ACCEPT 2>/dev/null || \
            iptables -A FORWARD -i "${INTERFACE}" -o "${wifi}" -j ACCEPT
        iptables -C FORWARD -i "${wifi}" -o "${INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
            iptables -A FORWARD -i "${wifi}" -o "${INTERFACE}" -m state --state RELATED,ESTABLISHED -j ACCEPT
        info "NAT configured via ${wifi}"
    fi

    ok "Network configured: ${SERVER_IP}/30"
}

# ---------------------------------------------------------------------------
# Start dnsmasq (DHCP + TFTP + PXE)
# ---------------------------------------------------------------------------
start_dnsmasq() {
    step "Starting dnsmasq (DHCP + TFTP + PXE)"

    # Stop any existing instance
    if [[ -f "${DNSMASQ_PID}" ]]; then
        kill "$(cat "${DNSMASQ_PID}")" 2>/dev/null || true
        rm -f "${DNSMASQ_PID}"
    fi

    # Also stop system dnsmasq if running
    systemctl stop dnsmasq 2>/dev/null || true

    # Create dnsmasq config
    cat > /tmp/pxe-dnsmasq.conf <<DNSMASQ
# PXE Boot Server Configuration

# Interface
interface=${INTERFACE}
bind-interfaces

# DHCP range (just one IP for the 3090)
dhcp-range=${CLIENT_IP},${CLIENT_IP},${NETMASK},12h

# Gateway (this machine, for internet access)
dhcp-option=3,${SERVER_IP}

# DNS servers
dhcp-option=6,1.1.1.1,8.8.8.8

# TFTP server
enable-tftp
tftp-root=${TFTP_ROOT}

# PXE boot for UEFI clients
dhcp-match=set:efi-x86_64,option:client-arch,7
dhcp-match=set:efi-x86_64,option:client-arch,9
dhcp-boot=tag:efi-x86_64,bootx64.efi

# PXE boot for BIOS clients (legacy)
dhcp-match=set:bios,option:client-arch,0
dhcp-boot=tag:bios,pxelinux.0

# Logging
log-queries
log-dhcp
log-facility=${LOG_DIR}/dnsmasq.log
DNSMASQ

    # Start dnsmasq
    dnsmasq --conf-file=/tmp/pxe-dnsmasq.conf --pid-file="${DNSMASQ_PID}"

    ok "dnsmasq started (DHCP: ${CLIENT_IP}, TFTP: ${TFTP_ROOT})"
}

# ---------------------------------------------------------------------------
# Start HTTP server
# ---------------------------------------------------------------------------
start_http() {
    step "Starting HTTP server on port ${HTTP_PORT}"

    # Stop any existing instance
    if [[ -f "${HTTP_PID}" ]]; then
        kill "$(cat "${HTTP_PID}")" 2>/dev/null || true
        rm -f "${HTTP_PID}"
    fi

    # Also symlink the ISO to HTTP root for direct download
    ln -sf "${ISO_PATH}" "${HTTP_ROOT}/${ISO_NAME}" 2>/dev/null || \
        cp "${ISO_PATH}" "${HTTP_ROOT}/${ISO_NAME}"

    # Start Python HTTP server
    cd "${HTTP_ROOT}"
    python3 -m http.server "${HTTP_PORT}" --bind "${SERVER_IP}" \
        > "${LOG_DIR}/http.log" 2>&1 &
    echo $! > "${HTTP_PID}"

    ok "HTTP server started: http://${SERVER_IP}:${HTTP_PORT}/"
}

# ---------------------------------------------------------------------------
# Stop all services
# ---------------------------------------------------------------------------
stop_services() {
    step "Stopping PXE services"

    if [[ -f "${DNSMASQ_PID}" ]]; then
        kill "$(cat "${DNSMASQ_PID}")" 2>/dev/null || true
        rm -f "${DNSMASQ_PID}"
        ok "dnsmasq stopped"
    fi

    if [[ -f "${HTTP_PID}" ]]; then
        kill "$(cat "${HTTP_PID}")" 2>/dev/null || true
        rm -f "${HTTP_PID}"
        ok "HTTP server stopped"
    fi
}

# ---------------------------------------------------------------------------
# Show status
# ---------------------------------------------------------------------------
show_status() {
    echo ""
    echo "=== PXE Server Status ==="
    echo ""

    if [[ -f "${DNSMASQ_PID}" ]] && kill -0 "$(cat "${DNSMASQ_PID}")" 2>/dev/null; then
        echo -e "  dnsmasq:     ${GREEN}RUNNING${NC} (PID $(cat "${DNSMASQ_PID}"))"
    else
        echo -e "  dnsmasq:     ${RED}STOPPED${NC}"
    fi

    if [[ -f "${HTTP_PID}" ]] && kill -0 "$(cat "${HTTP_PID}")" 2>/dev/null; then
        echo -e "  HTTP server: ${GREEN}RUNNING${NC} (PID $(cat "${HTTP_PID}")) — http://${SERVER_IP}:${HTTP_PORT}/"
    else
        echo -e "  HTTP server: ${RED}STOPPED${NC}"
    fi

    echo ""
    echo "  Interface:   ${INTERFACE}"
    echo "  Server IP:   ${SERVER_IP}"
    echo "  Client IP:   ${CLIENT_IP} (will be assigned via DHCP)"
    echo ""
    echo "  TFTP root:   ${TFTP_ROOT}"
    echo "  HTTP root:   ${HTTP_ROOT}"
    echo "  Logs:        ${LOG_DIR}/"
    echo ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    case "${1:-}" in
        start)
            check_root
            echo ""
            echo -e "${BOLD}${CYAN}========================================${NC}"
            echo -e "${BOLD}${CYAN}  PXE Boot Server for GPU-3090${NC}"
            echo -e "${BOLD}${CYAN}========================================${NC}"
            echo ""

            install_deps
            prepare_iso
            setup_tftp
            setup_network
            start_dnsmasq
            start_http

            echo ""
            echo -e "${BOLD}${GREEN}========================================${NC}"
            echo -e "${BOLD}${GREEN}  PXE Server Running${NC}"
            echo -e "${BOLD}${GREEN}========================================${NC}"
            echo ""
            echo "  The 3090 should now boot from the network automatically."
            echo ""
            echo "  1. Connect ethernet cable between this machine and the 3090"
            echo "  2. Power on the 3090"
            echo "  3. If BIOS has 'Network Boot' or 'PXE' enabled, it will boot Ubuntu"
            echo ""
            echo "  Monitor logs:"
            echo "    tail -f ${LOG_DIR}/dnsmasq.log"
            echo "    tail -f ${LOG_DIR}/http.log"
            echo ""
            echo "  Stop server:"
            echo "    sudo $0 stop"
            echo ""
            ;;
        stop)
            check_root
            stop_services
            echo ""
            ok "PXE server stopped."
            ;;
        status)
            show_status
            ;;
        *)
            echo "Usage: sudo $0 {start|stop|status}"
            echo ""
            echo "  start   - Start PXE boot server"
            echo "  stop    - Stop PXE boot server"
            echo "  status  - Show server status"
            exit 1
            ;;
    esac
}

main "$@"
