"""training.py — Extract training data from PTC traces.

Every PTC execution IS a chain of thought:
  intent → decompose → execute → aggregate → result

That's an instruction/reasoning/output triple.
That's a training example.
That's a LoRA growing smarter with every run.

The Hopf sphere: build out to feed in.

Formats:
  - alpaca: {instruction, input, output} — for supervised fine-tuning
  - sharegpt: [{from, value}, ...] — for chat/conversation fine-tuning
  - cot: {question, chain_of_thought, answer} — for reasoning training
  - raw: full trace as-is — for custom pipelines

Filters:
  - By scale: executive, department, captain
  - By department: security, runtime, web, etc.
  - By node: specific leaf worker
  - By status: completed, failed, escalated
"""

import json
import os
import sys
import hashlib
from datetime import datetime, timezone
from pathlib import Path


CAGE_ROOT = os.environ.get("CAGE_ROOT", str(Path(__file__).parent.parent))


# ── Trace collection ───────────────────────────────────────────


def collect_traces(source="mongodb", limit=1000, **filters):
    """Collect PTC traces from storage.

    Sources: mongodb, local (filesystem), inline (passed directly)
    """
    if source == "mongodb":
        return _collect_from_mongodb(limit, **filters)
    elif source == "local":
        return _collect_from_local(limit, **filters)
    elif source == "inline":
        return filters.get("traces", [])
    return []


def _collect_from_mongodb(limit, **filters):
    """Pull traces from MongoDB artifacts collection."""
    import subprocess
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return []

    query = json.dumps({"type": "ptc_trace"})
    try:
        result = subprocess.run(
            ["node", store_js, "get", "artifacts", query, str(limit)],
            capture_output=True, text=True, timeout=30, cwd=CAGE_ROOT,
        )
        if result.returncode == 0 and result.stdout.strip():
            docs = json.loads(result.stdout)
            traces = []
            for doc in docs:
                content = doc.get("content", "{}")
                if isinstance(content, str):
                    try:
                        trace = json.loads(content)
                        traces.append(trace)
                    except json.JSONDecodeError:
                        continue
                elif isinstance(content, dict):
                    traces.append(content)
            return traces
    except Exception:
        pass
    return []


def _collect_from_local(limit, **filters):
    """Pull traces from local filesystem."""
    traces_dir = os.path.join(CAGE_ROOT, "training", "traces")
    if not os.path.isdir(traces_dir):
        return []

    traces = []
    for f in sorted(Path(traces_dir).glob("*.json"), reverse=True)[:limit]:
        try:
            traces.append(json.loads(f.read_text()))
        except Exception:
            continue
    return traces


# ── Chain of Thought extraction ────────────────────────────────


def extract_cot(trace):
    """Extract a chain-of-thought from a PTC trace.

    The chain follows the tree topology:
    1. Intent (what was asked)
    2. Routing (which nodes matched)
    3. Decomposition (how it was broken down)
    4. Leaf execution (what each worker did)
    5. Aggregation (how results combined)
    6. Final result (what came back to root)
    """
    intent = trace.get("intent", "")
    leaf_results = trace.get("leaf_results", [])
    aggregated = trace.get("aggregated", {})

    # Build the chain
    steps = []

    # Step 1: Understand the intent
    steps.append({
        "step": "intent",
        "thought": f"Received intent: {intent}",
        "action": "analyze and route through the tree",
    })

    # Step 2: Decomposition
    task_count = trace.get("tasks_decomposed", 0)
    departments = set()
    for r in leaf_results:
        lineage = r.get("lineage", [])
        for node_id in lineage:
            if node_id.startswith("dept:"):
                departments.add(node_id)

    steps.append({
        "step": "decompose",
        "thought": f"Decomposed into {task_count} leaf tasks across {len(departments)} departments: {', '.join(sorted(departments))}",
        "action": "route to leaf workers",
    })

    # Step 3: Leaf execution (one step per leaf)
    for r in leaf_results:
        node_name = r.get("node_name", "unknown")
        node_id = r.get("node_id", "unknown")
        status = r.get("status", "unknown")
        files = r.get("files", []) if "files" in r else r.get("output", {}).get("files", []) if isinstance(r.get("output"), dict) else []
        rules = r.get("rules_applied", []) if "rules_applied" in r else []
        if isinstance(r.get("output"), dict):
            rules = r["output"].get("rules_applied", rules)

        steps.append({
            "step": "execute",
            "thought": f"Leaf worker {node_name} ({node_id}) executing: {r.get('intent', intent)}",
            "action": f"status={status}, files={files}, rules={rules}",
            "node_id": node_id,
            "lineage": r.get("lineage", []),
        })

    # Step 4: Aggregation
    if aggregated:
        agg_status = aggregated.get("status", "unknown")
        completed = aggregated.get("completed", 0)
        total = aggregated.get("children_count", 0)
        steps.append({
            "step": "aggregate",
            "thought": f"Aggregated results: {completed}/{total} children completed, status={agg_status}",
            "action": "combine and report",
        })

    return {
        "intent": intent,
        "chain_of_thought": steps,
        "departments_involved": sorted(departments),
        "leaf_count": len(leaf_results),
        "final_status": aggregated.get("status", "unknown") if aggregated else "unknown",
    }


