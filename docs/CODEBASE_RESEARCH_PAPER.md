# GentlyOS & claude-cage: A Sovereignty-First Operating System Architecture

**A Deep Technical Analysis for Developers**

*March 2026 — v1.0*

---

## Abstract

This paper presents a comprehensive technical analysis of the GentlyOS codebase: a sovereignty-first operating system comprising 28 Rust application crates (~80K LOC), a containerized sandbox infrastructure (claude-cage), an Axum + HTMX web dashboard (28K LOC), Move smart contracts on Sui, and a Python orchestration engine. The system implements an 8-layer defense-in-depth security model for running AI agents in isolated containers, a distributed knowledge graph with 96-dimensional hypercube embeddings, a 44-keyword instruction language achieving 94.7% token compression, and a blockchain-backed provenance system for inference quality mining. We examine the architectural decisions, cross-component data flows, security model, and novel abstractions that distinguish this system from conventional approaches.

---

## Table of Contents

1. [System Overview and Design Philosophy](#1-system-overview-and-design-philosophy)
2. [Repository Structure and Dual Workspaces](#2-repository-structure-and-dual-workspaces)
3. [Infrastructure Layer: claude-cage](#3-infrastructure-layer-claude-cage)
4. [Security Model: 8-Layer Defense-in-Depth](#4-security-model-8-layer-defense-in-depth)
5. [GentlyOS Application Layer: 28 Rust Crates](#5-gentlyos-application-layer-28-rust-crates)
6. [The Cryptographic Foundation: gently-core](#6-the-cryptographic-foundation-gently-core)
7. [Alexandria: The Distributed Knowledge Graph](#7-alexandria-the-distributed-knowledge-graph)
8. [CODIE: Compressed Operational Dense Instruction Encoding](#8-codie-compressed-operational-dense-instruction-encoding)
9. [GOO: The Unified Field Dashboard](#9-goo-the-unified-field-dashboard)
10. [Security Layer: FAFO and 16 Daemons](#10-security-layer-fafo-and-16-daemons)
11. [Inference Quality Mining and Three Kings Provenance](#11-inference-quality-mining-and-three-kings-provenance)
12. [PTC: Pass-Through Coordination Engine](#12-ptc-pass-through-coordination-engine)
13. [Organizational Tree: 35-Agent Virtual Organization](#13-organizational-tree-35-agent-virtual-organization)
14. [Sui/Move Smart Contracts](#14-suimove-smart-contracts)
15. [Web Dashboard: cage-web](#15-web-dashboard-cage-web)
16. [MongoDB: Fire-and-Forget Event Store](#16-mongodb-fire-and-forget-event-store)
17. [Cross-Cutting Architectural Patterns](#17-cross-cutting-architectural-patterns)
18. [Dependency Architecture and Build System](#18-dependency-architecture-and-build-system)
19. [Data Flow: Intent to Execution](#19-data-flow-intent-to-execution)
20. [Conclusion](#20-conclusion)

---

## 1. System Overview and Design Philosophy

GentlyOS is built on a singular thesis: **the user retains absolute sovereignty over their computation, data, and identity**. Every architectural decision flows from this principle:

- **Cryptographic split-knowledge**: secrets are XOR-split so neither half alone reveals anything. The Lock never leaves the device; the Key can be public (IPFS, NFT, website). The full secret exists only transiently during a "dance" operation.
- **Container isolation**: AI agents run inside hardened Docker containers with mandatory access controls, capability dropping, syscall filtering, and network egress restrictions.
- **Provenance on-chain**: high-quality inference steps are published to the Sui blockchain as Move resources with Three Kings provenance (WHO, WHAT, WHY), creating an immutable audit trail.
- **Self-training through use**: the inference quality mining loop means the system improves by being used — every interaction feeds a quality scoring pipeline that clusters, aggregates, and optimizes response patterns.

The system is not a monolith. It is a **polyglot architecture** spanning:

| Language | Component | LOC | Purpose |
|----------|-----------|-----|---------|
| Rust | 28 application crates + gently-cli + gentlyos-tui | ~80K | Core application layer |
| Rust | cage-web (Axum + HTMX) | ~28K | Web dashboard |
| Bash | 15 library modules + CLI entry point | ~5K | Container orchestration CLI |
| Python | PTC engine (12 modules) | ~5K | Tree coordination / orchestration |
| JavaScript | MongoDB store.js + seed scripts | ~2K | Fire-and-forget event store |
| Move | 3 smart contracts on Sui | ~500 | NFT, Collection, SYNTH token |
| CODIE | 9 orchestration programs | ~2K | Governance and workflow |

---

## 2. Repository Structure and Dual Workspaces

The repository contains two independent Cargo workspaces with separate dependency trees:

```
claude-cage/                          ← Root workspace (cage-web only)
├── Cargo.toml                        ← workspace members: [cage-web]
├── gentlyos-core/                    ← GentlyOS workspace (28 crates)
│   ├── Cargo.toml                    ← workspace members: [28 crates + 2 binaries]
│   ├── crates/gently-*/              ← All application crates
│   ├── gently-cli/                   ← Main CLI binary
│   └── gentlyos-tui/                 ← Terminal UI binary
├── cage-web/                         ← Infrastructure dashboard
├── lib/                              ← Bash CLI modules
├── docker/                           ← Container definitions
├── security/                         ← Seccomp + AppArmor profiles
├── ptc/                              ← Python Tree Compiler
├── mongodb/                          ← Node.js event store
├── sui/cage_nft/                     ← Move smart contracts
├── gentlyos/                         ← Organizational tree (35 agents)
├── codie-maps/                       ← 9 CODIE orchestration programs
└── docker-compose.yml                ← Service orchestration + security anchor
```

**Why two workspaces?** The root workspace compiles `cage-web` (the infrastructure dashboard) independently of the 28 GentlyOS crates. This means the dashboard can build and deploy without compiling 80K lines of application code. The GentlyOS workspace is self-contained under `gentlyos-core/` and has its own dependency tree optimized for cryptography, ML inference, and blockchain interaction.

---

## 3. Infrastructure Layer: claude-cage

### 3.1 Bash CLI Architecture

The entry point (`bin/claude-cage`, 75 LOC) sources 15 library modules in dependency order and dispatches to `cmd_*()` functions in `lib/cli.sh`:

```
bin/claude-cage
  └── sources (in order):
      config.sh → sandbox.sh → docker.sh → session.sh
      tui.sh → gui.sh → tree.sh → architect.sh → docs.sh
      integrations.sh → mongodb.sh → memory.sh → observability.sh
      lifecycle.sh → cli.sh
```

| Module | LOC | Responsibility |
|--------|-----|----------------|
| `config.sh` | 3.4K | YAML config loading (flat parser), `CAGE_CFG[]` associative array |
| `sandbox.sh` | 8.5K | Security flag construction, network filtering, iptables injection |
| `docker.sh` | 7.6K | Image build, container run/stop/destroy, label management |
| `session.sh` | 5.0K | Session metadata, name generation (`<adjective>-<noun>-<hex4>`) |
| `mongodb.sh` | 4.7K | Fire-and-forget writes via `node store.js`, never blocks CLI |
| `lifecycle.sh` | 9.7K | Idle reaping, memory reaping, garbage collection, session limits |
| `observability.sh` | 7.2K | CPU/memory/network metrics, health checks, dashboard |
| `memory.sh` | 5.1K | Session context compaction (Anthropic cookbook pattern) |
| `tree.sh` | 9.1K | Universal node tree operations, blast radius, intent routing |
| `cli.sh` | ~50K | All 40+ command implementations |

### 3.2 Docker Compose Services

`docker-compose.yml` defines 5 services sharing an `x-common` YAML anchor that establishes the security baseline:

```yaml
x-common: &common
  read_only: true
  security_opt:
    - no-new-privileges
    - seccomp=security/seccomp-default.json
  cap_drop: [ALL]
  cap_add: [CHOWN, DAC_OVERRIDE, SETGID, SETUID]
  tmpfs:
    - /tmp:rw,noexec,nosuid,size=512m
    - /run:rw,noexec,nosuid,size=64m
```

| Service | Image Base | Purpose | Ports |
|---------|-----------|---------|-------|
| `cli` | node:20-slim | Interactive Claude CLI with filtered network | — |
| `desktop` | ubuntu:24.04 | Claude Desktop via noVNC (Xvfb + x11vnc + openbox) | 6080, 5900 |
| `gently` | rust:1.75 (multi-stage) | GentlyOS application (MCP server + health) | 3000, 8080 |
| `ipfs` | ipfs/kubo:latest | IPFS daemon for content-addressed storage | 4001, 5001, 8081 |
| `cli-isolated` | node:20-slim | Claude CLI with `network_mode: none` | — |

Two networks provide isolation:
- **cage-filtered** (172.28.0.0/16): Bridge with ICC disabled (`enable_icc=false`). Containers cannot communicate with each other.
- **gently-internal** (172.29.0.0/24): Internal-only bridge with ICC enabled. Only `gently` and `ipfs` are connected.

### 3.3 Three Container Image Strategies

**CLI Image** (`docker/cli/Dockerfile`): Minimal 48-line Dockerfile. Base `node:20-slim`, installs `@anthropic-ai/claude-code` via npm, runs as non-root `cageuser` behind `tini` (proper PID 1 signal handling).

**Desktop Image** (`docker/desktop/Dockerfile`): 87 lines. Full X11 stack (Xvfb, x11vnc, noVNC, websockify, openbox, xterm) for browser-accessible Claude Desktop sessions.

**GentlyOS Image** (`docker/gentlyos/Dockerfile`): 80-line multi-stage build. Compiles all 28 Rust crates in a `rust:1.75` stage with Cargo layer caching (manifests first, then full build), copies binaries to `debian:bookworm-slim` for a minimal runtime image.

---

## 4. Security Model: 8-Layer Defense-in-Depth

The security model follows the principle that exploitation requires simultaneously breaking through all 8 layers. No single layer is sufficient — the defense is the *intersection* of all constraints.

### Layer 1: Read-Only Root Filesystem

The container's root filesystem is mounted read-only. Only two tmpfs mounts are writable:
- `/tmp` (512MB, `noexec,nosuid`) — runtime temporaries
- `/run` (64MB, `noexec,nosuid`) — PID files and sockets

This prevents persistent malware, filesystem-based privilege escalation, and unauthorized binary drops.

### Layer 2: Capability Dropping

```
cap_drop: ALL
cap_add: CHOWN, DAC_OVERRIDE, SETGID, SETUID
```

All 38 Linux capabilities are dropped. Only 4 are re-added — the minimum required for file ownership changes and user switching inside the container. Critical capabilities like `SYS_ADMIN`, `NET_RAW`, `SYS_PTRACE`, and `DAC_READ_SEARCH` remain dropped.

### Layer 3: Seccomp Syscall Filter

`security/seccomp-default.json` implements a **default-deny** syscall filter (`SCMP_ACT_ERRNO`). Approximately 147 syscalls are explicitly allowlisted across x86_64 and aarch64 architectures. The allowlist includes:

- **I/O**: read, write, open, openat, close, dup, lseek, mmap, mprotect
- **Process**: clone, fork, execve, exit, kill, wait4, waitpid
- **Network**: socket, bind, listen, connect, accept, send, recv
- **Filesystem**: mkdir, rmdir, rename, chmod, chown, stat, access
- **Time/Sync**: clock_gettime, nanosleep, futex, epoll_*

Notably absent: `mount`, `umount2`, `pivot_root`, `init_module`, `kexec_load`, `reboot`, `syslog`, `ptrace` (on some configurations), and `AF_VSOCK` (prevents VM backdoor escape).

### Layer 4: AppArmor Mandatory Access Control

`security/apparmor-profile` defines the `claude-cage` profile with `attach_disconnected,mediate_deleted` flags:

**Denied**: mount/umount/pivot_root, kernel module loading, ptrace (no debugging other processes), `/proc/sys/**` writes, raw/packet network access.

**Allowed**: TCP/UDP (inet/inet6), UNIX sockets, reads/writes to `/workspace/**`, `/home/cageuser/**`, `/tmp/**`, `/run/**`. System binaries execute under `rix` (read, inherit, execute).

### Layer 5: Resource Limits

```
cpus: 2          # CPU cores
mem_limit: 4g    # Memory ceiling
pids_limit: 512  # Process count limit
ulimits:
  nofile: 1024:2048    # File descriptors
  nproc: 256:512       # User processes
```

### Layer 6: Network Filtering

Network filtering uses a **post-launch iptables injection** pattern. After the container starts and receives an IP address on the `cage-filtered` bridge:

1. `sandbox_tier_hosts()` returns tier-appropriate allowed hosts
2. Each hostname is resolved via `getent hosts`
3. `iptables -I DOCKER-USER` rules allow traffic to resolved IPs
4. DNS is allowed on UDP/TCP port 53
5. A final `iptables -A DOCKER-USER -s $container_ip -j DROP` blocks everything else

**Tier-based access**:

| Tier | Allowed Hosts |
|------|--------------|
| free | api.anthropic.com, cdn.anthropic.com |
| basic | + github.com, pypi.org, files.pythonhosted.org |
| pro | + registry-1.docker.io, registry.npmjs.org |
| dev/founder/admin | Full outbound (no filtering) |

### Layer 7: no-new-privileges

The `no-new-privileges` flag prevents processes inside the container from gaining additional privileges via setuid binaries, capability escalation, or similar mechanisms.

### Layer 8: Bridge Network with ICC Disabled

The `cage-filtered` bridge network has inter-container communication (ICC) explicitly disabled:

```yaml
driver_opts:
  com.docker.network.bridge.enable_icc: "false"
```

Even if multiple containers are on the same network, they cannot communicate with each other. Only egress through the bridge to allowed hosts is permitted.

---

## 5. GentlyOS Application Layer: 28 Rust Crates

The 28 crates are organized into 5 domain groups. The dependency graph is strictly **acyclic** — no circular imports exist between crates.

### Dependency Tier Architecture

```
Tier 0 (Foundation — no internal deps):
  gently-core, gently-codie, gently-audio, gently-visual, gently-goo, gently-artisan

Tier 1 (Abstraction — uses only Tier 0):
  gently-search, gently-cipher, gently-btc, gently-feed, gently-dance,
  gently-network, gently-sandbox, gently-guardian, gently-ptc, gently-micro

Tier 2 (Knowledge — uses Tiers 0-1):
  gently-alexandria, gently-ipfs, gently-chain, gently-inference,
  gently-security, gently-mcp, gently-sim

Tier 3 (Intelligence — uses Tiers 0-2):
  gently-brain, gently-architect, gently-gateway, gently-web, gently-sploit

Tier 4 (Integration — uses everything):
  gently-cli (21 commands), gentlyos-tui (6 panels, 7 LLM providers)
```

### Domain Group Breakdown

**Alexandria (Knowledge)**: 5 crates — gently-alexandria (knowledge graph), gently-search (semantic routing), gently-inference (quality mining), gently-feed (living feed), gently-codie (instruction language)

**BS-Artisan (Craftsmanship)**: 9 crates — gently-artisan (toroidal storage), gently-core (crypto), gently-ipfs (CAS), gently-chain (Sui), gently-ptc (tree coordination), gently-architect (code gen), gently-brain (LLM), gently-mcp (MCP server), gently-micro (ESP32)

**FAFO (Defense)**: 6 crates — gently-security (16 daemons + FAFO), gently-sandbox (seccomp/AppArmor), gently-guardian (hardware detection), gently-cipher (ciphers), gently-network (packet capture), gently-sploit (exploitation framework)

**GOO (GUI)**: 5 crates — gently-goo (unified field), gently-visual (SVG), gently-audio (FFT), gently-dance (P2P protocol), gently-web (HTMX GUI)

**Other**: 3 crates — gently-btc (Bitcoin RPC), gently-gateway (API routing), gently-sim (SIM card security)

---

## 6. The Cryptographic Foundation: gently-core

`gently-core` (~2,954 LOC, 98% complete) is the Tier 0 foundation that all other crates build upon. It implements an **XOR split-knowledge security model**:

```
LOCK (Device A)  ⊕  KEY (Public)  =  FULL_SECRET
     │                  │                 │
     │                  │                 └── Only exists during "dance"
     │                  └── Can be anywhere (IPFS, NFT, website)
     └── NEVER leaves your device
```

### Module Structure

| Module | Purpose |
|--------|---------|
| `blob.rs` | Merkle blob storage: Hash, Tag (16 tag constants), Kind, Blob, Ref, Manifest, Index, BlobStore |
| `crypto/genesis.rs` | Genesis key generation and hierarchical derivation |
| `crypto/xor.rs` | `split_secret()` → Lock + Key pair. Neither half reveals the original. |
| `crypto/derivation.rs` | HKDF-SHA256 key derivation pipeline |
| `crypto/berlin.rs` | Berlin Clock: BTC-synced time-based key rotation (380 LOC) |
| `pattern/encoder.rs` | Visual + audio instruction encoding |
| `vault.rs` | Encrypted key vault with signature verification |

### Key Types

```rust
// Cryptographic primitives
pub struct GenesisKey { /* 32-byte root key */ }
pub struct SessionKey { /* derived per-session */ }
pub struct Lock { /* XOR half A — never leaves device */ }
pub struct Key  { /* XOR half B — can be public */ }
pub struct FullSecret { /* transient: Lock ⊕ Key */ }

// Time-based rotation
pub struct BerlinClock { /* BTC block-synced rotation */ }
pub struct TimeKey { /* time-derived key material */ }

// Content-addressed storage
pub struct Hash([u8; 32]);     // blake3 hash
pub struct Blob { hash: Hash, tags: Vec<Tag>, data: Vec<u8> }
pub struct BlobStore { /* persistent blob storage */ }
```

The **Berlin Clock** mechanism ties key rotation to Bitcoin block timestamps, creating a deterministic time source that cannot be manipulated without controlling the Bitcoin network. This enables time-locked secrets that expire naturally as the blockchain progresses.

---

## 7. Alexandria: The Distributed Knowledge Graph

`gently-alexandria` (~4,949 LOC, 85% complete) implements a **usage-driven distributed knowledge graph** built on frozen model weights. The central insight is that LLM weights already encode human knowledge — Alexandria provides the card catalog.

### The Five Query Dimensions

```
User: "What is X?"
Alexandria:
├── Forward:     "X is Y"                              (standard inference)
├── Rewind:      "Questions that lead to Y: [A, B, C]" (reverse provenance)
├── Orthogonal:  "X secretly connected to: [P, Q, R]"  (cross-domain links)
├── Reroute:     "Alternative proof: X→M→N→Y"          (path diversity)
└── Map:         *shows entire local topology*          (neighborhood view)
```

### SemanticTesseract: 8-Face 96-Dimensional Embedding

Alexandria's embedding space is not a flat vector — it is a **hypercube with 8 faces** (a tesseract). Each face encodes 12 dimensions for a total of 96 dimensions:

```rust
pub const DIMS_PER_FACE: usize = 12;
pub const TOTAL_DIMS: usize = 96; // 8 faces × 12 dims

pub struct SemanticTesseract {
    faces: [FaceEmbeddings; 8],
}

pub struct HyperPosition {
    face: usize,        // which face (0-7)
    coords: [f32; 12],  // position on that face
}
```

The 8 faces correspond to different semantic dimensions (e.g., factual, procedural, causal, analogical, temporal, spatial, social, emotional), enabling queries that navigate between faces via **wormholes** — cross-context semantic jumps.

### Distributed Sync Protocol

```rust
pub struct DistributedWormhole { /* IPFS pubsub + delta sync */ }
pub struct GraphDelta { /* incremental graph changes */ }
pub struct SyncProtocol {
    pubsub_topic: String,     // "/alexandria/deltas/v1"
    sync_interval_secs: u64,  // default: 60
}
```

Knowledge graphs synchronize via IPFS pubsub, exchanging deltas rather than full snapshots. Concepts have a decay half-life (default: 30 days) — unused knowledge gradually fades unless refreshed through use.

---

## 8. CODIE: Compressed Operational Dense Instruction Encoding

`gently-codie` (~7,505 LOC, 80% complete) is a 44-keyword instruction language that achieves **94.7% token reduction** versus natural language while remaining human-readable.

### Keyword Taxonomy

**Core 12 Semantic Keywords (dog-themed):**

| Keyword | Meaning | Example |
|---------|---------|---------|
| `pug` | Entry point / program | `pug LOGIN` |
| `bark` | Fetch / data binding | `bark user ← @db` |
| `chase` | Loop | `chase items` |
| `trick` | Function definition | `trick validate(input)` |
| `tag` | Variable | `tag counter = 0` |
| `sniff` | Validate | `sniff email format` |
| `fence` | Constraints/guards | `fence { admin_only }` |
| `sit` | Exact specification | `sit output = JSON` |
| `bone` | Immutable rule | `bone NOT: bypass_security` |
| `play` | Flexible/creative | `play with layout` |
| `treat` | Goal/return value | `treat → token` |
| `bury` | Checkpoint/save | `bury state` |

**Additional categories**: 6 logic gates (`and`, `or`, `not`, `xor`, `nand`, `nor`), 10 control flow keywords, 2 booleans (`wag`=true, `whine`=false), 5 geometric transforms, 5 dimensional operators, 4 meta/generation keywords.

### Compression Pipeline: Dehydration and Hydration

```
Human Form:                         Glyph Form:
pug LOGIN                           ρLOGIN⟨βuser←@db⟨⁇¬found→⊥⟩μ→token⟩
├── bark user ← @db
│   └── ? not found → whine         Hash: #c7f3a2b1
└── treat → token
```

**Dehydrate**: Human CODIE → Glyph string (60-80% smaller). Each keyword maps to a Unicode glyph (ρ, β, ⁇, ⊥, μ). Tree structure maps to angle bracket nesting.

**Hydrate**: Glyph string → Human CODIE. Fully reversible — no information loss.

**Hash-addressable**: Every CODIE program has a content-addressed hash. Programs can be passed as strings, stored by hash, and instantly hydrated from compressed form.

### CODIE as Executable Governance

The 9 `.codie` files in `codie-maps/` encode organizational governance as executable programs. For example, `master-orchestration.codie` (342 lines) defines:

```codie
pug MASTER_ORCHESTRATOR
|
+-- fence ABSOLUTE_LAWS
|   +-- bone NOT: bypass_guardian_database
|   +-- bone NOT: disable_security_daemons
|   +-- bone NOT: skip_validation_steps
|
+-- elf config <- bark @guardian/config
+-- cali ROUTE_REQUEST
|   +-- ? request_type == "change" -> cali HANDLE_CHANGE(request)
|   +-- ? request_type == "query"  -> cali HANDLE_QUERY(request)
+-- biz -> orchestrated_result
```

This is not documentation — it is parseable, hashable, and executable governance that the PTC engine can interpret and enforce.

---

## 9. GOO: The Unified Field Dashboard

`gently-goo` (~3,379 LOC, 80% complete, 70+ tests) implements a radical unification: **GUI rendering, attention routing, and machine learning are the same mathematical operation** controlled by a single parameter `k`.

### The Central Mathematical Insight

```
smooth_min(a, b, k):
    h = max(k - |a-b|, 0) / k
    result = min(a, b) - h*h*k*0.25
```

This function is the **dual of softmax**: `smooth_min(k)` ≡ `softmax(1/k)`. The same parameter `k` controls:

- **Visual blobbiness** — SDF (Signed Distance Field) blending between shapes. Higher k = more blending, shapes merge. Lower k = sharp edges.
- **Attention softness** — query temperature over sources. Same k value determines how broadly attention spreads across sources.
- **Learning smoothness** — gradient dampening across the field. Same k controls learning rate behavior.

### Architecture

```rust
pub struct GooEngine {
    pub field: GooField,                 // G(x,y,t,theta) — the unified field
    pub sources: Vec<GooSource>,         // SDF primitives (Sphere, Box, Torus, Line)
    pub render_config: RenderConfig,     // Pixel buffer output
    pub rhythm: Rhythm,                  // BPM-synced animation heartbeat
    pub sovereignty: SovereigntyGuard,   // Boundary/consent protection
    pub claude_avatar: Option<ClaudeAvatar>, // Claude's embodiment in the field
}
```

Everything is `G(x, y, t, θ)`. One function renders the GUI, routes attention, and drives learning. There is no separation between "display" and "computation" — **the field IS the interface**.

Key operations:
- `engine.tick(dt)` — advance simulation time
- `engine.sample(x, y)` — evaluate the field at a point
- `engine.render()` — produce RGBA pixel buffer
- `engine.query_attention(query)` — field queries = attention mechanism
- `engine.compute_gradient(target)` — learning = gradient descent in field space
- `engine.apply_gradient(steps, lr)` — update field based on gradients

### SovereigntyGuard

Even the GUI respects sovereignty. The `SovereigntyGuard` tracks consent and enforces boundary policies. FAFO severity levels apply — unauthorized field access triggers the same escalation ladder as security violations.

---

## 10. Security Layer: FAFO and 16 Daemons

`gently-security` (~8,576 LOC, 95% complete, 56 tests) goes far beyond passive defense. The FAFO (Find Around, Find Out) subsystem **actively punishes attackers** through an escalating response ladder.

### FAFO Response Ladder

```
Strike 1     GROWL      Warning logged, threat remembered
Strike 2     TARPIT     Waste attacker's time (5s → 30s delays)
Strike 3     POISON     Inject false info into attacker's context
Strike 5     DROWN      Flood with honeypot garbage
Strike 10    DESTROY    Permanent ban, nuke all sessions
CRITICAL     SAMSON     Scorched earth — burn everything down
```

The system remembers repeat offenders and increases aggression with each strike. Four operating modes provide granular control:

```rust
pub enum FafoMode {
    Passive,     // Log only, no active response
    Defensive,   // Isolate + tarpit
    Aggressive,  // Active countermeasures (poison, drown)
    Samson,      // Scorched earth (nuclear option)
}
```

### 16 Security Daemons

| Category | Daemons | Purpose |
|----------|---------|---------|
| Foundation | HashChainValidator, BtcAnchor, ForensicLogger | Integrity verification, Bitcoin timestamping, evidence chain |
| Traffic | TrafficSentinel, TokenWatchdog, CostGuardian | Rate monitoring, token leakage detection, spend limits |
| Detection | PromptAnalyzer, BehaviorProfiler, PatternMatcher, AnomalyDetector | Injection detection (28 indicators), behavioral baselines, anomaly scoring |
| Defense | SessionIsolator, TarpitController, ResponseMutator, RateLimitEnforcer | Container isolation, time-wasting, response corruption, throttling |
| Intel | ThreatIntelCollector, SwarmDefense | Threat intelligence aggregation, coordinated defense |

### SecurityController Orchestration

```rust
pub struct SecurityController {
    distiller: TokenDistiller,     // Detect token leakage
    limiter: RateLimiter,          // 5-layer rate limiting
    detector: ThreatDetector,      // Jailbreak/injection detection
    trust: TrustSystem,            // Assume-hostile trust management
    honeypot: HoneypotSystem,      // AI-irresistible traps
    fafo: FafoController,          // Escalating response
}
```

The **HoneypotSystem** deserves special note: it generates fake credentials, API keys, and endpoints that appear legitimate to AI models. When an AI agent attempts to use them, it immediately reveals malicious intent and triggers FAFO escalation.

---

## 11. Inference Quality Mining and Three Kings Provenance

`gently-inference` (~4,609 LOC, 90% complete) implements a **collective optimization loop** where the network trains itself through use.

### Quality Mining Pipeline

```
LLM Response ──► DECOMPOSE ──► Steps[] ──► SCORE ──► CLUSTER ──► AGGREGATE ──► OPTIMIZE
                     │                        │           │           │            │
                Alexandria              quality ≥ 0.7  semantic   high-quality  synthesize
                (link concepts)         filter       grouping    patterns      best response
```

1. **Decompose**: Break an LLM response into discrete reasoning steps
2. **Score**: Quality = user_accept×0.3 + outcome_success×0.4 + chain_referenced×0.2 + turning_point×0.1
3. **Cluster**: Group semantically similar steps across interactions
4. **Aggregate**: Extract patterns from high-quality clusters
5. **Optimize**: Synthesize the best response from aggregated patterns

### Step Types and GENOS Reward Multipliers

| Step Type | GENOS Multiplier | Description |
|-----------|-----------------|-------------|
| Conclude | 12× | Research synthesis |
| Pattern | 10× | Creative insight |
| Eliminate | 8× | BONEBLOB contribution |
| Specific | 6× | Implementation detail |
| Fact | 5× | Verified data |
| Suggest | 4× | Ideas |
| Correct | 3× | Bug fixes |
| Guess | 1× | Low until validated |

### Three Kings Provenance

When a step's quality score exceeds the threshold (0.7), it becomes eligible for on-chain publication:

```rust
pub struct ThreeKingsProvenance {
    pub gold: String,         // WHO — identity hash (blake3)
    pub myrrh: String,        // WHAT — model/context preservation hash
    pub frankincense: String, // WHY — intention hash
}
```

These three hashes are published as `ReasoningStep` Move resources on the Sui blockchain, creating an immutable provenance record. The naming references the biblical Three Kings, where each gift represents a different dimension of proof:

- **Gold** (WHO): Proves which agent or human created the reasoning
- **Myrrh** (WHAT): Preserves what model, context, and task produced it
- **Frankincense** (WHY): Records the intention behind the reasoning

---

## 12. PTC: Pass-Through Coordination Engine

The PTC (Pass-Through Coordination) engine is the orchestration nervous system. It exists in two implementations: a Rust crate (`gently-ptc`, ~1,084 LOC) for native Rust integration, and a Python engine (`ptc/engine.py`, ~900 LOC) for operational orchestration.

### The 8-Phase Pipeline

```
INTAKE → TRIAGE → PLAN → REVIEW → EXECUTE → VERIFY → INTEGRATE → SHIP
```

| Phase | Action | Description |
|-------|--------|-------------|
| **INTAKE** | Load tree, log intent | Parse `tree.json`, record incoming intent to MongoDB |
| **TRIAGE** | Route intent to nodes | Keyword matching against node metadata (crates_owned, functions, files) |
| **PLAN** | Decompose to leaf tasks | Walk down from matched departments to leaf workers |
| **REVIEW** | Approval gates | Risk-based blocking: Captain (≤3) → Director (≤6) → CTO (≤8) → Human (≤10) |
| **EXECUTE** | Run approved tasks | 6 executor modes: design, claude, shell, inspect, compose, native |
| **VERIFY** | Check results | Detect failures, calculate escalations |
| **INTEGRATE** | Aggregate up-tree | Apply rules at each node level (pass/block/transform/escalate) |
| **SHIP** | Store trace, report | Full trace to MongoDB as `ptc-trace-{run_id}`, return final status |

### Crate Graph and Blast Radius

`ptc/crate_graph.py` loads the dependency graph from `crate-graph.json` and provides:

- **`blast_radius(changed_crates)`**: Returns all transitively affected crates + nodes + risk score
- **`build_order(crates)`**: Topological sort by tier for correct compilation order
- **Risk scoring**: 80%+ affected crates = risk 9, 50%+ = 7, 30%+ = 6. Any tier-0 change automatically sets risk ≥ 7.

### Executor Modes

| Mode | Tool | Purpose |
|------|------|---------|
| `design` | — | Produce blueprints (no code changes) |
| `claude` | Claude Code subprocess | Invoke Claude for code generation |
| `shell` | Bash | Run shell commands |
| `inspect` | File I/O | Read files, analyze state |
| `compose` | Aggregation | Combine outputs from multiple sources |
| `native` | Python | Direct function invocation |

### Rust PTC Engine

The Rust implementation (`gently-ptc`) provides the same pipeline with a trait-based pluggable architecture:

```rust
pub trait Executor: Send + Sync {
    async fn execute(&self, task: &LeafTask) -> Result<LeafResult>;
}

pub trait PtcStorage: Send + Sync {
    async fn store_event(&self, event: &PtcEvent) -> Result<()>;
}

pub struct PtcEngine {
    pub tree: Arc<Tree>,
    pub executor: Box<dyn Executor>,
    pub storage: Box<dyn PtcStorage>,
}
```

---

## 13. Organizational Tree: 35-Agent Virtual Organization

The organizational tree (`gentlyos/tree.json`) defines a 35-node hierarchy following Google monorepo coordination patterns mapped onto the Kabbalistic Tree of Life:

### Hierarchy

```
root:human (1)
├── exec:cto (1)
│   ├── dept:foundation     → gently-core, gently-artisan
│   ├── dept:protocol       → gently-alexandria, gently-search
│   ├── dept:security       → gently-security, gently-sandbox
│   ├── dept:orchestration  → gently-ptc, gently-codie
│   ├── dept:runtime        → gently-brain, gently-gateway
│   ├── dept:tokenomics     → gently-chain, gently-inference
│   ├── dept:devops         → Docker, bash CLI, cage-web
│   └── dept:interface      → gently-web, gently-mcp
├── exec:vision (1)
│   └── ... design/strategy departments
└── 24 captains (leaf workers)
```

### Universal Node Schema

Every node — from human to captain to crate — instantiates the **same schema**:

```json
{
  "id": "scale:name",
  "name": "Human-readable name",
  "scale": "executive|department|captain|crate|module|sephira",
  "parent": "parent_id",
  "children": ["child_ids"],
  "inputs": [{"name": "...", "type": "task|data|event|dependency", "from": "..."}],
  "outputs": [{"name": "...", "type": "result|artifact|event|decision", "to": "..."}],
  "rules": [{"name": "...", "condition": "...", "action": "pass|block|transform|escalate|log"}],
  "escalation": {"target": "node_id", "threshold": 1-10, "cascade": ["path"]},
  "metadata": {"agent_id": "...", "crates_owned": [...], "sephira_mapping": "..."}
}
```

### Tree of Life Mapping

| Sephira | Department | Crate Domain | Principle |
|---------|-----------|-------------|-----------|
| Keter (Crown) | Interface | gently-web, gently-mcp | Top-level access |
| Chokmah/Binah (Wisdom/Understanding) | Protocol | gently-alexandria, gently-search | Knowledge duality |
| Daath (Hidden Knowledge) | Security | gently-security, gently-sandbox | Concealed protection |
| Chesed/Gevurah (Mercy/Strength) | DevOps | Docker, bash CLI, cage-web | Infrastructure balance |
| Tiferet (Beauty/Harmony) | Orchestration | gently-ptc, gently-codie | Central coordination |
| Netzach/Hod (Eternity/Splendor) | Runtime | gently-brain, gently-gateway | Execution duality |
| Yesod (Foundation) | Tokenomics | gently-chain, gently-inference | Value/proof |
| Malkuth (Kingdom) | Foundation | gently-core, gently-artisan | Physical substrate |

### Approval Cascade

Risk levels determine which tier of the tree must approve:

| Risk Level | Approver | Example |
|-----------|----------|---------|
| 1-3 | Captain (leaf worker) | Rename a variable, add a test |
| 4-6 | Director (department head) | Refactor a module, add a dependency |
| 7-8 | CTO (executive) | Cross-department change, API breaking change |
| 9-10 | Human (root) | Security policy change, data deletion |

---

## 14. Sui/Move Smart Contracts

Three Move modules in `sui/cage_nft/` implement the on-chain economic layer:

### nft.move — Object-Based NFTs

```move
public struct CageNFT has key, store {
    id: UID,
    name: String,
    description: String,
    image_url: String,
    creator: address,
    collection_id: ID,
}
```

Operations: `mint()`, `mint_to_collection()`, `burn()` (linear destruction), `transfer_nft()`. Events: `NFTMinted`, `NFTBurned`.

### collection.move — Capability-Gated Collections

```move
public struct Collection has key, store { /* shared object */ }
public struct CollectionCap has key, store { /* only creator holds this */ }
```

The `CollectionCap` pattern enforces that only the collection creator can mint new items. The collection itself is a shared object requiring consensus for mutations.

### synth_token.move — SYNTH (Proof-of-Reasoning Token)

```move
public struct SYNTH_TOKEN has drop {} // One-Time Witness
```

SYNTH is a standard Sui coin created via the One-Time Witness pattern (OTW). Key properties:

- **9 decimal places** (standard Sui precision)
- **No `copy` or `drop`** on the coin — impossible to duplicate or forget value
- `mint()` requires `TreasuryCap` (only cap holder can inflate supply)
- `burn()` linearly destructs the coin (conservation of value)
- Metadata is frozen on-chain (immutable after creation)

---

## 15. Web Dashboard: cage-web

`cage-web/` (~28K LOC) is a Rust web server built on Axum 0.8 + HTMX 2.0 + Askama templates. It serves as the operational interface for the entire system.

### Architecture

```rust
// main.rs — app state shared across all routes
pub struct AppState {
    pub cage_root: PathBuf,
    pub store_js: PathBuf,
    pub tree_json: PathBuf,
    pub codie_dir: PathBuf,
    pub programs: RwLock<Vec<Program>>,  // cached CODIE programs
}
```

17 route modules are merged into a single Axum router, served at `0.0.0.0:5000`.

### Subprocess Bridge Pattern

cage-web does not implement Docker, MongoDB, or PTC operations natively. Instead, it shells out to the existing CLI tools:

- **Docker**: `docker ps`, `docker inspect`, `docker logs`, `docker stop/start/rm`
- **Sessions**: `/bin/claude-cage start --mode {mode} --network {network}`
- **MongoDB**: `node store.js {command}` (40+ subcommands)
- **PTC**: `python3 -m ptc.engine --tree=... --intent=...`

This ensures the web dashboard and CLI always produce identical behavior.

### CODIE Parser (Native Rust)

The `codie_parser.rs` module (808 LOC) implements a full CODIE parser in Rust:

- Tokenizes 44 keywords into AST nodes
- Supports pipe-tree prefix stripping (`├──`, `└──`, `+--`)
- Brace block collection for nested structures
- Indentation-based child collection
- Keyword counting for program metrics
- Routes: `GET /codie` (list), `GET /codie/{name}` (detail), `POST /codie/{name}/execute` (run via PTC)

---

## 16. MongoDB: Fire-and-Forget Event Store

`mongodb/store.js` (~971 LOC) consolidates what would traditionally be 6+ systems into a single MongoDB Atlas cluster:

| Replaced System | MongoDB Feature |
|----------------|----------------|
| Redis | `cache` collection + TTL indexes |
| Message Queue | `tasks` collection + atomic `findOneAndUpdate` |
| Actor Registry | `agents` collection + heartbeat tracking |
| Vector Database | Atlas `$vectorSearch` + cosine fallback |
| Analytics DB | `analytics` collection + hourly time buckets |
| Event Store | `events` collection + structured logging |

### 40+ Commands

| Category | Commands |
|----------|----------|
| CRUD | put, log, get, search, aggregate, bulk, distinct, stats, ping, count |
| Cache | cache-set, cache-get, cache-del |
| Queue | queue-push, queue-pop, queue-ack, queue-fail, queue-stats |
| Agents | agent-register, agent-heartbeat, agent-list, agent-get, agent-deregister |
| Vectors | vector-upsert, vector-search |
| Feed | feed-post, feed-get, feed-boost |
| RLAIF | rlaif-capture, rlaif-export, rlaif-stats |
| Analytics | analytics-inc, analytics-get, analytics-top |

### Fire-and-Forget Pattern

All MongoDB writes from the bash CLI are backgrounded and disowned:

```bash
mongo_log "docker" "build:cli" >/dev/null 2>&1 &
disown 2>/dev/null
```

This ensures MongoDB latency never impacts CLI responsiveness. If MongoDB is unreachable, writes silently fail without affecting operations.

---

## 17. Cross-Cutting Architectural Patterns

### Pattern 1: Universal Node Schema

Every organizational element — human, department, captain, crate, module, sephira — instantiates the same JSON schema. Only the `scale` field differs. This enables a single tree parser, a single routing algorithm, and a single blast radius calculator to operate across all levels.

### Pattern 2: Fire-and-Forget Logging

MongoDB writes never block execution. All logging is backgrounded and disowned. This creates an **eventually-consistent audit trail** — the system optimizes for operational speed over logging completeness.

### Pattern 3: Dual-Write Storage

Artifacts are written to both MongoDB (fast search, operational queries) and IPFS (immutable, decentralized, content-addressed). The writes are asynchronous and independent — neither blocks the other.

### Pattern 4: Trait-Based Pluggability

Critical interfaces are defined as traits, enabling dependency injection:

```rust
pub trait Executor: Send + Sync { ... }      // PTC execution
pub trait PtcStorage: Send + Sync { ... }    // Event persistence
pub trait ChainHook: Send + Sync { ... }     // Blockchain publishing
pub trait CascadeModel: Send + Sync { ... }  // ML pipeline stages
```

### Pattern 5: Linear Types for Value Safety

Move's linear type system on Sui ensures:
- NFTs cannot be duplicated (`no copy`)
- Tokens cannot be forgotten (`no drop` without explicit burn)
- Value is always conserved across operations
- Capabilities (minting authority) have unique ownership

### Pattern 6: Tiered Access Control

Access control scales from hobbyist to enterprise without code changes:
- `free`: Anthropic API only
- `basic`: + GitHub, PyPI
- `pro`: + Docker, npm registries
- `dev/founder/admin`: Full outbound

### Pattern 7: Content-Addressable Everything

CODIE programs, knowledge graph nodes, blob storage, IPFS objects, and Three Kings provenance are all content-addressed via blake3 hashing. This enables:
- Deduplication across the entire system
- Integrity verification at every layer
- Programs passed as hash references instead of full payloads

---

## 18. Dependency Architecture and Build System

### Cargo Workspace Configuration

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "thin"         # Thin link-time optimization
codegen-units = 1    # Single codegen unit (slower build, better code)
strip = true         # Strip debug symbols
panic = "abort"      # Abort instead of unwind (smaller binary)

[profile.release-small]
inherits = "release"
opt-level = "z"      # Minimize binary size over speed
lto = true           # Full LTO
```

### External Dependency Categories

| Category | Crates | Count |
|----------|--------|-------|
| Crypto | sha2, hmac, hkdf, argon2, chacha20poly1305, blake3, zeroize | 7 |
| Async | tokio, async-trait, futures | 3 |
| Serialization | serde, serde_json, serde_yaml | 3 |
| Network | reqwest, rustls, hyper, trust-dns-resolver, pnet | 5 |
| CLI/TUI | clap, crossterm, ratatui, indicatif | 4 |
| Math/Rendering | glam, image | 2 |
| Blockchain | bitcoin (BTC RPC) | 1 |
| IPFS | ipfs-api-backend-hyper, cid, multihash | 3 |
| ML (optional) | ort, fastembed, candle | 3 |
| Database | rusqlite (bundled) | 1 |

### Makefile: 50+ Targets

The root Makefile orchestrates the entire system with targets spanning:

| Category | Targets | Examples |
|----------|---------|---------|
| Build | 5 | build, build-cli, build-desktop, build-gently, build-web |
| Run | 4 | run-cli, run-desktop, run-isolated, run-gently |
| MongoDB | 8 | mongo-install, mongo-ping, mongo-status, mongo-seed, mongo-search |
| Tree/PTC | 7 | tree, ptc, ptc-live, ptc-leaves, route, execute, ship |
| Docs | 8 | docs-generate, docs-status, docs-check, docs-interconnect, docs-search |
| Training | 4 | train-extract, train-pipeline, train-stack, train-preview |
| Federation | 5 | fork-init, fork-status, fork-pull, fork-push, fork-verify |
| Secrets | 6 | mia-init, mia-encrypt, mia-pin, mia-spawn, mia-list, mia-status |

---

## 19. Data Flow: Intent to Execution

The complete lifecycle of an intent flowing through the system:

```
1. User submits intent
   ├── via CLI:      claude-cage ptc run "add GPU monitoring"
   ├── via Web:      POST /codie/master-orchestration/execute
   └── via Makefile: make route INTENT="add GPU monitoring"

2. PTC INTAKE
   ├── Load tree.json (35 nodes)
   └── Log: ptc:phase INTAKE → MongoDB (fire-and-forget)

3. PTC TRIAGE
   ├── Route intent via keyword matching
   ├── Score nodes by relevance (crates_owned, files, functions)
   └── Top matches: dept:devops, capt:metrics, capt:docker

4. PTC PLAN
   ├── Crate graph: blast_radius(["gently-guardian"]) → 4 affected crates
   ├── Risk score: 5 (department-level approval needed)
   ├── Decompose to leaf tasks:
   │   ├── capt:metrics → "add GPU metric collection"
   │   └── capt:docker → "update Dockerfile for nvidia-smi"
   └── Build order: tier 0 deps first

5. PTC REVIEW
   ├── Risk 5 → Director approval required
   └── dept:devops rules: check if breaking_change && no_migration → block

6. PTC EXECUTE
   ├── capt:metrics: executor=claude → call Claude Code subprocess
   └── capt:docker: executor=shell → modify Dockerfile

7. PTC VERIFY
   ├── Check compilation: cargo build -p gently-guardian
   └── Check tests: cargo test -p gently-guardian

8. PTC INTEGRATE
   ├── Aggregate results up through dept:devops
   ├── Apply rules: sovereignty_check → pass
   └── Escalation check: risk 5 < CTO threshold 9 → no escalation

9. PTC SHIP
   ├── Store trace: ptc-trace-{run_id}.json → MongoDB
   ├── Quality mining: decompose reasoning → score steps
   ├── If quality ≥ 0.7: Three Kings → Sui (optional)
   └── Return: {status: "completed", tasks: 2, passed: 2}
```

---

## 20. Conclusion

GentlyOS represents an unconventional approach to operating system design: rather than starting from a kernel and building up, it starts from **sovereignty principles** and builds down. The resulting architecture is characterized by:

1. **Cryptographic sovereignty**: The XOR split-knowledge model ensures no single point of secret disclosure. The Berlin Clock ties key rotation to an external, immutable time source (Bitcoin blocks).

2. **Defense-in-depth without compromise**: 8 independent security layers mean that container exploitation requires simultaneously defeating filesystem restrictions, capability controls, syscall filters, MAC policies, resource limits, network filtering, privilege controls, and network isolation.

3. **Self-improving inference**: The quality mining loop means the system becomes more capable through use. High-quality reasoning steps are clustered, aggregated, and optimized — then optionally published to the blockchain as immutable provenance records.

4. **Unified mathematics**: The GOO field collapses the distinction between rendering, attention, and learning into a single mathematical object. This is not merely an optimization — it is a philosophical statement that perception, thought, and adaptation are the same operation at different scales.

5. **Executable governance**: CODIE programs encode organizational rules as parseable, hashable, and executable instructions. Governance is not a separate layer — it is code that runs through the same PTC pipeline as any other task.

6. **Polyglot pragmatism**: Rather than forcing everything into one language, the system uses Rust where performance and safety matter (core application, web dashboard), Python where flexibility matters (orchestration, ML), Bash where shell integration matters (container management), Move where linear types matter (value conservation), and JavaScript where ecosystem matters (MongoDB).

The total system — 28 Rust crates, 15 Bash modules, 12 Python modules, 3 Move contracts, 9 CODIE programs, and the web dashboard — operates as a coherent whole through the universal node schema and the PTC coordination protocol. Every component, from the lowest-level cryptographic primitive to the highest-level organizational tree, speaks the same language of nodes, edges, rules, and escalation cascades.

For developers approaching this codebase: start with `gently-core` (the cryptographic foundation), then read `gently-ptc` (the coordination protocol), then `gently-security` (the defense layer). Everything else is built on these three pillars.

---

*Document generated from source analysis of the claude-cage repository at commit HEAD, March 2026.*
