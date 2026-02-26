# GentlyOS - Claude Context

**Last Updated**: 2026-02-25
**Lines of Code**: ~79,000+ (existing)
**Crates**: 28 Rust crates + TUI (gently-spl deleted, 4 new crates added)
**ISO**: `dist/gentlyos-alpine-1.1.1-x86_64.iso` (232MB, bootable)
**Economic Layer**: Sui/Move (Solana removed)

---

## What Claude Code Built

### Sui/Move Architecture Pivot (Session 8 - 2026-02-25)

Solana is dead, Sui/Move is the new economic layer. Move linear types replace JSON schemas — resources are physical, the compiler is the physics engine.

**Deleted:**
- `crates/gently-spl/` (8 files, ~5k LOC) — Solana SPL integration
- `contracts/solana/` (811 LOC) — Anchor rewards program
- All disabled CLI commands (Install/Mint/Wallet/Token/Certify/Perm/Genos)
- `solana-sdk`, `solana-client` workspace dependencies

**Created 4 new crates:**

```
gently-chain/         # Sui Rust SDK wrapper ✓ BUILT (7 files)
├── client.rs         # SuiClient, SuiNetwork (Devnet/Testnet/Mainnet)
├── objects.rs        # SuiObject, ObjectOwner, ObjectQuery
├── transactions.rs   # PtbBuilder (Programmable Transaction Blocks)
├── events.rs         # EventFilter, SuiEvent subscription
├── types.rs          # ReasoningStep (Move resource), ObjectID, AnchoredContent
└── three_kings.rs    # Gold/Myrrh/Frankincense provenance (blake3 hashes)

gently-ptc/           # PTC Brain engine ✓ BUILT (9 files)
├── tree.rs           # UniversalNode, NodeScale, Tree (JSON loading)
├── decompose.rs      # route_intent() keyword scoring, walk_down() DFS
├── aggregate.rs      # Result aggregation (status priority merging)
├── executor.rs       # 5 modes: Design/Inspect/Shell/Claude/Plan
├── task.rs           # LeafTask, LeafResult, TaskStatus
├── escalation.rs     # EscalationLevel (Info→Emergency)
├── phase.rs          # 7 phases: INTAKE→TRIAGE→PLAN→EXECUTE→VERIFY→INTEGRATE→SHIP
└── storage.rs        # PtcEvent, PtcStorage async trait

gently-sandbox/       # Agent isolation ✓ BUILT (7 files + 2 configs)
├── seccomp.rs        # SeccompProfile, 34 syscall allowlist
├── apparmor.rs       # AppArmorProfile generation
├── capabilities.rs   # Linux capability drop (10 variants)
├── limits.rs         # ResourceLimits (cgroup config)
├── network.rs        # NetworkPolicy, iptables rules
├── violation.rs      # ViolationType → FAFO strike escalation
├── security/seccomp-agents.json   # Seccomp allowlist config
└── security/apparmor-gentlyos     # AppArmor profile template

gently-goo/           # GOO unified field dashboard ✓ BUILT (12 files, 70 tests)
├── field.rs          # GooField, smooth_min(a,b,k), evaluate()
├── source.rs         # GooSource, SdfPrimitive (Sphere/Box/Torus/Line)
├── render.rs         # RenderConfig, render_field()
├── attend.rs         # AttentionQuery (temperature = 1/k in smooth_min)
├── learn.rs          # GradientStep, compute_gradient()
├── score.rs          # ScoreTemplate (health/activity/focus)
├── cascade.rs        # CascadeModel trait, CascadeChain
├── rhythm.rs         # Rhythm (BPM, phase, swing, pulse)
├── specialist.rs     # Specialist agent routing by field proximity
├── sense.rs          # SovereigntyGuard, consent tracking, FAFO severity
└── claude.rs         # ClaudeAvatar, Mood enum, animated embodiment
```

