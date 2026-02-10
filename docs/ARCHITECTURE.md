# ARCHITECTURE.md

Comprehensive architecture documentation for claude-cage / gently-dev.

---

## 1. Overview

claude-cage is **gently-dev** -- the development sandbox that ships with GentlyOS. One project, one tree, one launcher. The self-hosting goal: build GentlyOS from inside GentlyOS using `nixos-rebuild switch`.

The system is a dockerized sandbox for running Claude CLI and Claude Desktop in isolated containers with defense-in-depth security. It combines container orchestration, a 35-node agent tree, a 38-crate Rust workspace dependency graph, a Pass-Through Coordination engine, a 12-keyword DSL interpreter, and a living documentation circle into a single unified development environment.

Everything maps to a single recursive pattern: a node has inputs, outputs, children, a parent, rules for what passes through it, and an escalation path when it can't decide. That pattern holds at every scale -- from a Rust crate to a department to a sephira on the Tree of Life.

---

## 2. Project Structure

```
claude-cage/
|-- launch                        # Single-entry launcher script
|-- bin/claude-cage                # CLI tool (sources 15 lib modules)
|-- Makefile                       # Build, run, clean, mongo, observability targets
|-- docker-compose.yml             # Three services: cli, desktop, cli-isolated
|
|-- gentlyos/                      # The Tree + crate graph + universal schema
|   |-- tree.json                  # 35-node agent hierarchy
|   |-- crate-graph.json           # 38-crate dependency graph (7 tiers)
|   |-- universal-node.schema.json # JSON schema every node follows
|   +-- seed.js                    # Seeds docs, tree, nodes into MongoDB
|
|-- ptc/                           # Pass-Through Coordination engine (Python)
|   |-- engine.py                  # 8-phase PTC cycle
|   |-- executor.py                # 7 execution modes + CODIE interpreter
|   |-- crate_graph.py             # Crate dependency graph operations
|   |-- docs.py                    # Documentation Circle system
|   |-- architect.py               # Blueprint generation
|   |-- embeddings.py              # Vector embeddings (sentence-transformers)
|   |-- ipfs.py                    # IPFS dual-storage
|   |-- git_ops.py                 # Git operations
|   |-- huggingface.py             # HuggingFace integration
|   |-- federation.py              # Federation protocol
|   |-- lora.py                    # LoRA training
|   |-- training.py                # Training pipeline
|   |-- nounproject.py             # Noun Project API
|   +-- porkbun.py                 # Porkbun DNS API
|
|-- projects/
|   |-- Gently-nix/                # GentlyOS Rust workspace (38 crates)
|   +-- headless-ubuntu-auto/      # Headless GPU server provisioning
|
|-- docker/
|   |-- cli/Dockerfile             # node:20-slim + claude-code
|   +-- desktop/
|       |-- Dockerfile             # ubuntu:24.04 + X11 stack
|       +-- entrypoint-desktop.sh  # Xvfb + openbox + x11vnc + noVNC
|
|-- security/
|   |-- seccomp-default.json       # ~147 syscall allowlist
|   +-- apparmor-profile           # Deny mount/ptrace/raw-net/kernel-modules
|
|-- lib/                           # 15 bash library modules (4,914 lines)
|   |-- config.sh                  # YAML config loading
|   |-- sandbox.sh                 # Security flag construction
|   |-- docker.sh                  # Docker build/run/stop/exec
|   |-- session.sh                 # Session metadata management
|   |-- tui.sh                     # Terminal UI components
|   |-- gui.sh                     # GUI helpers
|   |-- tree.sh                    # Tree operations
|   |-- architect.sh               # Architecture tooling
|   |-- docs.sh                    # Documentation commands
|   |-- integrations.sh            # External integrations
|   |-- mongodb.sh                 # MongoDB fire-and-forget wrapper
|   |-- memory.sh                  # Session memory compaction
|   |-- observability.sh           # Container metrics and health checks
|   |-- lifecycle.sh               # Container lifecycle management
|   +-- cli.sh                     # Command parsing, all cmd_*() functions
|
|-- cage-web/                      # Rust + HTMX dashboard
|   +-- src/
|       |-- main.rs                # axum 0.8 server
|       |-- subprocess.rs          # Shell-out wrappers
|       +-- codie_parser.rs        # Native Rust CODIE parser
|
|-- mongodb/
|   |-- store.js                   # CLI: put, log, get, search, aggregate, etc.
|   |-- seed-artifacts.js          # Batch artifact loader
|   +-- package.json               # mongodb driver only
|
|-- config/
|   +-- default.yaml               # All defaults (flat key:value)
|
+-- docs/                          # Generated doc artifacts (JSON) + this file
```

