# GentlyOS Dev ISO — Installation Guide

Bootable live USB with full Rust development environment.
Edit the GUI while using the GUI.

```
ISO:  gentlyos-dev-1.1.1-dev-x86_64.iso (822 MB)
Base: Alpine Linux 3.21 (musl-native)
```

---

## What's Inside

| Component | Details |
|-----------|---------|
| Alpine Linux 3.21 | musl-native, OpenRC, Linux LTS kernel |
| Rust toolchain | rustup + stable + cargo, clippy, rustfmt |
| cargo-watch | Hot-reload — auto-rebuild on file change |
| gentlyos-core | Full source tree (28 crates, ~80K LOC) |
| Pre-built workspace | Warm cargo cache — incremental rebuilds in seconds |
| gently-web | ONE SCENE Web GUI, auto-starts on port 3000 |
| gently CLI | 21 commands, ready to use |
| Overlayfs persistence | Edits survive reboot (requires persistence partition) |

---

## Requirements

### Build Machine

- Linux host (NixOS, Ubuntu, Arch — any x86_64)
- Root access (chroot + mount)
- ~10 GB free disk (build artifacts, cleaned after)
- Internet access (downloads Alpine, Rust, crates)
- Tools: `mksquashfs xorriso grub-mkrescue rsync wget`

**NixOS:**
```bash
nix-shell -p squashfsTools xorriso grub2 mtools rsync wget
```

**Ubuntu/Debian:**
```bash
sudo apt install squashfs-tools xorriso grub-common grub-pc-bin grub-efi-amd64-bin mtools rsync wget
```

### Target Machine

- x86_64 CPU
- 2 GB RAM minimum (4 GB recommended)
- USB port (boot from USB)
- BIOS or UEFI boot support

### USB Drive

- 4 GB minimum (ISO only, no persistence)
- 16 GB+ recommended (ISO + persistence partition)
- USB 3.0 recommended for faster builds

---

## Step 1: Build the ISO

```bash
cd ~/Desktop/claude-cage/gentlyos-core
sudo ./scripts/deploy/build-alpine-dev-iso.sh
```

Build takes 30-60 minutes (compiles all 28 Rust crates inside chroot).

Output: `dist/gentlyos-dev-1.1.1-dev-x86_64.iso`

### Custom Version

```bash
sudo GENTLY_VERSION=2.0.0 ./scripts/deploy/build-alpine-dev-iso.sh
```

### Via build-all.sh

```bash
sudo ./scripts/deploy/build-all.sh --dev-iso
```

### Build Phases

```
[1/9] Download Alpine 3.21 minirootfs
[2/9] Install system + dev packages (build-base, openssl-dev, pkgconf...)
[3/9] Install Rust toolchain (rustup + stable + cargo-watch)
[4/9] Copy gentlyos-core source tree
[5/9] Build workspace (warm cargo cache) ← this is the slow part
[6/9] Configure dev environment (services, gently-dev script)
[7/9] Set up overlayfs persistence support
[8/9] Create squashfs (XZ compression)
[9/9] Build GRUB ISO
```

---

## Step 2: Flash to USB

### Option A: With Persistence (recommended)

Creates two partitions: boot + writable data.
Source code edits, cargo cache, configs survive reboot.

```bash
sudo ./scripts/deploy/flash-usb-dev.sh dist/gentlyos-dev-1.1.1-dev-x86_64.iso /dev/sdX
```

Replace `/dev/sdX` with your USB device. Use `lsblk` to identify it.

The flasher will:
1. Write the ISO to partition 1
2. Create an ext4 partition labeled `gentlyos-data` using remaining space
3. Verify the layout

### Option B: Without Persistence (quick)

Simple dd — all changes lost on reboot.

```bash
sudo dd if=dist/gentlyos-dev-1.1.1-dev-x86_64.iso of=/dev/sdX bs=4M status=progress
sync
```

### Option C: Add Persistence Later

If you used Option B and want persistence afterwards:

```bash
# 1. Find the USB device
lsblk

# 2. Create a new partition after the ISO data
sudo fdisk /dev/sdX
# Press: n → p → (defaults) → w

# 3. Format with the magic label
sudo mkfs.ext4 -L gentlyos-data /dev/sdX2
```

The boot script auto-detects any partition labeled `gentlyos-data`.

---

## Step 3: Boot

1. Insert USB into target machine
2. Enter BIOS/UEFI boot menu (F12, F2, DEL, or ESC at power-on)
3. Select the USB drive
4. GRUB menu appears:

```
GentlyOS Dev              ← Normal boot (auto-detects persistence)
GentlyOS Dev (Verbose)    ← Boot with kernel messages
GentlyOS Dev (RAM Only)   ← Forces no persistence
```

5. Auto-login as `gently` user (password: `gently`)
6. gently-web starts automatically on port 3000

---

## Step 4: Develop

### See the GUI

Open a browser on the machine (or from another machine on the same network):

```
http://localhost:3000
```

### Start Hot-Reload Mode

Switch to tty2 (Alt+F2) or open a terminal:

```bash
gently-dev
```

This:
1. Stops the production gently-web service
2. Starts cargo-watch monitoring `crates/gently-web/src/`
3. On file change: rebuilds → restarts server automatically

