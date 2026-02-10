# PTC -- Pass-Through Coordination

Technical reference for the PTC execution engine, the coordination layer for GentlyOS.

---

## 1. Overview

PTC (Pass-Through Coordination) is the execution engine that drives the GentlyOS 35-agent tree. It implements a top-down decomposition and bottom-up aggregation cycle: intent enters at the root of the tree, decomposes DOWN through departments to leaf captains, leaves EXECUTE, and results aggregate UP through the tree back to the root.

The engine runs 8 sequential phases -- INTAKE, TRIAGE, PLAN, REVIEW, EXECUTE, VERIFY, INTEGRATE, SHIP -- each with specific responsibilities and gates. Every phase logs to MongoDB. Every artifact is stored. The full execution trace is persisted as a `ptc_trace` artifact.

The core principle: **the first shall be last and the last shall be first**. Executives receive intent but do no work. Captains (leaf nodes) do all the work. Results flow upward, with rules at each level able to transform, filter, block, or escalate.

**Source files:**

- `/home/zero20nez/Desktop/claude-cage/ptc/engine.py` -- PTC engine (8 phases, tree loading, routing, decomposition, aggregation)
- `/home/zero20nez/Desktop/claude-cage/ptc/executor.py` -- Leaf executor (8 execution modes, approval gate, CODIE interpreter)
- `/home/zero20nez/Desktop/claude-cage/ptc/crate_graph.py` -- Crate dependency graph (blast radius, build order, tier scoping)
- `/home/zero20nez/Desktop/claude-cage/ptc/docs.py` -- Circular documentation system (staleness tracking, cross-references, triple storage)
- `/home/zero20nez/Desktop/claude-cage/ptc/architect.py` -- Blueprint system (cache-first design, validation, verification)
- `/home/zero20nez/Desktop/claude-cage/gentlyos/tree.json` -- The 35-agent tree definition

---

## 2. The 8 Phases

Each PTC run proceeds through all 8 phases in order. The `run()` function in `engine.py` orchestrates the full cycle. A unique `run_id` (format: `ptc-<epoch>`) identifies the trace. Every phase fires a `ptc:phase` event to MongoDB.

### Phase 1: INTAKE

Receive intent, load infrastructure, classify the run.

- Load `tree.json` via `load_tree()`. Index all nodes by ID.
- Extract tree metadata (`_meta`) and coordination config (`coordination`).
- Attempt to load the crate dependency graph via `crate_graph.load_graph()`. If the graph file (`gentlyos/crate-graph.json`) is unavailable, proceed without crate awareness.
- Log the INTAKE event with intent, target, dry_run flag, tree filename, and node count.

### Phase 2: TRIAGE

Map the intent string to matching tree nodes using keyword matching.

- `route_intent()` scores every node against the intent words. The scoring function checks: node name, node ID, `crates_owned`, `files`, and `functions` metadata.
- Leaf nodes get a +0.5 score boost when they have any match (they are the workers).
- Results are sorted by score descending, then by ID for stability.
- The top 10 matches are logged to MongoDB. The top 5 are included in the trace.

### Phase 3: PLAN

Decompose intent into leaf-level tasks. Each task targets a specific captain node.

- **With crate graph**: `_extract_crates_from_intent()` scans the intent for `gently-*` and `gentlyos-*` crate names. If found, `blast_radius()` computes all transitively affected crates and their owning tree nodes. Tasks are decomposed from affected nodes, not just keyword matches.
- **Without crate graph** (or no crate names in intent): `decompose()` fans out from matched nodes. Department-level matches decompose to their captain children. Direct leaf matches are taken as-is. Executive nodes are skipped (too broad).
- `_walk_down()` recursively walks from any node to its leaves, building task dicts containing: `node_id`, `node_name`, `scale`, `intent`, `lineage` (root-to-leaf path), `files`, `functions`, `rules`, and `escalation`.
- Tasks are deduplicated by `node_id` (a leaf might be reached from multiple routing paths).

### Phase 4: REVIEW

Risk assessment and approval gating. Every task is reviewed before execution.