# ── Format: Alpaca ─────────────────────────────────────────────


def to_alpaca(trace, include_cot=True):
    """Convert a PTC trace to Alpaca format.

    {instruction, input, output}

    The instruction is the intent.
    The input is the tree context (which tree, which nodes exist).
    The output is the chain of thought + final result.
    """
    cot = extract_cot(trace)
    intent = trace.get("intent", "")
    tree_title = trace.get("tree_title", "unknown")

    # Build the output as structured reasoning
    output_parts = []

    if include_cot:
        output_parts.append("## Reasoning")
        for step in cot["chain_of_thought"]:
            output_parts.append(f"**{step['step'].upper()}**: {step['thought']}")
            if step.get("action"):
                output_parts.append(f"  Action: {step['action']}")
        output_parts.append("")

    output_parts.append("## Result")
    output_parts.append(f"Status: {cot['final_status']}")
    output_parts.append(f"Departments: {', '.join(cot['departments_involved'])}")
    output_parts.append(f"Leaf workers: {cot['leaf_count']}")

    # Per-leaf results
    for r in trace.get("leaf_results", []):
        lineage = " → ".join(r.get("lineage", []))
        output_parts.append(f"- {r.get('node_name', '?')} [{r.get('status', '?')}]: {lineage}")

    return {
        "instruction": f"Execute this intent through the {tree_title} tree: {intent}",
        "input": json.dumps({
            "tree": tree_title,
            "available_departments": cot["departments_involved"],
            "leaf_count": cot["leaf_count"],
        }),
        "output": "\n".join(output_parts),
    }


# ── Format: ShareGPT ──────────────────────────────────────────


def to_sharegpt(trace):
    """Convert a PTC trace to ShareGPT multi-turn format.

    Simulates a conversation:
    human: the intent
    system: tree routing
    assistant: decomposition + execution + aggregation
    """
    cot = extract_cot(trace)
    intent = trace.get("intent", "")
    tree_title = trace.get("tree_title", "unknown")

    conversation = [
        {
            "from": "system",
            "value": f"You are a PTC coordinator for the {tree_title}. Route intents through the tree, decompose to leaf workers, execute, and aggregate results bottom-up.",
        },
        {
            "from": "human",
            "value": intent,
        },
    ]

    # Decomposition turn
    decomp_parts = [f"Routing through {tree_title}...\n"]
    decomp_parts.append(f"Decomposed into {cot['leaf_count']} leaf tasks across {len(cot['departments_involved'])} departments:")
    for dept in cot["departments_involved"]:
        decomp_parts.append(f"  - {dept}")

    conversation.append({
        "from": "assistant",
        "value": "\n".join(decomp_parts),
    })

    # Execution turn (one per leaf)
    exec_parts = ["Executing leaf tasks:\n"]
    for r in trace.get("leaf_results", []):
        node_name = r.get("node_name", "?")
        status = r.get("status", "?")
        lineage = " → ".join(r.get("lineage", []))
        exec_parts.append(f"[{status}] {node_name}: {lineage}")

    conversation.append({
        "from": "assistant",
        "value": "\n".join(exec_parts),
    })

    # Aggregation turn
    agg = trace.get("aggregated", {})
    agg_parts = ["Aggregating results bottom-up:\n"]
    agg_parts.append(f"Status: {agg.get('status', 'unknown')}")
    agg_parts.append(f"Completed: {trace.get('tasks_completed', 0)}/{trace.get('tasks_decomposed', 0)}")
    if agg.get("escalated"):
        agg_parts.append(f"ESCALATED to {agg.get('escalation_target')}")

    conversation.append({
        "from": "assistant",
        "value": "\n".join(agg_parts),
    })

    return {"conversations": conversation}