### Edit Source

```bash
vim ~/gentlyos-core/crates/gently-web/src/templates.rs
```

Save the file → cargo-watch detects the change → rebuilds gently-web → restarts the server. Refresh your browser to see changes.

### Watch a Different Crate

```bash
GENTLY_DEV_CRATE=gently-goo gently-dev
# or
gently-dev watch gently-alexandria
```

### Full Rebuild

```bash
gently-dev build
```

### Return to Production Mode

```bash
gently-dev stop
```

Stops cargo-watch, restarts the pre-built gently-web binary.

---

## Persistence

### How It Works

On boot, `/etc/local.d/persistence.start` runs:

1. Looks for a partition labeled `gentlyos-data`
2. If found: mounts it, sets up overlayfs on `/home/gently`
3. All changes to `/home/gently` (source code, cargo cache, configs) write to the persistence partition
4. On shutdown: syncs and cleanly unmounts

### Check Status

```bash
# In the welcome banner:
#   Storage: PERSISTENT (changes saved to USB)
#   Storage: RAM ONLY (changes lost on reboot)

# Or manually:
mount | grep gentlyos
```

### Reset Persistence

To start fresh (wipe all saved changes):

```bash
# From the host machine (not the live USB):
sudo mount /dev/sdX2 /mnt
sudo rm -rf /mnt/upper /mnt/work
sudo umount /mnt
```

Next boot will re-initialize from the squashfs base.

---

## Directory Layout (on the live system)

```
/home/gently/
├── gentlyos-core/           ← Full source tree
│   ├── crates/              ← 28 gently-* crates
│   │   ├── gently-web/      ← Edit this for GUI changes
│   │   ├── gently-goo/      ← GOO unified field
│   │   ├── gently-security/ ← FAFO defense
│   │   └── ...
│   ├── gently-cli/          ← Main CLI binary
│   ├── gentlyos-tui/        ← Terminal UI
│   ├── target/release/      ← Pre-built binaries + warm dep cache
│   └── Cargo.toml           ← Workspace (28 crates)
├── .cargo/                  ← Rust toolchain + cargo-watch
├── .gently/                 ← GentlyOS data directory
└── .profile                 ← Dev environment setup

/opt/gently/bin/
├── gently                   ← Pre-built CLI binary
├── gently-web               ← Pre-built web binary (production service)
└── gently-dev               ← Hot-reload switch script
```

---

## Commands Reference

| Command | Description |
|---------|-------------|
| `gently-dev` | Start cargo-watch hot-reload (stops production service) |
| `gently-dev stop` | Stop hot-reload, restore production service |
| `gently-dev watch <crate>` | Watch a specific crate for changes |
| `gently-dev build` | One-shot full workspace rebuild |
| `gently --help` | CLI documentation (21 commands) |
| `gently status` | System status |
| `gently security status` | FAFO security dashboard |
| `rustc --version` | Check Rust version |
| `cargo test -p gently-web` | Run tests for a crate |

---

## Troubleshooting

### Build fails at Phase 3 (Rust install)

Chroot needs network access. Ensure `/etc/resolv.conf` exists on the build host.

```bash
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf
```

### Build fails at Phase 5 (cargo build)

Memory issue — cargo needs ~4 GB RAM for a full workspace build. Close other apps or add swap:

```bash
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### USB doesn't boot

- Verify BIOS is set to boot from USB (check boot order)
- Try both UEFI and Legacy/CSM mode
- Re-flash: `sudo dd if=... of=/dev/sdX bs=4M conv=fsync && sync`

### No persistence after reboot

- Check the partition label: `sudo blkid | grep gentlyos-data`
- If missing: `sudo e2label /dev/sdX2 gentlyos-data`
- Kernel needs overlay module — boot verbose to check: `modprobe overlay`

### cargo-watch not rebuilding

- Check you're editing the watched path: `crates/gently-web/src/`
- Ensure cargo is on PATH: `source ~/.cargo/env`
- Try manual build: `cd ~/gentlyos-core && cargo build -p gently-web --release`

### gently-web won't start

```bash
# Check service status
sudo rc-service gently-web status

# Check if port 3000 is in use
ss -tlnp | grep 3000

# Start manually
/opt/gently/bin/gently-web -h 0.0.0.0 -p 3000
```

---

## Network Access

The live system uses DHCP on `eth0`. For WiFi or static IP:

```bash
# WiFi (if hardware is supported)
sudo apk add wpa_supplicant wireless-tools
sudo wpa_passphrase "SSID" "password" > /etc/wpa_supplicant/wpa_supplicant.conf
sudo wpa_supplicant -B -i wlan0 -c /etc/wpa_supplicant/wpa_supplicant.conf
sudo udhcpc -i wlan0

# Static IP
sudo ip addr add 10.0.0.2/24 dev eth0
sudo ip route add default via 10.0.0.1
```

---

## Security Notes

- Default user `gently` has passwordless sudo
- SSH is client-only (no sshd running)
- Firewall is not enabled by default
- The live system is meant for development, not production deployment
- For production: use the standard `build-alpine-iso.sh` (232 MB, read-only, no toolchain)
