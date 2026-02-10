# claude-cage

**The development sandbox for GentlyOS.** One project. One tree. One launcher.

claude-cage (aka **gently-dev**) is a dockerized sandbox for running Claude CLI and Claude Desktop in isolated containers with defense-in-depth security. It ships WITH GentlyOS and is the tooling layer that builds, orchestrates, and self-hosts the 38-crate Rust workspace.

## Quick Start

```bash
git clone git@github.com:Zero2oneZ/claude-cage.git
cd claude-cage
./launch              # Interactive menu
./launch cli          # Sandboxed Claude CLI
./launch cli --bare   # Claude CLI on host (no container)
./launch Gently-nix   # Claude CLI with GentlyOS workspace mounted
```

## What's Inside

```
claude-cage/
├── launch                      # Unified launcher script
├── bin/claude-cage             # Full CLI tool (15 bash modules)
├── gentlyos/
│   ├── tree.json               # 35-agent PTC tree (3 Exec + 8 Dir + 24 Capt)
│   ├── crate-graph.json        # 38-crate dependency graph (7 tiers)
│   └── universal-node.schema.json
├── ptc/
│   ├── engine.py               # Pass-Through Coordination (8 phases)
│   ├── executor.py             # Leaf executor (7 modes: native/claude/shell/codie/...)
│   ├── crate_graph.py          # Dependency graph: blast_radius, build_order
│   ├── docs.py                 # Circular Documentation System
│   └── architect.py            # Blueprint system
├── projects/
│   └── Gently-nix/             # The 38-crate Rust workspace
├── docker/
│   ├── cli/Dockerfile          # node:20-slim + claude-code
│   └── desktop/Dockerfile      # ubuntu:24.04 + Xvfb + noVNC
├── security/
│   ├── seccomp-default.json    # 147-syscall allowlist
│   └── apparmor-profile        # MAC confinement
├── lib/                        # 15 bash modules (4,900+ lines)
├── cage-web/                   # Rust (axum) + HTMX dashboard
├── mongodb/                    # Fire-and-forget event store
├── config/default.yaml         # All configuration defaults
└── docs/                       # Generated docs (50 JSON artifacts + design .docx)
```

## Launcher Reference

| Command | Description |
|---------|-------------|
| `./launch` | Interactive numbered menu |
| `./launch cli` | Sandboxed Claude CLI (filtered network, 8 security layers) |
| `./launch cli --bare` | Claude CLI directly on host |
| `./launch desktop` | Desktop mode (noVNC at localhost:6080) |
| `./launch isolated` | CLI with zero network access |
| `./launch project <name>` | CLI with a project directory mounted |
| `./launch <project-name>` | Shortcut (e.g. `./launch Gently-nix`) |
| `./launch shell [session]` | Bash into a running container |
| `./launch claude-in [session]` | Run `claude` inside a running container |
| `./launch build [cli\|desktop\|all]` | Build container images |
| `./launch stop [name\|--all]` | Stop session(s) |
| `./launch status` | System status overview |
| `./launch teardown` | Full cleanup (containers + volumes + images) |

See [docs/LAUNCHER.md](docs/LAUNCHER.md) for full reference.

## Architecture

### The Consolidated System

GentlyOS is a 38-crate Rust workspace organized in 7 tiers. claude-cage is the development tooling that builds and orchestrates it. They are one project:

```
                    ┌─────────────────────────┐
                    │     Human Architect      │
                    └────────────┬────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │       CTO Agent          │
                    └────────────┬────────────┘
                                 │
        ┌────────┬────────┬──────┴──────┬────────┬────────┐
        │        │        │             │        │        │
   Foundation Protocol Orchestration Runtime Security  DevOps
   (tier 0)  (tier 1-5) (tier 0,3,6) (tier 3) (tier 4) (tier 6)
        │        │        │             │        │        │
      Types    Wire    CODIE         Exec    Audit    Build
      Crypto   P2P     PTC           State   Harden   Release
      Errors   Alex    Context       Memory  Incident Infra
```

