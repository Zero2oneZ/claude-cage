# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Is This

claude-cage is a dockerized sandbox for running Claude CLI and Claude Desktop in isolated containers with defense-in-depth security. Two modes: CLI (interactive TTY) and Desktop (Xvfb + noVNC in browser at localhost:6080).

## Build & Run Commands

```bash
# Build container images
make build            # Build both CLI and Desktop images
make build-cli        # Build CLI image only
make build-desktop    # Build Desktop image only

# Run containers
make run-cli          # Interactive Claude CLI session
make run-desktop      # Desktop mode (detached), access at http://localhost:6080
make run-isolated     # CLI with no network access

# Stop & clean
make stop             # Stop all containers
make clean            # Stop + remove containers
make clean-volumes    # Also remove persistent volumes
make clean-images     # Remove built images
make clean-all        # Full cleanup (containers + volumes + images)

# Security
make load-apparmor    # Load AppArmor profile (requires sudo)
make verify-sandbox   # Inspect container security settings (read-only, caps, memory, seccomp)

# Status
make status           # Show running cage containers
make logs             # Follow container logs

# Install CLI tool system-wide
make install          # Symlink bin/claude-cage to /usr/local/bin (requires sudo)
```

### CLI Tool (`bin/claude-cage` or `claude-cage` after install)

```bash
claude-cage start [--mode cli|desktop] [--mount ./dir] [--network none|filtered|host]
claude-cage stop [name|--all]
claude-cage shell <name>          # Attach bash to running container
claude-cage list                  # Show all sessions
claude-cage destroy <name>        # Remove container + volume
claude-cage config [--validate]   # Show or validate configuration
```

## Architecture

### Bash Library Architecture (`lib/`)

The CLI is a modular bash application. `bin/claude-cage` sources all five library modules, then dispatches commands via `cmd_<command>()` functions in `lib/cli.sh`.

| Module | Responsibility |
|---|---|
| `lib/cli.sh` | Command parsing, argument handling, all `cmd_*()` functions |
| `lib/docker.sh` | Docker build, run, stop, destroy, exec, inspect |
| `lib/sandbox.sh` | Constructs security flags, creates filtered network, applies iptables rules, verifies sandbox |
| `lib/session.sh` | Session metadata (create/list/status/remove), name generation (`adjective-noun-hex4`) |
| `lib/config.sh` | YAML config loading (default + user override at `~/.config/claude-cage/config.yaml`), validation |

### Container Images (`docker/`)

- **CLI** (`docker/cli/Dockerfile`): `node:20-slim` base, installs `@anthropic-ai/claude-code` via npm, runs as non-root `cageuser`, entrypoint is `tini` → `claude`
- **Desktop** (`docker/desktop/Dockerfile`): `ubuntu:24.04` base, full X11 stack (Xvfb + openbox + x11vnc + noVNC/websockify), launches xterm with Claude CLI

### Security Model (defense-in-depth layers)

1. **Read-only root filesystem** with tmpfs at `/tmp` (512m) and `/run` (64m)
2. **Capabilities**: ALL dropped, only CHOWN/DAC_OVERRIDE/SETGID/SETUID re-added
3. **Seccomp** (`security/seccomp-default.json`): ~147 syscall allowlist, AF_VSOCK blocked
4. **AppArmor** (`security/apparmor-profile`): denies mount/ptrace/raw-network/kernel-module access
5. **Resource limits**: 2 CPUs, 4GB memory, 512 PIDs, limited file descriptors
6. **Network filtering**: `sandbox_apply_network_filter()` uses iptables post-launch to restrict outbound to `allowed_hosts` only (default: `api.anthropic.com`, `cdn.anthropic.com`)
7. **no-new-privileges** flag prevents escalation
8. **Bridge network** (`cage-filtered`) with inter-container communication disabled

### Docker Compose Services

`docker-compose.yml` defines three services sharing an `x-common` anchor for security baseline:
- `cli` — interactive TTY with filtered network
- `desktop` — detached with noVNC ports (6080, 5900)
- `cli-isolated` — network_mode: none

### Configuration

`config/default.yaml` holds all defaults. User overrides go in `~/.config/claude-cage/config.yaml`. Config is loaded into the `CAGE_CFG[]` associative array. The YAML parser is minimal (flat key:value only — no nested structures).

## Key Implementation Details

- Session names are auto-generated as `<adjective>-<noun>-<hex4>` (e.g., "swift-fox-a1b2")
- Containers are labeled `managed-by=claude-cage` for discovery via `docker ps --filter`
- Session metadata is stored in `~/.local/share/claude-cage/sessions/<name>/metadata`
- Network filtering happens *after* container launch via `sandbox_apply_network_filter()` — it resolves `allowed_hosts` to IPs and injects iptables rules
- Desktop entrypoint (`docker/desktop/entrypoint-desktop.sh`) starts services sequentially: Xvfb → openbox → x11vnc → websockify → xterm, with EXIT trap cleanup
- `docker-compose.yml` and the CLI tool (`lib/docker.sh`) both construct equivalent security flags independently — changes to security policy must be updated in both places

## No Tests or Linting

There is no test suite or linting configuration. The primary verification mechanism is `make verify-sandbox` which inspects a running container's security settings.

## Subproject: headless-ubuntu-auto

`projects/headless-ubuntu-auto/` is a separate 24-file project for headless GPU server provisioning (2x RTX 3090). It has its own Makefile and is independent of the main claude-cage codebase. See its own README for details.
