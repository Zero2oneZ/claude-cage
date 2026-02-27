#!/bin/bash
# GentlyOS Dev USB Flasher
# Creates a two-partition USB: boot (ISO) + data (persistence)
#
# Partition layout:
#   sdb1: FAT32 "GENTLYOS"      — GRUB + squashfs (ISO contents)
#   sdb2: ext4  "gentlyos-data"  — Overlayfs persistence (rest of USB)
#
# Usage: sudo ./flash-usb-dev.sh <iso-path> <usb-device>
#   Example: sudo ./flash-usb-dev.sh dist/gentlyos-dev-1.1.1-dev-x86_64.iso /dev/sdb
#
# The persistence partition stores /home/gently as an overlayfs upper layer.
# Source code edits, cargo cache, configs — everything survives reboot.

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

ISO_PATH="${1:-}"
USB_DEVICE="${2:-}"

echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     GentlyOS Dev USB Flasher                     ║${NC}"
echo -e "${CYAN}║     Two-partition layout with persistence         ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
echo ""

# ── Validate inputs ──
if [ "$(id -u)" -ne 0 ]; then
    echo -e "${RED}Error: must run as root${NC}"
    echo "  sudo $0 $*"
    exit 1
fi

if [ -z "$ISO_PATH" ]; then
    echo -e "${RED}Error: ISO path required${NC}"
    echo ""
    echo "Usage: sudo $0 <iso-path> <usb-device>"
    echo "  Example: sudo $0 dist/gentlyos-dev-1.1.1-dev-x86_64.iso /dev/sdb"
    exit 1
fi

if [ ! -f "$ISO_PATH" ]; then
    echo -e "${RED}Error: ISO not found at ${ISO_PATH}${NC}"
    echo ""
    echo "Build the dev ISO first:"
    echo "  sudo ./scripts/deploy/build-alpine-dev-iso.sh"
    exit 1
fi

# ── List USB drives if device not specified ──
if [ -z "$USB_DEVICE" ]; then
    echo "Available block devices:"
    echo ""
    lsblk -d -o NAME,SIZE,MODEL,TRAN,RM | grep -v "^loop" | head -20
    echo ""
    read -p "Enter USB device (e.g., /dev/sdb): " USB_DEVICE
fi

if [ ! -b "$USB_DEVICE" ]; then
    echo -e "${RED}Error: ${USB_DEVICE} is not a valid block device${NC}"
    exit 1
fi

# ── Safety checks ──
DEVICE_NAME=$(basename "$USB_DEVICE")

# Block system drives
if echo "$USB_DEVICE" | grep -qE "(nvme0n1|sda)$"; then
    echo -e "${RED}DANGER: ${USB_DEVICE} appears to be a system drive!${NC}"
    echo ""
    read -p "Are you ABSOLUTELY sure? Type 'YES DESTROY IT' to continue: " confirm
    if [ "$confirm" != "YES DESTROY IT" ]; then
        echo "Aborted."
        exit 1
    fi
fi

# Show what we're about to destroy
echo "Target device: ${USB_DEVICE}"
echo ""
lsblk "$USB_DEVICE" 2>/dev/null || true
echo ""

ISO_SIZE_MB=$(( $(stat -c%s "$ISO_PATH") / 1024 / 1024 ))
DISK_SIZE_MB=$(( $(blockdev --getsize64 "$USB_DEVICE") / 1024 / 1024 ))
PERSIST_SIZE_MB=$(( DISK_SIZE_MB - ISO_SIZE_MB - 100 )) # 100MB buffer

echo "ISO size:         ${ISO_SIZE_MB} MB"
echo "USB size:         ${DISK_SIZE_MB} MB"
echo "Persistence:      ~${PERSIST_SIZE_MB} MB"
echo ""

if [ "$PERSIST_SIZE_MB" -lt 500 ]; then
    echo -e "${YELLOW}Warning: Only ${PERSIST_SIZE_MB}MB for persistence.${NC}"
    echo "  Recommend at least 8GB USB for dev workflow."
fi

echo -e "${YELLOW}WARNING: This will ERASE ALL DATA on ${USB_DEVICE}${NC}"
echo ""
read -p "Continue? [y/N] " confirm
if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# ── Unmount existing partitions ──
echo ""
echo "Unmounting ${USB_DEVICE}..."
for part in ${USB_DEVICE}*; do
    umount "$part" 2>/dev/null || true