### Crate Dependency Graph (7 Tiers)

| Tier | Name | Crates | CODIE Keyword |
|------|------|--------|---------------|
| 0 | Foundation | gently-core, gently-codie, gently-artisan, +4 | `bone` |
| 1 | Knowledge (core) | gently-feed, gently-btc | `blob` |
| 2 | Knowledge (graph) | gently-alexandria, gently-search, gently-ipfs | `blob` |
| 3 | Intelligence | gently-brain, gently-agents, gently-mcp, +4 | `cali` |
| 4 | Security | gently-cipher, gently-guardian, gently-security, +2 | `fence` |
| 5 | Network | gently-network, gently-bridge, gently-gateway, +2 | `bark` |
| 6 | Application | gently-web, gently-cli, gently-gooey, +5 | `biz` |

Changing `gently-core` (tier 0) triggers a blast radius of 38/38 crates across all 7 tiers. See [docs/CRATE-GRAPH.md](docs/CRATE-GRAPH.md).

### PTC Engine (Pass-Through Coordination)

Intent enters at root. Decomposes DOWN through the tree. Leaves EXECUTE. Results aggregate UP.

**8 Phases:** INTAKE -> TRIAGE -> PLAN -> REVIEW -> EXECUTE -> VERIFY -> INTEGRATE -> SHIP

**Execution Modes:** native (cargo/nix), claude, shell, codie, design, inspect, compose, plan

**Approval Cascade:** Risk 1-3 auto-approved | 4-6 logged | 7-8 CTO approval | 9-10 human required

See [docs/PTC.md](docs/PTC.md).

### Security Model (8 Layers)

| Layer | Mechanism |
|-------|-----------|
| 1 | Read-only root filesystem + tmpfs |
| 2 | All capabilities dropped, 4 re-added |
| 3 | Seccomp: 147-syscall allowlist |
| 4 | AppArmor: deny mount/ptrace/raw-network |
| 5 | Resource limits: 2 CPU, 4GB RAM, 512 PIDs |
| 6 | Network filtering: iptables allowlist |
| 7 | no-new-privileges flag |
| 8 | Bridge network, inter-container comms disabled |

See [docs/SECURITY.md](docs/SECURITY.md).

### Documentation Circle

Documentation is not dead text. It's a living bidirectional graph tracked by file hash. Change code, docs flag themselves stale. 50 node artifacts, triple-stored (MongoDB + IPFS + local JSON).

```bash
./launch cli --bare    # then inside claude-cage:
make docs-status       # Coverage + staleness
make docs-check        # Find stale docs
make docs-refresh      # Regenerate stale docs
make docs-interconnect # Rebuild the circle
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Projects

| Project | Description |
|---------|-------------|
| `Gently-nix` | The 38-crate GentlyOS Rust workspace |
| `headless-ubuntu-auto` | GPU server provisioning (2x RTX 3090) |
| `test-apps` | Test applications (JS) |
| `test-apps-rust` | Test applications (Rust) |

## Configuration

Default config at `config/default.yaml`. User overrides at `~/.config/claude-cage/config.yaml`.

| Option | Default | Description |
|--------|---------|-------------|
| `mode` | `cli` | `cli` or `desktop` |
| `network` | `filtered` | `none`, `filtered`, or `host` |
| `execution_mode` | `docker` | `docker`, `native`, or `hybrid` |
| `cpus` | `2` | CPU core limit |
| `memory` | `4g` | Memory limit |
| `allowed_hosts` | `api.anthropic.com,cdn.anthropic.com` | Network allowlist |

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Full system architecture |
| [LAUNCHER.md](docs/LAUNCHER.md) | Launcher reference |
| [CRATE-GRAPH.md](docs/CRATE-GRAPH.md) | Crate dependency graph |
| [SECURITY.md](docs/SECURITY.md) | Security model (8 layers) |
| [PTC.md](docs/PTC.md) | PTC engine (8 phases) |
| [CLAUDE.md](CLAUDE.md) | Claude Code project instructions |

## License

MIT
