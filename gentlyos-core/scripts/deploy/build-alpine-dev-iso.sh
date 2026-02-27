#!/bin/sh
# GentlyOS Alpine Dev ISO Builder
# Full development image: Rust toolchain, source tree, cargo-watch hot-reload
#
# Creates a bootable live USB with:
#   - Alpine Linux 3.21 (musl-native)
#   - Rust toolchain (rustup + stable + cargo-watch)
#   - Full gentlyos-core source tree (~80K LOC)
#   - Pre-built workspace (warm cargo cache for fast incremental rebuilds)
#   - Overlayfs persistence on USB (edits survive reboot)
#   - gently-web auto-starts on boot
#   - `gently-dev` switches to cargo-watch hot-reload mode
#
# Workflow:
#   1. Boot from USB → gently-web runs automatically
#   2. Open browser → http://localhost:3000
#   3. Open tty2 → `gently-dev` → switches to hot-reload
#   4. Edit source → cargo-watch rebuilds → browser refresh = changes
#
# Build requires: squashfs-tools xorriso grub mtools rsync wget
# Build time: ~30-60 min (Rust compile inside chroot)
# ISO size: ~1.5-2.5 GB (compressed)
#
# Usage: sudo ./scripts/deploy/build-alpine-dev-iso.sh
#        sudo GENTLY_VERSION=2.0.0 ./scripts/deploy/build-alpine-dev-iso.sh

set -e

VERSION="${GENTLY_VERSION:-1.1.1-dev}"
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO_ROOT="$(cd "${PROJECT_ROOT}/.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/build/alpine-dev-iso"
DIST_DIR="${PROJECT_ROOT}/dist"
ISO_NAME="gentlyos-dev-${VERSION}-x86_64.iso"

ALPINE_VERSION="3.21"
ALPINE_MIRROR="http://mirror.leaseweb.com/alpine"

# ── Cleanup handler (unmount bind mounts on exit) ──────────────
cleanup_mounts() {
    echo "Cleaning up bind mounts..."
    umount "${BUILD_DIR}/rootfs/proc" 2>/dev/null || true
    umount "${BUILD_DIR}/rootfs/sys" 2>/dev/null || true
    umount "${BUILD_DIR}/rootfs/dev/pts" 2>/dev/null || true
    umount "${BUILD_DIR}/rootfs/dev" 2>/dev/null || true
}
trap cleanup_mounts EXIT

echo "╔══════════════════════════════════════════════════════════╗"
echo "║       GentlyOS Dev ISO Builder v${VERSION}              ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "  Project root: ${PROJECT_ROOT}"
echo "  Build dir:    ${BUILD_DIR}"
echo "  Output:       ${DIST_DIR}/${ISO_NAME}"
echo ""

# ── Preflight checks ──────────────────────────────────────────
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: must run as root (chroot + mount requires privileges)"
    echo "  sudo $0"
    exit 1
fi

rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/rootfs"
mkdir -p "${BUILD_DIR}/iso/boot/grub"
mkdir -p "${BUILD_DIR}/iso/live"
mkdir -p "${DIST_DIR}"

for cmd in mksquashfs xorriso grub-mkrescue rsync wget; do
    if ! command -v $cmd >/dev/null 2>&1; then
        echo "Error: $cmd not found."
        echo "Install: sudo apt install squashfs-tools xorriso grub-common grub-pc-bin grub-efi-amd64-bin mtools rsync wget"
        exit 1
    fi
done

# ════════════════════════════════════════════════════════════════
# Phase 1: Download Alpine minirootfs
# ════════════════════════════════════════════════════════════════
echo "[1/9] Downloading Alpine ${ALPINE_VERSION} minirootfs..."

