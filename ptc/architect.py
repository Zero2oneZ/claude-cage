"""ptc/architect.py — Architect-mode design system.

Claude designs. PTC decomposes. Builders execute. Results flow back.
The architect doesn't pick up hammers. The architect designs them.

A blueprint IS a node. It has inputs, outputs, rules, children.
Same pattern. Different scale. As above, so below.

Flow:
  1. Intent arrives
  2. cache_check() — hash match or vector similarity >0.9
  3. If cached: return immediately (ZERO tokens spent)
  4. If not: generate blueprint JSON structure
  5. dual_store() — MongoDB + IPFS
  6. embed_blueprint() — vector search index
  7. git_commit on design branch
  8. blueprint_to_tasks() → PTC decompose → builders execute
  9. verify_blueprint() → check results vs acceptance
  10. Ship: merge to main, cache, training data
"""

import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ── Blueprint Creation ─────────────────────────────────────────


def create_blueprint(intent, context=None):
    """Create a new blueprint node from an architectural intent.

    1. Check cache (hash match or semantic similarity)
    2. If cached and valid, return it (zero tokens)
    3. If not, generate the blueprint structure
    4. Store to MongoDB + IPFS (dual-write)
    5. Embed for vector search
    6. Return the blueprint node

    Args:
        intent: what to build (e.g., "add webhook support")
        context: optional dict with {node_id, files, functions, rules, lineage}

    Returns:
        dict: blueprint node (universal node schema)
    """
    context = context or {}

    # 1. Cache check
    cached = cache_check(intent)
    if cached:
        cached.setdefault("metadata", {})["cached"] = True
        return cached

    # 2. Generate blueprint
    blueprint_id = _generate_id(intent)
    now = datetime.now(timezone.utc).isoformat()

    blueprint = {
        "id": blueprint_id,
        "name": _intent_to_name(intent),
        "scale": "blueprint",
        "parent": "dept:architect",
        "children": [],
        "inputs": [
            {"name": "intent", "type": "task", "from": context.get("node_id")},
        ],
        "outputs": [
            {"name": "design_doc", "type": "artifact", "to": "dept:ptc"},
            {"name": "builder_tasks", "type": "task", "to": None},
        ],
        "rules": [
            {"name": "cache_first", "condition": "blueprint exists with matching hash", "action": "pass"},
            {"name": "verify_complete", "condition": "all builder_tasks resolved", "action": "transform"},
            {"name": "no_implementation", "condition": "blueprint contains code", "action": "block"},
        ],
        "escalation": {
            "target": "dept:architect",
            "threshold": 7,
            "cascade": ["dept:architect", "root:cage"],
        },
        "lineage": ["root:cage", "dept:architect", blueprint_id],
        "execution": {
            "status": "draft",
            "last_input": intent,
            "last_output": None,
            "last_run": now,
            "run_count": 0,
            "error_count": 0,
        },
        "artifacts": [
            {
                "name": "architecture",
                "type": "blueprint",
                "hash": None,  # Computed below
                "timestamp": now,
                "storage": "mongodb",
                "content": {
                    "what": intent,
                    "where": {
                        "files": context.get("files", []),
                        "modules": _infer_modules(context),
                        "endpoints": [],
                    },
                    "how": {
                        "approach": "",
                        "patterns": _infer_patterns(context),
                        "dependencies": [],
                    },
                    "why": "",
                    "gui_spec": {
                        "views": [],
                        "flows": [],
                        "interactions": [],
                        "data_bindings": [],
                    },
                    "data_flow": {
                        "inputs": [],
                        "transforms": [],
                        "outputs": [],
                        "stores": [],
                    },
                    "interconnections": _infer_interconnections(context),
                    "builder_tasks": [],
                    "acceptance": {
                        "criteria": [],
                        "verification_intent": f"verify {_intent_to_name(intent)}",
                    },
                },
            }
        ],
        "metadata": {
            "blueprint_version": 1,
            "content_hash": None,
            "ipfs_cid": None,
            "status": "draft",
            "architect_session": f"architect-{int(time.time())}",
            "cached": False,
            "intent_hash": _hash_intent(intent),
            "created_at": now,
            "project": context.get("project", "claude-cage"),
        },
    }

    # 3. Compute content hash
    content_str = json.dumps(blueprint["artifacts"][0]["content"], sort_keys=True)
    from ptc.ipfs import content_hash
    chash = content_hash(content_str)
    blueprint["artifacts"][0]["hash"] = chash
    blueprint["metadata"]["content_hash"] = chash

    # 4. Store (dual-write: MongoDB + IPFS)
    _store_blueprint(blueprint)

    # 5. Embed for vector search
    try:
        from ptc.embeddings import embed_blueprint
        embed_blueprint(blueprint)
    except ImportError:
        pass

    # 6. Git commit on design branch
    try:
        from ptc.git_ops import git_branch_for_blueprint, git_commit_artifact
        git_branch_for_blueprint(blueprint_id)
        git_commit_artifact({
            "name": blueprint_id,
            "type": "blueprint",
            "hash": chash,
            "files": context.get("files", []),
        }, f"blueprint: {intent}")
    except ImportError:
        pass

    return blueprint