# ── Format: CoT (Chain of Thought) ─────────────────────────────


def to_cot(trace):
    """Convert to explicit chain-of-thought format.

    {question, chain_of_thought, answer}

    This is the purest training signal — the tree topology
    IS the reasoning structure.
    """
    cot = extract_cot(trace)

    # Build the CoT string
    cot_steps = []
    for i, step in enumerate(cot["chain_of_thought"], 1):
        cot_steps.append(f"Step {i} ({step['step']}): {step['thought']}")

    return {
        "question": trace.get("intent", ""),
        "chain_of_thought": "\n".join(cot_steps),
        "answer": f"Executed {cot['leaf_count']} leaf tasks across {len(cot['departments_involved'])} departments. Final status: {cot['final_status']}.",
        "metadata": {
            "tree": trace.get("tree_title", "unknown"),
            "departments": cot["departments_involved"],
            "leaf_count": cot["leaf_count"],
            "duration_ms": trace.get("duration_ms", 0),
        },
    }


# ── Per-node training data ─────────────────────────────────────


def extract_per_node(traces):
    """Extract training data grouped by node.

    Each node accumulates its own training examples:
    - What intents reached it
    - What it produced
    - What rules it applied
    - Whether it succeeded or failed

    This is how captain-level LoRAs get their data.
    """
    node_data = {}

    for trace in traces:
        for r in trace.get("leaf_results", []):
            node_id = r.get("node_id", "unknown")
            if node_id not in node_data:
                node_data[node_id] = {
                    "node_id": node_id,
                    "node_name": r.get("node_name", "unknown"),
                    "scale": r.get("scale", "unknown"),
                    "lineage": r.get("lineage", []),
                    "examples": [],
                    "total_runs": 0,
                    "completed": 0,
                    "failed": 0,
                }

            entry = node_data[node_id]
            entry["total_runs"] += 1
            if r.get("status") in ("completed", "planned"):
                entry["completed"] += 1
            elif r.get("status") == "failed":
                entry["failed"] += 1

            entry["examples"].append({
                "intent": r.get("intent", trace.get("intent", "")),
                "status": r.get("status", "unknown"),
                "output": r.get("output"),
                "trace_id": trace.get("run_id", "unknown"),
                "timestamp": trace.get("timestamp"),
            })

    return node_data


def extract_per_department(traces):
    """Extract training data grouped by department.

    Department-level LoRAs learn coordination patterns:
    - Which captains get activated for which intents
    - How results aggregate
    - When to escalate
    """
    dept_data = {}

    for trace in traces:
        # Group leaf results by department
        dept_results = {}
        for r in trace.get("leaf_results", []):
            lineage = r.get("lineage", [])
            dept = None
            for node_id in lineage:
                if node_id.startswith("dept:"):
                    dept = node_id
                    break
            if not dept:
                continue

            if dept not in dept_results:
                dept_results[dept] = []
            dept_results[dept].append(r)

        for dept, results in dept_results.items():
            if dept not in dept_data:
                dept_data[dept] = {
                    "department": dept,
                    "examples": [],
                    "captains_activated": set(),
                    "total_tasks": 0,
                }

            entry = dept_data[dept]
            entry["total_tasks"] += len(results)
            for r in results:
                entry["captains_activated"].add(r.get("node_id", ""))

            entry["examples"].append({
                "intent": trace.get("intent", ""),
                "captains": [r.get("node_id") for r in results],
                "statuses": [r.get("status") for r in results],
                "trace_id": trace.get("run_id", "unknown"),
            })

    # Convert sets to lists for JSON serialization
    for dept in dept_data.values():
        dept["captains_activated"] = sorted(dept["captains_activated"])

    return dept_data


# ── Export: write training files ───────────────────────────────


