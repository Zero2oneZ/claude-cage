# GentlyOS Bootable Linux Build Specification

## Overview

This document specifies the bootable Linux build for GentlyOS - a privacy-focused, Claude-integrated development environment with blockchain-anchored audit trails.

## Base System

### Distribution: Alpine Linux 3.21
**Rationale**:
- Already using musl target (`x86_64-unknown-linux-musl`)
- Minimal footprint (~5MB base)
- Security-focused (no glibc attack surface)
- Fast boot time (~2 seconds to shell)

### Kernel: Linux 6.12 LTS
- Minimal config (no unnecessary drivers)
- Enable: IPFS/FUSE, USB mass storage, virtio, crypto modules
- Disable: Bluetooth, WiFi (add as modules if needed)

## Partition Layout

```
/dev/sda1  512MB   EFI System Partition (FAT32)
/dev/sda2  4GB     /boot (ext4, LUKS encrypted)
/dev/sda3  REST    LVM (LUKS encrypted)
  └─ gentlyos-root  20GB   / (ext4)
  └─ gentlyos-home  REST   /home (ext4, XOR-split encryption)
  └─ gentlyos-swap  4GB    swap (encrypted)
```

## Boot Process

### Stage 1: UEFI/GRUB
```
UEFI → GRUB → initramfs → LUKS unlock → LVM → rootfs
```

### Stage 2: Init System (OpenRC)
```
1. Mount filesystems
2. Start networking (if configured)
3. Start IPFS daemon (pinned content)
4. Start gently-security (16 daemons)
5. Start gently-guardian (hardware detection)
6. Start Berlin Clock sync (BTC block monitoring)
7. Launch Claude sandbox environment
8. Present TUI login
```

## Core Package Set

### System Base (~50MB)
```
alpine-base busybox openrc eudev
e2fsprogs lvm2 cryptsetup grub-efi
```

### Runtime Dependencies (~100MB)
```
musl libgcc libstdc++ zlib openssl
sqlite-libs curl jq git
```

### GentlyOS Binaries (~50MB)
```
/opt/gently/bin/gently          # Main CLI
/opt/gently/bin/gently-tui      # Terminal UI
/opt/gently/bin/gently-web      # Web server
/opt/gently/bin/gently-mcp      # MCP server for Claude
```

### Claude Integration (~10MB)
```
/opt/gently/bin/claude          # Claude Code CLI (symlinked)
/opt/gently/etc/claude.sh       # BTC-anchored wrapper
/opt/gently/etc/audit.sh        # Hash chain audit
```

### IPFS Daemon (~30MB)
```
/usr/bin/ipfs                   # Kubo IPFS daemon
/etc/ipfs/config                # Localhost-only config
```

## Security Configuration

### Sandboxed Claude Environment

```toml
# /opt/gently/etc/claude-sandbox.toml

[sandbox]
type = "bubblewrap"  # or "firejail"

[network]
allow_outbound = ["api.anthropic.com:443", "blockchain.info:443"]
deny_inbound = true

[filesystem]
read_only = ["/", "/usr", "/opt/gently"]
read_write = ["/home/gently/.gently", "/tmp"]
hidden = ["/root/.ssh", "/root/.git-credentials"]

[audit]
btc_anchor = true
hash_chain = true
log_all_api_calls = true

[limits]
max_memory = "4G"
max_cpu_percent = 80
max_tokens_per_session = 100000
```

### FAFO Security Activation

```bash
# /etc/init.d/gently-security
description="GentlyOS Security Daemons"
command="/opt/gently/bin/gently"
command_args="security start --all-daemons --fafo-enabled"
```

### Berlin Clock Service

```bash
# /etc/init.d/berlin-clock
description="BTC-synced key rotation"
command="/opt/gently/bin/gently"
command_args="btc berlin-clock --rotate-interval=300"
```

## Private Claude Instance Configuration

### Sandboxed Execution Wrapper

```bash
#!/bin/sh
# /opt/gently/bin/claude-sandbox

# Source BTC block for deterministic branching
BTC=$(curl -s https://blockchain.info/latestblock | jq -r '.height' 2>/dev/null || echo "0")
BRANCH=$((BTC % 7 + 1))

# Audit session start
/opt/gently/etc/audit.sh "claude_start:branch-${BRANCH}:btc-${BTC}"

# Run Claude in sandbox
bwrap \
  --ro-bind / / \
  --bind /home/gently/.gently /home/gently/.gently \
  --bind /tmp /tmp \
  --unshare-net \
  --share-net \
  --setenv GENTLY_BTC_BLOCK "$BTC" \
  --setenv GENTLY_BRANCH "$BRANCH" \
  /opt/gently/bin/claude "$@"

EXIT_CODE=$?

# Audit session end
/opt/gently/etc/audit.sh "claude_end:branch-${BRANCH}:btc-${BTC}:exit-${EXIT_CODE}"

exit $EXIT_CODE
```

### API Key Management

