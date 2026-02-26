#!/bin/sh
# GentlyOS Alpine-based ISO Builder
# Creates bootable live USB image using Alpine Linux (musl-native)

set -e

VERSION="${GENTLY_VERSION:-1.1.1}"
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/build/alpine-iso"
DIST_DIR="${PROJECT_ROOT}/dist"
ISO_NAME="gentlyos-alpine-${VERSION}-x86_64.iso"

# Alpine version to use
ALPINE_VERSION="3.21"
ALPINE_MIRROR="http://mirror.leaseweb.com/alpine"

echo "Building GentlyOS Alpine-based ISO v${VERSION}..."
echo "Project root: ${PROJECT_ROOT}"

# Cleanup and setup
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/rootfs"
mkdir -p "${BUILD_DIR}/iso/boot/grub"
mkdir -p "${BUILD_DIR}/iso/live"
mkdir -p "${DIST_DIR}"

# Check dependencies
for cmd in mksquashfs xorriso grub-mkrescue mformat; do
    if ! command -v $cmd >/dev/null 2>&1; then
        echo "Error: $cmd not found. Install with: apk add squashfs-tools xorriso grub mtools"
        exit 1
    fi
done

# Step 1: Download and extract Alpine minirootfs
echo "[1/7] Downloading Alpine minirootfs..."
ROOTFS_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/x86_64/alpine-minirootfs-${ALPINE_VERSION}.0-x86_64.tar.gz"
wget -q -O "${BUILD_DIR}/alpine-minirootfs.tar.gz" "${ROOTFS_URL}" || {
    echo "Trying alternative URL..."
    # Try with latest patch version
    ROOTFS_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/x86_64/alpine-minirootfs-3.21.5-x86_64.tar.gz"
    wget -q -O "${BUILD_DIR}/alpine-minirootfs.tar.gz" "${ROOTFS_URL}"
}
tar -xzf "${BUILD_DIR}/alpine-minirootfs.tar.gz" -C "${BUILD_DIR}/rootfs"

# Step 2: Configure the rootfs
echo "[2/7] Configuring system..."

# Copy DNS config for network access in chroot
cp /etc/resolv.conf "${BUILD_DIR}/rootfs/etc/resolv.conf"

# Set hostname
echo "gentlyos" > "${BUILD_DIR}/rootfs/etc/hostname"

# Configure repositories
cat > "${BUILD_DIR}/rootfs/etc/apk/repositories" << EOF
${ALPINE_MIRROR}/v${ALPINE_VERSION}/main
${ALPINE_MIRROR}/v${ALPINE_VERSION}/community
EOF

# Create setup script to run inside chroot
cat > "${BUILD_DIR}/rootfs/setup.sh" << 'SETUP'
#!/bin/sh
set -e

# Update and install packages
apk update
apk add --no-cache \
    openrc \
    alpine-base \
    linux-lts \
    linux-firmware-none \
    mkinitfs \
    e2fsprogs \
    dosfstools \
    openssh-client \
    curl \
    jq \
    git \
    vim \
    sudo \
    shadow \
    bash \
    htop

# Enable essential services
rc-update add devfs sysinit
rc-update add dmesg sysinit
rc-update add mdev sysinit
rc-update add hwdrivers sysinit
rc-update add modules boot
rc-update add sysctl boot
rc-update add hostname boot
rc-update add bootmisc boot
rc-update add syslog boot
rc-update add networking default
rc-update add local default
rc-update add killprocs shutdown
rc-update add mount-ro shutdown
rc-update add savecache shutdown

# Create gently user
adduser -D -s /bin/bash gently
echo "gently:gently" | chpasswd
echo "gently ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Configure auto-login on tty1
mkdir -p /etc/conf.d
cat > /etc/inittab << 'INITTAB'
::sysinit:/sbin/openrc sysinit
::sysinit:/sbin/openrc boot
::wait:/sbin/openrc default
tty1::respawn:/bin/login -f gently
tty2::respawn:/sbin/getty 38400 tty2
tty3::respawn:/sbin/getty 38400 tty3
::ctrlaltdel:/sbin/reboot
::shutdown:/sbin/openrc shutdown
INITTAB

# Configure bash profile
cat >> /home/gently/.profile << 'PROFILE'
export PATH=/opt/gently/bin:$PATH
export GENTLY_DATA_DIR=$HOME/.gently

# Welcome message on first login
if [ -z "$GENTLY_WELCOMED" ]; then
    export GENTLY_WELCOMED=1
    clear
    cat << 'BANNER'

 ██████╗ ███████╗███╗   ██╗████████╗██╗  ██╗   ██╗
██╔════╝ ██╔════╝████╗  ██║╚══██╔══╝██║  ╚██╗ ██╔╝
██║  ███╗█████╗  ██╔██╗ ██║   ██║   ██║   ╚████╔╝
██║   ██║██╔══╝  ██║╚██╗██║   ██║   ██║    ╚██╔╝
╚██████╔╝███████╗██║ ╚████║   ██║   ███████╗██║
 ╚═════╝ ╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚══════╝╚═╝

  Content-Addressable Security OS    v1.1.1

  Commands:
    gently --help     CLI documentation
    gently setup      Initial configuration
    gently-web        Start ONE SCENE Web GUI

BANNER
fi
PROFILE
chown gently:gently /home/gently/.profile

# Setup .gently directory
mkdir -p /home/gently/.gently
chown -R gently:gently /home/gently/.gently