**Modified existing crates:**
- `gently-inference`: `solana.rs` → `chain.rs`, added `ChainHook` trait, `ThreeKingsProvenance`, `NullChainHook`
- `gently-ipfs`: added `sui_bridge.rs` — `IpfsSuiBridge` for CID anchoring on Sui
- `gently-cli`: removed ~730 LOC of disabled SPL commands and stubs

#### Three Kings Provenance

```
Gold         = WHO created (identity hash, blake3)
Myrrh        = WHAT model/context (preservation hash)
Frankincense = WHY it matters (intention hash)
```

Quality steps (>= 0.7) get published as `ReasoningStep` Move resources on Sui with Three Kings metadata.

#### GOO Key Math

```
smooth_min(a, b, k):
  h = max(k - |a-b|, 0) / k
  result = min(a,b) - h*h*k*0.25

Same k parameter controls:
  - Visual blobbiness (SDF smooth union)
  - Attention softness (1/k = temperature)
  - Learning smoothness (gradient dampening)
```

---

### Protocol Integration Analysis (Session 7 - 2026-01-23)

Imported research specs from DeathStar and created comprehensive integration analysis.

| Document | Lines | Purpose |
|----------|-------|---------|
| `DEV_DOCS/RESEARCH_SPECS.md` | 728 | BS-ARTISAN, Alexandria, GOO, SYNTH specs |
| `DEV_DOCS/CODIE_SPEC.md` | 494 | 12-keyword instruction language |
| `DEV_DOCS/GAP_ANALYSIS.md` | 200 | Spec vs implementation gaps |
| `DEV_DOCS/BUILD_STEPS.md` | 400 | Atomic implementation steps |
| `DEV_DOCS/PTC_SECURITY_MAP.md` | 300 | Security touchpoint enforcement |
| `DEV_DOCS/PROTOCOL_INTEGRATION.md` | 250 | Cross-protocol integration map |

#### Gap Analysis Results

| Protocol | Spec | Implementation | Gap |
|----------|------|----------------|-----|
| BS-ARTISAN | Full | 0% | **CRITICAL** |
| GOO | Full | 15% | **HIGH** |
| CODIE | Full | 0% | **MEDIUM** |
| Tesseract | In spec | 100% | Complete |
| BBBCP | In spec | 100% | Complete |

#### New Crates BUILT (Session 7)

```
gently-artisan/  # BS-ARTISAN toroidal storage ✓ BUILT
├── lib.rs       # Module exports, r = tokens/2π formula
├── coord.rs     # TorusCoordinate (major/minor angles)
├── torus.rs     # Torus + TorusPoint (blake3 hash, PTC)
├── foam.rs      # Multi-torus container + genesis anchor
├── flux.rs      # FluxLine transformation mechanics
├── barf.rs      # BARF retrieval (XOR distance + topological boost)
└── winding.rs   # WindingLevel 1-6 refinement

gently-codie/    # 12-keyword instruction language ✓ BUILT
├── lib.rs       # Module exports
├── token.rs     # 12 keywords (pug,bark,spin,cali,elf,turk,fence,pin,bone,blob,biz,anchor)
├── lexer.rs     # CodieLexer tokenizer
├── ast.rs       # CodieAst + SourceKind (PTC: Vault)
└── parser.rs    # CodieParser (tree structure aware)

gently-goo/      # Unified GUI field ✓ BUILT (see Session 8)
├── field.rs     # GooField, smooth_min
├── source.rs    # SDF primitives (Sphere/Box/Torus/Line)
├── attend.rs    # Attention as field query
├── learn.rs     # Gradient learning
├── sense.rs     # Sovereignty protection
└── claude.rs    # Claude embodiment
```

#### PTC Security Enforcement

All new protocols MUST use PTC (Permission To Change) for:
- Cryptographic operations → Use existing Berlin Clock, XOR
- Vault access (`$`) → Cold execution sandbox
- Hash resolution (`#`) → BTC anchor verification
- Threat detection → FAFO escalation

See `DEV_DOCS/PTC_SECURITY_MAP.md` for full rules.

---

### Bootable Alpine ISO (Session 6 - 2026-01-20)

Created Alpine-based live ISO (Debian approach failed due to musl/glibc incompatibility).