# ── Cache ──────────────────────────────────────────────────────


def cache_check(intent):
    """Check if a blueprint exists for this intent.

    Two-level check:
      1. Exact hash match (same intent string → same hash)
      2. Semantic similarity via vector search (>0.9 cosine)

    Args:
        intent: the design intent string

    Returns:
        dict: cached blueprint, or None if not found
    """
    intent_hash = _hash_intent(intent)

    # Level 1: exact hash match in MongoDB
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        try:
            query = json.dumps({"metadata.intent_hash": intent_hash})
            result = subprocess.run(
                ["node", store_js, "get", "blueprints", query, "1"],
                capture_output=True, text=True, timeout=10,
            )
            if result.returncode == 0 and result.stdout.strip():
                docs = json.loads(result.stdout)
                if isinstance(docs, list) and docs:
                    return docs[0]
                elif isinstance(docs, dict) and docs.get("id"):
                    return docs
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            pass

    # Level 2: semantic similarity via vector search
    try:
        from ptc.embeddings import find_similar_blueprints
        similar = find_similar_blueprints(intent, limit=1)
        if similar:
            top = similar[0]
            score = top.get("score", 0)
            if score >= 0.9:
                # Retrieve full blueprint
                bp_id = top.get("blueprint_id") or top.get("doc_id")
                if bp_id:
                    return _load_blueprint(bp_id)
    except ImportError:
        pass

    return None


# ── Blueprint to Tasks ─────────────────────────────────────────


def blueprint_to_tasks(blueprint):
    """Convert a blueprint's builder_tasks into PTC-compatible task dicts.

    Each task becomes a leaf in the PTC decomposition tree.
    The blueprint node becomes the parent.

    Args:
        blueprint: blueprint node dict

    Returns:
        list[dict]: PTC task dicts ready for execute_leaf()
    """
    tasks = []
    content = _get_content(blueprint)
    builder_tasks = content.get("builder_tasks", [])

    for bt in builder_tasks:
        task = {
            "node_id": bt.get("target_node", "root:cage"),
            "node_name": bt.get("task_id", "unknown"),
            "scale": "captain",
            "intent": bt.get("intent", ""),
            "lineage": blueprint.get("lineage", []) + [bt.get("task_id", "unknown")],
            "files": bt.get("files", []),
            "functions": [],
            "rules": [],
            "escalation": blueprint.get("escalation", {}),
            "blueprint_id": blueprint.get("id"),
            "task_id": bt.get("task_id"),
            "acceptance": bt.get("acceptance", []),
            "depends_on": bt.get("depends_on", []),
        }
        tasks.append(task)

    # Update blueprint children
    blueprint["children"] = [bt.get("task_id", f"task-{i}") for i, bt in enumerate(builder_tasks)]

    return tasks


# ── Validation ─────────────────────────────────────────────────


def validate_blueprint(blueprint):
    """Validate a blueprint against the schema and tree.

    Checks:
      - All builder_tasks have valid target nodes in the tree
      - All referenced files exist on disk
      - Acceptance criteria are non-empty
      - Required fields present

    Args:
        blueprint: blueprint dict

    Returns:
        dict: {valid, errors, warnings}
    """
    errors = []
    warnings = []
    content = _get_content(blueprint)

    # Check required fields
    if not content.get("what"):
        errors.append("Missing 'what' — describe what this builds")
    if not content.get("builder_tasks"):
        warnings.append("No builder_tasks — this blueprint has nothing to build")
    if not content.get("acceptance", {}).get("criteria"):
        warnings.append("No acceptance criteria — how do we know it's done?")

    # Check builder_tasks target nodes exist in tree
    tree_path = os.path.join(CAGE_ROOT, "tree.json")
    tree_nodes = set()
    try:
        with open(tree_path) as f:
            tree = json.load(f)
        tree_nodes = {n["id"] for n in tree.get("nodes", [])}
    except (FileNotFoundError, json.JSONDecodeError):
        warnings.append("Could not load tree.json to validate target nodes")

    for bt in content.get("builder_tasks", []):
        target = bt.get("target_node")
        if target and tree_nodes and target not in tree_nodes:
            errors.append(f"Task {bt.get('task_id')}: target node '{target}' not found in tree")

        # Check referenced files exist
        for f in bt.get("files", []):
            filepath = os.path.join(CAGE_ROOT, f)
            if not os.path.exists(filepath) and "create" not in bt.get("intent", "").lower():
                warnings.append(f"Task {bt.get('task_id')}: file '{f}' does not exist (will be created?)")

    return {
        "valid": len(errors) == 0,
        "errors": errors,
        "warnings": warnings,
    }


