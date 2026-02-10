# Launcher Reference

## Overview

The `launch` script is the single entry point for all claude-cage operations. Located at the project root (`./launch`), it handles Docker daemon startup, image builds, network creation, and session management automatically. No manual setup steps are required -- the launcher runs preflight checks and provisions any missing infrastructure before executing the requested command.

## Usage

```bash
./launch [command] [options]
```

## Commands

### `./launch` (no arguments)

Opens an interactive numbered menu with the following options:

1. CLI mode (sandboxed)
2. CLI mode (bare, no container)
3. Desktop mode (noVNC)
4. Isolated mode (air-gapped)
5. Launch project
6. Attach shell to session
7. Run Claude inside running container

Plus build, status, and quit options. Select by number.

### `./launch cli`

Starts a sandboxed Claude CLI session with filtered network access and all 8 security layers applied. Auto-builds the CLI image if it does not exist. Auto-creates the `cage-filtered` bridge network if it does not exist. The session runs interactively in the foreground.

### `./launch cli --bare`

Runs Claude CLI directly on the host with no container. Bypasses all Docker isolation. Use this when you want speed over isolation, or when Docker is unavailable.

### `./launch desktop [port]`

Starts Desktop mode with a full X11 stack (Xvfb + openbox + x11vnc + noVNC/websockify). Runs detached. The default port is 6080. Access the desktop environment via browser at `http://localhost:<port>`.

```bash
./launch desktop          # Port 6080
./launch desktop 7080     # Custom port
```

### `./launch isolated`

Starts a CLI session with `network_mode: none`. No network access whatsoever. Resource limits are tightened to 1 CPU and 2GB RAM. Use this for air-gapped execution where no outbound connectivity is acceptable.

### `./launch project <name>`

Finds a project in the `projects/` directory and mounts it into the container at `/workspace/<name>`. If the named project is not found, displays a list of available projects.

```bash
./launch project Gently-nix
./launch project headless-ubuntu-auto
```

### `./launch <project-name>`

Shortcut form. If the command matches a directory name under `projects/`, the launcher treats it as a project launch automatically. Equivalent to `./launch project <name>`.

```bash
./launch Gently-nix       # Same as: ./launch project Gently-nix
```

### `./launch shell [session]`

Attaches a bash shell to a running session. If no session name is provided, auto-selects the most recently started session.

```bash
./launch shell                    # Attach to most recent session
./launch shell swift-fox-a1b2     # Attach to specific session
```

### `./launch claude-in [session]`

Runs `claude` inside an already-running container. Use this when you want a second Claude session in the same sandbox without starting a new container. If no session name is provided, auto-selects the most recently started session.

```bash
./launch claude-in                # Run in most recent session
./launch claude-in swift-fox-a1b2 # Run in specific session
```

### `./launch build [cli|desktop|all]`

Builds container images. Accepts a target argument:

- `cli` -- Build the CLI image only
- `desktop` -- Build the Desktop image only
- `all` -- Build both images (default when no target is specified)

```bash
./launch build            # Build all images
./launch build cli        # Build CLI image only
./launch build desktop    # Build Desktop image only
```

### `./launch stop <name|--all>`

Stops a running session by name, or stops all running sessions with `--all`.

```bash
./launch stop swift-fox-a1b2    # Stop one session
./launch stop --all             # Stop all sessions
```

### `./launch status`

Displays a system overview including:

- Docker daemon status
- Built container images
- Network configuration
- Running sessions
- Available projects in `projects/`

### `./launch teardown`

Full cleanup. Stops all running containers, then removes containers, volumes, images, and the `cage-filtered` network. Prompts for confirmation before proceeding. This is destructive and resets the environment to a clean state.

### `./launch help`

Prints a help summary with all available commands and a list of launchable projects.

## Security Flags

Every container launch (cli, desktop, isolated, project) applies the following security flags:

### Resource Limits

| Flag | Value |
|------|-------|
| `--cpus` | 2 |
| `--memory` | 4g |
| `--pids-limit` | 512 |
| `--ulimit nofile` | 1024:2048 |
| `--ulimit nproc` | 256:512 |

### Filesystem

| Flag | Value |
|------|-------|
| `--read-only` | Root filesystem is read-only |
| `--tmpfs /tmp` | 512m, rw, nosuid, nodev, noexec |
| `--tmpfs /run` | 64m, rw, nosuid, nodev, noexec |

### Capabilities

| Flag | Value |
|------|-------|
| `--cap-drop` | ALL |
| `--cap-add` | CHOWN, DAC_OVERRIDE, SETGID, SETUID |

