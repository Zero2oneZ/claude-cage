#!/usr/bin/env bash
# =============================================================================
# build-iso.sh — Build a custom Ubuntu 24.04 autoinstall ISO
# =============================================================================
# This script downloads the official Ubuntu 24.04.1 Server ISO, injects the
# autoinstall configuration (with LUKS passphrase), embeds forensic tools,
# and repacks a bootable ISO ready for unattended installation.
#
# Usage:
#   ./scripts/build-iso.sh
#
# Requirements: xorriso, p7zip-full (or 7zip), wget, sha256sum
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly CONFIG_DIR="${PROJECT_DIR}/config"
readonly FORENSICS_DIR="${PROJECT_DIR}/forensics"

readonly ISO_NAME="ubuntu-24.04.1-live-server-amd64.iso"
readonly ISO_URL="https://releases.ubuntu.com/24.04.1/${ISO_NAME}"
readonly ISO_SHA256="e240e4b801f7bb68c20d1356b60968ad0c33a41d00d828e74ceb3364a0317be9"

readonly OUTPUT_ISO="ubuntu-24.04-gpu3090-autoinstall.iso"

# ---------------------------------------------------------------------------
# Color helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'  # No Color

info()    { echo -e "${BLUE}[INFO]${NC}    $*"; }
success() { echo -e "${GREEN}[OK]${NC}      $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}    $*"; }
error()   { echo -e "${RED}[ERROR]${NC}   $*" >&2; }
step()    { echo -e "\n${CYAN}${BOLD}▶ $*${NC}"; }

# ---------------------------------------------------------------------------
# Cleanup handler
# ---------------------------------------------------------------------------
WORK_DIR=""

