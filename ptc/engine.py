"""engine.py — Pass-Through Coordination engine.

Intent enters at root. Decomposes DOWN through the tree.
Leaves EXECUTE. Results aggregate UP to root.
Every step → MongoDB. Every artifact → stored.
The first shall be last and the last shall be first.

Usage:
    python3 -m ptc.engine --tree tree.json --intent "add GPU monitoring"
    python3 -m ptc.engine --tree tree.json --intent "fix auth bug" --target dept:security
    python3 -m ptc.engine --tree tree.json --node capt:docker --task "build ARM image"
"""

import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


CAGE_ROOT = os.environ.get("CAGE_ROOT", str(Path(__file__).parent.parent))


# ── Tree loading ───────────────────────────────────────────────


def load_tree(tree_path):
    """Load tree.json, return nodes indexed by id."""
    with open(tree_path) as f:
        tree = json.load(f)
    nodes = {n["id"]: n for n in tree.get("nodes", [])}
    meta = tree.get("_meta", {})
    coordination = tree.get("coordination", {})
    return nodes, meta, coordination


def find_root(nodes):
    """Find the root node (no parent)."""
    for nid, node in nodes.items():
        if node.get("parent") is None:
            return nid
    return None


def get_lineage(nodes, node_id):
    """Get full path from root to this node. Self-identification."""
    lineage = []
    current = node_id
    while current:
        lineage.append(current)
        current = nodes.get(current, {}).get("parent")
    lineage.reverse()
    return lineage


def get_leaves(nodes, root_id=None):
    """Get all leaf nodes (no children) under a root."""
    if root_id is None:
        root_id = find_root(nodes)
    leaves = []

    def walk(nid):
        node = nodes.get(nid)
        if not node:
            return
        children = node.get("children", [])
        if not children:
            leaves.append(nid)
        else:
            for c in children:
                walk(c)

    walk(root_id)
    return leaves


# ── Routing: intent → target nodes ─────────────────────────────


def route_intent(nodes, intent):
    """Route an intent string to matching nodes, scored by relevance."""
    words = intent.lower().split()
    matches = []

    for nid, node in nodes.items():
        score = 0
        owned = node.get("metadata", {}).get("crates_owned", [])
        files = node.get("metadata", {}).get("files", [])
        funcs = node.get("metadata", {}).get("functions", [])
        text = " ".join([
            node["name"], nid,
            " ".join(owned), " ".join(files), " ".join(funcs),
        ]).lower()

        for word in words:
            if word in text:
                score += 1

        # Boost leaf nodes — they're the workers
        if not node.get("children"):
            score += 0.5 if score > 0 else 0

        if score > 0:
            matches.append((score, nid, node))

    matches.sort(key=lambda x: (-x[0], x[1]))
    return matches


# ── Decompose: top → down ──────────────────────────────────────


def decompose(nodes, intent, target_id=None):
    """Decompose an intent into leaf-level tasks.

    Flow: intent → fan out to ALL matching departments → decompose to captains.
    The first shall be last — leaves do the work. Parents aggregate.
    Returns list of task dicts for leaf execution.
    """
    tasks = []

    if target_id:
        # Direct targeting — decompose from this node down
        _walk_down(nodes, target_id, intent, tasks)
    else:
        # Fan out: find ALL matching nodes, then decompose each
        matches = route_intent(nodes, intent)
        if not matches:
            return []

        # Collect unique subtrees to decompose
        # Prefer department-level matches (they fan out to captains)
        # but include leaf matches too (direct hits)
        seen_subtrees = set()
        for score, nid, node in matches:
            # Skip root/executive — too broad
            if node.get("scale") in ("executive",) and node.get("parent") is not None:
                continue

            # If this is a leaf, take it directly
            if not node.get("children"):
                if nid not in seen_subtrees:
                    seen_subtrees.add(nid)
                    _walk_down(nodes, nid, intent, tasks)
                continue

            # If this is a department/branch, decompose it
            if nid not in seen_subtrees:
                # Don't decompose if a child is already targeted
                children_targeted = any(c in seen_subtrees for c in node.get("children", []))
                if not children_targeted:
                    seen_subtrees.add(nid)
                    _walk_down(nodes, nid, intent, tasks)

    # Deduplicate by node_id (a leaf might be reached from multiple paths)
    seen = set()
    deduped = []
    for t in tasks:
        if t["node_id"] not in seen:
            seen.add(t["node_id"])
            deduped.append(t)

    return deduped


