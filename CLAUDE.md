# CLAUDE.md — GentlyOS

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## What Is This

**GentlyOS** — a sovereignty-first operating system built in Rust. This repository contains the complete system: 28 application crates (~80K LOC), a containerized sandbox infrastructure (claude-cage), a web dashboard, Move smart contracts on Sui, and a Python orchestration engine.

**claude-cage** is the infrastructure layer — the dockerized sandbox that runs Claude CLI/Desktop in isolated containers with 8-layer defense-in-depth security. It is the installation guide, the LLM interface, and the container design that GentlyOS is built upon.

## Repository Structure

```
claude-cage/                     ← Root: infrastructure + orchestration
├── gentlyos-core/               ← APPLICATION: 28 Rust crates, ~80K LOC
│   ├── crates/                  ← All gently-* crates (the real code)
│   ├── gently-cli/              ← Main CLI binary (21 commands)
│   ├── gentlyos-tui/            ← Terminal UI (6 panels, 7 LLM providers)
│   ├── DEV_DOCS/                ← Specs, gap analysis, build steps
│   ├── SYNTHESTASIA/            ← Design documents, protocols
│   └── Cargo.toml               ← GentlyOS workspace (28 crates)
│
├── cage-web/                    ← DASHBOARD: Axum + HTMX SSR (28K LOC)
│   └── src/                     ← Routes, CODIE parser, subprocess wrappers
│
├── lib/                         ← BASH CLI: 15 library modules
├── bin/claude-cage              ← CLI entry point (sources all lib/)
├── docker/                      ← Dockerfiles (cli, desktop, gentlyos)
├── security/                    ← Seccomp + AppArmor profiles
├── mongodb/                     ← Fire-and-forget event store (Node.js)
├── ptc/                         ← Python Tree Compiler (12 modules, 8 phases)
├── gentlyos/                    ← Organizational tree (35 agents, schema)
├── sui/cage_nft/                ← Move smart contracts (NFT, Collection, SYNTH)
├── codie-maps/                  ← 9 CODIE orchestration programs
├── config/                      ← Default YAML configuration
├── .claude/                     ← Agents, commands, hooks, skills
├── Cargo.toml                   ← Root workspace (cage-web only)
└── docker-compose.yml           ← Container services + security anchor
```

## Two Workspaces

| Workspace | Path | Members | Purpose |
|-----------|------|---------|---------|
| **Root** | `Cargo.toml` | `cage-web` | Infrastructure dashboard |
| **GentlyOS** | `gentlyos-core/Cargo.toml` | 28 `gently-*` crates + CLI + TUI | Application layer |

These are separate Cargo workspaces with independent dependency trees. GentlyOS is excluded from the root workspace.

## GentlyOS Application Crates (28)

All real code lives in `gentlyos-core/crates/`. Organized by domain:

### Alexandria (Knowledge)
| Crate | LOC | Status | Purpose |
|-------|-----|--------|---------|
| `gently-alexandria` | 4,949 | 85% | Knowledge graph, Tesseract 8D embedding, wormhole sync |
| `gently-search` | 1,800 | 80% | Alexandria routing, BBBCP constraints, collapse engine |
| `gently-inference` | 4,609 | 90% | Quality mining, step decomposition, Three Kings provenance |
| `gently-feed` | 1,200 | 70% | Living feed, charge/decay, bridge documents |
| `gently-codie` | 7,505 | 80% | 44-keyword instruction language, lexer, parser, compression |

### BS-Artisan (Craftsmanship)
| Crate | LOC | Status | Purpose |
|-------|-----|--------|---------|
| `gently-artisan` | 1,409 | 90% | Toroidal storage (r=tokens/2pi), Foam, BARF retrieval |
| `gently-core` | 2,954 | 98% | Crypto primitives, genesis keys, XOR splits, Berlin Clock |
| `gently-ipfs` | 1,700 | 85% | Content-addressed storage, Sui bridge |
| `gently-chain` | 764 | 40% | Sui/Move SDK: client, objects, PTB, events, Three Kings |
| `gently-ptc` | 1,084 | 70% | PTC Brain: tree decompose, execute, aggregate, 7 phases |
| `gently-architect` | 1,500 | 55% | Code generation, SQLite knowledge base |
| `gently-brain` | 2,100 | 75% | LLM orchestration, Claude API, Alexandria integration |
| `gently-mcp` | 1,100 | 50% | MCP server scaffolding |
| `gently-micro` | 1,100 | — | Microcontroller interface (ESP32/Arduino) |