ROOTFS_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/x86_64/alpine-minirootfs-${ALPINE_VERSION}.0-x86_64.tar.gz"
wget -q --show-progress -O "${BUILD_DIR}/alpine-minirootfs.tar.gz" "${ROOTFS_URL}" || {
    echo "  Trying patch version..."
    ROOTFS_URL="${ALPINE_MIRROR}/v${ALPINE_VERSION}/releases/x86_64/alpine-minirootfs-3.21.5-x86_64.tar.gz"
    wget -q --show-progress -O "${BUILD_DIR}/alpine-minirootfs.tar.gz" "${ROOTFS_URL}"
}
tar -xzf "${BUILD_DIR}/alpine-minirootfs.tar.gz" -C "${BUILD_DIR}/rootfs"
echo "  Done."

# ════════════════════════════════════════════════════════════════
# Phase 2: System + dev packages
# ════════════════════════════════════════════════════════════════
echo "[2/9] Installing system and dev packages..."

cp /etc/resolv.conf "${BUILD_DIR}/rootfs/etc/resolv.conf"
echo "gentlyos-dev" > "${BUILD_DIR}/rootfs/etc/hostname"

cat > "${BUILD_DIR}/rootfs/etc/apk/repositories" << EOF
${ALPINE_MIRROR}/v${ALPINE_VERSION}/main
${ALPINE_MIRROR}/v${ALPINE_VERSION}/community
EOF

cat > "${BUILD_DIR}/rootfs/setup-base.sh" << 'SETUP_BASE'
#!/bin/sh
set -e

apk update

# ── Base system (production equivalent) ──
apk add --no-cache \
    openrc alpine-base linux-lts linux-firmware-none mkinitfs \
    e2fsprogs dosfstools util-linux \
    openssh-client curl jq git vim nano sudo shadow bash htop tmux \
    less grep findutils coreutils procps

# ── Dev build dependencies ──
apk add --no-cache \
    build-base gcc g++ musl-dev openssl-dev openssl-libs-static \
    pkgconf sqlite-dev perl linux-headers zlib-dev zlib-static

# ── Enable essential services ──
for svc in devfs dmesg mdev hwdrivers; do
    rc-update add $svc sysinit
done
for svc in modules sysctl hostname bootmisc syslog; do
    rc-update add $svc boot
done
rc-update add networking default
rc-update add local default
for svc in killprocs mount-ro savecache; do
    rc-update add $svc shutdown
done

# ── Create gently user ──
adduser -D -s /bin/bash gently
echo "gently:gently" | chpasswd
echo "gently ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# ── Auto-login on tty1, normal getty on tty2-3 ──
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

# ── Network config (DHCP on all interfaces) ──
cat > /etc/network/interfaces << 'NETCFG'
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
NETCFG