| Artifact | Size | Notes |
|----------|------|-------|
| `dist/gentlyos-alpine-1.1.1-x86_64.iso` | 232MB | Bootable, UEFI+BIOS |
| `scripts/deploy/build-alpine-iso.sh` | 280 lines | Alpine-native builder |

**ISO Contents:**
- Alpine Linux 3.21 base (musl)
- Linux kernel 6.12.63-lts
- gently CLI + gently-web binaries
- Auto-login as `gently` user
- gently-web service starts on boot

**Build requires:** `apk add squashfs-tools xorriso grub mtools`

---

### ONE SCENE Web GUI (Session 5 - 2026-01-05)

Premium Alexandria GUI - HTMX + Axum for paid users.

| File | Lines | Purpose |
|------|-------|---------|
| `gently-web/src/templates.rs` | 1,128 | ONE SCENE HTML templates |
| `gently-web/src/handlers.rs` | 402 | Route handlers + Alexandria API |
| `gently-web/src/main.rs` | 122 | Web server binary |
| `gently-web/src/state.rs` | 122 | Application state |
| `gently-web/src/lib.rs` | 83 | Router setup |
| `gently-web/src/routes.rs` | 48 | Route definitions |

#### Features

- **ONE SCENE Architecture**: Single adaptive interface, no page navigation
- **HTMX Reactivity**: Server-driven updates without JS framework
- **Alexandria Integration**: Graph visualization, BBBCP queries, Tesseract view, 5W dimensions
- **Living Feed**: Charge/decay items with boost interaction
- **Chat Interface**: Placeholder for LLM integration
- **Security Panel**: Real-time security events

#### Alexandria Premium Panels

| Panel | Route | Function |
|-------|-------|----------|
| Graph | `/htmx/alexandria/graph` | Knowledge graph visualization |
| BBBCP | `/htmx/alexandria/bbbcp` | BONE/CIRCLE/BLOB query interface |
| Tesseract | `/htmx/alexandria/tesseract` | 8D hypercube face visualization |
| 5W Query | `/htmx/alexandria/5w` | WHO/WHAT/WHERE/WHEN/WHY collapse |

---

### Inference Quality Mining (Session 3)

Collective Inference Optimization - The network trains itself through USE.

| File | Lines | Purpose |
|------|-------|---------|
| `gently-inference/src/step.rs` | 200 | InferenceStep, StepType (8 types) |
| `gently-inference/src/score.rs` | 200 | Quality scoring formula |
| `gently-inference/src/decompose.rs` | 250 | Response → Steps extraction |
| `gently-inference/src/cluster.rs` | 300 | Semantic clustering (cosine sim) |
| `gently-inference/src/aggregate.rs` | 250 | Cross-prompt step aggregation |
| `gently-inference/src/optimize.rs` | 300 | Response synthesis |
| `gently-inference/src/boneblob.rs` | 250 | BONEBLOB constraint generation |
| `gently-inference/src/chain.rs` | 350 | Chain hooks, GENOS rewards, Three Kings provenance |

#### The Quality Formula

```
quality = user_accept * 0.3
        + outcome_success * 0.4
        + chain_referenced * 0.2
        + turning_point * 0.1

THRESHOLD: 0.7 = USEFUL
```

#### Step Types

| Type | GENOS Multiplier | Purpose |
|------|-----------------|---------|
| Conclude | 12x | Research synthesis |
| Pattern | 10x | Creative insight |
| Eliminate | 8x | BONEBLOB contribution |
| Specific | 6x | Implementation detail |
| Fact | 5x | Verified data |
| Suggest | 4x | Ideas |
| Correct | 3x | Bug fixes |
| Guess | 1x | Low until validated |

#### BONEBLOB Integration

```
High-quality (>=0.7) → BONES (constraints)
    Eliminate → "MUST NOT: {content}"
    Fact      → "ESTABLISHED: {content}"
    Pattern   → "PATTERN: {content}"

Low-quality (<0.7) → CIRCLE (eliminations)
    Guess/Suggest → "AVOID: {content}"
```