---

## 3. The Tree

**Source:** `gentlyos/tree.json`

The tree is a 35-node hierarchy that defines the entire organizational structure. Every node follows the same `universal-node.schema.json` shape.

### Node Shape

Every node has these fields:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique ID (e.g., `dept:security`, `capt:codie`) |
| `name` | string | Human-readable name |
| `scale` | string | `executive`, `department`, or `captain` |
| `parent` | string/null | Parent node ID |
| `children` | string[] | Child node IDs |
| `inputs` | object[] | Named inputs with type and source |
| `outputs` | object[] | Named outputs with type and destination |
| `rules` | object[] | Condition/action pairs |
| `escalation` | object | `{target, threshold, cascade}` |
| `metadata` | object | `{agent_id, crates_owned, tier, sephira_mapping, ...}` |

### Hierarchy (35 nodes)

```
root:human — Human Architect (Tom) [executive]
  |
  +-- exec:cto — CTO Agent [executive]
  |     |
  |     +-- dept:foundation    (Malkuth)       — 3 captains: types, crypto, errors
  |     +-- dept:protocol      (Chokmah/Binah) — 3 captains: wire, p2p, alexandria
  |     +-- dept:orchestration (Tiferet)       — 3 captains: codie, ptc, context
  |     +-- dept:runtime       (Netzach/Hod)   — 3 captains: exec, state, memory
  |     +-- dept:tokenomics    (Yesod)         — 3 captains: rewards, proof, economics
  |     +-- dept:security      (Daath)         — 3 captains: audit, hardening, incident
  |     +-- dept:interface     (Keter)         — 3 captains: api, claude, ux
  |     +-- dept:devops        (Gevurah/Chesed)— 3 captains: build, release, infra
  |
  +-- exec:vision — Vision Alignment Agent [executive]
```

**Counts:** 1 root + 2 executives + 8 departments + 24 captains = 35 nodes

### Tree of Life Mapping

The department structure maps to the Kabbalistic Tree of Life. Each sephira IS a department node with the same universal shape.

| Sephira | Department | Role |
|---------|-----------|------|
| Keter (Crown) | Interface | User-facing entry point |
| Chokmah (Wisdom) | Protocol | Core abstractions, wire format |
| Binah (Understanding) | Protocol | Alexandria knowledge graph |
| Daath (Knowledge) | Security | Hidden sephira -- touches everything |
| Chesed (Mercy) | DevOps | Release grace |
| Gevurah (Severity) | DevOps | Build discipline |
| Tiferet (Beauty) | Orchestration | Center -- CODIE, PTC, context |
| Netzach (Victory) | Runtime | Execution persistence |
| Hod (Splendor) | Runtime | Execution precision |
| Yesod (Foundation) | Tokenomics | Foundation of value |
| Malkuth (Kingdom) | Foundation | Primitives, leaf dependencies |

### Crate Ownership (metadata.crates_owned)

Each department and captain node owns specific `gently-*` crates. Example:

- `dept:foundation` owns: gently-core, gently-codie, gently-artisan, gently-audio, gently-visual, gently-goo, gently-synth
- `dept:protocol` owns: gently-bridge, gently-network, gently-dance, gently-livepeer, gently-alexandria, gently-ipfs, gently-feed
- `dept:security` owns: ALL (cross-cutting review authority)

---

## 4. Crate Dependency Graph

**Source:** `gentlyos/crate-graph.json`
**Code:** `ptc/crate_graph.py`

38 crates organized into 7 tiers (0 = Foundation through 6 = Application). Each crate has a tier, path, dependency list, and owning tree node.

### Tier Structure

| Tier | Name | Crates | Description |
|------|------|--------|-------------|
| 0 | Foundation | 7 | gently-core, gently-codie, gently-artisan, gently-audio, gently-visual, gently-goo, gently-synth |
| 1 | Knowledge (core) | 2 | gently-feed, gently-btc |
| 2 | Knowledge (graph) | 3 | gently-alexandria, gently-search, gently-ipfs |
| 3 | Intelligence | 7 | gently-brain, gently-inference, gently-agents, gently-micro, gently-mcp, gently-ged, gently-behavior |
| 4 | Security | 5 | gently-security, gently-cipher, gently-guardian, gently-sim, gently-sploit |
| 5 | Network | 5 | gently-network, gently-gateway, gently-bridge, gently-dance, gently-livepeer |
| 6 | Application | 9 | gently-web, gently-architect, gently-document, gently-gooey, gently-commerce, gently-google, gently-tiktok, gently-cli, gentlyos-tui |