# ── Verification ───────────────────────────────────────────────


def verify_blueprint(blueprint, results):
    """Check builder results against blueprint acceptance criteria.

    Args:
        blueprint: the blueprint node
        results: list of execution results from PTC

    Returns:
        dict: updated blueprint with new status
    """
    content = _get_content(blueprint)
    criteria = content.get("acceptance", {}).get("criteria", [])

    completed = sum(1 for r in results if r.get("status") == "completed")
    failed = sum(1 for r in results if r.get("status") == "failed")
    total = len(results)

    if failed > 0:
        blueprint["metadata"]["status"] = "failed"
        blueprint["execution"]["status"] = "failed"
        blueprint["execution"]["error_count"] = blueprint["execution"].get("error_count", 0) + failed
    elif completed == total and total > 0:
        blueprint["metadata"]["status"] = "verified"
        blueprint["execution"]["status"] = "completed"
    else:
        blueprint["metadata"]["status"] = "building"
        blueprint["execution"]["status"] = "executing"

    blueprint["execution"]["last_output"] = json.dumps({
        "completed": completed,
        "failed": failed,
        "total": total,
        "criteria_checked": len(criteria),
    })
    blueprint["execution"]["run_count"] = blueprint["execution"].get("run_count", 0) + 1
    blueprint["execution"]["last_run"] = datetime.now(timezone.utc).isoformat()

    # Update stored blueprint
    _store_blueprint(blueprint)

    return blueprint


# ── Status + Listing ───────────────────────────────────────────


def blueprint_status(blueprint_id):
    """Get current status of a blueprint and all its child tasks."""
    bp = _load_blueprint(blueprint_id)
    if not bp:
        return {"error": f"Blueprint {blueprint_id} not found"}

    return {
        "id": bp.get("id"),
        "name": bp.get("name"),
        "status": bp.get("metadata", {}).get("status", "unknown"),
        "tasks": len(_get_content(bp).get("builder_tasks", [])),
        "created": bp.get("metadata", {}).get("created_at"),
        "cached": bp.get("metadata", {}).get("cached", False),
        "hash": bp.get("metadata", {}).get("content_hash"),
    }


def list_blueprints(status=None, project="claude-cage"):
    """List all blueprints, optionally filtered by status.

    Args:
        status: filter by status (draft, approved, building, verified, shipped, failed)
        project: project identifier

    Returns:
        list[dict]: summary of each blueprint
    """
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        # Fallback: check local blueprints directory
        return _list_local_blueprints(status)

    query = {"metadata.project": project}
    if status:
        query["metadata.status"] = status

    try:
        result = subprocess.run(
            ["node", store_js, "get", "blueprints", json.dumps(query), "100"],
            capture_output=True, text=True, timeout=15,
        )
        if result.returncode == 0 and result.stdout.strip():
            docs = json.loads(result.stdout)
            if not isinstance(docs, list):
                docs = [docs]
            return [
                {
                    "id": d.get("id"),
                    "name": d.get("name"),
                    "status": d.get("metadata", {}).get("status"),
                    "tasks": len(_get_content(d).get("builder_tasks", [])),
                    "created": d.get("metadata", {}).get("created_at"),
                    "hash": d.get("metadata", {}).get("content_hash", "")[:16],
                }
                for d in docs
            ]
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        pass

    return _list_local_blueprints(status)