#### Storage

```
~/.gently/inference/
├── inferences.jsonl      # Query + response records
├── steps.jsonl           # Individual reasoning steps
├── clusters.json         # Semantic clustering state
└── pending_genos.jsonl   # GENOS reward queue
```

---

### FAFO Security + Berlin Clock (Session 2)

"A rabid pitbull behind a fence" - Aggressive defense with time-based key rotation.

| File | Lines | Purpose |
|------|-------|---------|
| `gently-core/src/crypto/berlin.rs` | 380 NEW | BTC-synced time-based key rotation |
| `gently-security/src/fafo.rs` | 600 NEW | FAFO escalating response system |
| `gently-cli/src/main.rs` | +250 | `/security` command with dashboard |

#### Berlin Clock Key Rotation

```
BTC Block Timestamp → Slot (ts / 300) → HKDF → Time-Bound Key

Forward secrecy: Old slots cannot derive current keys
Sync: Any node with master + BTC time = same key
Grace period: 2 previous slots for decryption
```

#### FAFO Response Ladder

```
Strike 1-2:  TARPIT   - Waste attacker's time
Strike 3-4:  POISON   - Corrupt attacker's context
Strike 5-7:  DROWN    - Flood with honeypot garbage
Strike 10+:  DESTROY  - Permanent termination
CRITICAL:    SAMSON   - Scorched earth (nuclear option)
```

#### CLI Commands

```
gently security status   - Dashboard with FAFO stats
gently security fafo     - FAFO mode control
gently security daemons  - 16 security daemons status
gently security test     - Threat simulation
```

---

### BONEBLOB BIZ Constraint System (Session 1)

Philosophy → Compiler. Words became executable geometry.

```
BONE BLOB BIZ CIRCLE PIN
         ↓
constraint.rs + tesseract.rs
```

| File | Lines | Purpose |
|------|-------|---------|
| `gently-search/src/constraint.rs` | 325 NEW | Constraint optimization engine |
| `gently-alexandria/src/tesseract.rs` | +57 | BONEBLOB methods on 8-face hypercube |
| `gently-guardian/src/lib.rs` | +101 | Platform detection (macOS/Windows/Linux) |
| `gentlyos-tui/` | 5,693 NEW | Full terminal UI with BONEBLOB integration |

### The Math

```
Intelligence = Capability × Constraint / Search Space

BONES   → Preprompt constraints (immutable rules)
CIRCLE  → 70% elimination per pass (via negativa)
PIN     → Solution finder in bounded space
BIZ     → Solution → new constraint (fixed-point iteration)

Convergence: 5 passes × 70% elimination = 0.24% remaining
Guaranteed by Banach Fixed-Point Theorem
```

### Key Integration Points

1. **Tesseract `eliminated` face** (dims 48-95) stores "What it ISN'T"
2. **ConstraintBuilder** bridges Alexandria graph → BONEBLOB constraints
3. **72-domain router** feeds domain context to constraint system
4. **LlmWorker** optionally routes through BONEBLOB pipeline

### TUI Commands

```
/boneblob on|off  - Toggle constraint optimization
/boneblob         - Show pipeline status
/provider [name]  - Switch LLM (claude/gpt/deepseek/grok/ollama)
/status           - System + BONEBLOB stats
```

---

## Current State (v1.0.0)

### Completed Sprints

| Sprint | Focus | Status |
|--------|-------|--------|
| 1 | Persistence + Embeddings | DONE |
| 2 | Intelligence Integration | DONE |
| 3 | Security Hardening | DONE |
| 4 | Distribution & Install | DONE |
| 5 | Polish & Stability | DONE |

### Production-Ready Crates