cleanup() {
    if [[ -n "${WORK_DIR}" && -d "${WORK_DIR}" ]]; then
        info "Cleaning up temporary directory: ${WORK_DIR}"
        rm -rf "${WORK_DIR}"
    fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Step 1: Check and install required tools
# ---------------------------------------------------------------------------
check_dependencies() {
    step "Checking required tools"

    local missing=()

    if ! command -v xorriso &>/dev/null; then
        missing+=("xorriso")
    fi

    # Accept either 7z (from p7zip-full or 7zip) or 7zz (from 7zip)
    if ! command -v 7z &>/dev/null && ! command -v 7zz &>/dev/null; then
        missing+=("p7zip-full")
    fi

    if ! command -v wget &>/dev/null; then
        missing+=("wget")
    fi

    if ! command -v sha256sum &>/dev/null; then
        missing+=("coreutils")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        warn "Missing tools: ${missing[*]}"
        info "Attempting to install via apt..."
        sudo apt-get update -qq
        sudo apt-get install -y -qq "${missing[@]}"
        success "Dependencies installed"
    else
        success "All required tools are present"
    fi
}

# ---------------------------------------------------------------------------
# Step 2: Download the Ubuntu ISO if not already present
# ---------------------------------------------------------------------------
download_iso() {
    step "Checking for Ubuntu ISO"

    if [[ -f "${PROJECT_DIR}/${ISO_NAME}" ]]; then
        info "ISO already exists: ${ISO_NAME}"
    else
        info "Downloading ${ISO_NAME} from ${ISO_URL} ..."
        wget --show-progress -O "${PROJECT_DIR}/${ISO_NAME}" "${ISO_URL}"
        success "Download complete"
    fi
}

# ---------------------------------------------------------------------------
# Step 3: Verify SHA256 checksum
# ---------------------------------------------------------------------------
verify_iso() {
    step "Verifying ISO checksum"

    local actual_sha256
    actual_sha256="$(sha256sum "${PROJECT_DIR}/${ISO_NAME}" | awk '{print $1}')"

    if [[ "${actual_sha256}" == "${ISO_SHA256}" ]]; then
        success "SHA256 checksum matches"
    else
        error "Checksum mismatch!"
        error "  Expected: ${ISO_SHA256}"
        error "  Got:      ${actual_sha256}"
        error "The ISO file may be corrupted. Delete it and re-run this script."
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Step 4: Prompt for LUKS passphrase
# ---------------------------------------------------------------------------
get_luks_passphrase() {
    step "LUKS Disk Encryption Passphrase"

    local pass1 pass2

    while true; do
        echo -en "${BOLD}Enter LUKS passphrase: ${NC}"
        read -rs pass1
        echo

        if [[ -z "${pass1}" ]]; then
            warn "Passphrase cannot be empty. Try again."
            continue
        fi

        if [[ ${#pass1} -lt 8 ]]; then
            warn "Passphrase must be at least 8 characters. Try again."
            continue
        fi

        echo -en "${BOLD}Confirm LUKS passphrase: ${NC}"
        read -rs pass2
        echo

        if [[ "${pass1}" == "${pass2}" ]]; then
            LUKS_PASSPHRASE="${pass1}"
            success "Passphrase confirmed"
            return
        else
            warn "Passphrases do not match. Try again."
        fi
    done
}

# ---------------------------------------------------------------------------
# Step 5: Extract ISO into working directory
# ---------------------------------------------------------------------------
extract_iso() {
    step "Extracting ISO to temporary working directory"

    WORK_DIR="$(mktemp -d /tmp/build-iso-XXXXXX)"
    info "Working directory: ${WORK_DIR}"

    local iso_extract="${WORK_DIR}/iso"
    mkdir -p "${iso_extract}"

    # Use xorriso to extract the ISO contents
    xorriso -osirrox on -indev "${PROJECT_DIR}/${ISO_NAME}" \
        -extract / "${iso_extract}" 2>/dev/null

    # Make all extracted files writable (ISO contents are read-only)
    chmod -R u+w "${iso_extract}"

    success "ISO extracted successfully"
}

# ---------------------------------------------------------------------------
# Step 6: Inject autoinstall config and meta-data
# ---------------------------------------------------------------------------
inject_autoinstall() {
    step "Injecting autoinstall configuration"

    local iso_extract="${WORK_DIR}/iso"
    local server_dir="${iso_extract}/server"

    # Create the nocloud datasource directory
    mkdir -p "${server_dir}"

    # Copy autoinstall.yaml as user-data
    cp "${CONFIG_DIR}/autoinstall.yaml" "${server_dir}/user-data"

    # Copy meta-data
    cp "${CONFIG_DIR}/meta-data" "${server_dir}/meta-data"

    success "Autoinstall files placed in /server/ (nocloud datasource)"

    # Replace LUKS passphrase placeholder
    info "Injecting LUKS passphrase into autoinstall config"

    # Use a Python one-liner for safe string replacement (avoids sed delimiter issues)
    python3 -c "
import sys
path = '${server_dir}/user-data'
with open(path, 'r') as f:
    content = f.read()
content = content.replace('LUKS_PASSPHRASE_PLACEHOLDER', sys.stdin.read().strip())
with open(path, 'w') as f:
    f.write(content)
" <<< "${LUKS_PASSPHRASE}"

    success "LUKS passphrase injected"

    # Inject the generated SSH key if it exists
    local gen_key="${PROJECT_DIR}/keys/3090-headless.pub"
    if [[ -f "${gen_key}" ]]; then
        local key_line
        key_line="$(cat "${gen_key}")"
        info "Adding generated SSH key to authorized-keys in autoinstall"
        # Append the key under the authorized-keys list in user-data
        sed -i "/authorized-keys:/a\\      - ${key_line}" "${server_dir}/user-data"
        success "Generated SSH key added to autoinstall config"
    else
        warn "No generated key at ${gen_key} — only the default key will be authorized"
        warn "Run 'make keys' first for a dedicated 3090 keypair"
    fi
}

# ---------------------------------------------------------------------------
# Step 7: Copy forensic scripts and patterns into the ISO
# ---------------------------------------------------------------------------
inject_forensics() {
    step "Embedding forensic tools into the ISO"

    local iso_extract="${WORK_DIR}/iso"
    local iso_forensics="${iso_extract}/forensics"

    if [[ -d "${FORENSICS_DIR}" ]]; then
        cp -a "${FORENSICS_DIR}" "${iso_forensics}"
        success "Forensics directory copied to /forensics/ in ISO"
    else
        warn "No forensics directory found at ${FORENSICS_DIR} — skipping"
    fi

    # Also copy scripts (e.g., post-install.sh) if they exist
    if [[ -d "${SCRIPT_DIR}" ]]; then
        mkdir -p "${iso_extract}/scripts"
        for script in "${SCRIPT_DIR}"/*.sh; do
            [[ -f "${script}" ]] || continue
            local basename
            basename="$(basename "${script}")"
            # Skip build-iso.sh and flash-usb.sh from the ISO
            if [[ "${basename}" == "build-iso.sh" || "${basename}" == "flash-usb.sh" ]]; then
                continue
            fi
            cp "${script}" "${iso_extract}/scripts/"
        done
        success "Helper scripts copied to /scripts/ in ISO"
    fi
}

# ---------------------------------------------------------------------------
# Step 8: Modify GRUB configuration for autoinstall
# ---------------------------------------------------------------------------
modify_grub() {
    step "Modifying GRUB configuration for autoinstall boot"

    local iso_extract="${WORK_DIR}/iso"
    local grub_cfg="${iso_extract}/boot/grub/grub.cfg"

    if [[ ! -f "${grub_cfg}" ]]; then
        error "grub.cfg not found at expected path: ${grub_cfg}"
        error "Searching for grub.cfg..."
        find "${iso_extract}" -name "grub.cfg" -type f 2>/dev/null || true
        exit 1
    fi

    # Backup original
    cp "${grub_cfg}" "${grub_cfg}.orig"

    # ── ZERO-TOUCH BOOT: no monitor, no keyboard, no interaction ──
    #
    # 1. timeout=0 — instant boot, no menu visible
    # 2. autoinstall ds=nocloud — tells subiquity to run unattended
    # 3. console=ttyS0 — mirror output to serial (for debug if serial connected)
    # 4. Remove "Try or Install" menu prompts — force first entry
    # 5. Kill the countdown timer completely

    info "Setting GRUB timeout to 0 (instant boot, no menu)..."
    sed -i 's/^set timeout=.*/set timeout=0/' "${grub_cfg}"

    # Also kill any hidden timeout or countdown
    sed -i '/^set timeout_style=/d' "${grub_cfg}"
    sed -i '/^set timeout=/a set timeout_style=hidden' "${grub_cfg}"

    # Add autoinstall parameters to ALL linux boot lines.
    # The ds=nocloud parameter tells cloud-init where to find user-data and meta-data.
    # console=ttyS0 enables serial output (debug via serial if available).
    sed -i '/^\s*linux\s/ {
        /autoinstall/! s|$| autoinstall ds=nocloud\\;s=/cdrom/server/ console=ttyS0,115200n8|
    }' "${grub_cfg}"

    # Force the first menuentry to be the only one that matters.
    # Remove any "submenu" or "Try Ubuntu" entries that would pause.
    # Replace any remaining timeouts.
    sed -i 's/^set timeout=30/set timeout=0/g' "${grub_cfg}"
    sed -i 's/^set timeout=5/set timeout=0/g' "${grub_cfg}"

    success "GRUB configured: timeout=0, hidden menu, autoinstall params, serial console"

    # Also patch the EFI GRUB config if present (UEFI boot path)
    local efi_grub="${iso_extract}/boot/grub/grub.cfg"
    local efi_grub2="${iso_extract}/EFI/BOOT/grub.cfg"
    if [[ -f "${efi_grub2}" && "${efi_grub2}" != "${grub_cfg}" ]]; then
        info "Patching EFI GRUB config at ${efi_grub2}..."
        sed -i 's/^set timeout=.*/set timeout=0/' "${efi_grub2}"
        sed -i '/^\s*linux\s/ {
            /autoinstall/! s|$| autoinstall ds=nocloud\\;s=/cdrom/server/ console=ttyS0,115200n8|
        }' "${efi_grub2}"
        success "EFI GRUB config patched"
    fi

    # Show the modified boot entries for verification
    info "Modified GRUB boot entries:"
    grep -n "linux\s\|set timeout" "${grub_cfg}" | head -10 | while IFS= read -r line; do
        echo "    ${line}"
    done
}

# ---------------------------------------------------------------------------
# Step 9: Repack the ISO with UEFI + BIOS boot support
# ---------------------------------------------------------------------------
repack_iso() {
    step "Repacking custom ISO"

    local iso_extract="${WORK_DIR}/iso"
    local original_iso="${PROJECT_DIR}/${ISO_NAME}"
    local output_iso="${PROJECT_DIR}/${OUTPUT_ISO}"

    info "Building ISO with xorriso (simple BIOS boot method)..."

    # Use simpler xorriso command that works reliably for BIOS boot
    # Most consumer boards use BIOS/Legacy boot anyway
    xorriso -as mkisofs \
        -r -J -joliet-long \
        -V "UBUNTU-AUTOINSTALL" \
        -iso-level 3 \
        -b boot/grub/i386-pc/eltorito.img \
        -c boot.catalog \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -o "${output_iso}" \
        "${iso_extract}" 2>&1 | tail -5

    if [[ -f "${output_iso}" ]]; then
        success "Custom ISO built successfully"
        echo ""
        info "Output: ${output_iso}"

        # Show file size
        local iso_size
        iso_size="$(du -h "${output_iso}" | cut -f1)"
        info "Size:   ${iso_size}"
    else
        err "ISO build failed"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo -e "${BOLD}${CYAN}"
    echo "============================================================================="
    echo "  Headless Ubuntu 24.04 — Custom Autoinstall ISO Builder"
    echo "============================================================================="
    echo -e "${NC}"
    echo "  Project:   ${PROJECT_DIR}"
    echo "  Output:    ${OUTPUT_ISO}"
    echo "  Target:    gpu-3090 (2x RTX 3090 + Intel iGPU, NVMe + HDD, LUKS2)"
    echo ""

    check_dependencies
    download_iso
    verify_iso
    get_luks_passphrase
    extract_iso
    inject_autoinstall
    inject_forensics
    modify_grub
    repack_iso

    echo ""
    echo -e "${GREEN}${BOLD}============================================================================="
    echo "  BUILD COMPLETE"
    echo "=============================================================================${NC}"
    echo ""
    echo "  Custom ISO: ${PROJECT_DIR}/${OUTPUT_ISO}"
    echo ""
    echo "  Next steps:"
    echo "    1. Flash to USB:  sudo ./scripts/flash-usb.sh ${OUTPUT_ISO}"
    echo "    2. Boot the target machine from the USB drive"
    echo "    3. Installation will proceed automatically"
    echo "    4. After reboot, SSH in:  ssh zero20nez@<ip-address>"
    echo ""
    echo -e "  ${YELLOW}WARNING: The autoinstall will ERASE ALL DATA on /dev/nvme0n1 and /dev/sda${NC}"
    echo ""
}

main "$@"