def _walk_down(nodes, nid, intent, tasks):
    """Walk down from a node, collecting leaf tasks."""
    n = nodes.get(nid)
    if not n:
        return
    children = n.get("children", [])

    if not children:
        # LEAF — this is where work happens
        task = {
            "node_id": nid,
            "node_name": n["name"],
            "scale": n["scale"],
            "intent": intent,
            "lineage": get_lineage(nodes, nid),
            "files": n.get("metadata", {}).get("files", []),
            "functions": n.get("metadata", {}).get("functions", []),
            "rules": n.get("rules", []),
            "escalation": n.get("escalation", {}),
        }
        tasks.append(task)
    else:
        # BRANCH — decompose to all children
        for child_id in children:
            _walk_down(nodes, child_id, intent, tasks)


# ── Execute: leaf nodes do the work ────────────────────────────


def execute_leaf(task, dry_run=False):
    """Execute a leaf-level task.

    In dry_run mode: returns the task plan without executing.
    In live mode: invokes the executor to do real work.
    """
    result = {
        "node_id": task["node_id"],
        "node_name": task["node_name"],
        "scale": task["scale"],
        "intent": task["intent"],
        "lineage": task["lineage"],
        "status": "pending",
        "started_at": None,
        "completed_at": None,
        "output": None,
        "artifacts": [],
        "error": None,
    }

    if dry_run:
        result["status"] = "planned"
        result["output"] = {
            "plan": f"Would execute: {task['intent']}",
            "files": task["files"],
            "functions": task["functions"],
            "rules_applied": [r["name"] for r in task["rules"]],
        }
        return result

    # Live execution
    result["started_at"] = datetime.now(timezone.utc).isoformat()
    result["status"] = "executing"

    try:
        output = _invoke_executor(task)
        result["status"] = "completed"
        result["output"] = output
        result["completed_at"] = datetime.now(timezone.utc).isoformat()
    except Exception as e:
        result["status"] = "failed"
        result["error"] = str(e)
        result["completed_at"] = datetime.now(timezone.utc).isoformat()

        # Check escalation
        esc = task.get("escalation", {})
        if esc.get("target"):
            result["escalated_to"] = esc["target"]
            result["escalation_reason"] = str(e)

    return result


def _invoke_executor(task):
    """Invoke the actual executor for a leaf task.

    The executor is the bridge between the tree and the real world.
    It can: invoke Claude, run shell commands, create files, query APIs.
    """
    from ptc.executor import execute
    return execute(task)


# ── Aggregate: bottom → up ─────────────────────────────────────


def aggregate(nodes, results, target_id=None):
    """Aggregate leaf results back up through the tree.

    Each parent node aggregates its children's results.
    Rules at each level can transform, filter, or escalate.
    Returns the final aggregated result at the target (or root).
    """
    if not results:
        return {"status": "no_results", "summary": "No leaf tasks executed"}

    # Index results by node_id
    result_map = {r["node_id"]: r for r in results}

    if target_id is None:
        target_id = find_root(nodes)

    # Build aggregation from leaves up
    def aggregate_node(nid):
        node = nodes.get(nid)
        if not node:
            return None

        children = node.get("children", [])

        if not children:
            # Leaf — return its result if we have one
            return result_map.get(nid)

        # Branch — aggregate children
        child_results = []
        for child_id in children:
            cr = aggregate_node(child_id)
            if cr is not None:
                child_results.append(cr)

        if not child_results:
            return None

        # Apply this node's rules to the aggregated results
        rules = node.get("rules", [])
        blocked = False
        escalated = False

        for rule in rules:
            action = rule.get("action", "pass")
            if action == "block":
                # Check if any child failed
                if any(r.get("status") == "failed" for r in child_results):
                    blocked = True
            elif action == "escalate":
                # Check if risk exceeds threshold
                failed_count = sum(1 for r in child_results if r.get("status") == "failed")
                if failed_count > 0:
                    escalated = True

        # Determine aggregate status
        statuses = [r.get("status") for r in child_results]
        if all(s == "completed" for s in statuses):
            agg_status = "completed"
        elif any(s == "failed" for s in statuses):
            agg_status = "partial" if any(s == "completed" for s in statuses) else "failed"
        else:
            agg_status = "in_progress"

        if blocked:
            agg_status = "blocked"
        if escalated:
            agg_status = "escalated"
            esc = node.get("escalation", {})

        return {
            "node_id": nid,
            "node_name": node["name"],
            "scale": node["scale"],
            "status": agg_status,
            "lineage": get_lineage(nodes, nid),
            "children_results": child_results,
            "children_count": len(child_results),
            "completed": sum(1 for s in statuses if s == "completed"),
            "failed": sum(1 for s in statuses if s == "failed"),
            "blocked": blocked,
            "escalated": escalated,
            "escalation_target": node.get("escalation", {}).get("target") if escalated else None,
        }

    return aggregate_node(target_id)