def export_training_data(traces, output_dir, formats=None, filters=None):
    """Export training data in multiple formats.

    Creates:
      output_dir/
        alpaca.jsonl          — full dataset, Alpaca format
        sharegpt.jsonl        — full dataset, ShareGPT format
        cot.jsonl             — full dataset, CoT format
        by_node/
          <node_id>.jsonl     — per-node training examples
        by_department/
          <dept_id>.jsonl     — per-department training examples
        by_scale/
          <scale>.jsonl       — per-scale training examples
        manifest.json         — dataset metadata, counts, hashes
    """
    if formats is None:
        formats = ["alpaca", "sharegpt", "cot"]
    if filters is None:
        filters = {}

    os.makedirs(output_dir, exist_ok=True)
    os.makedirs(os.path.join(output_dir, "by_node"), exist_ok=True)
    os.makedirs(os.path.join(output_dir, "by_department"), exist_ok=True)
    os.makedirs(os.path.join(output_dir, "by_scale"), exist_ok=True)

    # Apply filters
    filtered_traces = _apply_filters(traces, filters)

    manifest = {
        "created": datetime.now(timezone.utc).isoformat(),
        "total_traces": len(traces),
        "filtered_traces": len(filtered_traces),
        "formats": formats,
        "filters": filters,
        "files": {},
    }

    # Full dataset exports
    if "alpaca" in formats:
        path = os.path.join(output_dir, "alpaca.jsonl")
        count = _write_jsonl(path, [to_alpaca(t) for t in filtered_traces])
        manifest["files"]["alpaca"] = {"path": "alpaca.jsonl", "count": count}

    if "sharegpt" in formats:
        path = os.path.join(output_dir, "sharegpt.jsonl")
        count = _write_jsonl(path, [to_sharegpt(t) for t in filtered_traces])
        manifest["files"]["sharegpt"] = {"path": "sharegpt.jsonl", "count": count}

    if "cot" in formats:
        path = os.path.join(output_dir, "cot.jsonl")
        count = _write_jsonl(path, [to_cot(t) for t in filtered_traces])
        manifest["files"]["cot"] = {"path": "cot.jsonl", "count": count}

    # Per-node exports
    node_data = extract_per_node(filtered_traces)
    for node_id, data in node_data.items():
        safe_name = node_id.replace(":", "_").replace("/", "_")
        path = os.path.join(output_dir, "by_node", f"{safe_name}.jsonl")
        count = _write_jsonl(path, data["examples"])
        manifest["files"][f"node:{node_id}"] = {
            "path": f"by_node/{safe_name}.jsonl",
            "count": count,
            "total_runs": data["total_runs"],
            "completed": data["completed"],
            "failed": data["failed"],
        }

    # Per-department exports
    dept_data = extract_per_department(filtered_traces)
    for dept_id, data in dept_data.items():
        safe_name = dept_id.replace(":", "_")
        path = os.path.join(output_dir, "by_department", f"{safe_name}.jsonl")
        count = _write_jsonl(path, data["examples"])
        manifest["files"][f"dept:{dept_id}"] = {
            "path": f"by_department/{safe_name}.jsonl",
            "count": count,
            "captains": data["captains_activated"],
            "total_tasks": data["total_tasks"],
        }

    # Per-scale exports
    scale_data = _group_by_scale(filtered_traces)
    for scale, examples in scale_data.items():
        path = os.path.join(output_dir, "by_scale", f"{scale}.jsonl")
        count = _write_jsonl(path, examples)
        manifest["files"][f"scale:{scale}"] = {
            "path": f"by_scale/{scale}.jsonl",
            "count": count,
        }

    # Write manifest
    manifest_path = os.path.join(output_dir, "manifest.json")
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    # Content hash of the full dataset
    h = hashlib.sha256()
    for t in filtered_traces:
        h.update(json.dumps(t, sort_keys=True).encode())
    manifest["dataset_hash"] = h.hexdigest()[:16]

    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    return manifest


def _apply_filters(traces, filters):
    """Filter traces by criteria."""
    result = traces

    if "status" in filters:
        result = [t for t in result if t.get("aggregated", {}).get("status") == filters["status"]]

    if "min_tasks" in filters:
        result = [t for t in result if t.get("tasks_decomposed", 0) >= filters["min_tasks"]]

    if "department" in filters:
        dept = filters["department"]
        result = [t for t in result if any(
            dept in r.get("lineage", []) for r in t.get("leaf_results", [])
        )]

    if "node" in filters:
        node = filters["node"]
        result = [t for t in result if any(
            r.get("node_id") == node for r in t.get("leaf_results", [])
        )]

    return result


