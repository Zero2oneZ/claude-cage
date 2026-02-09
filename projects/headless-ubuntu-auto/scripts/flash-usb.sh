#!/usr/bin/env bash
# =============================================================================
# flash-usb.sh — Write the custom autoinstall ISO to a USB drive
# =============================================================================
# Safely writes an ISO image to a removable USB device with multiple layers
# of confirmation to prevent accidental data loss.
#
# Usage:
#   sudo ./scripts/flash-usb.sh [path-to-iso]
#
# If no ISO path is given, the script looks for the default output ISO in the
# project directory.
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly DEFAULT_ISO="${PROJECT_DIR}/ubuntu-24.04.1-autoinstall-gpu3090.iso"

# ---------------------------------------------------------------------------
# Color helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}    $*"; }
success() { echo -e "${GREEN}[OK]${NC}      $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}    $*"; }
error()   { echo -e "${RED}[ERROR]${NC}   $*" >&2; }

# ---------------------------------------------------------------------------
# Step 1: Must run as root
# ---------------------------------------------------------------------------
check_root() {
    if [[ "${EUID}" -ne 0 ]]; then
        error "This script must be run as root."
        echo ""
        echo "  Usage:  sudo $0 [path-to-iso]"
        echo ""
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Step 2: Determine the ISO file to flash
# ---------------------------------------------------------------------------
resolve_iso() {
    if [[ -n "${1:-}" ]]; then
        ISO_PATH="$1"
    else
        ISO_PATH="${DEFAULT_ISO}"
    fi

    if [[ ! -f "${ISO_PATH}" ]]; then
        error "ISO file not found: ${ISO_PATH}"
        echo ""
        echo "  Build it first:  ./scripts/build-iso.sh"
        echo "  Or specify path: sudo $0 /path/to/custom.iso"
        echo ""
        exit 1
    fi

    readonly ISO_PATH
    local iso_size
    iso_size="$(du -h "${ISO_PATH}" | cut -f1)"
    info "ISO file: ${ISO_PATH} (${iso_size})"
}

# ---------------------------------------------------------------------------
# Step 3: List removable USB devices
# ---------------------------------------------------------------------------
list_usb_devices() {
    echo ""
    echo -e "${CYAN}${BOLD}Available USB devices:${NC}"
    echo "--------------------------------------------------------------------------"
    printf "  %-10s %-10s %-40s\n" "DEVICE" "SIZE" "MODEL"
    echo "--------------------------------------------------------------------------"

    local found=0

    # List block devices that are USB-attached disks
    while IFS= read -r line; do
        local dev size model tran rm_flag type
        dev="$(echo "${line}" | awk '{print $1}')"
        size="$(echo "${line}" | awk '{print $2}')"
        tran="$(echo "${line}" | awk '{print $3}')"
        rm_flag="$(echo "${line}" | awk '{print $4}')"
        type="$(echo "${line}" | awk '{print $5}')"
        model="$(echo "${line}" | awk '{for(i=6;i<=NF;i++) printf "%s ", $i; print ""}')"
        model="$(echo "${model}" | xargs)"  # Trim whitespace

        # Accept devices that are USB transport OR removable
        if [[ "${type}" == "disk" ]] && { [[ "${tran}" == "usb" ]] || [[ "${rm_flag}" == "1" ]]; }; then
            printf "  %-10s %-10s %-40s\n" "${dev}" "${size}" "${model:-Unknown}"
            USB_DEVICES+=("${dev}")
            found=1
        fi
    done < <(lsblk -dno NAME,SIZE,TRAN,RM,TYPE,MODEL 2>/dev/null)

    echo "--------------------------------------------------------------------------"

    if [[ ${found} -eq 0 ]]; then
        echo ""
        error "No removable USB devices found."
        echo "  Make sure your USB drive is plugged in and detected by the system."
        exit 1
    fi

    echo ""
}

# ---------------------------------------------------------------------------
# Step 4: User selects a device with multiple confirmations
# ---------------------------------------------------------------------------
select_device() {
    local selected=""

    echo -en "${BOLD}Enter the device name to write to (e.g., sdc): ${NC}"
    read -r selected

    # Strip /dev/ prefix if user included it
    selected="${selected#/dev/}"

    # Validate: must be in our list of USB devices
    local valid=0
    for d in "${USB_DEVICES[@]}"; do
        if [[ "${d}" == "${selected}" ]]; then
            valid=1
            break
        fi
    done

    if [[ ${valid} -eq 0 ]]; then
        error "'${selected}' is not a valid USB device from the list above."
        exit 1
    fi

    TARGET_DEVICE="/dev/${selected}"

    # Show what will happen
    echo ""
    echo -e "${RED}${BOLD}=========================================================================="
    echo "  WARNING: ALL DATA ON ${TARGET_DEVICE} WILL BE PERMANENTLY DESTROYED"
    echo "==========================================================================${NC}"
    echo ""
    echo "  Device:  ${TARGET_DEVICE}"
    echo "  Size:    $(lsblk -dno SIZE "${TARGET_DEVICE}" 2>/dev/null || echo "unknown")"
    echo "  Model:   $(lsblk -dno MODEL "${TARGET_DEVICE}" 2>/dev/null | xargs || echo "unknown")"
    echo ""
    echo "  ISO:     ${ISO_PATH}"
    echo ""

    # Require exact device name confirmation
    echo -e "${YELLOW}${BOLD}To confirm, type the device name '${selected}' exactly:${NC}"
    echo -en "  > "
    read -r confirmation

    if [[ "${confirmation}" != "${selected}" ]]; then
        error "Confirmation did not match. Aborting."
        exit 1
    fi

    success "Confirmed: writing to ${TARGET_DEVICE}"
    readonly TARGET_DEVICE
}

# ---------------------------------------------------------------------------
# Step 5: Unmount any mounted partitions on the device
# ---------------------------------------------------------------------------
unmount_partitions() {
    info "Unmounting any mounted partitions on ${TARGET_DEVICE}..."

    local part_count=0
    while IFS= read -r part; do
        local mountpoint
        mountpoint="$(lsblk -no MOUNTPOINT "/dev/${part}" 2>/dev/null | head -1)"
        if [[ -n "${mountpoint}" ]]; then
            info "  Unmounting /dev/${part} (${mountpoint})"
            umount "/dev/${part}" 2>/dev/null || umount -l "/dev/${part}" 2>/dev/null || true
            part_count=$((part_count + 1))
        fi
    done < <(lsblk -nro NAME "${TARGET_DEVICE}" 2>/dev/null | tail -n +2)

    if [[ ${part_count} -gt 0 ]]; then
        success "Unmounted ${part_count} partition(s)"
    else
        info "No mounted partitions found"
    fi

    # Brief pause to let the kernel settle
    sleep 1
}

# ---------------------------------------------------------------------------
# Step 6: Write the ISO to the USB device
# ---------------------------------------------------------------------------
write_iso() {
    echo ""
    info "Writing ISO to ${TARGET_DEVICE}..."
    info "This may take several minutes depending on USB speed."
    echo ""

    dd if="${ISO_PATH}" of="${TARGET_DEVICE}" bs=4M conv=fsync status=progress

    success "ISO written to ${TARGET_DEVICE}"
}

# ---------------------------------------------------------------------------
# Step 7: Sync and finalize
# ---------------------------------------------------------------------------
finalize() {
    info "Syncing filesystem buffers..."
    sync
    success "Sync complete"

    # Inform the kernel about partition table changes
    partprobe "${TARGET_DEVICE}" 2>/dev/null || true

    echo ""
    echo -e "${GREEN}${BOLD}=========================================================================="
    echo "  USB FLASH COMPLETE"
    echo "==========================================================================${NC}"
    echo ""
    echo "  Device: ${TARGET_DEVICE}"
    echo "  ISO:    $(basename "${ISO_PATH}")"
    echo ""
    echo "  Next steps (NO MONITOR NEEDED — fully automated):"
    echo ""
    echo "    1. Eject USB:   sudo eject ${TARGET_DEVICE}"
    echo "    2. Plug USB + ethernet into the 3090 machine"
    echo "    3. Power on the machine"
    echo ""
    echo -e "  ${BOLD}BOOT ORDER:${NC} The machine must boot from USB."
    echo "    Option A: BIOS is already set to USB-first (check after BIOS flash)"
    echo "    Option B: Many motherboards auto-detect bootable USB"
    echo "    Option C: If it boots to Windows instead, you need to change BIOS"
    echo "              boot order ONCE (requires a monitor temporarily):"
    echo "              - Power on, mash F2/F12/DEL to enter BIOS"
    echo "              - Set USB as first boot device"
    echo "              - Save and reboot"
    echo "              - After this, remove the monitor — never needed again"
    echo ""
    echo "    4. Once booted from USB, EVERYTHING is automatic:"
    echo "       - Forensic scan of Windows C: drive"
    echo "       - Wipe all drives"
    echo "       - Install Ubuntu 24.04 + LUKS2 encryption"
    echo "       - Reboot into fresh Ubuntu"
    echo "       - SSH server starts automatically"
    echo "       - Beacon broadcasts on UDP:9999"
    echo ""
    echo "    5. On YOUR machine, run:  make find"
    echo "       (detects the 3090 via beacon or network scan)"
    echo ""
    echo "    6. Then:  make provision  →  make dropbear  →  make verify"
    echo ""
    echo -e "  ${YELLOW}WARNING: NVMe + HDD on the target will be WIPED.${NC}"
    echo -e "  ${YELLOW}Forensic scan results saved to /opt/headless-setup/forensic-report/${NC}"
    echo ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    echo -e "${BOLD}${CYAN}"
    echo "============================================================================="
    echo "  Headless Ubuntu 24.04 — USB Flash Tool"
    echo "============================================================================="
    echo -e "${NC}"

    USB_DEVICES=()

    check_root
    resolve_iso "${1:-}"
    list_usb_devices
    select_device
    unmount_partitions
    write_iso
    finalize
}

main "$@"