# ── Cleanup ──
rm -rf /var/cache/apk/*
SETUP_BASE
chmod +x "${BUILD_DIR}/rootfs/setup-base.sh"

# Bind-mount /proc /sys /dev for chroot operations
mount --bind /proc "${BUILD_DIR}/rootfs/proc"
mount --bind /sys "${BUILD_DIR}/rootfs/sys"
mount --bind /dev "${BUILD_DIR}/rootfs/dev"
mkdir -p "${BUILD_DIR}/rootfs/dev/pts"
mount --bind /dev/pts "${BUILD_DIR}/rootfs/dev/pts"

chroot "${BUILD_DIR}/rootfs" /setup-base.sh
rm "${BUILD_DIR}/rootfs/setup-base.sh"
echo "  Done."

# ════════════════════════════════════════════════════════════════
# Phase 3: Rust toolchain + cargo-watch
# ════════════════════════════════════════════════════════════════
echo "[3/9] Installing Rust toolchain (this takes a few minutes)..."

cat > "${BUILD_DIR}/rootfs/install-rust.sh" << 'RUST_INSTALL'
#!/bin/sh
set -e

# Install rustup as gently user
su - gently -s /bin/sh -c '
    export HOME=/home/gently
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | \
        sh -s -- -y --default-toolchain stable --profile default 2>&1 | tail -3

    . "$HOME/.cargo/env"
    echo "  rustc $(rustc --version)"
    echo "  cargo $(cargo --version)"

    echo "  Installing cargo-watch..."
    cargo install cargo-watch 2>&1 | tail -2
    echo "  cargo-watch $(cargo-watch --version 2>/dev/null || echo installed)"
'
RUST_INSTALL
chmod +x "${BUILD_DIR}/rootfs/install-rust.sh"
chroot "${BUILD_DIR}/rootfs" /install-rust.sh
rm "${BUILD_DIR}/rootfs/install-rust.sh"
echo "  Done."

# ════════════════════════════════════════════════════════════════
# Phase 4: Copy source tree
# ════════════════════════════════════════════════════════════════
echo "[4/9] Copying gentlyos-core source tree..."

mkdir -p "${BUILD_DIR}/rootfs/home/gently/gentlyos-core"

rsync -a --info=progress2 \
    --exclude='.git' \
    --exclude='target' \
    --exclude='dist' \
    --exclude='build' \
    --exclude='.import-bucket' \
    "${PROJECT_ROOT}/" "${BUILD_DIR}/rootfs/home/gently/gentlyos-core/"

chroot "${BUILD_DIR}/rootfs" chown -R gently:gently /home/gently/gentlyos-core

SRC_SIZE=$(du -sh "${BUILD_DIR}/rootfs/home/gently/gentlyos-core" | cut -f1)
echo "  Source tree: ${SRC_SIZE}"

# ════════════════════════════════════════════════════════════════
# Phase 5: Build workspace (warm cargo cache)
# ════════════════════════════════════════════════════════════════
echo "[5/9] Building workspace (warming cargo cache)..."
echo "  This takes 15-45 minutes depending on hardware."
echo "  All 28 crates will be compiled so incremental rebuilds are instant."
echo ""

BUILD_START=$(date +%s)

cat > "${BUILD_DIR}/rootfs/build-workspace.sh" << 'BUILD_WS'
#!/bin/sh
set -e
su - gently -s /bin/sh -c '
    . "$HOME/.cargo/env"
    cd "$HOME/gentlyos-core"

    echo "  cargo build --release ..."
    cargo build --release 2>&1 | grep -E "Compiling|Finished|error" | tail -20

    echo ""
    echo "  Build artifacts:"
    ls -lh target/release/gently target/release/gently-web 2>/dev/null || echo "  (binaries may have different names)"

    # Show dep cache size
    echo "  Dep cache: $(du -sh target/release/deps 2>/dev/null | cut -f1 || echo "N/A")"
    echo "  Total target/: $(du -sh target/ 2>/dev/null | cut -f1 || echo "N/A")"
'
BUILD_WS
chmod +x "${BUILD_DIR}/rootfs/build-workspace.sh"
chroot "${BUILD_DIR}/rootfs" /build-workspace.sh
rm "${BUILD_DIR}/rootfs/build-workspace.sh"

BUILD_END=$(date +%s)
BUILD_MINS=$(( (BUILD_END - BUILD_START) / 60 ))
echo "  Build completed in ${BUILD_MINS} minutes."

# ════════════════════════════════════════════════════════════════
# Phase 6: Dev environment + services
# ════════════════════════════════════════════════════════════════
echo "[6/9] Configuring dev environment..."

# ── Install pre-built binaries to /opt/gently/bin ──
mkdir -p "${BUILD_DIR}/rootfs/opt/gently/bin"
if [ -f "${BUILD_DIR}/rootfs/home/gently/gentlyos-core/target/release/gently" ]; then
    cp "${BUILD_DIR}/rootfs/home/gently/gentlyos-core/target/release/gently" \
       "${BUILD_DIR}/rootfs/opt/gently/bin/"
    chmod +x "${BUILD_DIR}/rootfs/opt/gently/bin/gently"
    echo "  Installed: gently CLI"
fi
if [ -f "${BUILD_DIR}/rootfs/home/gently/gentlyos-core/target/release/gently-web" ]; then
    cp "${BUILD_DIR}/rootfs/home/gently/gentlyos-core/target/release/gently-web" \
       "${BUILD_DIR}/rootfs/opt/gently/bin/"
    chmod +x "${BUILD_DIR}/rootfs/opt/gently/bin/gently-web"
    echo "  Installed: gently-web"
fi

# ── gently-web OpenRC service (production binary, auto-starts) ──
cat > "${BUILD_DIR}/rootfs/etc/init.d/gently-web" << 'WEBSERVICE'
#!/sbin/openrc-run

name="GentlyOS Web GUI"
description="ONE SCENE Web Interface (production binary)"

command="/opt/gently/bin/gently-web"
command_args="-h 0.0.0.0 -p 3000"
command_user="gently"
command_background="yes"
pidfile="/run/${RC_SVCNAME}.pid"

depend() {
    need net local
    after firewall
}
WEBSERVICE
chmod +x "${BUILD_DIR}/rootfs/etc/init.d/gently-web"
chroot "${BUILD_DIR}/rootfs" rc-update add gently-web default

# ── gently-dev: hot-reload switch script ──
cat > "${BUILD_DIR}/rootfs/opt/gently/bin/gently-dev" << 'GENTLY_DEV'
#!/bin/bash
# GentlyOS Dev Mode — cargo-watch hot-reload for gently-web
#
# Usage:
#   gently-dev          Start hot-reload (stops production service)
#   gently-dev stop     Stop hot-reload, restore production service
#   gently-dev watch    Watch a specific crate (default: gently-web)
#   gently-dev build    One-shot full rebuild

set -e

# Ensure cargo is on PATH
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

WORKSPACE="$HOME/gentlyos-core"
WATCH_CRATE="${GENTLY_DEV_CRATE:-gently-web}"

case "${1:-start}" in
    start)
        echo ""
        echo "  ┌─────────────────────────────────────────┐"
        echo "  │  GentlyOS Dev Mode — Hot Reload          │"
        echo "  └─────────────────────────────────────────┘"
        echo ""
        echo "  Stopping production service..."
        sudo rc-service gently-web stop 2>/dev/null || true
        sleep 1

        echo "  Starting cargo-watch..."
        echo "  Watching: crates/${WATCH_CRATE}/src/"
        echo "  Server:   http://localhost:3000"
        echo ""
        echo "  Edit files → auto-rebuild → refresh browser"
        echo "  Press Ctrl+C to stop"
        echo ""

        cd "${WORKSPACE}"
        exec cargo watch \
            -w "crates/${WATCH_CRATE}/src" \
            -w "crates/${WATCH_CRATE}/Cargo.toml" \
            -s "cargo build --release -p ${WATCH_CRATE} && echo '--- Restarting ${WATCH_CRATE} ---' && pkill -f 'gently-web.*3000' 2>/dev/null; sleep 0.5; ./target/release/gently-web -h 0.0.0.0 -p 3000 &"
        ;;

    stop)
        echo "  Stopping dev mode..."
        pkill -f "cargo.watch" 2>/dev/null || true
        pkill -f "cargo-watch" 2>/dev/null || true
        pkill -f "gently-web" 2>/dev/null || true
        sleep 1
        echo "  Restarting production service..."
        sudo rc-service gently-web start
        echo "  Production gently-web restored."
        ;;

    watch)
        # Watch a custom crate
        WATCH_CRATE="${2:-gently-web}"
        echo "  Watching crate: ${WATCH_CRATE}"
        cd "${WORKSPACE}"
        exec cargo watch \
            -w "crates/${WATCH_CRATE}/src" \
            -x "build --release -p ${WATCH_CRATE}"
        ;;

    build)
        echo "  Full workspace rebuild..."
        cd "${WORKSPACE}"
        cargo build --release 2>&1
        echo "  Done."
        ;;

    *)
        echo "GentlyOS Dev Mode"
        echo ""
        echo "Usage: gently-dev [command]"
        echo ""
        echo "Commands:"
        echo "  start           Stop production, start cargo-watch hot-reload (default)"
        echo "  stop            Stop hot-reload, restore production service"
        echo "  watch [crate]   Watch a specific crate for changes"
        echo "  build           One-shot full workspace rebuild"
        echo ""
        echo "Environment:"
        echo "  GENTLY_DEV_CRATE    Crate to watch (default: gently-web)"
        echo ""
        echo "Quick start:"
        echo "  1. Run 'gently-dev' to enter hot-reload mode"
        echo "  2. Edit ~/gentlyos-core/crates/gently-web/src/"
        echo "  3. cargo-watch rebuilds and restarts automatically"
        echo "  4. Refresh http://localhost:3000 to see changes"
        ;;
esac
GENTLY_DEV
chmod +x "${BUILD_DIR}/rootfs/opt/gently/bin/gently-dev"

# ── Bash profile ──
cat > "${BUILD_DIR}/rootfs/home/gently/.profile" << 'BASH_PROFILE'
# GentlyOS Dev Environment
export PATH="/opt/gently/bin:$HOME/.cargo/bin:$PATH"
export GENTLY_DATA_DIR="$HOME/.gently"
export EDITOR=vim
export CARGO_TERM_COLOR=always

# Source cargo env
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

# Welcome on first login
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

  DEV IMAGE — Edit the GUI while using the GUI

  Web GUI:    http://localhost:3000 (auto-started)
  Source:     ~/gentlyos-core/crates/gently-web/src/

  Commands:
    gently-dev          Start hot-reload mode (cargo-watch)
    gently-dev stop     Return to production mode
    gently-dev build    Full workspace rebuild
    gently --help       CLI documentation

BANNER

    # Show persistence status
    if mount | grep -q "gentlyos-data"; then
        echo "  Storage: PERSISTENT (changes saved to USB)"
    else
        echo "  Storage: RAM ONLY (changes lost on reboot)"
        echo "  Enable: flash USB with flash-usb-dev.sh"
    fi
    echo ""
fi
BASH_PROFILE
chroot "${BUILD_DIR}/rootfs" chown gently:gently /home/gently/.profile

# ── .gently data directory ──
chroot "${BUILD_DIR}/rootfs" sh -c \
    'mkdir -p /home/gently/.gently && chown -R gently:gently /home/gently/.gently'

echo "  Done."

# ════════════════════════════════════════════════════════════════
# Phase 7: Overlayfs persistence support
# ════════════════════════════════════════════════════════════════
echo "[7/9] Setting up persistence support..."

# ── Boot-time persistence mount (local.d runs before gently-web) ──
cat > "${BUILD_DIR}/rootfs/etc/local.d/persistence.start" << 'PERSIST_START'
#!/bin/sh
# GentlyOS Persistence — overlayfs on /home/gently
# Looks for a partition labeled "gentlyos-data" (ext4)
# If found: overlayfs gives writable /home/gently that survives reboot
# If not found: /home/gently is squashfs (read-only base), tmpfs overlay

PERSIST_LABEL="gentlyos-data"
PERSIST_MNT="/mnt/persist"
HOME_DIR="/home/gently"

# Load overlay kernel module
modprobe overlay 2>/dev/null || true

# Find persistence partition by label
PERSIST_DEV=""
for attempt in 1 2 3; do
    PERSIST_DEV=$(blkid -L "${PERSIST_LABEL}" 2>/dev/null || true)
    [ -n "${PERSIST_DEV}" ] && break
    sleep 1  # USB devices may need a moment
done

if [ -z "${PERSIST_DEV}" ]; then
    echo "persistence: no partition labeled '${PERSIST_LABEL}'"
    echo "persistence: running in RAM-only mode"
    exit 0
fi

echo "persistence: found ${PERSIST_DEV}"

# Mount the persistence partition
mkdir -p "${PERSIST_MNT}"
if ! mount -t ext4 "${PERSIST_DEV}" "${PERSIST_MNT}" 2>/dev/null; then
    echo "persistence: mount failed for ${PERSIST_DEV}, trying auto-detect..."
    mount "${PERSIST_DEV}" "${PERSIST_MNT}" 2>/dev/null || {
        echo "persistence: FAILED — running in RAM-only mode"
        exit 0
    }
fi

# Initialize on first boot (copy base home into upper layer)
if [ ! -d "${PERSIST_MNT}/upper" ]; then
    echo "persistence: first boot — initializing storage..."
    mkdir -p "${PERSIST_MNT}/upper"
    mkdir -p "${PERSIST_MNT}/work"
    # Copy current home as initial upper layer
    cp -a "${HOME_DIR}/." "${PERSIST_MNT}/upper/"
    echo "persistence: initialized $(du -sh "${PERSIST_MNT}/upper" | cut -f1)"
fi

# Mount overlayfs: lower=squashfs home, upper=persistent storage
if mount -t overlay overlay \
    -o "lowerdir=${HOME_DIR},upperdir=${PERSIST_MNT}/upper,workdir=${PERSIST_MNT}/work" \
    "${HOME_DIR}" 2>/dev/null; then
    echo "persistence: overlayfs mounted on ${HOME_DIR}"
else
    # Fallback: bind mount (no overlayfs support in kernel)
    echo "persistence: overlayfs unavailable, using bind mount"
    mount --bind "${PERSIST_MNT}/upper" "${HOME_DIR}"
fi

# Ensure correct ownership
chown gently:gently "${HOME_DIR}"

echo "persistence: ENABLED — changes in ${HOME_DIR} will survive reboot"
PERSIST_START
chmod +x "${BUILD_DIR}/rootfs/etc/local.d/persistence.start"

# ── Shutdown: sync and unmount ──
cat > "${BUILD_DIR}/rootfs/etc/local.d/persistence.stop" << 'PERSIST_STOP'
#!/bin/sh
echo "persistence: syncing..."
sync
umount /home/gently 2>/dev/null || true
umount /mnt/persist 2>/dev/null || true
echo "persistence: unmounted"
PERSIST_STOP
chmod +x "${BUILD_DIR}/rootfs/etc/local.d/persistence.stop"

echo "  Done."

# ════════════════════════════════════════════════════════════════
# Phase 8: Create squashfs
# ════════════════════════════════════════════════════════════════
echo "[8/9] Creating squashfs filesystem..."
echo "  This takes a few minutes (compressing ~2-3 GB with XZ)..."

# Unmount bind mounts BEFORE creating squashfs
umount "${BUILD_DIR}/rootfs/dev/pts" 2>/dev/null || true
umount "${BUILD_DIR}/rootfs/proc" 2>/dev/null || true
umount "${BUILD_DIR}/rootfs/sys" 2>/dev/null || true
umount "${BUILD_DIR}/rootfs/dev" 2>/dev/null || true

# Clean up caches (keep cargo registry for fast installs)
rm -rf "${BUILD_DIR}/rootfs/var/cache/apk/"*
rm -rf "${BUILD_DIR}/rootfs/tmp/"*

# Set a working resolv.conf for runtime
echo "nameserver 8.8.8.8" > "${BUILD_DIR}/rootfs/etc/resolv.conf"
echo "nameserver 8.8.4.4" >> "${BUILD_DIR}/rootfs/etc/resolv.conf"

# Prepare kernel
KERNEL_FILE="${BUILD_DIR}/rootfs/boot/vmlinuz-lts"
INITRD_FILE="${BUILD_DIR}/rootfs/boot/initramfs-lts"
if [ ! -f "${KERNEL_FILE}" ]; then
    echo "Error: kernel not found at ${KERNEL_FILE}"
    ls "${BUILD_DIR}/rootfs/boot/"
    exit 1
fi
cp "${KERNEL_FILE}" "${BUILD_DIR}/iso/boot/vmlinuz"
cp "${INITRD_FILE}" "${BUILD_DIR}/iso/boot/initramfs"

# Create squashfs (XZ compression for best ratio)
SQUASH_START=$(date +%s)
mksquashfs "${BUILD_DIR}/rootfs" "${BUILD_DIR}/iso/live/filesystem.squashfs" \
    -comp xz -b 256K -Xbcj x86 \
    -no-exports -no-recovery \
    -progress

SQUASH_END=$(date +%s)
SQUASH_MINS=$(( (SQUASH_END - SQUASH_START) / 60 ))
SQUASH_SIZE=$(du -sh "${BUILD_DIR}/iso/live/filesystem.squashfs" | cut -f1)
echo "  Squashfs: ${SQUASH_SIZE} (compressed in ${SQUASH_MINS} min)"

# ════════════════════════════════════════════════════════════════
# Phase 9: Build ISO
# ════════════════════════════════════════════════════════════════
echo "[9/9] Building ISO..."

cat > "${BUILD_DIR}/iso/boot/grub/grub.cfg" << 'GRUB_CFG'
set timeout=5
set default=0

set menu_color_normal=white/black
set menu_color_highlight=black/white

menuentry "GentlyOS Dev" {
    linux /boot/vmlinuz modules=loop,squashfs,overlay,sd-mod,usb-storage nomodeset quiet
    initrd /boot/initramfs
}

menuentry "GentlyOS Dev (Verbose)" {
    linux /boot/vmlinuz modules=loop,squashfs,overlay,sd-mod,usb-storage nomodeset
    initrd /boot/initramfs
}

menuentry "GentlyOS Dev (RAM Only)" {
    linux /boot/vmlinuz modules=loop,squashfs,sd-mod,usb-storage nomodeset toram quiet
    initrd /boot/initramfs
}

menuentry "GentlyOS Dev (Safe — no GPU)" {
    linux /boot/vmlinuz modules=loop,squashfs,overlay,sd-mod,usb-storage nomodeset noapic acpi=off
    initrd /boot/initramfs
}
GRUB_CFG

grub-mkrescue -o "${DIST_DIR}/${ISO_NAME}" "${BUILD_DIR}/iso" \
    --product-name="GentlyOS Dev" \
    --product-version="${VERSION}" 2>/dev/null

# Checksums
cd "${DIST_DIR}"
sha256sum "${ISO_NAME}" > "${ISO_NAME}.sha256"

ISO_SIZE=$(du -h "${DIST_DIR}/${ISO_NAME}" | cut -f1)
ISO_SHA=$(cut -d' ' -f1 "${ISO_NAME}.sha256")

# ── Cleanup build artifacts ──
rm -rf "${BUILD_DIR}"

# ════════════════════════════════════════════════════════════════
# Done
# ════════════════════════════════════════════════════════════════
echo ""
echo "══════════════════════════════════════════════════════════"
echo "  Dev ISO built successfully!"
echo "══════════════════════════════════════════════════════════"
echo ""
echo "  File:   ${DIST_DIR}/${ISO_NAME}"
echo "  Size:   ${ISO_SIZE}"
echo "  SHA256: ${ISO_SHA}"
echo ""
echo "  Includes:"
echo "    Alpine Linux ${ALPINE_VERSION} (musl-native)"
echo "    Rust toolchain (rustup + stable)"
echo "    cargo-watch (hot-reload)"
echo "    gentlyos-core source tree (28 crates)"
echo "    Pre-built workspace (warm cargo cache)"
echo "    Overlayfs persistence support"
echo ""
echo "  Flash to USB:"
echo "    sudo ./scripts/deploy/flash-usb-dev.sh ${DIST_DIR}/${ISO_NAME} /dev/sdX"
echo ""
echo "  Or quick flash (no persistence):"
echo "    sudo dd if=${DIST_DIR}/${ISO_NAME} of=/dev/sdX bs=4M status=progress"
echo ""