def _group_by_scale(traces):
    """Group training examples by node scale."""
    scale_data = {}
    for trace in traces:
        for r in trace.get("leaf_results", []):
            scale = r.get("scale", "unknown")
            if scale not in scale_data:
                scale_data[scale] = []
            scale_data[scale].append({
                "intent": r.get("intent", trace.get("intent", "")),
                "node_id": r.get("node_id"),
                "node_name": r.get("node_name"),
                "status": r.get("status"),
                "lineage": r.get("lineage", []),
                "output": r.get("output"),
            })
    return scale_data


def _write_jsonl(path, items):
    """Write items as JSONL."""
    count = 0
    with open(path, "w") as f:
        for item in items:
            f.write(json.dumps(item, default=str) + "\n")
            count += 1
    return count


# ── CLI ────────────────────────────────────────────────────────


def main():
    import argparse
    parser = argparse.ArgumentParser(description="PTC Training Data Extraction")
    parser.add_argument("action", choices=["extract", "preview", "stats"],
                       help="extract: write training files, preview: show sample, stats: show counts")
    parser.add_argument("--source", default="local", choices=["mongodb", "local", "inline"],
                       help="Where to read traces from")
    parser.add_argument("--traces-dir", default=None,
                       help="Directory with trace JSON files (for local source)")
    parser.add_argument("--output", "-o", default=None,
                       help="Output directory for training data")
    parser.add_argument("--format", nargs="+", default=["alpaca", "sharegpt", "cot"],
                       help="Output formats")
    parser.add_argument("--limit", type=int, default=1000,
                       help="Max traces to process")
    parser.add_argument("--filter-status", help="Filter by aggregation status")
    parser.add_argument("--filter-dept", help="Filter by department")
    parser.add_argument("--filter-node", help="Filter by node ID")
    parser.add_argument("--trace", help="Single trace JSON file to process")

    args = parser.parse_args()

    # Collect traces
    if args.trace:
        with open(args.trace) as f:
            traces = [json.load(f)]
    else:
        traces = collect_traces(source=args.source, limit=args.limit)

    if not traces:
        print("No traces found. Run some PTC executions first:")
        print("  claude-cage ptc run \"your intent\" --live")
        print("  # or generate a trace file:")
        print("  claude-cage ptc run \"your intent\" --json > training/traces/my-trace.json")
        sys.exit(0)

    # Build filters
    filters = {}
    if args.filter_status:
        filters["status"] = args.filter_status
    if args.filter_dept:
        filters["department"] = args.filter_dept
    if args.filter_node:
        filters["node"] = args.filter_node

    if args.action == "stats":
        print(f"Traces: {len(traces)}")
        total_tasks = sum(t.get("tasks_decomposed", 0) for t in traces)
        total_completed = sum(t.get("tasks_completed", 0) for t in traces)
        print(f"Total leaf tasks: {total_tasks}")
        print(f"Completed: {total_completed}")

        node_data = extract_per_node(traces)
        print(f"Unique nodes: {len(node_data)}")

        dept_data = extract_per_department(traces)
        print(f"Departments: {len(dept_data)}")
        for dept, data in sorted(dept_data.items()):
            print(f"  {dept}: {data['total_tasks']} tasks, {len(data['captains_activated'])} captains")

    elif args.action == "preview":
        trace = traces[0]
        print("=== ALPACA FORMAT ===")
        print(json.dumps(to_alpaca(trace), indent=2))
        print("\n=== COT FORMAT ===")
        print(json.dumps(to_cot(trace), indent=2))
        print("\n=== SHAREGPT FORMAT ===")
        print(json.dumps(to_sharegpt(trace), indent=2))

    elif args.action == "extract":
        output_dir = args.output or os.path.join(CAGE_ROOT, "training", "datasets",
                                                   datetime.now().strftime("%Y%m%d_%H%M%S"))
        manifest = export_training_data(traces, output_dir, formats=args.format, filters=filters)
        print(f"Training data exported to: {output_dir}")
        print(f"  Traces: {manifest['filtered_traces']}")
        print(f"  Files: {len(manifest['files'])}")
        for name, info in manifest["files"].items():
            print(f"    {name}: {info.get('count', '?')} examples → {info['path']}")


if __name__ == "__main__":
    main()
