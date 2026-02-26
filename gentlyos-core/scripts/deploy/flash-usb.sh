#!/bin/bash
# GentlyOS USB Flasher
# Writes the ISO to a USB drive

set -e

ISO_PATH="${1:-dist/gentlyos-1.1.1-amd64.iso}"
USB_DEVICE="${2:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo ""
echo "╔════════════════════════════════════════════╗"
echo "║       GentlyOS USB Flasher                 ║"
echo "╚════════════════════════════════════════════╝"
echo ""

# Check if ISO exists
if [ ! -f "$ISO_PATH" ]; then
    echo -e "${RED}Error: ISO not found at $ISO_PATH${NC}"
    echo ""
    echo "Build the ISO first:"
    echo "  ./scripts/deploy/build-iso.sh"
    exit 1
fi

# List available drives
echo "Available USB drives:"
echo ""
lsblk -d -o NAME,SIZE,MODEL,TRAN | grep -E "usb|removable" || lsblk -d -o NAME,SIZE,MODEL | grep -v "loop\|sr"
echo ""

# Prompt for device if not provided
if [ -z "$USB_DEVICE" ]; then
    read -p "Enter USB device (e.g., /dev/sdb): " USB_DEVICE
fi

# Safety checks
if [ ! -b "$USB_DEVICE" ]; then
    echo -e "${RED}Error: $USB_DEVICE is not a valid block device${NC}"
    exit 1
fi

if [[ "$USB_DEVICE" == *"sda"* ]] || [[ "$USB_DEVICE" == *"nvme0n1"* ]]; then
    echo -e "${RED}WARNING: $USB_DEVICE looks like your system drive!${NC}"
    read -p "Are you ABSOLUTELY sure? Type 'YES I AM SURE' to continue: " confirm
    if [ "$confirm" != "YES I AM SURE" ]; then
        echo "Aborted."
        exit 1
    fi
fi

echo ""
echo -e "${YELLOW}WARNING: This will ERASE ALL DATA on $USB_DEVICE${NC}"
echo ""
read -p "Continue? [y/N] " confirm
if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Unmount if mounted
echo "Unmounting $USB_DEVICE..."
umount ${USB_DEVICE}* 2>/dev/null || true

# Flash the ISO
echo ""
echo "Flashing GentlyOS to $USB_DEVICE..."
echo "This may take several minutes..."
echo ""

dd if="$ISO_PATH" of="$USB_DEVICE" bs=4M status=progress conv=fsync

# Sync
sync

echo ""
echo -e "${GREEN}════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SUCCESS! GentlyOS has been flashed to USB ${NC}"
echo -e "${GREEN}════════════════════════════════════════════${NC}"
echo ""
echo "Next steps:"
echo "  1. Remove the USB drive"
echo "  2. Insert it into target machine"
echo "  3. Boot from USB (F12/F2/DEL at BIOS)"
echo "  4. ONE SCENE will auto-launch"
echo ""
echo "Default login: admin / gently2026"
echo ""