- `_review_task()` delegates to `executor._check_approval()`.
- Each task gets a risk score (1-10) calculated by `_calculate_risk()`.
- Tasks are classified as approved or blocked based on the approval cascade (see Section 4).
- Blocked tasks never reach the EXECUTE phase. They are added to results with status `"blocked"`.
- The REVIEW event logs total, approved, and blocked counts.

### Phase 5: EXECUTE

Run all approved tasks. Blocked tasks are skipped.

- If the crate graph is loaded, approved tasks are sorted by `_sort_tasks_by_tier()` -- tier 0 nodes execute before tier 3. Tasks with no tier info sort last.
- Each approved task is dispatched to `execute_leaf()`. In dry_run mode, the task returns a plan without executing. In live mode, `_invoke_executor()` calls `executor.execute()`.
- Each execution fires a `ptc:execute` event with node, intent, phase, and approval level.
- Blocked tasks are appended to results with their block reason, risk score, and escalation target.

### Phase 6: VERIFY

Check execution results. Detect failures and trigger escalations.

- Count completed, failed, and blocked results.
- For each failed result, check the originating node's `escalation` config. If the node has an escalation target, create an escalation record with `from`, `to`, `reason`, and `cascade` chain.
- Log the VERIFY event with counts and escalation count.

### Phase 7: INTEGRATE

Aggregate results bottom-up through the tree. Apply rules at each level.

- `aggregate()` walks the tree from target (or root) downward, collecting leaf results and building a hierarchical aggregation.
- At each branch node, rules are applied:
  - `"block"` action: if any child failed, the branch is blocked.
  - `"escalate"` action: if any child failed, escalation is triggered.
- Aggregate status is determined: `"completed"` (all children completed), `"partial"` (some completed, some failed), `"failed"` (all failed), `"blocked"`, or `"escalated"`.
- Escalation events are fired to MongoDB with cascade chains.

### Phase 8: SHIP

Final output. Build the execution trace, store it, report results.

- Determine overall status: `"completed"`, `"failed"`, `"blocked"`, `"partial"`, or `"partial_blocked"`.
- Build the full trace dict containing: run_id, intent, target, dry_run, tree info, all phase names, task counts, leaf results, aggregated result, escalations, duration in milliseconds, and timestamp.
- Store the full trace as a `ptc_trace` artifact via `store_artifact()` (dual-write to MongoDB + IPFS).
- Log the SHIP event with final status and timing.

---

## 3. Execution Modes

The executor (`ptc/executor.py`) supports 8 execution modes, selected by `_detect_mode()` based on intent keywords and task metadata. Mode detection priority matters -- CODIE is checked first to avoid false matches with shell keywords.

### native

Runs cargo, nix, or nixos-rebuild commands directly on the host. Three sub-modes:

- **cargo**: `cargo build -p <crate>`, `cargo test -p <crate>`, `cargo clippy -p <crate>`, `cargo fmt --all --check`. Workspace root defaults to `projects/Gently-nix/`. Timeout: 300s.
- **nix**: `nix build .#<target>`, `nix develop --command echo 'devshell OK'`, `nix flake check`. Timeout: 600s.
- **rebuild**: `nixos-rebuild switch`. Always risk 9, always blocked, always requires human approval. See `_execute_native_rebuild()`.

The tier rebuild function `_execute_tier_rebuild()` loads the crate graph, computes blast radius, and builds each affected crate in tier order. Stops on first failure.