### Security Options

| Flag | Value |
|------|-------|
| `--security-opt seccomp` | `security/seccomp-default.json` (~147 syscall allowlist, AF_VSOCK blocked) |
| `--security-opt apparmor` | `claude-cage` (applied if the AppArmor profile is loaded) |
| `--security-opt` | `no-new-privileges` |

### Network (filtered mode)

| Flag | Value |
|------|-------|
| `--network` | `cage-filtered` |
| `--dns` | `1.1.1.1` |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Passed into the container if set in the host environment. Optional for Claude Max subscribers who authenticate interactively. |
| `CAGE_DATA_DIR` | Directory for session data storage. Defaults to `~/.local/share/claude-cage`. |

## Volume Mounts

The following volumes are mounted into every container:

| Mount | Container Path | Mode | Purpose |
|-------|---------------|------|---------|
| `cage-data-<session>` | `/home/cageuser/.claude` | rw | Persistent session data (Docker named volume) |
| `CAGE_ROOT` | `/workspace/claude-cage` | rw | The claude-cage project itself |
| `universal-node.schema.json` | `/workspace/claude-cage/gentlyos/universal-node.schema.json` | ro | GentlyOS node schema |
| `templates/` | `/workspace/claude-cage/templates/` | ro | Template files |

Additional mounts are added when using the `project` command or `--mount` option:

| Mount | Container Path | Mode |
|-------|---------------|------|
| `projects/<name>` | `/workspace/<name>` | rw |

## Session Names

Session names are auto-generated in the format `<adjective>-<noun>-<hex4>`:

- **12 adjectives**: e.g., swift, calm, bold, keen, warm, cool, dark, wild, pure, wise, free, deep
- **12 nouns**: e.g., fox, owl, elk, ram, jay, bee, cod, ant, yak, emu, eel, bat
- **Hex suffix**: 4 random hexadecimal characters

Examples: `swift-fox-a1b2`, `calm-owl-f3e7`, `bold-elk-09cd`

All containers are labeled `managed-by=claude-cage` for discovery via `docker ps --filter`.

## Network Setup

The `cage-filtered` bridge network is created automatically on first use with the following configuration:

| Setting | Value |
|---------|-------|
| Driver | bridge |
| Subnet | 172.28.0.0/16 |
| Inter-container communication (ICC) | Disabled |
| IP masquerade | Enabled |

After container launch, `sandbox_apply_network_filter()` resolves the `allowed_hosts` list to IP addresses and injects iptables rules to restrict outbound traffic. Default allowed hosts:

- `api.anthropic.com`
- `cdn.anthropic.com`

## Preflight Checks

Before executing any command that requires Docker, the launcher verifies the following (in order):

1. **Docker binary exists** -- Checks that `docker` is available on PATH.
2. **Docker daemon running** -- Queries the daemon. If not running, attempts to start it via `systemctl start docker`.
3. **cage-filtered network exists** -- Checks for the bridge network. Creates it with the configuration above if missing.
4. **Container image exists** -- Checks for the required image (`cli` or `desktop`). Triggers an automatic build if the image is not found.
5. **Data directories exist** -- Ensures `CAGE_DATA_DIR` and session metadata directories are present. Creates them if missing.

All checks are silent on success. Failures print a diagnostic message and exit with a non-zero status.

## Examples

### First run from scratch

Nothing is built yet. The launcher handles everything:

```bash
./launch cli
```

This will: verify Docker is running (start it if not), create the `cage-filtered` network, build the CLI image, generate a session name, create the data volume, and drop you into an interactive Claude CLI session.

### Daily development workflow

Start a sandboxed session and begin working:

```bash
./launch cli
```

Check what is running:

```bash
./launch status
```

When done:

```bash
./launch stop --all
```

### Working on Gently-nix

Launch directly into the project:

```bash
./launch Gently-nix
```

This mounts `projects/Gently-nix` into the container at `/workspace/Gently-nix` alongside the main claude-cage workspace.

### Running multiple sessions

Start a sandboxed CLI session:

```bash
./launch cli
```

In another terminal, start a second session for a project:

```bash
./launch project headless-ubuntu-auto
```

List all running sessions:

```bash
./launch status
```

Attach a shell to a specific session:

```bash
./launch shell swift-fox-a1b2
```

### Attaching Claude to a running container

If you already have a sandboxed session running and want a second Claude instance inside it:

```bash
./launch claude-in swift-fox-a1b2
```

This runs `claude` inside the existing container without starting a new one. Both sessions share the same filesystem, network restrictions, and resource limits.