# Create gently-web service
cat > /etc/init.d/gently-web << 'SERVICE'
#!/sbin/openrc-run

name="GentlyOS Web GUI"
description="ONE SCENE Web Interface"

command="/opt/gently/bin/gently-web"
command_args="-h 0.0.0.0 -p 3000"
command_user="gently"
command_background="yes"
pidfile="/run/${RC_SVCNAME}.pid"

depend() {
    need net
    after firewall
}
SERVICE
chmod +x /etc/init.d/gently-web
rc-update add gently-web default

# Cleanup
rm -rf /var/cache/apk/*
SETUP

chmod +x "${BUILD_DIR}/rootfs/setup.sh"

# Run setup in chroot (this works because we're Alpine->Alpine)
echo "[3/7] Running chroot setup..."
chroot "${BUILD_DIR}/rootfs" /setup.sh
rm "${BUILD_DIR}/rootfs/setup.sh"

# Step 3: Install GentlyOS binaries
echo "[4/7] Installing GentlyOS binaries..."
mkdir -p "${BUILD_DIR}/rootfs/opt/gently/bin"

if [ -f "${PROJECT_ROOT}/target/release/gently" ]; then
    cp "${PROJECT_ROOT}/target/release/gently" "${BUILD_DIR}/rootfs/opt/gently/bin/"
    chmod +x "${BUILD_DIR}/rootfs/opt/gently/bin/gently"
    echo "  Installed: gently ($(du -h "${PROJECT_ROOT}/target/release/gently" | cut -f1))"
else
    echo "Warning: gently binary not found at ${PROJECT_ROOT}/target/release/gently"
fi

if [ -f "${PROJECT_ROOT}/target/release/gently-web" ]; then
    cp "${PROJECT_ROOT}/target/release/gently-web" "${BUILD_DIR}/rootfs/opt/gently/bin/"
    chmod +x "${BUILD_DIR}/rootfs/opt/gently/bin/gently-web"
    echo "  Installed: gently-web ($(du -h "${PROJECT_ROOT}/target/release/gently-web" | cut -f1))"
else
    echo "Warning: gently-web binary not found"
fi

# Copy genesis data if exists
if [ -d "${HOME}/.gently" ]; then
    echo "  Copying genesis data..."
    mkdir -p "${BUILD_DIR}/rootfs/etc/gently"
    cp -r "${HOME}/.gently"/* "${BUILD_DIR}/rootfs/etc/gently/" 2>/dev/null || true
fi

# Step 4: Prepare kernel and initramfs
echo "[5/7] Preparing kernel..."
KERNEL_VERSION=$(ls "${BUILD_DIR}/rootfs/lib/modules/" | head -1)
if [ -z "$KERNEL_VERSION" ]; then
    echo "Error: No kernel found in rootfs"
    exit 1
fi

cp "${BUILD_DIR}/rootfs/boot/vmlinuz-lts" "${BUILD_DIR}/iso/boot/vmlinuz"
cp "${BUILD_DIR}/rootfs/boot/initramfs-lts" "${BUILD_DIR}/iso/boot/initramfs"

# Step 5: Create squashfs
echo "[6/7] Creating squashfs filesystem..."
mksquashfs "${BUILD_DIR}/rootfs" "${BUILD_DIR}/iso/live/filesystem.squashfs" \
    -comp xz -b 256K -Xbcj x86 -quiet

# Step 6: Configure bootloader
echo "[7/7] Creating ISO..."

# GRUB config for live boot
cat > "${BUILD_DIR}/iso/boot/grub/grub.cfg" << 'GRUBCFG'
set timeout=5
set default=0

insmod all_video
insmod gfxterm
terminal_output gfxterm

set menu_color_normal=white/black
set menu_color_highlight=black/white

menuentry "GentlyOS Live" {
    linux /boot/vmlinuz modules=loop,squashfs,sd-mod,usb-storage quiet
    initrd /boot/initramfs
}

menuentry "GentlyOS Live (Verbose)" {
    linux /boot/vmlinuz modules=loop,squashfs,sd-mod,usb-storage
    initrd /boot/initramfs
}

menuentry "GentlyOS Live (To RAM)" {
    linux /boot/vmlinuz modules=loop,squashfs,sd-mod,usb-storage toram quiet
    initrd /boot/initramfs
}
GRUBCFG

# Create ISO
grub-mkrescue -o "${DIST_DIR}/${ISO_NAME}" "${BUILD_DIR}/iso" \
    --product-name="GentlyOS" \
    --product-version="${VERSION}" 2>/dev/null

# Calculate checksums
cd "${DIST_DIR}"
sha256sum "${ISO_NAME}" > "${ISO_NAME}.sha256"
md5sum "${ISO_NAME}" > "${ISO_NAME}.md5"

# Cleanup
rm -rf "${BUILD_DIR}"

echo ""
echo "=========================================="
echo "ISO built successfully!"
echo "=========================================="
echo "  File: ${DIST_DIR}/${ISO_NAME}"
echo "  Size: $(du -h "${DIST_DIR}/${ISO_NAME}" | cut -f1)"
echo "  SHA256: $(cat "${ISO_NAME}.sha256" | cut -d' ' -f1)"
echo ""
echo "To write to USB:"
echo "  dd if=${DIST_DIR}/${ISO_NAME} of=/dev/sdX bs=4M status=progress"
echo ""
