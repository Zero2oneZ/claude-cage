# Crate Dependency Graph

## Overview

GentlyOS is a 38-crate Rust workspace located at `projects/Gently-nix/`. The crate dependency graph, defined in `gentlyos/crate-graph.json`, serves as the single source of truth for all build ordering, blast radius calculation, and tier-aware task decomposition across the system.

The Python module `ptc/crate_graph.py` provides programmatic access to the graph, exposing functions for dependency traversal, build sequencing, risk assessment, and node-to-crate mapping. All PTC (Plan-Task-Check) operations that touch crate compilation or change impact use this module.

---

## Tier Architecture

The workspace is organized into 7 tiers (0 through 6). Each tier has a name, a CODIE keyword mapping, and a Tree of Life (sephira) correspondence. Build order flows strictly upward: tier 0 must complete before tier 1, tier 1 before tier 2, and so on.

### Tier 0 -- Foundation (bone / Malkuth)

Core primitives, crypto, and base types. Everything else depends on this tier.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-core` | `crates/foundation/gently-core` | Foundation types, crypto primitives (SHA2, HMAC, Argon2) |
| `gently-codie` | `crates/foundation/gently-codie` | 12-keyword instruction DSL |
| `gently-artisan` | `crates/foundation/gently-artisan` | Toroidal knowledge storage (BS-ARTISAN) |
| `gently-audio` | `crates/foundation/gently-audio` | Audio processing (cpal, dasp, rustfft) |
| `gently-visual` | `crates/foundation/gently-visual` | Visual/SVG rendering |
| `gently-goo` | `crates/foundation/gently-goo` | Unified distance field engine (SYNTHESTASIA) |
| `gently-synth` | `crates/foundation/gently-synth` | Token and smart contract interface ($SYNTH) |

### Tier 1 -- Knowledge Core (blob / Hod)

Data feeds and anchoring.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-feed` | `crates/knowledge/gently-feed` | Data feed aggregation |
| `gently-btc` | `crates/knowledge/gently-btc` | Bitcoin data (genesis anchoring) |

### Tier 2 -- Knowledge Graph (blob / Chokmah)

Knowledge graph, search, and distributed storage.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-alexandria` | `crates/knowledge/gently-alexandria` | Knowledge graph |
| `gently-search` | `crates/knowledge/gently-search` | Search indexing |
| `gently-ipfs` | `crates/knowledge/gently-ipfs` | IPFS integration |

### Tier 3 -- Intelligence (cali / Tiferet)

LLM orchestration, agents, and reasoning.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-brain` | `crates/intelligence/gently-brain` | LLM orchestration |
| `gently-inference` | `crates/intelligence/gently-inference` | Model inference engine |
| `gently-agents` | `crates/intelligence/gently-agents` | Five-element agent pipeline (SPIRIT->AIR->WATER->EARTH->FIRE) |
| `gently-micro` | `crates/intelligence/gently-micro` | Microservice runtime |
| `gently-mcp` | `crates/intelligence/gently-mcp` | MCP protocol support |
| `gently-ged` | `crates/intelligence/gently-ged` | Generative Educational Device (SYNTHESTASIA) |
| `gently-behavior` | `crates/intelligence/gently-behavior` | Behavioral learning + adaptive UI (SYNTHESTASIA) |

### Tier 4 -- Security (fence / Daath)

Crypto, guardians, and audit.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-security` | `crates/security/gently-security` | Core security framework |
| `gently-cipher` | `crates/security/gently-cipher` | Encryption/decryption (SHA1, MD5, MD4, BS58) |
| `gently-guardian` | `crates/security/gently-guardian` | 16 security daemons + SQLite tracking |
| `gently-sim` | `crates/security/gently-sim` | Security simulation |
| `gently-sploit` | `crates/security/gently-sploit` | Exploit detection/analysis |

### Tier 5 -- Network (bark / Netzach)

Networking, gateway, and bridge.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-network` | `crates/network/gently-network` | Base networking |
| `gently-gateway` | `crates/network/gently-gateway` | API gateway |
| `gently-bridge` | `crates/network/gently-bridge` | IPC bridge (Limbo Layer, port 7335) |
| `gently-dance` | `crates/network/gently-dance` | XOR reconstruction protocol |
| `gently-livepeer` | `crates/network/gently-livepeer` | Decentralized video transcoding |