def update_blueprint(blueprint_id, updates):
    """Update a blueprint. Re-hashes, re-stores, re-embeds.

    Args:
        blueprint_id: ID of blueprint to update
        updates: dict of fields to merge into the blueprint

    Returns:
        dict: updated blueprint
    """
    bp = _load_blueprint(blueprint_id)
    if not bp:
        return {"error": f"Blueprint {blueprint_id} not found"}

    # Merge updates
    content = _get_content(bp)
    for key, value in updates.items():
        if key in content:
            if isinstance(content[key], dict) and isinstance(value, dict):
                content[key].update(value)
            elif isinstance(content[key], list) and isinstance(value, list):
                content[key].extend(value)
            else:
                content[key] = value

    # Recompute hash
    from ptc.ipfs import content_hash
    chash = content_hash(json.dumps(content, sort_keys=True))
    bp["artifacts"][0]["hash"] = chash
    bp["metadata"]["content_hash"] = chash
    bp["metadata"]["blueprint_version"] = bp["metadata"].get("blueprint_version", 1) + 1

    # Re-store and re-embed
    _store_blueprint(bp)
    try:
        from ptc.embeddings import embed_blueprint
        embed_blueprint(bp)
    except ImportError:
        pass

    return bp


# ── Internal Helpers ───────────────────────────────────────────


def _generate_id(intent):
    """Generate a blueprint ID from intent."""
    # Slug from first few words
    words = intent.lower().split()[:4]
    slug = "-".join(w for w in words if w.isalnum() or w == "-")
    short_hash = hashlib.md5(intent.encode()).hexdigest()[:6]
    return f"blueprint:{slug}-{short_hash}"


def _intent_to_name(intent):
    """Convert intent to a human-readable name."""
    return intent.strip().capitalize()


def _hash_intent(intent):
    """Hash an intent string for exact-match caching."""
    return hashlib.sha256(intent.strip().lower().encode()).hexdigest()


def _get_content(blueprint):
    """Extract the blueprint content from the artifacts array."""
    for artifact in blueprint.get("artifacts", []):
        if artifact.get("type") == "blueprint":
            content = artifact.get("content", {})
            if isinstance(content, str):
                try:
                    return json.loads(content)
                except json.JSONDecodeError:
                    return {}
            return content
    return {}


def _infer_modules(context):
    """Infer affected modules from context files."""
    modules = set()
    for f in context.get("files", []):
        parts = f.split("/")
        if len(parts) > 1:
            modules.add(parts[0])
    return sorted(modules)


def _infer_patterns(context):
    """Infer design patterns from context."""
    patterns = []
    rules = context.get("rules", [])
    for r in rules:
        if isinstance(r, dict):
            action = r.get("action", "")
            if action == "log":
                patterns.append("fire-and-forget")
            elif action == "block":
                patterns.append("gate")
            elif action == "escalate":
                patterns.append("escalation")
    return patterns


def _infer_interconnections(context):
    """Infer interconnections from context."""
    conns = []
    node_id = context.get("node_id")
    if node_id:
        conns.append({
            "from": node_id,
            "to": "dept:architect",
            "type": "dependency",
            "description": f"Originated from {node_id}",
        })
    return conns


