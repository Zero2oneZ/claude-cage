"""crate_graph.py — Crate dependency graph for GentlyOS workspace.

Loads gentlyos/crate-graph.json and provides:
  - load_graph()         — load and index the crate graph
  - dependents(crate)    — all crates that depend on this one (transitive)
  - build_order(crates)  — sort crates by tier for correct build sequence
  - blast_radius(crates) — changed crates -> affected crates + nodes + risk
  - tier_rebuild_scope()  — changed tier -> all tiers that need rebuilding
"""

import json
import os
from collections import deque
from pathlib import Path


CAGE_ROOT = os.environ.get("CAGE_ROOT", str(Path(__file__).parent.parent))
GRAPH_PATH = os.path.join(CAGE_ROOT, "gentlyos", "crate-graph.json")


def load_graph(path=None):
    """Load crate-graph.json and return indexed graph data.

    Returns dict with:
      crates:       {name: {tier, path, deps, node, description}}
      tiers:        {tier_num: {name, build_order, ...}}
      reverse_deps: {name: set of crates that depend on it}
    """
    path = path or GRAPH_PATH
    with open(path) as f:
        raw = json.load(f)

    crates = raw.get("crates", {})
    tiers = raw.get("tiers", {})

    # Build reverse dependency index: crate -> set of dependents
    reverse_deps = {name: set() for name in crates}
    for name, info in crates.items():
        for dep in info.get("deps", []):
            if dep in reverse_deps:
                reverse_deps[dep].add(name)

    return {
        "crates": crates,
        "tiers": tiers,
        "reverse_deps": reverse_deps,
    }


def dependents(graph, crate):
    """All crates that depend on this one, transitively.

    BFS from the crate through reverse_deps.
    Returns set of crate names (excludes the input crate).
    """
    reverse_deps = graph["reverse_deps"]
    if crate not in reverse_deps:
        return set()

    visited = set()
    queue = deque([crate])
    while queue:
        current = queue.popleft()
        for dep in reverse_deps.get(current, set()):
            if dep not in visited:
                visited.add(dep)
                queue.append(dep)

    return visited


def build_order(graph, crates):
    """Sort crates by tier (ascending) for correct build sequence.

    Tier 0 builds before tier 3. Within same tier, alphabetical.
    Returns ordered list of crate names.
    """
    crate_data = graph["crates"]
    crate_list = [c for c in crates if c in crate_data]
    crate_list.sort(key=lambda c: (crate_data[c]["tier"], c))
    return crate_list


def blast_radius(graph, changed_crates):
    """Given changed crates, return all affected crates + their tree nodes + risk level.

    Returns dict with:
      changed:  list of directly changed crates
      affected: list of all transitively affected crates (sorted by tier)
      nodes:    set of affected tree node IDs
      tiers:    set of affected tier numbers
      risk:     1-10 risk level based on blast radius
      summary:  human-readable summary
    """
    crate_data = graph["crates"]
    all_affected = set()

    for crate in changed_crates:
        if crate in crate_data:
            all_affected.add(crate)
            all_affected.update(dependents(graph, crate))

    # Collect affected tree nodes and tiers
    nodes = set()
    tiers = set()
    for crate in all_affected:
        info = crate_data.get(crate, {})
        node = info.get("node")
        if node:
            nodes.add(node)
        tiers.add(info.get("tier", -1))

    # Sort affected crates by build order
    ordered = build_order(graph, list(all_affected))

    # Calculate risk from blast radius
    total_crates = len(crate_data)
    affected_count = len(all_affected)
    affected_ratio = affected_count / total_crates if total_crates > 0 else 0

    if affected_ratio > 0.8:
        risk = 9
    elif affected_ratio > 0.5:
        risk = 7
    elif affected_ratio > 0.3:
        risk = 6
    elif affected_ratio > 0.15:
        risk = 5
    elif affected_ratio > 0.05:
        risk = 3
    else:
        risk = 2

    # Tier 0 changes always bump risk
    if 0 in tiers and len(changed_crates) > 0:
        changed_tiers = {crate_data[c]["tier"] for c in changed_crates if c in crate_data}
        if 0 in changed_tiers:
            risk = max(risk, 7)

    return {
        "changed": list(changed_crates),
        "affected": ordered,
        "affected_count": affected_count,
        "total_crates": total_crates,
        "nodes": sorted(nodes),
        "tiers": sorted(tiers),
        "risk": min(10, risk),
        "summary": (
            f"{affected_count}/{total_crates} crates affected across "
            f"{len(tiers)} tiers, {len(nodes)} nodes — risk {min(10, risk)}"
        ),
    }


def tier_rebuild_scope(graph, changed_tier):
    """Return all tiers that need rebuilding (changed tier + all higher).

    Lower tiers are dependencies of higher tiers, so changing tier N
    requires rebuilding tiers N, N+1, ..., max_tier.
    Returns sorted list of tier numbers.
    """
    tiers = graph["tiers"]
    max_tier = max(int(t) for t in tiers.keys()) if tiers else 0
    return list(range(changed_tier, max_tier + 1))


def crates_in_tier(graph, tier):
    """Return all crate names in a given tier."""
    return [
        name for name, info in graph["crates"].items()
        if info.get("tier") == tier
    ]


def crates_for_node(graph, node_id):
    """Return all crate names owned by a tree node."""
    return [
        name for name, info in graph["crates"].items()
        if info.get("node") == node_id
    ]