### Tier 6 -- Application (biz / Keter)

Web, CLI, and all user-facing applications.

| Crate | Path | Description |
|-------|------|-------------|
| `gently-web` | `crates/application/gently-web` | Web interface (HTMX) |
| `gently-architect` | `crates/application/gently-architect` | Architecture tool |
| `gently-document` | `crates/application/gently-document` | Three-chain document engine (SYNTHESTASIA) |
| `gently-gooey` | `crates/application/gently-gooey` | 2D application builder (SYNTHESTASIA) |
| `gently-commerce` | `crates/application/gently-commerce` | Vibe commerce + TradingView (SYNTHESTASIA) |
| `gently-google` | `crates/application/gently-google` | Google APIs (YouTube, Ads, Analytics) |
| `gently-tiktok` | `crates/application/gently-tiktok` | TikTok posting infrastructure |
| `gently-cli` | `crates/application/gently-cli` | Main CLI binary (produces 'gently' executable) |
| `gentlyos-tui` | `crates/application/gentlyos-tui` | Terminal UI |

### Tier Summary

| Tier | Name | Crate Count | CODIE Keyword | Sephira |
|------|------|-------------|---------------|---------|
| 0 | Foundation | 7 | bone | Malkuth |
| 1 | Knowledge (core) | 2 | blob | Hod |
| 2 | Knowledge (graph) | 3 | blob | Chokmah |
| 3 | Intelligence | 7 | cali | Tiferet |
| 4 | Security | 5 | fence | Daath |
| 5 | Network | 5 | bark | Netzach |
| 6 | Application | 9 | biz | Keter |
| **Total** | | **38** | | |

---

## Dependency Rules

Dependencies flow strictly downward. Higher-tier crates depend on lower-tier crates, never the reverse. Every crate in the workspace depends on `gently-core` (tier 0), either directly or transitively.

Within a tier, crates may depend on each other. For example, `gently-goo` (tier 0) depends on both `gently-core` and `gently-visual`, both of which are also tier 0.

### Example Dependency Chains

**Leaf crate with no dependents:**
```
gently-core (T0)
  <- gently-synth (T0)
```
`gently-synth` depends only on `gently-core`. Nothing depends on `gently-synth`.

**Deep chain through multiple tiers:**
```
gently-core (T0)
  <- gently-artisan (T0)
    <- gently-alexandria (T2)
      <- gently-search (T2)
```
Changing `gently-artisan` ripples up through the knowledge graph layer.

**Wide fan-out at application tier:**
```
gently-core (T0)
  <- gently-brain (T3)
    <- gently-agents (T3)
      <- gently-cli (T6)
    <- gently-inference (T3)
    <- gently-ged (T3)
    <- gently-behavior (T3)
```
`gently-brain` is a key hub in the intelligence tier with four direct dependents.

**Cross-tier dependency (security into network):**
```
gently-core (T0)
  <- gently-cipher (T4)
    <- gently-dance (T5)
```
`gently-dance` (network tier) depends on `gently-cipher` (security tier).

**Deepest chain to CLI binary:**
```
gently-core (T0)
  <- gently-brain (T3)
    <- gently-agents (T3)
      <- gently-cli (T6)

gently-core (T0)
  <- gently-network (T5)
    <- gently-bridge (T5)
      <- gently-cli (T6)
```
`gently-cli` has five direct dependencies spanning tiers 0, 3, and 5: `gently-core`, `gently-brain`, `gently-agents`, `gently-bridge`, and `gently-mcp`.