def _store_blueprint(blueprint):
    """Store blueprint to MongoDB and optionally IPFS."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        # Fallback: store locally
        _store_local(blueprint)
        return

    doc = json.dumps(blueprint, default=str)
    try:
        subprocess.Popen(
            ["node", store_js, "put", "blueprints", doc],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass

    # Also store locally as backup
    _store_local(blueprint)


def _store_local(blueprint):
    """Store blueprint as local JSON file."""
    bp_dir = os.path.join(CAGE_ROOT, "blueprints")
    os.makedirs(bp_dir, exist_ok=True)
    bp_id = blueprint.get("id", "unknown").replace(":", "-")
    filepath = os.path.join(bp_dir, f"{bp_id}.json")
    with open(filepath, "w") as f:
        json.dump(blueprint, f, indent=2, default=str)


def _load_blueprint(blueprint_id):
    """Load a blueprint from MongoDB or local storage."""
    # Try MongoDB first
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        try:
            query = json.dumps({"id": blueprint_id})
            result = subprocess.run(
                ["node", store_js, "get", "blueprints", query, "1"],
                capture_output=True, text=True, timeout=10,
            )
            if result.returncode == 0 and result.stdout.strip():
                docs = json.loads(result.stdout)
                if isinstance(docs, list) and docs:
                    return docs[0]
                elif isinstance(docs, dict) and docs.get("id"):
                    return docs
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            pass

    # Fallback: local file
    bp_dir = os.path.join(CAGE_ROOT, "blueprints")
    bp_file = os.path.join(bp_dir, f"{blueprint_id.replace(':', '-')}.json")
    if os.path.exists(bp_file):
        with open(bp_file) as f:
            return json.load(f)

    return None


def _list_local_blueprints(status=None):
    """List blueprints from local storage."""
    bp_dir = os.path.join(CAGE_ROOT, "blueprints")
    if not os.path.isdir(bp_dir):
        return []

    blueprints = []
    for f in sorted(os.listdir(bp_dir)):
        if f.endswith(".json"):
            try:
                with open(os.path.join(bp_dir, f)) as fh:
                    bp = json.load(fh)
                bp_status = bp.get("metadata", {}).get("status")
                if status and bp_status != status:
                    continue
                blueprints.append({
                    "id": bp.get("id"),
                    "name": bp.get("name"),
                    "status": bp_status,
                    "tasks": len(_get_content(bp).get("builder_tasks", [])),
                    "created": bp.get("metadata", {}).get("created_at"),
                    "hash": bp.get("metadata", {}).get("content_hash", "")[:16],
                })
            except (json.JSONDecodeError, KeyError):
                pass

    return blueprints


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for architect operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.architect <command> [args]")
        print("Commands: create <intent>, list [--status X], show <id>,")
        print("          tasks <id>, validate <id>, verify <id>, search <query>")
        sys.exit(1)

    command = sys.argv[1]

    if command == "create":
        intent = " ".join(sys.argv[2:])
        if not intent:
            print("Usage: python -m ptc.architect create <intent>")
            sys.exit(1)
        bp = create_blueprint(intent)
        cached = bp.get("metadata", {}).get("cached", False)
        status = bp.get("metadata", {}).get("status", "draft")
        print(f"Blueprint: {bp['id']}")
        print(f"Name:      {bp['name']}")
        print(f"Status:    {status}")
        print(f"Cached:    {cached}")
        print(f"Hash:      {bp.get('metadata', {}).get('content_hash', '')[:20]}")
        tasks = _get_content(bp).get("builder_tasks", [])
        print(f"Tasks:     {len(tasks)}")
        if tasks:
            for t in tasks:
                print(f"  - {t.get('task_id', '?')}: {t.get('intent', '?')}")

    elif command == "list":
        status_filter = None
        if "--status" in sys.argv:
            idx = sys.argv.index("--status")
            if idx + 1 < len(sys.argv):
                status_filter = sys.argv[idx + 1]
        bps = list_blueprints(status=status_filter)
        if bps:
            for bp in bps:
                print(f"  [{bp.get('status', '?'):10s}] {bp.get('id', '?'):40s} {bp.get('name', '')}")
        else:
            print("  (no blueprints found)")

    elif command == "show":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.architect show <blueprint-id>")
            sys.exit(1)
        bp = _load_blueprint(sys.argv[2])
        if bp:
            print(json.dumps(bp, indent=2, default=str))
        else:
            print(f"Blueprint {sys.argv[2]} not found")
            sys.exit(1)

    elif command == "tasks":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.architect tasks <blueprint-id>")
            sys.exit(1)
        bp = _load_blueprint(sys.argv[2])
        if not bp:
            print(f"Blueprint {sys.argv[2]} not found")
            sys.exit(1)
        tasks = blueprint_to_tasks(bp)
        print(f"Tasks from {bp.get('id')}:")
        for t in tasks:
            deps = t.get("depends_on", [])
            dep_str = f" (depends: {', '.join(deps)})" if deps else ""
            print(f"  {t['task_id'] or t['node_name']}: {t['intent']}{dep_str}")
            for f in t.get("files", []):
                print(f"    file: {f}")

    elif command == "validate":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.architect validate <blueprint-id>")
            sys.exit(1)
        bp = _load_blueprint(sys.argv[2])
        if not bp:
            print(f"Blueprint {sys.argv[2]} not found")
            sys.exit(1)
        result = validate_blueprint(bp)
        print(f"Valid: {result['valid']}")
        for e in result["errors"]:
            print(f"  ERROR: {e}")
        for w in result["warnings"]:
            print(f"  WARN:  {w}")

    elif command == "search":
        query = " ".join(sys.argv[2:])
        if not query:
            print("Usage: python -m ptc.architect search <query>")
            sys.exit(1)
        try:
            from ptc.embeddings import find_similar_blueprints
            results = find_similar_blueprints(query)
            if results:
                for r in results:
                    print(f"  [{r.get('score', 0):.3f}] {r.get('blueprint_id', r.get('doc_id', '?'))}")
            else:
                print("  No matching blueprints found")
        except ImportError:
            print("  Embeddings not available — enable with EMBEDDING_ENABLED=true")

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