done
sleep 1

# ── Step 1: Write ISO to USB ──
echo ""
echo -e "${CYAN}[1/3] Writing ISO to USB...${NC}"
echo "  This may take a few minutes."
echo ""

dd if="$ISO_PATH" of="$USB_DEVICE" bs=4M status=progress conv=fsync
sync
sleep 2

echo ""
echo "  ISO written."

# ── Step 2: Create persistence partition ──
echo ""
echo -e "${CYAN}[2/3] Creating persistence partition...${NC}"

# Find the end of the ISO data (next available sector)
# The ISO occupies the first part of the disk. We need to create
# a new partition after it.

# Re-read partition table
partprobe "$USB_DEVICE" 2>/dev/null || true
sleep 1

# Get the end of the last partition (in sectors)
SECTOR_SIZE=$(blockdev --getss "$USB_DEVICE")
LAST_END=$(sfdisk -l "$USB_DEVICE" 2>/dev/null | grep "^${USB_DEVICE}" | awk '{print $3}' | sort -n | tail -1)

if [ -z "$LAST_END" ]; then
    # If sfdisk can't parse, use ISO size to calculate
    LAST_END=$(( (ISO_SIZE_MB * 1024 * 1024 / SECTOR_SIZE) + 2048 ))
fi

# Start new partition after the ISO with 1MB alignment
PERSIST_START=$(( ((LAST_END / 2048) + 1) * 2048 ))

echo "  Creating ext4 partition at sector ${PERSIST_START}..."

# Add partition using sfdisk
echo "${PERSIST_START},," | sfdisk --append "$USB_DEVICE" 2>/dev/null || {
    # Fallback: use fdisk
    echo "  sfdisk failed, trying fdisk..."
    (
        echo n      # New partition
        echo p      # Primary
        echo ""     # Default partition number
        echo ""     # Default first sector (after ISO)
        echo ""     # Default last sector (end of disk)
        echo w      # Write
    ) | fdisk "$USB_DEVICE" 2>/dev/null || true
}

partprobe "$USB_DEVICE" 2>/dev/null || true
sleep 2

# Find the new partition
PERSIST_PART=""
for part in ${USB_DEVICE}2 ${USB_DEVICE}p2; do
    if [ -b "$part" ]; then
        PERSIST_PART="$part"
        break
    fi
done

if [ -z "$PERSIST_PART" ]; then
    echo -e "${YELLOW}Warning: Could not find persistence partition.${NC}"
    echo "  The ISO was written successfully. You can create the"
    echo "  persistence partition manually:"
    echo ""
    echo "  1. sudo fdisk ${USB_DEVICE}"
    echo "  2. Create partition 2 (use remaining space)"
    echo "  3. sudo mkfs.ext4 -L gentlyos-data ${USB_DEVICE}2"
    echo ""
    echo "  Or boot without persistence (RAM-only mode)."
else
    echo "  Formatting ${PERSIST_PART} as ext4..."
    mkfs.ext4 -L "gentlyos-data" -q "$PERSIST_PART"
    echo "  Persistence partition created: ${PERSIST_PART}"
fi

# ── Step 3: Verify ──
echo ""
echo -e "${CYAN}[3/3] Verifying...${NC}"
echo ""
lsblk "$USB_DEVICE" -o NAME,SIZE,FSTYPE,LABEL,MOUNTPOINT
echo ""

# Check label
if [ -n "$PERSIST_PART" ] && blkid -L "gentlyos-data" >/dev/null 2>&1; then
    echo -e "  Persistence label: ${GREEN}gentlyos-data (OK)${NC}"
else
    echo -e "  Persistence label: ${YELLOW}not found${NC}"
fi

sync

echo ""
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  GentlyOS Dev USB ready!${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════${NC}"
echo ""
echo "  Boot from this USB to start developing."
echo ""
echo "  First boot:"
echo "    1. Select 'GentlyOS Dev' at GRUB menu"
echo "    2. Auto-login as 'gently' (password: gently)"
echo "    3. Browser: http://localhost:3000"
echo "    4. Run 'gently-dev' to start hot-reload mode"
echo ""
echo "  Your edits in ~/gentlyos-core/ persist across reboots."
echo ""
