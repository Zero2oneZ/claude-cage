# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Is This

claude-cage is a dockerized sandbox for running Claude CLI and Claude Desktop in isolated containers with defense-in-depth security. Two modes: CLI (interactive TTY) and Desktop (Xvfb + noVNC in browser at localhost:6080).

**Core Principle: One pattern. Every scale. Same shape.** claude-cage itself is a tree. Every project you create with it is a tree. The universal node schema (`universal-node.schema.json`) is the foundational infrastructure primitive — not a feature of any sub-project, but THE pattern everything builds on.

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

# MongoDB
make mongo-install    # Install MongoDB store dependencies (npm)
make mongo-ping       # Test MongoDB Atlas connectivity
make mongo-status     # Show event/artifact counts

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
claude-cage init <dir> [--name n] # Initialize new project with tree infrastructure
claude-cage tree show [tree.json] # Show tree hierarchy
claude-cage tree node <path> <id> # Get node details as JSON
claude-cage tree blast-radius <path> <targets>  # Calculate blast radius
claude-cage tree route <path> <intent>          # Route intent through tree
claude-cage web                   # Launch web dashboard (localhost:5000)
claude-cage observe               # Show observability dashboard
```

## Architecture

### Bash Library Architecture (`lib/`)

The CLI is a modular bash application. `bin/claude-cage` sources all library modules, then dispatches commands via `cmd_<command>()` functions in `lib/cli.sh`.

| Module | Responsibility |
|---|---|
| `lib/cli.sh` | Command parsing, argument handling, all `cmd_*()` functions |
| `lib/docker.sh` | Docker build, run, stop, destroy, exec, inspect |
| `lib/sandbox.sh` | Constructs security flags, creates filtered network, applies iptables rules, verifies sandbox |
| `lib/session.sh` | Session metadata (create/list/status/remove), name generation (`adjective-noun-hex4`) |
| `lib/config.sh` | YAML config loading (default + user override at `~/.config/claude-cage/config.yaml`), validation |
| `lib/tree.sh` | Universal tree operations: load, show, node, blast-radius, route, init, add-node, seed |
| `lib/mongodb.sh` | MongoDB fire-and-forget storage: key/value writes, event logging, artifact storage |

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

### MongoDB Store (`mongodb/`)

Fire-and-forget key/value store backed by MongoDB Atlas. All CLI events, session lifecycle, docker operations, and artifacts are logged asynchronously — writes never block the CLI.

**Stack:** Node.js + `mongodb` driver (native, no ODM overhead)

**Files:**
- `mongodb/store.js` — CLI: `put`, `log`, `get`, `search`, `aggregate`, `bulk`, `distinct`, `stats`, `ping`, `count`
- `mongodb/seed-artifacts.js` — Batch-loads all project artifacts into MongoDB
- `mongodb/package.json` — just the `mongodb` driver
- `mongodb/.env` — `MONGODB_URI`, `MONGODB_DB`, `MONGODB_CLUSTER0_ADMIN` (never committed)

**Bash wrapper** (`lib/mongodb.sh`):
- `mongo_init` — sources `.env`, checks node/deps, sets `MONGO_READY`
- `mongo_put <collection> <json>` — fire-and-forget insert (backgrounded)
- `mongo_log <type> <key> [value]` — structured event to `events` collection
- `mongo_get <collection> [query] [limit]` — synchronous query
- `mongo_log_session <event> <name> [meta]` — session lifecycle events
- `mongo_log_command <cmd> [args...]` — CLI command logging
- `mongo_store_artifact <name> <type> <content>` — store code/config/output
- `mongo_log_project <project> <type> <key> [value]` — per-project tagging

**Collections:**
- `events` — all structured events (type, key, value, _ts, _host, _project)
- `artifacts` — code, configs, outputs (name, type, content, project)
- Custom collections via `mongo_put`

**Integration points** (all fire-and-forget, zero blocking):
- `lib/cli.sh` — logs every command dispatch (start, stop, destroy, build)
- `lib/session.sh` — logs session create, status change, remove
- `lib/docker.sh` — logs container run, stop, destroy, build

```bash
make mongo-install   # npm install in mongodb/
make mongo-ping      # test Atlas connectivity
make mongo-status    # show event/artifact counts
make mongo-search Q="search text"  # search artifacts
make mongo-stats     # full collection statistics
make mongo-events N=20             # recent events
```

### Session Memory (`lib/memory.sh`)

Background session context compaction following Anthropic cookbook patterns. Stores compacted session summaries in MongoDB for cross-session learning.

- `memory_init` — ensure memory directory exists
- `memory_save <session> <context_json>` — persist to disk + MongoDB
- `memory_load <session>` — retrieve session memory
- `memory_compact <session>` — summarize and compact session history
- `memory_list` — show all saved memories
- `memory_clean [days]` — remove old memories (default: 30 days)
- `memory_search <pattern>` — find sessions by content

### Observability (`lib/observability.sh`)

Container metrics, health checks, and operational dashboards.

- `obs_snapshot <session>` — capture current container metrics to MongoDB
- `obs_health <session>` — quick health check (healthy/degraded/unhealthy)
- `obs_dashboard` — show metrics for all running sessions
- `obs_log_timing <operation> <start_epoch>` — log operation duration
- `obs_events_summary` — aggregate event stats from MongoDB

```bash
make observe         # observability dashboard
make health          # health check for all sessions
claude-cage observe  # same via CLI
claude-cage health <session>
```

### Atlas CLI Integration

MongoDB Atlas infrastructure is managed via the `atlas` CLI (v1.35.0, installed at `~/bin/atlas`).

**Slash command:** `/atlas <subcommand>` — defined in `.claude/commands/atlas.md`
**Skill:** `.claude/skills/atlas-cli/SKILL.md` — auto-activates on Atlas/MongoDB topics

Common operations:
```bash
/atlas login            # Authenticate with Atlas
/atlas whitelist-add    # Add current IP to access list
/atlas clusters         # List clusters
/atlas ping             # Full connectivity test (auth + IP + driver)
/atlas setup            # Guided first-time setup
```

### Subagents (`.claude/agents/`)

Specialized agents for delegation via the Task tool (cookbook pattern: markdown with YAML frontmatter).

| Agent | Description | Tools |
|-------|-------------|-------|
| `session-manager` | Manages container sessions — start, stop, inspect, troubleshoot | Bash, Read, Grep |
| `security-auditor` | 8-layer security audit — seccomp, AppArmor, caps, rootfs, limits | Bash, Read, Grep, Glob |
| `mongo-analyst` | Queries MongoDB store — events, artifacts, analytics | Bash, Read |

Usage: Claude automatically delegates via the Task tool when the user's request matches the agent description.

### Slash Commands (`.claude/commands/`)

| Command | Description |
|---------|-------------|
| `/atlas <cmd>` | MongoDB Atlas management (login, whitelist, clusters, ping) |
| `/session <cmd>` | Session lifecycle (start, stop, list, inspect, destroy) |
| `/mongo <cmd>` | Query MongoDB store (events, artifacts, search, aggregate) |
| `/build [target]` | Build container images (cli, desktop, all, rebuild) |
| `/status` | System status overview (sessions, images, MongoDB, network) |
| `/security-audit [name]` | Run 8-layer security audit on a container |

### PostToolUse Hooks (`.claude/hooks/`)

Hooks fire automatically after tool calls (cookbook pattern: read JSON from stdin, log to MongoDB).

| Hook | Matcher | Description |
|------|---------|-------------|
| `command-logger.py` | Bash | Logs all bash commands to MongoDB `events` + local `audit/command_log.json` |
| `session-tracker.py` | Bash, Write | Detects docker/session lifecycle commands, logs transitions |

Configured in `.claude/settings.local.json` via the `PostToolUse` hook pattern.

### Output Styles (`.claude/output-styles/`)

| Style | Description |
|-------|-------------|
| `ops` | DevOps/operations — compact status cards, metrics tables, action-oriented |
| `debug` | Troubleshooting — verbose output, stack traces, step-by-step diagnosis |

### Universal Node Tree (root infrastructure)

**One pattern. Every scale. Same shape.** A node has: inputs, outputs, children, a parent, rules for what passes through it, and an escalation path when it can't decide.

**Root files (not a sub-feature — THE foundation):**
- `universal-node.schema.json` — The ONE JSON schema. Every node, every scale, every project.
- `tree.json` — claude-cage's own architecture as a tree (28 nodes: 2 executives, 7 departments, 19 captains)
- `lib/tree.sh` — Tree operations for ANY project
- `templates/project/tree.json` — Starter tree for `claude-cage init`

**Self-describing architecture:** claude-cage eats its own cooking. Its own `tree.json` maps every lib module, security layer, and subsystem as universal nodes. The same pattern it scaffolds into every new project.

**CLI commands:**
```bash
claude-cage init <dir> [--name n]          # Scaffold new project with tree
claude-cage tree show [tree.json]          # Render tree hierarchy
claude-cage tree node <path> <id>          # Get single node as JSON
claude-cage tree blast-radius <path> <t>   # Calculate affected nodes + risk
claude-cage tree route <path> <intent>     # Route intent to matching nodes
claude-cage tree seed <path> [project]     # Seed tree into MongoDB
```

**`lib/tree.sh` functions:**
- `tree_load` / `tree_show` / `tree_node` — Read and display trees
- `tree_blast_radius` — Find affected nodes, calculate risk (1-10), determine approval level
- `tree_route` — Route intent keywords to matching nodes
- `tree_init` — Create new tree.json (from template or minimal default)
- `tree_add_node` — Add node to existing tree, update parent's children
- `tree_seed` — Seed full tree + individual nodes into MongoDB

**Container integration:** When containers launch, `universal-node.schema.json` and `templates/` are mounted read-only at `/opt/cage/` so Claude inside the sandbox can scaffold new projects.

### PTC — Pass-Through Coordination (`ptc/`)

**The tree isn't a map — it's a machine.** Intent flows DOWN. Artifacts flow UP. As above, so below.

**Core files:**
- `ptc/engine.py` — The PTC engine: decompose, execute, aggregate, store
- `ptc/executor.py` — Leaf node executor: inspect, shell, claude, compose modes

**The cycle:**
1. **INTAKE** — Intent enters at root
2. **TRIAGE/PLAN** — Decompose through tree: route intent to departments, fan out to captains
3. **EXECUTE** — Leaf nodes (captains) do the actual work
4. **VERIFY** — Each result checked against node rules
5. **INTEGRATE** — Results aggregate bottom-up: captain → department → executive → root
6. **SHIP** — Full execution trace stored to MongoDB

**Execution modes:**
- `inspect` — Read files, analyze, report (triggered by: show, check, verify, audit)
- `shell` — Run safe, known commands (triggered by: build, run, install, deploy)
- `claude` — Invoke Claude Code with full node context (triggered by: create, add, implement, fix)
- `plan` — Return what would be done without doing it (default/dry-run)

**Self-identification:** Every node carries a `lineage` — full path from root to itself. Any fragment can reconstruct its place in the tree. The `execution` field tracks status, last input/output, run counts. The `artifacts` field tracks what each node has produced, with content hashes for IPFS addressing.

**CLI:**
```bash
claude-cage ptc run "intent" [--tree path] [--target node] [--live] [-v]
claude-cage ptc exec <node-id> "task" [--live]
claude-cage ptc leaves [tree.json]
claude-cage ptc tree [tree.json]
```

**Makefile:**
```bash
make ptc INTENT="add GPU monitoring"       # Dry run
make ptc-live INTENT="verify sandbox"      # Live execution
make ptc-leaves                             # Show all workers
```

### Training Protocol (`ptc/training.py`, `ptc/lora.py`)

**Every PTC trace IS a chain of thought. Extract. Train. Stack. Grow.**

The Hopf sphere: PTC runs generate traces → traces become training data → training data trains LoRAs → LoRAs improve executors → better traces. Build out to feed in.

**Training data extraction** (`ptc/training.py`):
- Alpaca format (instruction/input/output) — for supervised fine-tuning
- ShareGPT format (multi-turn conversations) — for chat fine-tuning
- CoT format (question/chain_of_thought/answer) — for reasoning training
- Per-node, per-department, per-scale splitting — each feeds its own LoRA

**LoRA pipeline** (`ptc/lora.py`):
- L0: `ptc-base` — trained on ALL traces (universal tree coordination)
- L1: `ptc-scale-*` — executive reasoning, department coordination, captain execution
- L2: `ptc-dept_*` — security patterns, runtime patterns, web patterns, etc.
- L3: `ptc-capt_*` — leaf-specific expertise (one per worker)
- Stack: base + scale + department + captain = specialized agent

**Hardware:** QLoRA (4-bit NF4 quantization) on 2x RTX 3090 24GB.

**CLI:**
```bash
claude-cage train extract [--source local|mongodb] [--output dir]
claude-cage train pipeline [--tree path] [--model name]
claude-cage train stack [--tree path]
claude-cage train preview [trace.json]
```

**Makefile:**
```bash
make train-extract     # Extract training data from traces
make train-pipeline    # Generate full LoRA pipeline
make train-stack       # Show stacking order
```

### GentlyOS Recursive Tree (`gentlyos/`)

**Core Insight: One pattern. Every scale. Same shape.**

A node has: inputs, outputs, children, a parent, rules for what passes through it, and an escalation path when it can't decide. That's a crate. That's a department. That's a sephira. That's a knowledge node. That's a CODIE primitive. One struct, parameterized by scale.

**Files:**
- `gentlyos/tree.json` — The full GentlyOS tree: 34 agents (1 Human + 2 Executives + 8 Directors + 24 Captains), sephirot mapping, coordination protocol. Uses the root `universal-node.schema.json`.
- `gentlyos/seed.js` — Seeds documents, tree, and nodes into MongoDB

**Node Scales:** `executive`, `department`, `captain`, `crate`, `module`, `sephira`, `knowledge`, `reasoning`, `primitive`

**Tree of Life → Department Mapping:**
| Sephira | Department | Role |
|---------|-----------|------|
| Keter | Interface | Crown — user-facing entry point |
| Chokmah/Binah | Protocol | Core abstractions, Alexandria |
| Daath | Security | Hidden, touches everything |
| Chesed/Gevurah | DevOps | Mercy/judgment in releases |
| Tiferet | Orchestration | Center — CODIE, PTC, context |
| Netzach/Hod | Runtime | Execution, pillar balance |
| Yesod | Tokenomics | Foundation of value |
| Malkuth | Foundation | Primitives, leaf deps |

**Coordination Protocol (8 phases):** INTAKE → TRIAGE → PLAN → REVIEW → EXECUTE → VERIFY → INTEGRATE → SHIP

**Approval Cascade:** Risk 1-3: Captain | 4-6: Director | 7-8: CTO | 9-10: Human

**Orchestrator Agent:** `.claude/agents/gentlyos-orchestrator.md` — ONE agent that reads the tree and routes. Not 34 files. One pattern.

**Slash Command:** `/gentlyos <subcommand>` — route, node, blast-radius, tree, seed, sephirot, approve

**Web Dashboard:** GentlyOS Tree view at `http://localhost:5000` — interactive tree hierarchy, node details, sephirot mapping, coordination phases