**Total: 38 crates**

### Reverse Dependency Index

The graph loader builds a reverse dependency index at load time:

```python
reverse_deps = {name: set() for name in crates}
for name, info in crates.items():
    for dep in info.get("deps", []):
        if dep in reverse_deps:
            reverse_deps[dep].add(name)
```

This enables blast_radius calculation: given a changed crate, BFS through `reverse_deps` yields all transitively affected crates.

### Key Functions (ptc/crate_graph.py)

| Function | Signature | Description |
|----------|-----------|-------------|
| `load_graph()` | `(path=None) -> dict` | Load crate-graph.json, build reverse dep index |
| `dependents()` | `(graph, crate) -> set` | All crates that transitively depend on this one |
| `build_order()` | `(graph, crates) -> list` | Sort crates by tier for correct build sequence |
| `blast_radius()` | `(graph, changed_crates) -> dict` | Changed crates -> affected crates + nodes + risk |
| `tier_rebuild_scope()` | `(graph, changed_tier) -> list` | Changed tier -> all tiers needing rebuild |
| `crates_in_tier()` | `(graph, tier) -> list` | All crate names in a given tier |
| `crates_for_node()` | `(graph, node_id) -> list` | All crates owned by a tree node |

### Blast Radius Risk Calculation

```
affected_ratio = affected_count / total_crates

> 0.8  -> risk 9
> 0.5  -> risk 7
> 0.3  -> risk 6
> 0.15 -> risk 5
> 0.05 -> risk 3
else   -> risk 2

Tier 0 changes always bump risk to at least 7.
```

---

## 5. PTC Engine

**Source:** `ptc/engine.py`

Pass-Through Coordination. Intent enters at root, decomposes DOWN through the tree, leaves EXECUTE, results aggregate UP to root. Every step goes to MongoDB. Every artifact is stored.

### 8 Phases

```
INTAKE -> TRIAGE -> PLAN -> REVIEW -> EXECUTE -> VERIFY -> INTEGRATE -> SHIP
```

| Phase | Description |
|-------|-------------|
| **INTAKE** | Load tree, log intent, load crate graph if available |
| **TRIAGE** | Route intent to matching nodes via keyword scoring |
| **PLAN** | Decompose to leaf tasks; if crate graph loaded and intent mentions crates, use blast_radius for targeting instead of keyword matching |
| **REVIEW** | Check approval gates for each task before execution |
| **EXECUTE** | Leaf nodes do the work (sorted by crate tier if graph loaded -- tier 0 before tier 3) |
| **VERIFY** | Check results, detect failures, build escalation list |
| **INTEGRATE** | Aggregate bottom-up through tree, apply rules at each level, fire escalations |
| **SHIP** | Final report, store full trace as artifact, update tree state |

### Intent Routing

`route_intent()` scores each node against the intent string by matching words against:
- Node name and ID
- `metadata.crates_owned`
- `metadata.files`
- `metadata.functions`

Leaf nodes get a +0.5 score boost (they are the workers).

### Decomposition

`decompose()` fans out from matched nodes to their leaf children. If a target node is specified, decomposition starts from that node. Otherwise, all matching departments and direct leaf hits are decomposed. Results are deduplicated by node_id.

### Approval Cascade

| Risk Level | Approval | Action |
|------------|----------|--------|
| 1-3 | Captain auto-approved | Execute immediately |
| 4-6 | Director logged | Log and proceed |
| 7-8 | CTO blocks | Block -- requires CTO approval |
| 9-10 | Human required | Block -- requires human approval |

Risk is calculated from:
- Base risk by scale: executive=8, department=6, captain=3
- High-risk intent words: delete, destroy, force, nixos-rebuild (+3)
- Medium-risk intent words: deploy, push, release, nix build (+1)
- Sensitive file paths: security/, docker/, .env (+1)
- More than 3 rules applied: -1 (more constrained = lower risk)

### Usage

```bash
python3 -m ptc.engine --tree tree.json --intent "add GPU monitoring"
python3 -m ptc.engine --tree tree.json --intent "fix auth bug" --target dept:security
python3 -m ptc.engine --tree tree.json --node capt:docker --task "build ARM image"
python3 -m ptc.engine --tree tree.json --intent "rebuild gently-core" --live
```