---

## Blast Radius

When a crate changes, `blast_radius()` computes all transitively affected crates using BFS through the reverse dependency index. This determines which crates need recompilation and which tree nodes are impacted.

### Risk Level Mapping

Risk is derived from the ratio of affected crates to total crates (38):

| Affected Ratio | Risk Level | Label |
|----------------|------------|-------|
| 1-5% (1-2 crates) | 2 | Low |
| 6-15% (3-5 crates) | 3 | Low |
| 16-30% (6-11 crates) | 5 | Medium |
| 31-50% (12-19 crates) | 6 | Medium |
| 51-80% (20-30 crates) | 7 | High |
| 81%+ (31-38 crates) | 9 | Critical |

Special rule: any change to a tier 0 crate automatically sets risk to at least 7 (high), regardless of the affected ratio.

### Example: Changing gently-core

`gently-core` is the universal dependency. Every other crate depends on it directly or transitively.

```
Changed:  [gently-core]
Affected: 38/38 crates
Tiers:    0, 1, 2, 3, 4, 5, 6 (all)
Risk:     9 (critical)
```

This is the maximum blast radius. A change to `gently-core` requires a full workspace rebuild.

### Example: Changing gently-cipher

`gently-cipher` (tier 4) has one direct dependent: `gently-dance` (tier 5).

```
Changed:  [gently-cipher]
Affected: 2/38 crates (gently-cipher, gently-dance)
Tiers:    4, 5
Risk:     2 (low)
```

### Example: Changing gently-brain

`gently-brain` (tier 3) is a hub with four direct dependents, one of which (`gently-agents`) has its own dependent.

```
Changed:  [gently-brain]
Affected: 6/38 crates (gently-brain, gently-inference, gently-agents, gently-ged, gently-behavior, gently-cli)
Tiers:    3, 6
Risk:     5 (medium)
```

### Example: Changing gently-network

`gently-network` (tier 5) fans out to three direct dependents in the same tier, plus application-tier crates that depend on those.

```
Changed:  [gently-network]
Affected: 7/38 crates (gently-network, gently-gateway, gently-bridge, gently-livepeer, gently-web, gently-commerce, gently-google, gently-tiktok)
Tiers:    5, 6
Risk:     5 (medium)
```

---

## Build Order

`build_order()` sorts any set of crates by tier (ascending), with alphabetical ordering within each tier. This produces a correct compilation sequence where dependencies are always built before the crates that need them.

### Full workspace build order (tiers)

```
Tier 0 (build first):  gently-artisan, gently-audio, gently-codie, gently-core,
                        gently-goo, gently-synth, gently-visual
Tier 1:                 gently-btc, gently-feed
Tier 2:                 gently-alexandria, gently-ipfs, gently-search
Tier 3:                 gently-agents, gently-behavior, gently-brain, gently-ged,
                        gently-inference, gently-mcp, gently-micro
Tier 4:                 gently-cipher, gently-guardian, gently-security,
                        gently-sim, gently-sploit
Tier 5:                 gently-bridge, gently-dance, gently-gateway,
                        gently-livepeer, gently-network
Tier 6 (build last):    gently-architect, gently-cli, gently-commerce,
                        gently-document, gently-google, gently-gooey,
                        gently-tiktok, gently-web, gentlyos-tui
```

For partial rebuilds, `build_order()` accepts a subset of crates and returns only those crates, sorted correctly. This is used after `blast_radius()` to determine the minimal ordered rebuild set.

---

## Tree Node Mapping

Each crate is owned by a tree node at the `capt:` (captain) level in the PTC tree. This mapping connects the crate graph to the GentlyOS organizational hierarchy.