| Crate | Status | Notes |
|-------|--------|-------|
| gently-core | 98% | Crypto foundation, XOR splits, genesis keys, **Berlin Clock rotation** |
| gently-audio | 100% | FFT encoding/decoding with tests |
| gently-visual | 100% | SVG pattern generation |
| gently-dance | 85% | Full protocol state machine |
| gently-btc | 90% | Block promise logic |
| gently-ipfs | 85% | Thin wrapper (delegates to daemon) |
| gently-guardian | 80% | Hardware detection, cross-platform (sysinfo) |
| gently-security | 95% | 16/16 daemons, real hash chain, threat intel, **FAFO pitbull** |
| gently-feed | 70% | Charge/decay model works |
| gently-gateway | 70% | Pipeline architecture |
| gently-brain | 75% | Claude API real, Alexandria integration |
| gently-cipher | 50% | Ciphers work, analysis stubbed |
| gently-network | 60% | Visualization works |
| gently-architect | 55% | SQLite works, UI stubbed |
| gently-mcp | 50% | Server ready, handlers missing |
| gently-search | 80% | Alexandria routing, Tesseract projection, **BONEBLOB constraints** |
| gently-alexandria | 85% | Graph + Tesseract work, persistence, **elimination methods** |
| gently-sploit | 20% | Framework only |
| gently-sim | 80% | SIM card security: filesystem, applets, OTA, Simjacker |
| **gently-inference** | **90%** | **Inference quality mining: decompose, score, cluster, optimize, chain hooks** |
| **gently-web** | **85%** | **ONE SCENE Web GUI: HTMX + Axum, Alexandria integration** |
| **gently-artisan** | **90%** | **BS-ARTISAN toroidal storage: Torus, Foam, BARF retrieval** |
| **gently-codie** | **80%** | **12-keyword instruction language: lexer, parser, AST** |
| **gently-chain** | **40%** | **Sui/Move SDK wrapper: client, objects, PTB, events, Three Kings** |
| **gently-ptc** | **70%** | **PTC Brain: tree decompose, execute, aggregate, 7 phases** |
| **gently-sandbox** | **60%** | **Agent isolation: seccomp, AppArmor, capabilities, FAFO violations** |
| **gently-goo** | **80%** | **GOO unified field: SDF, attention, learning, sovereignty (70 tests)** |
| **gentlyos-tui** | **90%** | **Terminal UI: 6 panels, 7 LLM providers, BONEBLOB pipeline** |

---

## Installation

### One-Liner (Recommended)

```bash
curl -fsSL https://gentlyos.com/install.sh | sudo bash
```

Options:
- `--source` - Build from source instead of binary download
- `--skip-setup` - Skip the initial setup wizard

### First-Time Setup

```bash
gently setup           # Interactive wizard
gently setup --force   # Force re-initialization
```

Creates:
```
~/.gently/
├── alexandria/graph.json   # Knowledge graph
├── brain/knowledge.db      # SQLite knowledge base
├── feed/                   # Feed state
├── models/                 # Embedding models
├── vault/genesis.key       # Genesis key
└── config.toml             # User config
```

---

## Build Commands

```bash
# Build CLI (main binary)
cargo build --release -p gently-cli

# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Run CLI
./target/release/gently

# Run setup wizard
./target/release/gently setup
```

### Deployment Scripts

```bash
# Docker image
./scripts/deploy/build-docker.sh

# Debian package
./scripts/deploy/build-deb.sh

# All formats
./scripts/deploy/build-all.sh
```

---

## CLI Commands (21 total)

### Working
```
init, create, pattern, split, combine, status, demo, feed,
search, alexandria, cipher, network, brain, architect, ipfs,
sploit, crack, claude, vault, mcp, report, setup
```

Solana commands (install, mint, wallet, token, certify, perm, genos) were removed in Session 8.
Sui equivalents will be added via gently-chain when SDK is wired.

---

## Architecture

### Security Daemon Layers

```
Layer 1 (Foundation): HashChainValidator*, BtcAnchor, ForensicLogger
Layer 2 (Traffic):    TrafficSentinel, TokenWatchdog, CostGuardian
Layer 3 (Detection):  PromptAnalyzer, BehaviorProfiler, PatternMatcher, AnomalyDetector
Layer 4 (Defense):    SessionIsolator, TarpitController, ResponseMutator, RateLimitEnforcer
Layer 5 (Intel):      ThreatIntelCollector*, SwarmDefense

* = Real implementation (not stubbed)
```