---

## 6. Executor

**Source:** `ptc/executor.py`

The bridge between the tree and the real world. Receives a leaf task, determines execution mode, does the work, returns results. The executor knows nothing about the tree; the engine handles coordination.

### 7 Execution Modes

| Mode | Trigger | Description |
|------|---------|-------------|
| **native** | `cargo build`, `nix build`, `nixos-rebuild` | Direct host commands -- cargo/nix/rebuild sub-modes |
| **claude** | `create`, `add`, `implement`, `fix`, `write` | Invoke `claude --print` in non-interactive mode |
| **shell** | `build`, `run`, `install`, `deploy` | Safe shell commands via known-pattern allowlist |
| **codie** | `codie` in intent or `codie_program` in task | 12-keyword DSL interpreter |
| **design** | `design`, `architect`, `blueprint`, `draft` | Produce a blueprint, not code (architect mode) |
| **inspect** | `show`, `list`, `check`, `verify`, `audit` | Read files, stat, analyze, report |
| **compose** | Aggregation point | Combine multiple outputs |

Mode detection is keyword-based in `_detect_mode()`. CODIE is checked first (to prevent "codie build" matching shell mode).

### Native Sub-Modes

- **cargo**: `cargo build/test/clippy/fmt -p <crate>` or `--workspace`. Runs in `projects/Gently-nix/` workspace root. 300s timeout.
- **nix**: `nix build .#<target>`, `nix develop`, `nix flake check`. 600s timeout.
- **rebuild**: `nixos-rebuild switch`. Always risk 9, always blocked, always requires human approval.
- **tier_rebuild**: Load crate graph, compute blast radius, `cargo build -p <crate>` for each in tier order. Stops on first failure.

### Shell Mode Safety

`_intent_to_command()` converts intents to known-safe commands. Only recognized patterns are allowed:

```
make build-cli, make build-desktop, make status, make verify-sandbox,
make mongo-ping, make mongo-status, make tree,
cargo build/test/clippy -p <crate>, nix build, nix flake check
```

Unknown intents return `None` (command not executed).

### CODIE Interpreter

The full CODIE interpreter is embedded in the executor. It handles all 12 keywords:

| Keyword | AST Node | Interpreter Action |
|---------|----------|-------------------|
| `pug` | Entry | Set up execution context, execute children |
| `bark` | Fetch | Read files (`@fs/read`), system queries (`@system/*`), cargo ops (`@cargo/*`) |
| `spin` | Loop | Iterate over a collection |
| `cali` | Call | Map to safe shell commands, make targets, or Claude invocations |
| `elf` | Bind | Bind a variable in context |
| `turk` | Transform | Conditional transformation |
| `fence` | Guard | Check preconditions, halt on failure |
| `pin` | Const | Set immutable constant |
| `bone` | Rule | Constraint enforcement (negated rules = "must NOT happen") |
| `blob` | Struct | Define a data structure in context |
| `biz` | Return | Return final result |
| `anchor` | Checkpoint | Log to MongoDB, snapshot state |

The `CodieContext` class maintains:
- `variables` -- mutable variable bindings
- `constants` -- immutable values (via `pin`)
- `checkpoints` -- audit trail (via `anchor`)
- `structs` -- data structure definitions (via `blob`)

CODIE source is parsed by the cage-web Rust binary if available (`--parse-codie` flag), otherwise falls back to `_parse_codie_python()` -- a lightweight Python line parser that handles pipe-tree notation (`|  +--`), brace blocks, and all 12 keywords.

### Safe Call Patterns (CODIE cali)

```python
safe_calls = {
    "EXECUTE_INTENT", "BUILD", "TEST", "STATUS", "VERIFY", "SEED"
}
```

Unknown call patterns are logged but not executed.

Shell commands from CODIE are restricted to:

```python
allowed_prefixes = ["make ", "cargo ", "nix ", "rustc ", "rustfmt ",
                    "docker ps", "docker info", "node "]
```

---

## 7. Security Model

8 defense-in-depth layers. Applied identically by `launch`, `bin/claude-cage` (via `lib/sandbox.sh`), and `docker-compose.yml` (via `x-common` YAML anchor).

### Layer Summary