| Tree Node | Crates |
|-----------|--------|
| `capt:types` | gently-core, gently-artisan |
| `capt:codie` | gently-codie |
| `capt:ux` | gently-audio, gently-visual, gently-goo, gently-gooey, gentlyos-tui |
| `capt:rewards` | gently-synth |
| `capt:wire` | gently-feed, gently-network, gently-bridge |
| `capt:crypto` | gently-btc, gently-cipher |
| `capt:alexandria` | gently-alexandria, gently-search |
| `capt:p2p` | gently-ipfs, gently-dance, gently-livepeer |
| `capt:exec` | gently-brain, gently-inference, gently-agents, gently-micro |
| `capt:claude` | gently-mcp, gently-cli |
| `capt:context` | gently-ged, gently-document |
| `capt:memory` | gently-behavior |
| `capt:audit` | gently-security, gently-sploit |
| `capt:hardening` | gently-guardian, gently-sim |
| `capt:api` | gently-gateway, gently-web, gently-commerce, gently-google, gently-tiktok |
| `capt:build` | gently-architect |

When `blast_radius()` computes affected crates, it also collects the set of affected tree nodes. This tells the PTC engine which captains need to be involved in review and verification.

---

## API Reference

All functions are in `ptc/crate_graph.py`.

### load_graph(path=None) -> dict

Load `gentlyos/crate-graph.json` and return an indexed graph structure.

**Parameters:**
- `path` (str, optional): Path to the JSON file. Defaults to `$CAGE_ROOT/gentlyos/crate-graph.json`.

**Returns:** dict with three keys:
- `crates`: `{name: {tier, path, deps, node, description}}`
- `tiers`: `{tier_num_str: {name, build_order, sephira, description}}`
- `reverse_deps`: `{name: set of crate names that depend on it}`

The `reverse_deps` index is built at load time by inverting the `deps` arrays. It maps each crate to the set of crates that list it as a dependency.

```python
graph = load_graph()
graph["crates"]["gently-core"]["tier"]  # 0
graph["reverse_deps"]["gently-core"]    # {'gently-codie', 'gently-artisan', ...}
```

### dependents(graph, crate) -> set

Find all crates that depend on the given crate, transitively, using BFS through `reverse_deps`.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `crate` (str): Crate name to find dependents of.

**Returns:** Set of crate names (excludes the input crate itself). Returns empty set if the crate is not found.

```python
deps = dependents(graph, "gently-brain")
# {'gently-inference', 'gently-agents', 'gently-ged', 'gently-behavior', 'gently-cli'}
```

### build_order(graph, crates) -> list

Sort a set of crate names by tier (ascending), then alphabetically within each tier.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `crates` (list): Crate names to sort.

**Returns:** Ordered list of crate names. Unknown crates are silently filtered out.

```python
build_order(graph, ["gently-cli", "gently-core", "gently-brain"])
# ['gently-core', 'gently-brain', 'gently-cli']
```

### blast_radius(graph, changed_crates) -> dict

Compute the full impact of changing one or more crates.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `changed_crates` (list): Crate names that were modified.

**Returns:** dict with:
- `changed` (list): The input crates.
- `affected` (list): All transitively affected crates, sorted by build order.
- `affected_count` (int): Number of affected crates.
- `total_crates` (int): Total crates in workspace (38).
- `nodes` (list): Sorted list of affected tree node IDs.
- `tiers` (list): Sorted list of affected tier numbers.
- `risk` (int): Risk level from 1-10.
- `summary` (str): Human-readable summary string.

```python
result = blast_radius(graph, ["gently-core"])
result["summary"]
# "38/38 crates affected across 7 tiers, 16 nodes -- risk 9"
```

### tier_rebuild_scope(graph, changed_tier) -> list

Determine which tiers need rebuilding when a given tier changes. Since lower tiers are dependencies of higher tiers, changing tier N requires rebuilding tiers N through 6.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `changed_tier` (int): The tier number that changed.

**Returns:** Sorted list of tier numbers from `changed_tier` to `max_tier` (6).

```python
tier_rebuild_scope(graph, 3)
# [3, 4, 5, 6]
```

### crates_in_tier(graph, tier) -> list