# ── Store: everything → MongoDB ────────────────────────────────


def store_event(event_type, key, value=None):
    """Fire-and-forget event to MongoDB."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return

    payload = json.dumps(value) if value else "{}"
    try:
        subprocess.Popen(
            ["node", store_js, "log", event_type, key, payload],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass  # Fire and forget — never block


def store_artifact(name, artifact_type, content, project="claude-cage"):
    """Store an artifact — dual-write to MongoDB + IPFS.

    Always computes content hash. IPFS add runs in background if enabled.
    Falls back to MongoDB-only if IPFS unavailable. Fire-and-forget either way.
    """
    try:
        from ptc.ipfs import dual_store
        result = dual_store(name, artifact_type, content, project)
        return result
    except ImportError:
        pass

    # Fallback: MongoDB-only (original behavior)
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return

    doc = json.dumps({
        "name": name,
        "type": artifact_type,
        "content": content[:50000] if isinstance(content, str) else json.dumps(content)[:50000],
        "project": project,
        "_ts": datetime.now(timezone.utc).isoformat(),
    })
    try:
        subprocess.Popen(
            ["node", store_js, "put", "artifacts", doc],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


# ── The Machine: full PTC cycle ────────────────────────────────


def run(tree_path, intent, target_id=None, dry_run=True):
    """Run a full PTC cycle.

    1. Load tree
    2. Decompose intent → leaf tasks (TOP → DOWN)
    3. Execute leaf tasks (LEAVES DO THE WORK)
    4. Aggregate results (BOTTOM → UP)
    5. Store everything (→ MongoDB)

    Returns the full execution trace.
    """
    ts_start = time.time()
    run_id = f"ptc-{int(ts_start)}"

    # Phase 1: Load
    nodes, meta, coordination = load_tree(tree_path)
    phases = coordination.get("phases", [])

    store_event("ptc:start", run_id, {
        "intent": intent,
        "target": target_id,
        "dry_run": dry_run,
        "tree": os.path.basename(tree_path),
        "phase": "INTAKE",
    })

    # Phase 2: Decompose (TRIAGE + PLAN)
    tasks = decompose(nodes, intent, target_id)

    store_event("ptc:decompose", run_id, {
        "task_count": len(tasks),
        "leaf_nodes": [t["node_id"] for t in tasks],
        "phase": "PLAN",
    })

    if not tasks:
        return {
            "run_id": run_id,
            "intent": intent,
            "status": "no_match",
            "message": "No matching nodes found for intent",
            "duration_ms": int((time.time() - ts_start) * 1000),
        }

    # Phase 3: Execute leaves (EXECUTE)
    results = []
    for task in tasks:
        store_event("ptc:execute", f"{run_id}/{task['node_id']}", {
            "node": task["node_id"],
            "intent": task["intent"],
            "phase": "EXECUTE",
        })

        result = execute_leaf(task, dry_run=dry_run)
        results.append(result)

        store_event("ptc:result", f"{run_id}/{task['node_id']}", {
            "node": task["node_id"],
            "status": result["status"],
            "phase": "VERIFY",
        })

    # Phase 4: Aggregate (INTEGRATE)
    aggregated = aggregate(nodes, results, target_id)

    store_event("ptc:aggregate", run_id, {
        "status": aggregated.get("status") if aggregated else "empty",
        "phase": "INTEGRATE",
    })

    # Phase 5: Final report (SHIP)
    duration_ms = int((time.time() - ts_start) * 1000)

    trace = {
        "run_id": run_id,
        "intent": intent,
        "target": target_id,
        "dry_run": dry_run,
        "tree_path": tree_path,
        "tree_title": meta.get("title", "unknown"),
        "phases_used": ["INTAKE", "TRIAGE", "PLAN", "EXECUTE", "VERIFY", "INTEGRATE", "SHIP"],
        "tasks_decomposed": len(tasks),
        "tasks_executed": len(results),
        "tasks_completed": sum(1 for r in results if r.get("status") in ("completed", "planned")),
        "tasks_failed": sum(1 for r in results if r.get("status") == "failed"),
        "leaf_results": results,
        "aggregated": aggregated,
        "duration_ms": duration_ms,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

    store_event("ptc:complete", run_id, {
        "status": "completed",
        "tasks": len(tasks),
        "duration_ms": duration_ms,
        "phase": "SHIP",
    })

    # Store full trace as artifact
    store_artifact(
        f"ptc-trace-{run_id}",
        "ptc_trace",
        json.dumps(trace, indent=2),
    )

    return trace


# ── Display: render results for humans ─────────────────────────


def display_trace(trace, verbose=False):
    """Render a PTC trace for human consumption."""
    print(f"PTC RUN: {trace['run_id']}")
    print(f"{'=' * 60}")
    print(f"Intent:  {trace['intent']}")
    print(f"Tree:    {trace.get('tree_title', 'unknown')}")
    print(f"Mode:    {'DRY RUN' if trace.get('dry_run') else 'LIVE'}")
    print(f"Phases:  {' → '.join(trace.get('phases_used', []))}")
    print()

    # Decomposition
    print(f"DECOMPOSITION: {trace['tasks_decomposed']} leaf tasks")
    print(f"{'─' * 60}")
    for r in trace.get("leaf_results", []):
        lineage = " → ".join(r.get("lineage", []))
        status_icon = {
            "completed": "+",
            "planned": "~",
            "failed": "!",
            "executing": ">",
        }.get(r.get("status"), "?")
        print(f"  [{status_icon}] {r['node_name']} ({r['node_id']})")
        print(f"      Task: {r['intent']}")
        if r.get("files"):
            print(f"      Files: {', '.join(r['files'][:3])}")
        if verbose and r.get("output"):
            out = r["output"]
            if isinstance(out, dict):
                for k, v in out.items():
                    print(f"      {k}: {v}")
        print()

    # Aggregation
    agg = trace.get("aggregated")
    if agg:
        print(f"AGGREGATION")
        print(f"{'─' * 60}")
        _display_agg(agg, depth=0)
        print()

    # Summary
    print(f"SUMMARY")
    print(f"{'─' * 60}")
    print(f"  Tasks:     {trace['tasks_decomposed']} decomposed, {trace['tasks_executed']} executed")
    print(f"  Completed: {trace['tasks_completed']}")
    print(f"  Failed:    {trace['tasks_failed']}")
    print(f"  Duration:  {trace['duration_ms']}ms")


def _display_agg(agg, depth=0):
    """Recursively display aggregation tree."""
    if not agg:
        return
    indent = "  " * depth
    status = agg.get("status", "?")
    icon = {"completed": "+", "partial": "~", "failed": "!", "blocked": "X", "escalated": "^"}.get(status, "?")
    name = agg.get("node_name", agg.get("node_id", "?"))
    scale = agg.get("scale", "?")

    if "children_results" in agg:
        completed = agg.get("completed", 0)
        total = agg.get("children_count", 0)
        print(f"{indent}[{icon}] {name} [{scale}] — {completed}/{total} children completed")
        if agg.get("escalated"):
            print(f"{indent}    ^ ESCALATED to {agg.get('escalation_target')}")
        for child in agg.get("children_results", []):
            _display_agg(child, depth + 1)
    else:
        print(f"{indent}[{icon}] {name} [{scale}] — {status}")


# ── CLI ────────────────────────────────────────────────────────


def main():
    import argparse
    parser = argparse.ArgumentParser(description="PTC — Pass-Through Coordination engine")
    parser.add_argument("--tree", default="tree.json", help="Path to tree.json")
    parser.add_argument("--intent", help="Intent to execute through the tree")
    parser.add_argument("--target", help="Target node ID (skip routing)")
    parser.add_argument("--node", help="Execute directly at a specific leaf node")
    parser.add_argument("--task", help="Task description for --node mode")
    parser.add_argument("--live", action="store_true", help="Live execution (default: dry run)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument("--json", action="store_true", help="Output raw JSON trace")
    parser.add_argument("--show-tree", action="store_true", help="Show the tree and exit")
    parser.add_argument("--show-leaves", action="store_true", help="Show leaf nodes and exit")

    args = parser.parse_args()

    tree_path = args.tree
    if not os.path.exists(tree_path):
        # Try relative to CAGE_ROOT
        tree_path = os.path.join(CAGE_ROOT, args.tree)
        if not os.path.exists(tree_path):
            print(f"Error: tree not found at {args.tree}", file=sys.stderr)
            sys.exit(1)

    if args.show_tree:
        nodes, meta, _ = load_tree(tree_path)
        root = find_root(nodes)
        print(f"Tree: {meta.get('title', 'unknown')}")
        print(f"Nodes: {len(nodes)}")
        print(f"Root: {root}")
        print()
        _show_tree(nodes, root)
        return

    if args.show_leaves:
        nodes, _, _ = load_tree(tree_path)
        leaves = get_leaves(nodes)
        print(f"Leaf nodes ({len(leaves)}):")
        for leaf_id in leaves:
            node = nodes[leaf_id]
            lineage = " → ".join(get_lineage(nodes, leaf_id))
            print(f"  {node['name']:24} {leaf_id:28} {lineage}")
        return

    if args.node and args.task:
        # Direct leaf execution
        nodes, _, _ = load_tree(tree_path)
        node = nodes.get(args.node)
        if not node:
            print(f"Error: node {args.node} not found", file=sys.stderr)
            sys.exit(1)

        task = {
            "node_id": args.node,
            "node_name": node["name"],
            "scale": node["scale"],
            "intent": args.task,
            "lineage": get_lineage(nodes, args.node),
            "files": node.get("metadata", {}).get("files", []),
            "functions": node.get("metadata", {}).get("functions", []),
            "rules": node.get("rules", []),
            "escalation": node.get("escalation", {}),
        }
        result = execute_leaf(task, dry_run=not args.live)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            print(f"[{result['status']}] {result['node_name']}: {result['intent']}")
            if result.get("output"):
                print(json.dumps(result["output"], indent=2))
            if result.get("error"):
                print(f"Error: {result['error']}")
        return

    if not args.intent:
        parser.print_help()
        sys.exit(1)

    # Full PTC cycle
    trace = run(
        tree_path=tree_path,
        intent=args.intent,
        target_id=args.target,
        dry_run=not args.live,
    )

    if args.json:
        print(json.dumps(trace, indent=2))
    else:
        display_trace(trace, verbose=args.verbose)


def _show_tree(nodes, root_id, depth=0):
    """Render tree hierarchy."""
    node = nodes.get(root_id)
    if not node:
        return
    indent = "  " * depth
    children = node.get("children", [])
    leaf_mark = "" if children else " *"
    sephira = node.get("metadata", {}).get("sephira_mapping", "")
    extra = f" ({sephira})" if sephira else ""
    print(f"{indent}{'├── '}{node['name']} [{node['scale']}]{extra}{leaf_mark}")
    for child_id in children:
        _show_tree(nodes, child_id, depth + 1)


if __name__ == "__main__":
    main()