| # | Layer | Implementation |
|---|-------|---------------|
| 1 | Read-only root filesystem | `--read-only` with tmpfs at `/tmp` (512MB, noexec,nosuid) and `/run` (64MB, noexec,nosuid) |
| 2 | Capability drop | `--cap-drop ALL` then re-add CHOWN, DAC_OVERRIDE, SETGID, SETUID only |
| 3 | Seccomp profile | `security/seccomp-default.json`: ~147 syscall allowlist, default action SCMP_ACT_ERRNO, AF_VSOCK blocked |
| 4 | AppArmor profile | `security/apparmor-profile`: denies mount, ptrace, raw-network, kernel-module access |
| 5 | Resource limits | 2 CPUs, 4GB memory, 512 PIDs, ulimits (nofile 1024:2048, nproc 256:512) |
| 6 | Network filtering | `sandbox_apply_network_filter()` resolves `allowed_hosts` to IPs, injects iptables rules post-launch. Default allowed: `api.anthropic.com`, `cdn.anthropic.com` |
| 7 | No-new-privileges | `--security-opt no-new-privileges` prevents privilege escalation |
| 8 | Bridge network | `cage-filtered` with inter-container communication disabled (`enable_icc=false`), subnet `172.28.0.0/16` |

### Seccomp Profile

```json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "defaultErrnoRet": 1,
  "archMap": ["SCMP_ARCH_X86_64", "SCMP_ARCH_AARCH64"],
  "syscalls": [{ "names": [...~147 allowed syscalls...], "action": "SCMP_ACT_ALLOW" }]
}
```

### Verification

```bash
make verify-sandbox    # Inspects running container security settings
```

---

## 8. Container Architecture

### CLI Image

| Property | Value |
|----------|-------|
| Base | `node:20-slim` |
| Installs | `@anthropic-ai/claude-code` via npm |
| User | Non-root `cageuser` |
| Entrypoint | `tini` -> `claude` |
| Labels | `managed-by=claude-cage`, `cage.mode=cli` |

### Desktop Image

| Property | Value |
|----------|-------|
| Base | `ubuntu:24.04` |
| Installs | Xvfb, openbox, x11vnc, noVNC/websockify, xterm, Claude CLI |
| Ports | 5900 (VNC), 6080 (noVNC HTTP) |
| Entrypoint | `entrypoint-desktop.sh` |
| Startup sequence | Xvfb -> openbox -> x11vnc -> websockify -> xterm (with EXIT trap cleanup) |

### Docker Compose (docker-compose.yml)

Three services sharing an `x-common` anchor for security baseline:

```yaml
x-common: &common
  read_only: true
  security_opt: [no-new-privileges, seccomp=security/seccomp-default.json]
  cap_drop: [ALL]
  cap_add: [CHOWN, DAC_OVERRIDE, SETGID, SETUID]
  ulimits: {nofile: {soft: 1024, hard: 2048}, nproc: {soft: 256, hard: 512}}
  tmpfs: [/tmp:rw,noexec,nosuid,size=512m, /run:rw,noexec,nosuid,size=64m]
```

| Service | Mode | Network | Resources |
|---------|------|---------|-----------|
| `cli` | Interactive TTY | cage-filtered bridge | 2 CPU, 4GB RAM, 512 PIDs |
| `desktop` | Detached, ports 6080/5900 | cage-filtered bridge | 2 CPU, 4GB RAM, 512 PIDs |
| `cli-isolated` | Interactive TTY | `network_mode: none` | 1 CPU, 2GB RAM, 256 PIDs |

Security policy is duplicated between `docker-compose.yml` and `launch`/`lib/docker.sh`. Changes to security policy must be updated in both places.

---

## 9. Bash Library (lib/)

15 modules, 4,914 total lines. `bin/claude-cage` sources them in a specific order (source order matters for dependency resolution).

### Source Order

```bash
source "$CAGE_ROOT/lib/config.sh"
source "$CAGE_ROOT/lib/sandbox.sh"
source "$CAGE_ROOT/lib/docker.sh"
source "$CAGE_ROOT/lib/session.sh"
source "$CAGE_ROOT/lib/tui.sh"
source "$CAGE_ROOT/lib/gui.sh"
source "$CAGE_ROOT/lib/tree.sh"
source "$CAGE_ROOT/lib/architect.sh"
source "$CAGE_ROOT/lib/docs.sh"
source "$CAGE_ROOT/lib/integrations.sh"
source "$CAGE_ROOT/lib/mongodb.sh"
source "$CAGE_ROOT/lib/memory.sh"
source "$CAGE_ROOT/lib/observability.sh"
source "$CAGE_ROOT/lib/lifecycle.sh"
source "$CAGE_ROOT/lib/cli.sh"
```