Return all crate names belonging to a given tier.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `tier` (int): Tier number (0-6).

**Returns:** List of crate names.

```python
crates_in_tier(graph, 4)
# ['gently-security', 'gently-cipher', 'gently-guardian', 'gently-sim', 'gently-sploit']
```

### crates_for_node(graph, node_id) -> list

Return all crate names owned by a specific tree node.

**Parameters:**
- `graph` (dict): Graph returned by `load_graph()`.
- `node_id` (str): Tree node identifier (e.g., `"capt:exec"`).

**Returns:** List of crate names.

```python
crates_for_node(graph, "capt:exec")
# ['gently-brain', 'gently-inference', 'gently-agents', 'gently-micro']
```

---

## Integration Points

### ptc/engine.py -- Tier-Aware Decomposition

During the PLAN phase, the PTC engine uses `blast_radius()` to determine which crates are affected by a proposed change. The risk level feeds into the approval cascade:

- Risk 1-3: Captain can approve
- Risk 4-6: Director approval required
- Risk 7-8: CTO approval required
- Risk 9-10: Human approval required

The engine calls `build_order()` to sequence tasks so that lower-tier crate builds complete before higher-tier crates that depend on them.

### ptc/executor.py -- Native Build Sequencing

In native execution mode, the executor compiles Rust crates directly. It uses `build_order()` to determine the correct compilation sequence and `crates_in_tier()` to batch builds within a tier (crates in the same tier can be compiled in parallel if they do not depend on each other).

When a build fails, `blast_radius()` is used to determine which downstream tasks should be cancelled rather than attempted.

### gentlyos/tree.json -- Node Ownership

The `node` field in each crate entry references a captain-level node in the GentlyOS organizational tree (`gentlyos/tree.json`). This mapping establishes accountability: when a crate is affected by a change, the owning captain is responsible for review and verification.

`crates_for_node()` provides the reverse lookup, answering "which crates does this captain own?" This is used during the TRIAGE phase to assign work to the correct organizational unit.

---

## JSON Schema

The `gentlyos/crate-graph.json` file has the following structure:

```json
{
  "_meta": {
    "title": "string",
    "description": "string",
    "workspace": "string (path to Cargo.toml)",
    "version": "string (semver)"
  },

  "crates": {
    "<crate-name>": {
      "tier": "integer (0-6)",
      "path": "string (relative path from workspace root)",
      "deps": ["string (crate names this crate depends on)"],
      "node": "string (tree node ID, format: capt:<name>)",
      "description": "string (human-readable purpose)"
    }
  },

  "tiers": {
    "<tier-number-as-string>": {
      "name": "string (tier display name)",
      "build_order": "integer (1-indexed build sequence)",
      "sephira": "string (CODIE keyword)",
      "description": "string (tier purpose)"
    }
  }
}
```

### Field Details

**_meta**: Metadata about the graph file itself. The `workspace` field points to the Cargo.toml that defines the Rust workspace.

**crates**: Each key is a crate name matching the directory name and `Cargo.toml` package name. Fields:

| Field | Type | Description |
|-------|------|-------------|
| `tier` | int | Tier number (0-6). Determines build order priority. |
| `path` | string | Relative path from workspace root to the crate directory. |
| `deps` | array of strings | Direct dependencies on other crates in this workspace. Does not include external (crates.io) dependencies. `gently-core` has an empty array; all others list at least `gently-core`. |
| `node` | string | The captain-level tree node that owns this crate. Format: `capt:<name>`. |
| `description` | string | Brief description of the crate's purpose. |

**tiers**: Keyed by tier number as a string (`"0"` through `"6"`). Fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable tier name. |
| `build_order` | int | 1-indexed position in the build sequence. Tier 0 has `build_order: 1`, tier 6 has `build_order: 7`. |
| `sephira` | string | CODIE keyword mapping (bone, blob, cali, fence, bark, biz). |
| `description` | string | Brief description of the tier's purpose. |