**API Endpoints:**
- `GET /api/gentlyos/tree` — full tree
- `GET /api/gentlyos/node/<id>` — single node details
- `GET /api/gentlyos/blast-radius?crates=x,y` — calculate affected departments + risk level

```bash
node gentlyos/seed.js       # Seed docs + tree + nodes into MongoDB
make gentlyos-seed           # Same via Makefile
```

### GentlyOS Design Documents (root)

Four `.docx` documents define the GentlyOS architecture. Seeded into MongoDB as artifacts.

| Document | Type | Content |
|----------|------|---------|
| `GentlyOS_Virtual_Organization_System.docx` | Design doc | 34-agent hierarchy, Google monorepo model, coordination protocol |
| `GentlyOS_Workspace_System.docx` | Design doc | Universal workspace, Reverse Mermaid, Reflective App Builder |
| `Gently_Studio_Protocols.docx` | Design doc | Quad-Context (WHAT×WHEN×WHO×HOW), Alexandria, CODIE, $SYNTH |
| `Google_Infrastructure_Research.docx` | Research | Google ADK, supply chain analysis, GentlyOS as counter-architecture |

### GentlyWorkstation Frontend (`GentlyWorkstation.jsx`)

React component — the future GentlyOS workstation UI. Contains: project shelf, Claude chat panel, browser tabs, bottom panels (MongoDB, Terminal, GPU Monitor, Files, Activity Log), GPU stats, global search. Dark theme matching the web dashboard. Currently a standalone JSX file for development; will be integrated via a React build pipeline.

## No Tests or Linting

There is no test suite or linting configuration. The primary verification mechanism is `make verify-sandbox` which inspects a running container's security settings.

## Subproject: headless-ubuntu-auto

`projects/headless-ubuntu-auto/` is a separate 24-file project for headless GPU server provisioning (2x RTX 3090). It has its own Makefile and is independent of the main claude-cage codebase. See its own README for details.