### Hash Chain Validation

Real SHA256-linked audit chain:
- `AuditEntry` struct with index, timestamp, prev_hash, hash
- `HashChain::validate()` verifies chain integrity
- `HashChain::load/save()` for persistence
- Automatic tamper detection

### Threat Intel

Built-in LLM security patterns (28 indicators):
- Prompt injection detection ("ignore previous instructions", "DAN mode")
- System prompt extraction attempts
- Jailbreak patterns (roleplay, encoding tricks)
- Tool abuse patterns (file traversal, command injection)

---

## 28 Crates Overview

| Crate | Purpose |
|-------|---------|
| gently-core | Base types, genesis keys, XOR splits, Berlin Clock |
| gently-btc | Bitcoin RPC, block anchoring |
| **gently-chain** | **Sui/Move SDK: client, objects, PTB, events, Three Kings** |
| gently-dance | P2P dance protocol |
| gently-audio | Audio FFT encoding |
| gently-visual | SVG pattern generation |
| gently-feed | Living feed with charge/decay |
| gently-search | Alexandria-backed semantic search, BONEBLOB |
| gently-mcp | Model Context Protocol server |
| gently-architect | Code generation, project trees |
| gently-brain | LLM orchestration, knowledge graph |
| gently-network | Network capture, MITM, visualization |
| gently-ipfs | IPFS content-addressed storage, **Sui bridge** |
| gently-cipher | Cryptographic utilities, cracking |
| gently-sploit | Exploitation framework |
| gently-gateway | API routing, pipelines |
| gently-security | 16 daemons + FAFO pitbull |
| gently-guardian | Free tier node, hardware validation |
| gently-alexandria | Distributed knowledge mesh, Tesseract |
| gently-sim | SIM card security monitoring |
| **gently-inference** | **Inference quality mining + chain hooks + Three Kings** |
| **gently-web** | **ONE SCENE Web GUI for paid users** |
| **gently-artisan** | **BS-ARTISAN: Toroidal knowledge storage (r=tokens/2π)** |
| **gently-codie** | **CODIE: 12-keyword instruction language** |
| **gently-ptc** | **PTC Brain: tree decompose, execute, aggregate** |
| **gently-sandbox** | **Agent isolation: seccomp, AppArmor, capabilities** |
| **gently-goo** | **GOO unified field: SDF, attention, learning, sovereignty** |
| gently-micro | Microcontroller interface (ESP32/Arduino) |

---

## Key Files

### Core
- `Cargo.toml` - Workspace definition
- `gently-cli/src/main.rs` - Main CLI (4000+ lines)
- `web/install.sh` - Universal installer

### Intelligence
- `crates/gently-alexandria/src/graph.rs` - Knowledge graph
- `crates/gently-alexandria/src/tesseract.rs` - 8-face embedding projection
- `crates/gently-brain/src/orchestrator.rs` - AI orchestration
- `crates/gently-search/src/alexandria.rs` - Semantic search

### Security
- `crates/gently-security/src/daemons/foundation.rs` - Hash chain
- `crates/gently-security/src/daemons/intel.rs` - Threat detection
- `crates/gently-security/src/fafo.rs` - FAFO aggressive defense
- `crates/gently-guardian/src/hardware.rs` - Cross-platform hw detection

### Inference + Chain
- `crates/gently-inference/src/lib.rs` - InferenceEngine main API
- `crates/gently-inference/src/step.rs` - Step types and structures
- `crates/gently-inference/src/score.rs` - Quality scoring formula
- `crates/gently-inference/src/cluster.rs` - Semantic clustering
- `crates/gently-inference/src/boneblob.rs` - Constraint generation
- `crates/gently-inference/src/chain.rs` - ChainHook trait, Three Kings provenance, GENOS rewards