### Module Table

| Module | Lines | Responsibility |
|--------|-------|---------------|
| `cli.sh` | 1,446 | Command parsing, argument handling, all `cmd_*()` dispatch functions |
| `gui.sh` | 817 | GUI helpers and display components |
| `tui.sh` | 539 | Terminal UI components, menus, prompts |
| `tree.sh` | 282 | Tree operations: show, route, blast-radius, node detail |
| `lifecycle.sh` | 251 | Container lifecycle: start, stop, restart, health transitions |
| `docker.sh` | 244 | Docker build, run, stop, destroy, exec, inspect |
| `integrations.sh` | 204 | External service integrations (APIs, tools) |
| `observability.sh` | 192 | Container metrics, health checks, dashboards |
| `sandbox.sh` | 188 | Security flag construction, network filtering, verification |
| `session.sh` | 170 | Session metadata (create/list/status/remove), name generation |
| `memory.sh` | 145 | Session memory save/load/compact/clean (Anthropic cookbook pattern) |
| `config.sh` | 128 | YAML config loading (flat key:value), validation |
| `mongodb.sh` | 127 | MongoDB fire-and-forget wrapper (bash -> node store.js) |
| `architect.sh` | 111 | Architecture tooling, blueprint generation |
| `docs.sh` | 70 | Documentation commands (bash wrapper for ptc/docs.py) |

---

## 10. MongoDB Store

**Source:** `mongodb/store.js`

Fire-and-forget key/value store backed by MongoDB Atlas. Node.js + native `mongodb` driver (no ODM overhead).

### Commands

```bash
node store.js put <collection> '<json>'
node store.js log <type> <key> ['<value_json>']
node store.js get <collection> ['<query_json>'] [limit]
node store.js search <collection> '<query_text>' [limit]
node store.js aggregate <collection> '<pipeline_json>'
node store.js bulk <collection> '<docs_array_json>'
node store.js distinct <collection> '<field>' ['<query_json>']
node store.js stats
node store.js ping
node store.js count <collection> ['<query_json>']
```

### Collections

| Collection | Content |
|------------|---------|
| `events` | All structured events: type, key, value, `_ts`, `_host`, `_project` |
| `artifacts` | Code, configs, outputs: name, type, content, project |
| `docs` | Documentation circle artifacts: per-node docs with cross-refs |

### Bash Wrapper (lib/mongodb.sh)

| Function | Description |
|----------|-------------|
| `mongo_init` | Source .env, check node/deps, set `MONGO_READY` flag |
| `mongo_put <collection> <json>` | Fire-and-forget insert (backgrounded + disowned) |
| `mongo_log <type> <key> [value]` | Structured event to `events` collection |
| `mongo_get <collection> [query] [limit]` | Synchronous query |
| `mongo_log_session <event> <name> [meta]` | Session lifecycle events |
| `mongo_log_command <cmd> [args...]` | CLI command logging |
| `mongo_store_artifact <name> <type> <content>` | Store code/config/output |
| `mongo_log_project <project> <type> <key> [value]` | Per-project tagging |

All writes are backgrounded and disowned -- they never block the CLI. The engine (`ptc/engine.py`) also writes directly via `subprocess.Popen` with `stdout=DEVNULL, stderr=DEVNULL`.

### Environment

```
MONGODB_URI              — full connection string (preferred)
MONGODB_CLUSTER0_ADMIN   — fallback: auto-prepends mongodb+srv://
MONGODB_DB               — database name (default: claude_cage)
CAGE_PROJECT             — project tag (default: claude-cage)
```

Configuration stored in `mongodb/.env` (never committed).

---

## 11. Documentation Circle

**Source:** `ptc/docs.py`

A living bidirectional graph where documentation is part of the code itself. Stored the same way (MongoDB + IPFS + local JSON), interconnected bidirectionally, staleness-tracked by file hash. Change one side, the other knows.

### Architecture

- **50 node artifacts** (one per tree node + additional project/captain docs)
- **Three edge types:**
  - **Structural** -- tree parent/child/sibling hierarchy
  - **Code-shared** -- overlapping file ownership between nodes
  - **Semantic** -- vector similarity > 0.7 (via sentence-transformers)
- **All edges bidirectional** -- `make_bidirectional()` ensures every A->B gets B->A