### FAFO (Defense)
| Crate | LOC | Status | Purpose |
|-------|-----|--------|---------|
| `gently-security` | 8,576 | 95% | 16 daemons, FAFO pitbull (6 escalation levels), threat intel |
| `gently-sandbox` | 794 | 60% | Seccomp, AppArmor, capabilities, FAFO violation escalation |
| `gently-guardian` | 1,400 | 80% | Hardware detection, cross-platform (sysinfo) |
| `gently-cipher` | 1,500 | 50% | Crypto ciphers, password analysis |
| `gently-network` | 1,600 | 60% | Network capture, MITM, visualization |
| `gently-sploit` | — | 20% | Exploitation framework (skeleton only) |

### GOO (GUI — unsettled, post-consolidation)
| Crate | LOC | Status | Purpose |
|-------|-----|--------|---------|
| `gently-goo` | 3,379 | 80% | GOO unified field: SDF, attention, learning (70+ tests) |
| `gently-visual` | 1,200 | 100% | SVG pattern generation |
| `gently-audio` | 1,800 | 100% | FFT encoding/decoding, DSP |
| `gently-dance` | 2,100 | 85% | P2P dance protocol state machine |
| `gently-web` | 1,200 | 85% | ONE SCENE HTMX GUI, Alexandria routes |

### Other
| Crate | LOC | Status | Purpose |
|-------|-----|--------|---------|
| `gently-btc` | 1,600 | 90% | Bitcoin RPC, block anchoring |
| `gently-gateway` | 1,400 | 70% | API routing, pipeline architecture |
| `gently-sim` | 1,800 | 80% | SIM card security: filesystem, applets, OTA |

### Binaries
| Binary | Path | Purpose |
|--------|------|---------|
| `gently-cli` | `gentlyos-core/gently-cli/` | Main CLI (4000+ LOC, 21 commands) |
| `gentlyos-tui` | `gentlyos-core/gentlyos-tui/` | Terminal UI (6 panels, BONEBLOB pipeline) |

## Build & Run

### GentlyOS Application
```bash
cd gentlyos-core
cargo build --release -p gently-cli    # Main CLI binary
cargo build --release                   # All 28 crates
cargo test --workspace                  # Run all tests
cargo test -p gently-security --lib     # Security tests (56 pass)
cargo test -p gently-goo --lib          # GOO field tests (70+ pass)
```

### Cage Dashboard
```bash
make build-web        # Compile cage-web binary
make web-rs           # Start dashboard at http://localhost:5000
```

### Container Images
```bash
make build            # Build CLI + Desktop images
make build-cli        # CLI image only
make build-desktop    # Desktop image only
make build-gently     # GentlyOS + IPFS image

make run-cli          # Interactive Claude CLI session
make run-desktop      # Desktop (noVNC at localhost:6080)
make run-isolated     # CLI with no network
make run-gently       # GentlyOS + IPFS (detached)
```

### Operations
```bash
make stop             # Stop all containers
make status           # Show running cage containers
make logs             # Follow container logs
make verify-sandbox   # Inspect container security settings
make load-apparmor    # Load AppArmor profile (requires sudo)
make install          # Symlink bin/claude-cage to /usr/local/bin
```

### MongoDB
```bash
make mongo-install    # npm install in mongodb/
make mongo-ping       # Test Atlas connectivity
make mongo-status     # Show event/artifact counts
make mongo-seed       # Seed artifacts into MongoDB
make gentlyos-seed    # Seed tree + docs into MongoDB
```

### Sui/Move
```bash
cd sui/cage_nft
sui move build        # Compile Move contracts
sui move test         # Run Move tests
```