### Sui/Move (New — Session 8)
- `crates/gently-chain/src/client.rs` - SuiClient JSON-RPC wrapper
- `crates/gently-chain/src/types.rs` - ReasoningStep Move resource, ObjectID
- `crates/gently-chain/src/three_kings.rs` - Gold/Myrrh/Frankincense provenance
- `crates/gently-chain/src/transactions.rs` - PtbBuilder (Programmable Transaction Blocks)
- `crates/gently-ipfs/src/sui_bridge.rs` - IpfsSuiBridge CID anchoring

### PTC Brain (New — Session 8)
- `crates/gently-ptc/src/lib.rs` - PtcEngine: decompose → execute → aggregate
- `crates/gently-ptc/src/tree.rs` - UniversalNode, NodeScale, Tree loading
- `crates/gently-ptc/src/decompose.rs` - Intent routing, DFS walk to leaves
- `crates/gently-ptc/src/executor.rs` - 5 execution modes (Design/Inspect/Shell/Claude/Plan)
- `crates/gently-ptc/src/phase.rs` - 7 phases: INTAKE→SHIP

### Sandbox (New — Session 8)
- `crates/gently-sandbox/src/seccomp.rs` - Syscall allowlist (34 allowed)
- `crates/gently-sandbox/src/apparmor.rs` - AppArmor profile generation
- `crates/gently-sandbox/src/violation.rs` - Violation → FAFO strike escalation
- `security/seccomp-agents.json` - Seccomp config for Ollama agents
- `security/apparmor-gentlyos` - AppArmor profile template

### GOO Field (New — Session 8)
- `crates/gently-goo/src/field.rs` - GooField, smooth_min(a,b,k)
- `crates/gently-goo/src/source.rs` - SDF primitives (Sphere/Box/Torus/Line)
- `crates/gently-goo/src/attend.rs` - Attention as field query (temperature=1/k)
- `crates/gently-goo/src/sense.rs` - SovereigntyGuard, consent, FAFO severity
- `crates/gently-goo/src/claude.rs` - ClaudeAvatar, Mood, animated embodiment

### BS-ARTISAN
- `crates/gently-artisan/src/torus.rs` - Torus geometry, TorusPoint with blake3
- `crates/gently-artisan/src/foam.rs` - Multi-torus container, genesis anchor
- `crates/gently-artisan/src/barf.rs` - BARF retrieval (XOR + topological boost)
- `crates/gently-artisan/src/winding.rs` - WindingLevel 1-6 refinement stages

### CODIE Language
- `crates/gently-codie/src/token.rs` - 12 keywords definition
- `crates/gently-codie/src/lexer.rs` - CodieLexer tokenizer
- `crates/gently-codie/src/ast.rs` - CodieAst, SourceKind (PTC: Vault)
- `crates/gently-codie/src/parser.rs` - Tree-structure aware parser

---

## Environment

- Alpine Linux (bare metal)
- Rust 1.75+ toolchain
- Docker available for container builds
- Git repo on main branch

---

## Product Vision

**Editions:**
- **Home** (Free/Guardian) - Security as public good, earn by contributing
- **Business** ($29/mo) - Priority support, dedicated capacity
- **Studio** ($99/mo) - GPU protection, maximum security

**Sui/Move Integration** (in progress):
- `gently-chain` scaffolded with type stubs — needs real Sui SDK wiring
- Three Kings provenance ready for on-chain publishing
- IPFS-Sui bridge ready for CID anchoring
- Token/wallet/governance features to be rebuilt on Move

---

## Claude Operating Protocol

See **`CLAUDE_PROTOCOL.md`** for:
- Session init sequence
- Search-before-build rules
- Domain → Crate mapping
- Anti-duplication checklist
- Architecture stack
- Self-diagnosis protocol

## Development Documentation

See **`DEV_DOCS/`** for:
- `UPDATES.md` - Change log (update after significant work)
- `TEMP_BEHAV.md` - Toggle behaviors ON/OFF
- `DIRECTORY_SCOPE.md` - What goes where
- `DEV_HISTORY/` - Session history files

---

*This file exists so Claude can recover context if session is lost.*