**Detection keywords**: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`, `nix build`, `nix develop`, `nix flake`, `nixos-rebuild`, `rebuild crate`, `rebuild tier`.

### claude

Delegates to Claude CLI for AI-assisted tasks. Builds a structured instruction from task context (intent, node, scale, lineage, files, functions, rules, escalation path) and invokes `claude --print` in non-interactive mode. Output is captured, truncated to 10KB, and stored as a `claude_output` artifact in MongoDB. Timeout: 120s. Falls back to plan mode if the `claude` CLI is not in PATH.

**Detection keywords**: `create`, `add`, `implement`, `fix`, `refactor`, `write`, `update`, `modify`.

### shell

Runs safe shell commands constructed from known patterns. `_intent_to_command()` maps intents to an allowlist of commands:

- `make build-cli`, `make build-desktop`, `make status`, `make verify-sandbox`
- `make mongo-ping`, `make mongo-status`, `make tree`
- `cargo build/test/clippy` (with optional `-p <crate>`)
- `nix build`, `nix flake check`

Unknown intents return `None` and the task is skipped. Timeout: 30s.

**Detection keywords**: `build`, `run`, `install`, `deploy`, `start`, `stop`, `restart`.

### codie

Executes CODIE language programs. Two paths:

1. **From file**: If `task["codie_program"]` is set, loads the `.codie` file from `codie-maps/`.
2. **Generated**: `_build_codie_instruction()` converts PTC task metadata into CODIE source using the 12-keyword vocabulary.

The CODIE source is parsed into an AST (tries `cage-web` binary first, falls back to `_parse_codie_python()`), then interpreted by `CodieContext.execute()`. The interpreter maps each AST node type to a handler:

| Keyword | AST Type | Action |
|---------|----------|--------|
| `pug` | Entry | Entry point, set up execution context |
| `bark` | Fetch | Read files, run data source queries (`@fs/read`, `@system/*`, `@cargo/*`, `@toolchain/*`, `@validators/*`) |
| `elf` | Bind | Bind variable in context |
| `cali` | Call | Call a function (known safe calls: BUILD, TEST, STATUS, VERIFY, SEED, EXECUTE_INTENT) |
| `spin` | Loop | Loop over a collection |
| `turk` | Transform | Conditional transformation |
| `fence` | Guard | Guard block, check preconditions |
| `bone` | Rule | Rule check, constraint enforcement (supports `NOT:` negation) |
| `pin` | Const | Set immutable constant |
| `blob` | Struct | Define data structure |
| `biz` | Return | Return final result |
| `anchor` | Checkpoint | Log checkpoint to MongoDB, snapshot state |

Safe shell commands within CODIE are restricted to allowed prefixes: `make `, `cargo `, `nix `, `rustc `, `rustfmt `, `docker ps`, `docker info`, `node `.

**Detection keywords**: `codie` in intent, or `codie_program` key in task metadata. Checked first in mode detection.

### design

Architect mode. Produces a blueprint (not code) via `ptc/architect.py`. See Section 9 for the blueprint system.

**Detection keywords**: `design`, `architect`, `blueprint`, `specify`, `plan architecture`, `draft`.

### inspect

Read-only analysis. Iterates `task["files"]`, checks existence, reports file size and modification time.

**Detection keywords**: `show`, `list`, `check`, `verify`, `audit`, `status`, `inspect`, `read`.

### compose

Multi-step orchestration. Returns a composition summary with the node's lineage. Currently a lightweight aggregation point.

### plan

Default fallback mode. Returns what WOULD be done without doing it. Reports intent, node, files, functions, and applied rule names.

---

## 4. Approval Cascade

Every task passes through `_check_approval()` before execution. The approval gate computes a risk score and decides whether to approve, log, or block.

### Risk Levels

| Risk | Level | Action |
|------|-------|--------|
| 1-3 | Captain | Auto-approved. Safe operations. |
| 4-6 | Director | Logged but approved. Notable changes proceed with an audit trail. |
| 7-8 | CTO | Blocked. Requires CTO approval. Escalated to `exec:cto` (or the node's configured escalation target). |
| 9-10 | Human | Blocked. Requires human approval. Escalated to `root:human`. |

### Risk Calculation

`_calculate_risk()` computes risk from multiple factors:

**Base risk from scale:**

| Scale | Base Risk |
|-------|-----------|
| executive | 8 |
| department | 6 |
| captain | 3 |
| module | 2 |
| crate | 2 |

**Intent risk modifiers:**

- **High-risk words** (+3): `delete`, `destroy`, `drop`, `force`, `reset`, `remove`, `wipe`, `nuke`, `nixos-rebuild`
- **Medium-risk words** (+1): `deploy`, `push`, `release`, `migrate`, `update`, `modify`, `nix build`, `rebuild tier`

**File sensitivity** (+1): If any task file path contains `security/`, `docker/`, `.env`, `credentials`, or `config/`.

**Rule constraint** (-1): If the node has more than 3 rules (more constrained = lower risk).

**Blast radius** (from `crate_graph.blast_radius()`): Affects risk based on the percentage of crates impacted:

| Affected Ratio | Risk |
|----------------|------|
| > 80% | 9 |
| > 50% | 7 |
| > 30% | 6 |
| > 15% | 5 |
| > 5% | 3 |
| <= 5% | 2 |

Tier 0 changes always bump risk to at least 7.

Final risk is clamped to the range [1, 10].

---

## 5. Crate-Aware Decomposition

The PTC engine integrates with the crate dependency graph (`ptc/crate_graph.py`) to understand how changes propagate through the workspace.

### Crate Extraction

`_extract_crates_from_intent()` scans the intent string for `gently-*` and `gentlyos-*` patterns using regex. Only names that exist in the loaded graph are accepted.

### Blast Radius

`blast_radius(graph, changed_crates)` computes the full impact of a change:

1. For each changed crate, find all transitive dependents via BFS through the reverse dependency index.
2. Collect all affected tree node IDs (from `crate.node` mapping).
3. Collect all affected tier numbers.
4. Sort affected crates by build order (tier ascending, then alphabetical).
5. Calculate risk from the affected ratio (affected / total crates).
6. Tier 0 changes always bump risk to at least 7.

Returns: `changed`, `affected` (sorted), `affected_count`, `total_crates`, `nodes`, `tiers`, `risk`, and a human-readable `summary`.

### Task Decomposition from Affected Nodes

When crate names are found in the intent:

1. Compute blast radius.
2. Extract the set of affected tree node IDs.
3. For each affected node that exists in the tree, walk down to leaves and collect tasks.
4. Deduplicate by `node_id`.

This replaces the default keyword-matching decomposition with a dependency-aware decomposition.

### Tier-Ordered Execution

`_sort_tasks_by_tier()` sorts approved tasks by the minimum crate tier of their owning node. This ensures that foundation crates (tier 0) build before application crates (tier 3+). Tasks with no tier information sort last (tier 99).

### Supporting Functions

- `dependents(graph, crate)` -- BFS through reverse_deps, returns all transitively dependent crates.
- `build_order(graph, crates)` -- Sort by tier ascending, then alphabetical.
- `tier_rebuild_scope(graph, changed_tier)` -- Returns all tiers from `changed_tier` to `max_tier`.
- `crates_in_tier(graph, tier)` -- All crate names in a given tier.
- `crates_for_node(graph, node_id)` -- All crate names owned by a tree node.

---

## 6. Native Execution Mode

The native execution mode (`_execute_native()` in `executor.py`) handles direct cargo/nix/rebuild operations on the host.

### Cargo Sub-Mode

`_execute_native_cargo()` handles `cargo build`, `cargo test`, `cargo clippy`, and `cargo fmt`:

- Extracts crate name from intent via `_extract_crate_name()` (regex for `gently-*` / `gentlyos-*` patterns).
- If a crate name is found: `cargo <sub-command> -p <crate>`.
- If no crate name: workspace-wide command (e.g., `cargo build --workspace`).
- Working directory: `projects/Gently-nix/` (falls back to `CAGE_ROOT`).
- Timeout: 300s.

### Nix Sub-Mode

`_execute_native_nix()` handles nix operations:

- `nix flake check` -- if "flake" appears in intent.
- `nix develop --command echo 'devshell OK'` -- if "develop" appears.
- `nix build .#<target>` -- extracts target from intent (`.#<name>` pattern or `gently-*` crate name).
- Working directory: `projects/Gently-nix/` (falls back to `CAGE_ROOT`).
- Timeout: 600s.

### Rebuild Sub-Mode

`_execute_native_rebuild()` handles `nixos-rebuild switch`:

- Always returns blocked with risk 9.
- Always escalates to `root:human`.
- This is a hard gate -- no amount of automation approves a system rebuild.

### Tier Rebuild

`_execute_tier_rebuild()` orchestrates a full tier-aware workspace rebuild:

1. Load the crate graph.
2. Compute blast radius for the changed crates.
3. Get the affected crates sorted by build order (tier ascending).
4. For each crate: `cargo build -p <crate>`.
5. Stop on first failure.
6. Return results for all attempted builds, including per-crate tier, command, pass/fail status, and error output.

---

## 7. The Tree Structure

The tree is defined in `gentlyos/tree.json` with 35 agent nodes across 3 levels.

### Hierarchy

**3 Executives** (root + 2 children):

| ID | Name | Role |
|----|------|------|
| `root:human` | Human Architect (Tom) | Final authority. Risk >= 9 decisions. MVP scope guard. |
| `exec:cto` | CTO Agent | Routes tasks to 8 departments. Blast radius checks. Breaking change gates. |
| `exec:vision` | Vision Alignment Agent | Sovereignty, decentralization, proof-of-reasoning alignment scoring. |

**8 Directors** (departments under `exec:cto`):

| ID | Name | Sephira | Crates Owned | Tier(s) |
|----|------|---------|--------------|---------|
| `dept:foundation` | Foundation Department | Malkuth | gently-core, gently-codie, gently-artisan, gently-audio, gently-visual, gently-goo, gently-synth | 0 |
| `dept:protocol` | Protocol Department | Chokmah/Binah | gently-bridge, gently-network, gently-dance, gently-livepeer, gently-alexandria, gently-ipfs, gently-feed | 1, 2, 5 |
| `dept:orchestration` | Orchestration Department | Tiferet | gently-codie, gently-ged, gently-document | 0, 3, 6 |
| `dept:runtime` | Runtime Department | Netzach/Hod | gently-brain, gently-inference, gently-agents, gently-micro, gently-behavior | 3 |
| `dept:tokenomics` | Tokenomics Department | Yesod | gently-synth, gently-btc, gently-commerce | 0, 1, 6 |
| `dept:security` | Security Department | Daath | ALL (touches everything) | -- |
| `dept:interface` | Interface Department | Keter | gently-web, gently-cli, gently-mcp, gently-gateway, gently-gooey, gentlyos-tui | 3, 5, 6 |
| `dept:devops` | DevOps Department | Gevurah/Chesed | gently-architect | 6 |

**24 Captains** (leaf nodes under departments):

| Department | Captains |
|------------|----------|
| Foundation | `capt:types`, `capt:crypto`, `capt:errors` |
| Protocol | `capt:wire`, `capt:p2p`, `capt:alexandria` |
| Orchestration | `capt:codie`, `capt:ptc`, `capt:context` |
| Runtime | `capt:exec`, `capt:state`, `capt:memory` |
| Tokenomics | `capt:rewards`, `capt:proof`, `capt:economics` |
| Security | `capt:audit`, `capt:hardening`, `capt:incident` |
| Interface | `capt:api`, `capt:claude`, `capt:ux` |
| DevOps | `capt:build`, `capt:release`, `capt:infra` |

### Node Schema

Every node follows the universal node schema (`gentlyos/universal-node.schema.json`):

- `id` -- Unique identifier (format: `<scale>:<name>`)
- `name` -- Human-readable name
- `scale` -- Node level: `executive`, `department`, `captain`
- `parent` -- Parent node ID (null for root)
- `children` -- Array of child node IDs
- `inputs` -- Array of `{name, type, from}` describing what the node receives
- `outputs` -- Array of `{name, type, to}` describing what the node produces
- `rules` -- Array of `{name, condition, action}` constraints
- `escalation` -- `{target, threshold, cascade}` defining the escalation path
- `metadata` -- Scale-specific data: `crates_owned`, `tier`, `sephira_mapping`, `files`, `functions`, `performance_targets`, etc.

### Sephirot Mapping

The Tree of Life maps to departments:

| Sephira | Department | Meaning |
|---------|-----------|---------|
| Keter | Interface | Crown -- user-facing entry point |
| Chokmah/Binah | Protocol | Wisdom/understanding -- core abstractions |
| Daath | Security | Hidden knowledge -- touches everything |
| Chesed/Gevurah | DevOps | Mercy/judgment in releases |
| Tiferet | Orchestration | Beauty/center -- CODIE, PTC, context |
| Netzach/Hod | Runtime | Victory/splendor -- execution pillars |
| Yesod | Tokenomics | Foundation of value |
| Malkuth | Foundation | Kingdom -- leaf primitives |

### Coordination Protocol

Defined in `tree.json` under the `coordination` key:

- **Phases**: INTAKE, TRIAGE, PLAN, REVIEW, EXECUTE, VERIFY, INTEGRATE, SHIP
- **Approval cascade**: low (1-3) captain approves, medium (4-6) director approves, high (7-8) CTO approves, critical (9-10) human architect final call

---

## 8. Circular Documentation System

`ptc/docs.py` implements a self-tracking documentation system where documentation is interconnected with code and automatically detects when it drifts.

### Core Concept

Documentation is dead text -- it drifts from code. The fix: make documentation part of the code graph. Every tree node gets a doc artifact. Every doc references every related doc. When source files change, the doc is flagged stale.

### Node Artifacts

50 node artifacts are tracked (one per tree node with file ownership). Each doc contains:

- `node_id`, `title`, `scale`, `description`
- `what_it_does` -- inferred from rules and role metadata
- `owned_files` -- from the `OWNERSHIP_MAP` (maps 27 tree node IDs to source file paths)
- `entry_points` -- from node metadata functions
- `key_concepts` -- from rules + crates_owned
- `cross_refs` -- structural, code_shared, and semantic edges
- `staleness` -- source_hash, is_stale flag, last_verified timestamp
- `content_hash` -- SHA-256 of the doc content itself
- `ipfs_cid` -- IPFS content identifier (if stored)

### Three Edge Types

1. **Structural**: Parent/child/sibling relationships from the tree hierarchy. Computed from `tree.json`.
2. **Code-shared**: Nodes sharing file ownership. Computed by intersecting file sets in `OWNERSHIP_MAP`. For example, `capt:hardening` and `dept:security` both own `lib/sandbox.sh`.
3. **Semantic**: Vector similarity > 0.7 between doc text embeddings. Uses `sentence-transformers` via `ptc/embeddings.py` if available.

All edges are made bidirectional by `make_bidirectional()`.

### Staleness Detection

`compute_file_hash()` computes SHA-256 of concatenated owned file contents. `check_staleness()` compares the current hash against the stored `source_hash`. If they differ, the doc is stale.

`propagate_staleness()` follows all edges from a stale node and flags connected nodes as potentially affected.

### Triple Storage

Docs are stored in three locations:

1. **MongoDB** -- `docs` collection via `node store.js put docs <json>`
2. **IPFS** -- via `ptc.ipfs.dual_store()` if available
3. **Local JSON** -- `docs/<node-id>.json` files

### CLI

```bash
python3 -m ptc.docs generate <node_id>     # Generate doc for one node
python3 -m ptc.docs generate-all            # Generate docs for all nodes
python3 -m ptc.docs check-stale             # Check all docs for staleness
python3 -m ptc.docs refresh [node_id]       # Regenerate stale doc(s)
python3 -m ptc.docs interconnect            # Compute full bidirectional graph
python3 -m ptc.docs search <query> [N]      # Semantic/text search
python3 -m ptc.docs show <node_id>          # Display doc with cross-refs
python3 -m ptc.docs graph                   # Output interconnection graph JSON
python3 -m ptc.docs status                  # Coverage + staleness stats
```

---

## 9. Blueprint System

`ptc/architect.py` implements the architecture planning system. The architect designs; it does not build. A blueprint IS a node -- it has inputs, outputs, rules, children. Same pattern, different scale.

### Flow

1. Intent arrives.
2. `cache_check()` -- exact hash match or vector similarity > 0.9.
3. If cached: return immediately (zero tokens spent).
4. If not cached: generate a blueprint JSON structure following the universal node schema.
5. `dual_store()` -- MongoDB + IPFS.
6. `embed_blueprint()` -- index for vector search.
7. `git_commit` on a design branch (if `ptc.git_ops` is available).
8. `blueprint_to_tasks()` converts builder_tasks into PTC-compatible task dicts for leaf execution.
9. `verify_blueprint()` checks results against acceptance criteria.
10. Ship: cache, update status.

### Blueprint Structure

Each blueprint node contains:

- Standard universal node fields (id, name, scale, parent, children, inputs, outputs, rules, escalation)
- An `artifacts` array with a single `"blueprint"` type artifact containing:
  - `what` -- the intent
  - `where` -- affected files, modules, endpoints
  - `how` -- approach, patterns, dependencies
  - `why` -- rationale
  - `gui_spec` -- views, flows, interactions, data bindings
  - `data_flow` -- inputs, transforms, outputs, stores
  - `interconnections` -- dependencies on other nodes
  - `builder_tasks` -- decomposed implementation tasks
  - `acceptance` -- criteria and verification intent
- `metadata` with: blueprint_version, content_hash, ipfs_cid, status, intent_hash, cached flag

### Two-Level Cache

1. **Exact match**: SHA-256 hash of the normalized intent string, queried against `blueprints` collection in MongoDB.
2. **Semantic match**: Vector similarity >= 0.9 via `ptc.embeddings.find_similar_blueprints()`.

### Validation

`validate_blueprint()` checks:
- Required fields present (`what`, `builder_tasks`, `acceptance.criteria`)
- All builder_task `target_node` values exist in `tree.json`
- All referenced files exist on disk

### Verification

`verify_blueprint()` compares execution results against the blueprint:
- All tasks completed: status becomes `"verified"`
- Any tasks failed: status becomes `"failed"`
- Partial completion: status becomes `"building"`

### CLI

```bash
python3 -m ptc.architect create <intent>       # Create a new blueprint
python3 -m ptc.architect list [--status X]      # List blueprints
python3 -m ptc.architect show <id>              # Show blueprint JSON
python3 -m ptc.architect tasks <id>             # Extract PTC tasks
python3 -m ptc.architect validate <id>          # Validate against schema/tree
python3 -m ptc.architect search <query>         # Semantic search
```

---

## 10. CLI Usage

### Full PTC Cycle

```bash
# Dry run (default) -- plan tasks without executing
python3 -m ptc.engine --tree gentlyos/tree.json --intent "rebuild gently-core" --verbose

# Live execution
python3 -m ptc.engine --tree gentlyos/tree.json --intent "add GPU monitoring" --live

# Target a specific department
python3 -m ptc.engine --tree gentlyos/tree.json --intent "fix auth bug" --target dept:security

# JSON output
python3 -m ptc.engine --tree gentlyos/tree.json --intent "check sandbox" --json
```

### Direct Leaf Execution

```bash
# Execute a task directly at a specific node
python3 -m ptc.engine --tree gentlyos/tree.json --node capt:docker --task "build ARM image"

# Live direct execution
python3 -m ptc.engine --tree gentlyos/tree.json --node capt:codie --task "parse all .codie files" --live
```

### Tree Inspection

```bash
# Show the full tree hierarchy
python3 -m ptc.engine --tree gentlyos/tree.json --show-tree

# Show all leaf nodes (captains)
python3 -m ptc.engine --tree gentlyos/tree.json --show-leaves
```

### CLI Arguments

| Flag | Description |
|------|-------------|
| `--tree <path>` | Path to tree.json (default: `tree.json`, also tries `CAGE_ROOT/tree.json`) |
| `--intent <text>` | Intent to execute through the tree |
| `--target <node_id>` | Target node ID (skip routing, decompose from this node) |
| `--node <node_id>` | Execute directly at a specific leaf node (requires `--task`) |
| `--task <text>` | Task description for `--node` mode |
| `--live` | Live execution (default is dry run) |
| `--verbose`, `-v` | Verbose output (show per-task details) |
| `--json` | Output raw JSON trace |
| `--show-tree` | Show tree hierarchy and exit |
| `--show-leaves` | Show leaf nodes and exit |