## Infrastructure Layer (claude-cage)

### Bash CLI (`lib/`)

`bin/claude-cage` sources all 15 modules, dispatches via `cmd_*()` in `lib/cli.sh`.

| Module | Responsibility |
|--------|---------------|
| `cli.sh` | Command dispatch, all `cmd_*()` functions |
| `docker.sh` | Docker build, run, stop, destroy, exec |
| `sandbox.sh` | Security flags, filtered network, iptables rules |
| `session.sh` | Session metadata, name generation (`adjective-noun-hex4`) |
| `config.sh` | YAML config loading (default + user override) |
| `mongodb.sh` | Fire-and-forget MongoDB writes (backgrounded) |
| `memory.sh` | Session context compaction (Anthropic cookbook pattern) |
| `observability.sh` | Container metrics, health checks, dashboards |
| `lifecycle.sh` | Session reaping, garbage collection |
| `tree.sh` | GentlyOS tree operations |
| `architect.sh` | Architecture embedding, git integration |
| `docs.sh` | Documentation generation |
| `integrations.sh` | External APIs (HuggingFace, Porkbun, NounProject) |
| `tui.sh` | Terminal UI components |
| `gui.sh` | Interactive dashboard |

### Security Model (8 layers)

1. **Read-only root filesystem** with tmpfs at `/tmp` (512m) and `/run` (64m)
2. **Capabilities**: ALL dropped, only CHOWN/DAC_OVERRIDE/SETGID/SETUID re-added
3. **Seccomp** (`security/seccomp-default.json`): ~147 syscall allowlist
4. **AppArmor** (`security/apparmor-profile`): denies mount/ptrace/raw-network
5. **Resource limits**: 2 CPUs, 4GB memory, 512 PIDs
6. **Network filtering**: iptables post-launch, restrict to `allowed_hosts` only
7. **no-new-privileges** flag
8. **Bridge network** (`cage-filtered`) with ICC disabled

### Docker Compose Services

`docker-compose.yml` defines services sharing an `x-common` security anchor:
- `cli` — interactive TTY with filtered network
- `desktop` — detached with noVNC (ports 6080, 5900)
- `cli-isolated` — network_mode: none
- `gently` — GentlyOS application (ports 3000, 8080)
- `ipfs` — IPFS Kubo daemon (ports 4001, 5001, 8081)

### Dashboard (`cage-web/`)

Rust (axum 0.8) + HTMX 2.0 + askama templates. 28K LOC, 22 route modules.

Key routes: `/` (dashboard), `/sessions` (manage), `/tree` (GentlyOS hierarchy), `/codie` (programs), `/api/health` (JSON status).

Subprocess wrappers shell out to docker CLI, `node store.js` (MongoDB), and `python3 -m ptc.engine`. The CODIE parser is native Rust.

### MongoDB Store (`mongodb/`)

Fire-and-forget key/value store backed by MongoDB Atlas. Node.js + native `mongodb` driver.

- `store.js` — CLI: put, log, get, search, aggregate, bulk, stats, ping
- `seed-artifacts.js` — Batch-loads project artifacts
- `.env` — MONGODB_URI (never committed)

### Configuration

`config/default.yaml` holds all defaults. User overrides: `~/.config/claude-cage/config.yaml`. Loaded into `CAGE_CFG[]` associative array.

## GentlyOS Organizational Tree

35-agent virtual organization following Google monorepo coordination patterns.

**Files:**
- `gentlyos/tree.json` — Full tree (3 Executives + 8 Directors + 24 Captains)
- `gentlyos/universal-node.schema.json` — Schema every node follows
- `gentlyos/seed.js` — Seeds tree + docs into MongoDB

**Tree of Life mapping:**
| Sephira | Department | Crate Domain |
|---------|-----------|-------------|
| Keter | Interface | gently-web, gently-mcp |
| Chokmah/Binah | Protocol | gently-alexandria, gently-search |
| Daath | Security | gently-security, gently-sandbox |
| Chesed/Gevurah | DevOps | Docker, bash CLI, cage-web |
| Tiferet | Orchestration | gently-ptc, gently-codie |
| Netzach/Hod | Runtime | gently-brain, gently-gateway |
| Yesod | Tokenomics | gently-chain, gently-inference |
| Malkuth | Foundation | gently-core, gently-artisan |