### Staleness Tracking

Each doc stores a `source_hash` (SHA-256 of concatenated owned file contents). `check_staleness()` recomputes the hash and compares. When a node goes stale, `propagate_staleness()` follows all edges and flags connected nodes as potentially affected.

### Triple Storage

Every doc is stored in three locations:

1. **MongoDB** -- `docs` collection via `node store.js put docs`
2. **IPFS** -- via `ptc.ipfs.dual_store()` if available
3. **Local JSON** -- `docs/<node-id>.json` files

### Search

Three-tier fallback:

1. **Semantic search** -- `ptc.embeddings.semantic_search()` using sentence-transformers
2. **Text search** -- MongoDB `$text` via `node store.js search`
3. **Local keyword search** -- scan `docs/*.json` files

### Key Functions

| Function | Description |
|----------|-------------|
| `generate_doc(node_id, tree)` | Build doc artifact for one node from tree metadata + file analysis |
| `generate_all()` | Generate docs for all 35 tree nodes |
| `compute_structural_refs()` | Parent/child/sibling cross-refs from tree structure |
| `compute_code_refs()` | Find nodes sharing files (code_shared edges) |
| `compute_semantic_refs()` | Vector similarity > 0.7 (semantic edges) |
| `full_interconnect()` | Build complete bidirectional graph (THE CIRCLE) |
| `check_staleness()` | Recompute file hash, compare to stored hash |
| `propagate_staleness()` | Flag connected docs when source changes |
| `refresh_doc()` | Regenerate stale doc + re-embed + re-interconnect |
| `search_docs()` | Semantic search with text and local fallbacks |

### Usage

```bash
python3 -m ptc.docs generate-all         # Generate all docs
python3 -m ptc.docs check-stale          # Check for staleness
python3 -m ptc.docs refresh              # Regenerate all stale docs
python3 -m ptc.docs interconnect         # Compute full graph
python3 -m ptc.docs search "security"    # Semantic search
python3 -m ptc.docs status               # Coverage stats
```

---

## 12. Launcher

**Source:** `launch`

Single entry point. Handles Docker daemon startup, image builds, network creation, and container launch. All automated -- if Docker is not running, it starts it. If images are missing, it builds them. If the network does not exist, it creates it.

### Modes

| Mode | Command | Description |
|------|---------|-------------|
| Interactive | `./launch` | Menu-driven selection |
| CLI | `./launch cli` | Sandboxed Claude CLI with filtered network |
| Bare | `./launch cli --bare` | Claude CLI on host (no container) |
| Desktop | `./launch desktop [port]` | noVNC at localhost:6080 |
| Isolated | `./launch isolated` | CLI with no network |
| Project | `./launch project <name>` | CLI with project directory mounted |
| Shell | `./launch shell [session]` | Attach bash to running container |

### Session Names

Auto-generated as `<adjective>-<noun>-<hex4>`:

```bash
adj=(swift bold calm dark keen fast deep cool true wild sage pure)
noun=(fox owl elk ash oak ray arc vim gem ion rune pine)
# Example: swift-fox-a1b2, calm-oak-f3d7
```

### Preflight Sequence

1. `ensure_docker()` -- check for docker binary, start daemon if not running
2. `ensure_network()` -- create `cage-filtered` bridge if it does not exist
3. `ensure_image()` -- build CLI or Desktop image if not found
4. `ensure_dirs()` -- create `~/.local/share/claude-cage/sessions/`

### Container Labels

All containers are labeled for discovery:

```
managed-by=claude-cage
cage.mode=cli|desktop|cli-isolated
cage.session=<session-name>
cage.created=<ISO-8601>
```

Discovery: `docker ps --filter "label=managed-by=claude-cage"`

---

## 13. Cage-Web Dashboard

**Source:** `cage-web/`

Rust (axum 0.8) + HTMX 2.0 + askama templates. Zero React, zero Next.js. Dark theme CSS. Server-side rendering throughout.

### Stack

- **axum 0.8** -- HTTP framework
- **askama** -- compile-time templates
- **HTMX 2.0** -- hypermedia-driven interactivity
- **tower-http** -- static file serving

### Application State

```rust
pub struct AppState {
    pub cage_root: PathBuf,
    pub store_js: PathBuf,       // mongodb/store.js
    pub tree_path: PathBuf,      // gentlyos/tree.json
    pub codie_dir: PathBuf,      // codie-maps
    pub codie_programs: RwLock<Vec<Program>>,
}
```

