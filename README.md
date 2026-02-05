# claude-cage

Dockerized sandbox for running **Claude CLI** and **Claude Desktop** securely in isolated containers.

## What is this?

claude-cage wraps Claude Code (Anthropic's CLI) inside Docker containers with defense-in-depth security:

- **Filesystem isolation** — read-only root, bind-mounted workspaces only
- **Network filtering** — only Anthropic API endpoints reachable by default
- **Resource limits** — CPU, memory, PID caps to prevent runaway processes
- **Capability dropping** — all Linux capabilities dropped; minimal set re-added
- **Seccomp filtering** — syscall allowlist blocks dangerous kernel interfaces
- **AppArmor confinement** — mandatory access control profile (optional)
- **Non-root execution** — container processes run as unprivileged user
- **Session management** — named sessions with lifecycle control

Two modes are available:

| Mode | Description | Access |
|------|-------------|--------|
| `cli` | Interactive terminal | Direct TTY attach |
| `desktop` | GUI via Xvfb + noVNC | Browser at `http://localhost:6080` |

## Quick Start

### Prerequisites

- Docker Engine 20.10+
- Docker Compose v2
- `ANTHROPIC_API_KEY` environment variable set

### Using Docker Compose (simplest)

```bash
# Clone
git clone https://github.com/Zero2oneZ/claude-cage.git
cd claude-cage

# Build images
make build

# Run Claude CLI (interactive)
ANTHROPIC_API_KEY=sk-ant-... make run-cli

# Run Claude Desktop (browser)
ANTHROPIC_API_KEY=sk-ant-... make run-desktop
# Open http://localhost:6080
```

### Using the CLI tool

```bash
# Install
make install

# Start a CLI session with current directory mounted
claude-cage start --mode cli --mount . --api-key sk-ant-...

# Start Desktop accessible via browser
claude-cage start --mode desktop --port 6080:6080

# List sessions
claude-cage list

# Attach to a session
claude-cage shell --name <session-name>

# Stop
claude-cage stop --name <session-name>

# Destroy (removes container + data)
claude-cage destroy --name <session-name>
```

## Architecture

```
claude-cage/
├── bin/claude-cage              # CLI entrypoint
├── lib/
│   ├── cli.sh                   # Command parsing & dispatch
│   ├── docker.sh                # Docker lifecycle management
│   ├── sandbox.sh               # Security policy enforcement
│   ├── config.sh                # Configuration loading
│   └── session.sh               # Session state management
├── docker/
│   ├── cli/Dockerfile           # Claude CLI container
│   └── desktop/
│       ├── Dockerfile           # Claude Desktop container
│       ├── entrypoint-desktop.sh
│       └── openbox-rc.xml
├── security/
│   ├── seccomp-default.json     # Syscall allowlist
│   └── apparmor-profile         # MAC profile
├── config/default.yaml          # Default settings
├── docker-compose.yml           # Compose orchestration
└── Makefile                     # Build/run/install helpers
```

## Security Model

### Layers of Defense

```
┌─────────────────────────────────────────────┐
│  Host OS                                    │
│  ┌───────────────────────────────────────┐  │
│  │  Docker Engine                        │  │
│  │  ┌─────────────────────────────────┐  │  │
│  │  │  Network Filter (iptables)      │  │  │
│  │  │  ┌───────────────────────────┐  │  │  │
│  │  │  │  Seccomp (syscall filter) │  │  │  │
│  │  │  │  ┌─────────────────────┐  │  │  │  │
│  │  │  │  │  AppArmor (MAC)     │  │  │  │  │
│  │  │  │  │  ┌───────────────┐  │  │  │  │  │
│  │  │  │  │  │  Capabilities │  │  │  │  │  │
│  │  │  │  │  │  (dropped)    │  │  │  │  │  │
│  │  │  │  │  │  ┌─────────┐  │  │  │  │  │  │
│  │  │  │  │  │  │ Non-root│  │  │  │  │  │  │
│  │  │  │  │  │  │ user    │  │  │  │  │  │  │
│  │  │  │  │  │  │ ┌─────┐ │  │  │  │  │  │  │
│  │  │  │  │  │  │ │Claude│ │  │  │  │  │  │  │
│  │  │  │  │  │  │ └─────┘ │  │  │  │  │  │  │
│  │  │  │  │  │  └─────────┘  │  │  │  │  │  │
│  │  │  │  │  └───────────────┘  │  │  │  │  │
│  │  │  │  └─────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────┘  │  │  │
│  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### What's blocked

| Attack Surface | Mitigation |
|---|---|
| Arbitrary file access | Read-only root + bind mounts only |
| Network exfiltration | Filtered network — only `api.anthropic.com` allowed |
| Privilege escalation | `no-new-privileges`, all caps dropped |
| Kernel exploits | Seccomp syscall allowlist |
| Container escape | AppArmor, no mount/ptrace/raw-network |
| Resource exhaustion | CPU, memory, PID limits |
| Process snooping | Inter-container communication disabled |

### Network Policies

| Policy | Behavior |
|--------|----------|
| `none` | No network at all — fully air-gapped |
| `filtered` | Only `allowed_hosts` reachable (default: Anthropic API) |
| `host` | Full host network (not recommended for untrusted workloads) |

## Configuration

Default config at `config/default.yaml`. User overrides at `~/.config/claude-cage/config.yaml`.

```yaml
# Session defaults
mode: cli
persist: true
max_sessions: 5

# Resource limits
cpus: 2
memory: 4g

# Network
network: filtered
dns: 1.1.1.1
allowed_hosts: api.anthropic.com,cdn.anthropic.com

# Security
read_only_root: true
seccomp_profile: default
```

Key options:

| Option | Default | Description |
|--------|---------|-------------|
| `mode` | `cli` | `cli` or `desktop` |
| `network` | `filtered` | `none`, `filtered`, or `host` |
| `cpus` | `2` | CPU core limit |
| `memory` | `4g` | Memory limit |
| `allowed_hosts` | `api.anthropic.com` | Comma-separated allowlist for filtered mode |
| `read_only_root` | `true` | Mount root filesystem as read-only |

## CLI Reference

```
claude-cage <command> [options]

Commands:
  start       Launch a new sandboxed Claude session
  stop        Stop a running session
  shell       Attach a shell to a running session
  status      Inspect a session
  logs        Stream container logs
  list        List all sessions
  destroy     Remove a session and its data
  build       Build container images
  config      Show/validate configuration
  version     Print version
  help        Show help
```

### Examples

```bash
# Fully isolated (no network, ephemeral)
claude-cage start --mode cli --network none --ephemeral --mount ./project

# Resource-constrained
claude-cage start --mode cli --cpus 1 --memory 2g

# Desktop with custom resolution (set VNC_RESOLUTION in env)
claude-cage start --mode desktop --env VNC_RESOLUTION=2560x1440

# Pass extra environment variables
claude-cage start --mode cli --env MY_VAR=value --env OTHER=123
```

## Development

```bash
# Build images
make build

# Run tests (verify sandbox)
make verify-sandbox

# Load AppArmor profile
sudo make load-apparmor

# Full cleanup
make clean-all
```

## License

MIT