**Coordination:** 8 phases (INTAKE → TRIAGE → PLAN → REVIEW → EXECUTE → VERIFY → INTEGRATE → SHIP)

**Approval cascade:** Risk 1-3: Captain | 4-6: Director | 7-8: CTO | 9-10: Human

## PTC Engine (`ptc/`)

Python Tree Compiler — the orchestration nervous system. 12 modules, 8-phase pipeline.

| Module | Purpose |
|--------|---------|
| `engine.py` | 8-phase coordination (INTAKE through SHIP) |
| `executor.py` | 7 execution modes (native, claude, shell, codie, design, inspect, compose) |
| `crate_graph.py` | Dependency analysis + blast radius calculation |
| `architect.py` | Blueprint system (cache-first design) |
| `docs.py` | Circular documentation with staleness tracking |
| `embeddings.py` | Vector embeddings |
| `federation.py` | Inter-agent messaging |
| `git_ops.py` | Git operations |
| `huggingface.py` | HuggingFace API integration |
| `ipfs.py` | IPFS operations |
| `lora.py` | LoRA fine-tuning |
| `porkbun.py` | Domain registration |

## Sui/Move Contracts (`sui/cage_nft/`)

Move smart contracts on Sui (linear types, object-centric).

| Module | Purpose |
|--------|---------|
| `nft.move` | Object-based NFT with mint/burn/transfer |
| `collection.move` | Shared object NFT collection with cap-gated minting |
| `synth_token.move` | SYNTH coin (Proof-of-Reasoning token) |

**Three Kings Provenance:** Gold (WHO), Myrrh (WHAT), Frankincense (WHY) — blake3 hashes published as `ReasoningStep` Move resources when quality >= 0.7.

## Claude Integration (`.claude/`)

### Agents
| Agent | Description |
|-------|-------------|
| `gentlyos-orchestrator` | Reads tree.json, routes tasks to correct node |
| `session-manager` | Container session lifecycle |
| `security-auditor` | 8-layer security audit |
| `mongo-analyst` | Query MongoDB store |

### Slash Commands
| Command | Description |
|---------|-------------|
| `/atlas` | MongoDB Atlas management |
| `/session` | Session lifecycle |
| `/mongo` | Query MongoDB store |
| `/build` | Build container images |
| `/status` | System overview |
| `/security-audit` | Security audit |
| `/gentlyos` | Tree routing, blast radius |
| `/mia` | Encrypted secrets manager |
| `/route` | Route intent through tree |

### Hooks (PostToolUse)
| Hook | Trigger | Effect |
|------|---------|--------|
| `command-logger.py` | Bash | Logs commands to MongoDB + audit/ |
| `session-tracker.py` | Bash, Write | Tracks container lifecycle |

## Key Implementation Notes

- Session names: `<adjective>-<noun>-<hex4>` (e.g., "swift-fox-a1b2")
- Containers labeled `managed-by=claude-cage` for discovery
- Network filtering happens *after* container launch (resolves hosts, injects iptables)
- `docker-compose.yml` and `lib/docker.sh` construct equivalent security flags independently
- All MongoDB writes are fire-and-forget (backgrounded, never block CLI)
- GentlyOS crate compilation requires the `gentlyos-core/` workspace, not root

## Import Bucket (`.import-bucket/`)

Transient staging area for external imports. Contents gitignored, only README tracked. Drop in, process, clean up.

## Subproject: headless-ubuntu-auto

`projects/headless-ubuntu-auto/` — headless GPU server provisioning (2x RTX 3090 Ti). Independent Makefile and docs.

## No Test Suite

No global test suite or linting config. Verification:
- `make verify-sandbox` — inspect container security
- `cargo test -p gently-security` — 56 security tests
- `cargo test -p gently-goo` — 70+ GOO field tests
- `sui move test` — Move contract tests