### Routes

| Route | Method | Purpose |
|-------|--------|---------|
| `/` | GET | Dashboard with sessions, health, quick actions |
| `/sessions` | GET | Session list (HTMX fragment) |
| `/sessions/new` | POST | Create session |
| `/sessions/{name}` | GET | Session detail with logs |
| `/sessions/{name}/stop` | POST | Stop session |
| `/sessions/{name}/start` | POST | Start session |
| `/sessions/{name}/destroy` | DELETE | Destroy session |
| `/tree` | GET | GentlyOS tree hierarchy |
| `/tree/{node_id}` | GET | Node detail |
| `/tree/blast-radius` | GET | Risk calculation |
| `/codie` | GET | CODIE programs grid |
| `/codie/{name}` | GET | Program source + AST |
| `/codie/{name}/execute` | POST | Execute program plan |
| `/codie/parse` | POST | Parse raw CODIE source |
| `/api/health` | GET | JSON health status |
| `/api/sessions` | GET | JSON session list |
| `/api/gentlyos/tree` | GET | JSON tree |

### Architecture Pattern

The CODIE parser (`cage-web/src/codie_parser.rs`) is native Rust -- the only component that runs natively. Everything else shells out via `cage-web/src/subprocess.rs`:

- Docker CLI for session management
- `node store.js` for MongoDB queries
- `python3 -m ptc.engine` for PTC execution

### Build & Run

```bash
make build-web        # Compile Rust binary
make web-rs           # Start dashboard at http://localhost:5000
make codie-seed       # Parse .codie files and seed to MongoDB
make codie-parse FILE=path.codie  # Parse a single .codie file
```

---

## 14. GentlyOS Coordination Protocol

The coordination protocol unifies the tree, the crate graph, the PTC engine, and the documentation circle into a single operational model.

### Core Insight

The tree IS the org chart IS the build system IS the documentation graph. One pattern, every scale, same shape.

- A **tree node** routes tasks to children and escalates failures to parents.
- A **crate** has dependencies (inputs) and dependents (outputs).
- A **doc artifact** has cross-references (structural + code-shared + semantic edges).
- A **PTC phase** maps directly to a coordination phase.

### 8 Coordination Phases

```
INTAKE -> TRIAGE -> PLAN -> REVIEW -> EXECUTE -> VERIFY -> INTEGRATE -> SHIP
```

These phases match exactly between:
- `gentlyos/tree.json` (`coordination.phases`)
- `ptc/engine.py` (`run()` function)

### Approval Cascade

```
coordination.approval_cascade:
  low_1_3:      "captain approves"
  medium_4_6:   "director approves after captain"
  high_7_8:     "CTO approves after director + captain"
  critical_9_10: "human architect final call"
```

Implemented in `ptc/executor.py` `_check_approval()`:

```
Risk 1-3  -> Auto-approved (captain level)
Risk 4-6  -> Logged, proceed (director level)
Risk 7-8  -> BLOCKED, requires CTO approval
Risk 9-10 -> BLOCKED, requires human approval
```

### Data Flow

```
Human intent
  |
  v
INTAKE: load tree + crate graph
  |
  v
TRIAGE: keyword match -> scored node list
  |
  v
PLAN: decompose to leaf tasks (use blast_radius if crates mentioned)
  |
  v
REVIEW: check each task against approval cascade
  |
  v
EXECUTE: run leaf tasks in tier order (tier 0 before tier 3)
  |        modes: native | claude | shell | codie | design | inspect | compose
  |
  v
VERIFY: check results, detect failures, build escalation list
  |
  v
INTEGRATE: aggregate bottom-up, apply rules, fire escalations
  |
  v
SHIP: store trace, store artifacts, return report
```

### Self-Hosting Path

The end state is GentlyOS building itself from inside GentlyOS:

1. Intent: "rebuild gently-core" enters at `root:human`
2. PTC routes to `dept:foundation` -> `capt:types`
3. Blast radius calculation shows all 38 crates affected (tier 0 change)
4. Risk = 7+ (tier 0 rebuild), requires CTO approval
5. After approval, `_execute_native_cargo()` runs `cargo build -p gently-core`
6. Tier rebuild cascades through all affected crates in build order
7. Final step: `nixos-rebuild switch` (risk 9, always human approval)

That final `nixos-rebuild switch` is the self-hosting moment: the system rebuilds itself using its own coordination protocol.