```toml
# /opt/gently/etc/keys.toml
# XOR-split: LOCK stays on device, KEY in this file

[claude]
key_path = "/home/gently/.gently/vault/claude.key"
lock_path = "/home/gently/.gently/vault/claude.lock"  # Generated on first boot

[ipfs]
identity_path = "/home/gently/.gently/vault/ipfs-identity.json"

[btc]
# Read-only, no private keys
rpc_url = "https://blockchain.info"
```

## Development Environment

### Rust Toolchain (Preinstalled)

```
rustc 1.92.0 (stable-x86_64-unknown-linux-musl)
cargo, rustfmt, clippy, rust-analyzer
```

### GentlyOS Development Tools

```bash
# Build GentlyOS from source
gently dev build --release

# Run test suite
gently dev test --all

# Start development TUI
gently dev tui

# Hot-reload web interface
gently dev web --watch
```

## ISO Build Process

```bash
#!/bin/bash
# /root/gentlyos/scripts/build-gentlyos-iso.sh

VERSION="1.0.0"
WORKDIR="/tmp/gentlyos-build"
OUTPUT="gentlyos-${VERSION}-amd64.iso"

# 1. Create Alpine base
mkdir -p $WORKDIR/{boot,EFI,live}
debootstrap --variant=minbase --arch=amd64 alpine $WORKDIR/rootfs

# 2. Install GentlyOS
cp -r /opt/gently $WORKDIR/rootfs/opt/
cp -r /root/.gently $WORKDIR/rootfs/etc/skel/.gently

# 3. Install Claude wrapper
cp /opt/gently/bin/claude-sandbox $WORKDIR/rootfs/usr/local/bin/claude

# 4. Configure services
for svc in gently-security berlin-clock ipfs; do
  cp /etc/init.d/$svc $WORKDIR/rootfs/etc/init.d/
  ln -s /etc/init.d/$svc $WORKDIR/rootfs/etc/runlevels/default/
done

# 5. Create squashfs
mksquashfs $WORKDIR/rootfs $WORKDIR/live/filesystem.squashfs -comp xz

# 6. Build EFI boot
grub-mkstandalone -O x86_64-efi \
  -o $WORKDIR/EFI/BOOT/BOOTX64.EFI \
  "boot/grub/grub.cfg=/tmp/grub-embed.cfg"

# 7. Create ISO
xorriso -as mkisofs \
  -o $OUTPUT \
  -isohybrid-mbr /usr/share/syslinux/isohdpfx.bin \
  -c boot.catalog \
  -b boot/grub/eltorito.img \
  -no-emul-boot -boot-load-size 4 -boot-info-table \
  -eltorito-alt-boot \
  -e EFI/efiboot.img \
  -no-emul-boot -isohybrid-gpt-basdat \
  $WORKDIR

echo "Built: $OUTPUT"
sha256sum $OUTPUT > ${OUTPUT}.sha256
```

## USB Installation

```bash
# Flash to USB
dd if=gentlyos-1.0.0-amd64.iso of=/dev/sdX bs=4M status=progress

# Or use the GentlyOS tool
gently flash --iso gentlyos-1.0.0-amd64.iso --device /dev/sdX
```

## First Boot Sequence

1. **LUKS Password**: User enters disk encryption password
2. **Genesis Key Split**: System generates LOCK (stored locally), user receives KEY
3. **Claude Authentication**: OAuth flow with Anthropic
4. **IPFS Identity**: Generate or import peer identity
5. **Berlin Clock Sync**: Connect to Bitcoin blockchain for time anchoring
6. **Security Activation**: Start 16 security daemons
7. **TUI Welcome**: Present GentlyOS terminal interface

## Persistence Strategy

### What's Retained:
- `/home/gently/.gently/` - All GentlyOS data, knowledge graphs, keys
- `/home/gently/.claude/` - Claude session history and settings
- `/home/gently/.config/` - Application configurations
- `/opt/gently/` - GentlyOS binaries and scripts

### What's Reset on Boot:
- `/tmp/` - Temporary files
- Session tokens (regenerated from XOR split)
- Network connections

## Recovery Options

1. **Live Mode**: Boot without touching disk, access existing data read-only
2. **Recovery Mode**: Mount encrypted partitions, fix issues
3. **Factory Reset**: Regenerate genesis, keep LOCK, wipe data
4. **SAMSON Mode**: Scorched earth - wipe everything including LOCK

## Hardware Requirements

### Minimum:
- CPU: x86_64 with AES-NI
- RAM: 2GB
- Storage: 32GB
- Network: Ethernet or WiFi

### Recommended:
- CPU: Intel Alder Lake or newer / AMD Zen 3+
- RAM: 8GB+
- Storage: 128GB NVMe
- Network: Gigabit Ethernet

## File Checksums

All binaries signed with genesis key and anchored to Bitcoin blockchain.
Verification: `gently verify --file /opt/gently/bin/gently --btc-anchor`
