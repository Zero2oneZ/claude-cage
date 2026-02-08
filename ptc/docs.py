"""ptc/docs.py — Circular Documentation System.

Documentation is dead text. It drifts from code. It lies.
The fix: make documentation part of the code itself — stored the same way
(MongoDB + IPFS + vector embeddings + git), interconnected bidirectionally,
staleness-tracked by file hash. Change one side, the other knows. One circle.

Every tree node gets a doc artifact. Every doc references every related doc.
The interconnection graph IS the circle — structural edges (tree parent/child),
code-shared edges (overlapping files), and semantic edges (vector similarity > 0.7).
All bidirectional. The circle completes.
"""

import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ── Ownership Map ────────────────────────────────────────────────
# Maps tree node IDs to actual file paths they own.
# Shared files between nodes create automatic code_shared edges.
# That IS the circle.

OWNERSHIP_MAP = {
    "root:cage":         ["bin/claude-cage", "Makefile", "tree.json", "CLAUDE.md", "README.md"],
    "dept:runtime":      ["lib/docker.sh", "lib/session.sh", "docker/cli/Dockerfile", "docker/desktop/Dockerfile"],
    "capt:docker":       ["lib/docker.sh"],
    "capt:compose":      ["docker-compose.yml"],
    "capt:images":       ["docker/cli/Dockerfile", "docker/desktop/Dockerfile"],
    "dept:security":     ["lib/sandbox.sh", "security/apparmor-profile", "security/seccomp-default.json"],
    "capt:sandbox":      ["lib/sandbox.sh"],
    "capt:seccomp":      ["security/seccomp-default.json"],
    "capt:apparmor":     ["security/apparmor-profile"],
    "capt:network":      ["lib/sandbox.sh"],
    "dept:sessions":     ["lib/session.sh"],
    "capt:lifecycle":    ["lib/session.sh"],
    "capt:memory":       [],
    "dept:config":       ["lib/config.sh", "config/default.yaml"],
    "capt:yaml":         ["config/default.yaml"],
    "capt:cli-parse":    ["lib/cli.sh"],
    "dept:observe":      ["lib/cli.sh"],
    "capt:metrics":      [],
    "capt:mongodb":      ["mongodb/store.js", "mongodb/vector-setup.js"],
    "dept:web":          ["web/app.py", "web/templates/index.html"],
    "capt:flask":        ["web/app.py"],
    "capt:frontend":     ["web/templates/index.html"],
    "dept:tree":         ["lib/tree.sh", "tree.json", "universal-node.schema.json"],
    "capt:tree-ops":     ["lib/tree.sh"],
    "capt:scaffold":     ["lib/tree.sh"],
    "dept:ptc":          ["ptc/engine.py", "ptc/executor.py"],
    "capt:engine":       ["ptc/engine.py"],
    "capt:executor":     ["ptc/executor.py"],
    "dept:training":     ["ptc/training.py"],
    "capt:extract":      ["ptc/training.py"],
    "capt:lora":         ["ptc/training.py"],
    "dept:architect":    ["ptc/architect.py", "ptc/ipfs.py", "ptc/embeddings.py", "ptc/git_ops.py"],
    "capt:blueprints":   ["ptc/architect.py", "schemas/blueprint.schema.json"],
    "capt:ipfs":         ["ptc/ipfs.py"],
    "capt:vectors":      ["ptc/embeddings.py"],
    "capt:docs":         ["ptc/docs.py", "lib/docs.sh"],
    "project:gentlyos":       ["gentlyos/tree.json"],
    "capt:federation":        ["ptc/federation.py"],
    "project:test-apps":      ["projects/test-apps/kv-store.js", "projects/test-apps/chat-server.js", "projects/test-apps/task-api.js"],
    "capt:kv-store":          ["projects/test-apps/kv-store.js"],
    "capt:chat-server":       ["projects/test-apps/chat-server.js"],
    "capt:task-api":          ["projects/test-apps/task-api.js"],
    "project:test-apps-rust": ["projects/test-apps-rust/src/kv_store.rs", "projects/test-apps-rust/src/chat_server.rs", "projects/test-apps-rust/src/task_api.rs"],
    "capt:kv-store-rs":       ["projects/test-apps-rust/src/kv_store.rs"],
    "capt:chat-server-rs":    ["projects/test-apps-rust/src/chat_server.rs"],
    "capt:task-api-rs":       ["projects/test-apps-rust/src/task_api.rs"],
}


# ── Tree Loading ─────────────────────────────────────────────────


def load_tree(path=None):
    """Load tree.json, return nodes_by_id dict.

    Args:
        path: path to tree.json (defaults to CAGE_ROOT/tree.json)

    Returns:
        dict: {node_id: node_dict, ...}
    """
    if path is None:
        path = os.path.join(CAGE_ROOT, "tree.json")
    with open(path) as f:
        tree = json.load(f)
    return {n["id"]: n for n in tree.get("nodes", [])}


def get_ownership_map():
    """Return the ownership map, resolving paths relative to CAGE_ROOT.

    Returns:
        dict: {node_id: [absolute_file_paths]}
    """
    result = {}
    for node_id, files in OWNERSHIP_MAP.items():
        result[node_id] = [os.path.join(CAGE_ROOT, f) for f in files]
    return result


# ── File Hashing ─────────────────────────────────────────────────


def compute_file_hash(files):
    """SHA-256 of concatenated file contents.

    Args:
        files: list of file paths (relative to CAGE_ROOT or absolute)

    Returns:
        str: "sha256:<hex>" or "sha256:empty" if no files
    """
    hasher = hashlib.sha256()
    found = False
    for f in sorted(files):
        fpath = f if os.path.isabs(f) else os.path.join(CAGE_ROOT, f)
        if os.path.isfile(fpath):
            try:
                with open(fpath, "rb") as fh:
                    hasher.update(fh.read())
                found = True
            except (OSError, PermissionError):
                pass
    if not found:
        return "sha256:empty"
    return f"sha256:{hasher.hexdigest()}"


# ── Doc Generation ───────────────────────────────────────────────


def generate_doc(node_id, tree, ownership=None):
    """Build doc artifact for one node from tree metadata + file analysis.

    Args:
        node_id: tree node ID (e.g. "dept:security")
        tree: nodes_by_id dict from load_tree()
        ownership: ownership map (defaults to OWNERSHIP_MAP)

    Returns:
        dict: doc artifact ready for storage
    """
    if ownership is None:
        ownership = OWNERSHIP_MAP

    node = tree.get(node_id)
    if not node:
        return {"error": f"Node {node_id} not found in tree"}

    now = datetime.now(timezone.utc).isoformat()
    owned_files = ownership.get(node_id, [])
    metadata = node.get("metadata", {})

    # Extract key concepts from node metadata and rules
    key_concepts = []
    for r in node.get("rules", []):
        if isinstance(r, dict) and r.get("name"):
            key_concepts.append(r["name"])
    crates = metadata.get("crates_owned", [])
    key_concepts.extend(crates)

    # Entry points from metadata
    entry_points = metadata.get("functions", [])

    # What it does — inferred from rules and role
    what_it_does = []
    role = metadata.get("role", "")
    if role:
        what_it_does.append(role)
    for r in node.get("rules", []):
        if isinstance(r, dict) and r.get("condition"):
            what_it_does.append(f"Rule: {r['name']} — {r['condition']} → {r.get('action', 'pass')}")

    # Compute file hash for staleness tracking
    source_hash = compute_file_hash(owned_files)

    # Content hash of the doc itself
    doc_content = {
        "node_id": node_id,
        "title": node.get("name", node_id),
        "description": role or f"{node.get('name', '')} ({node.get('scale', '')})",
        "what_it_does": what_it_does,
        "owned_files": owned_files,
        "entry_points": entry_points,
        "key_concepts": list(set(key_concepts)),
    }
    content_str = json.dumps(doc_content, sort_keys=True)
    content_hash = f"sha256:{hashlib.sha256(content_str.encode()).hexdigest()}"

    doc = {
        "node_id": node_id,
        "title": node.get("name", node_id),
        "scale": node.get("scale", ""),
        "description": role or f"{node.get('name', '')} ({node.get('scale', '')})",
        "what_it_does": what_it_does,
        "owned_files": owned_files,
        "entry_points": entry_points,
        "key_concepts": list(set(key_concepts)),
        "cross_refs": {},
        "staleness": {
            "source_hash": source_hash,
            "is_stale": False,
            "last_verified": now,
        },
        "content_hash": content_hash,
        "ipfs_cid": None,
        "project": "claude-cage",
        "_ts": now,
    }

    return doc


def generate_all(tree_path=None):
    """Generate docs for all nodes in the tree.

    Args:
        tree_path: path to tree.json

    Returns:
        list[dict]: all doc artifacts
    """
    tree = load_tree(tree_path)
    docs = []
    for node_id in tree:
        doc = generate_doc(node_id, tree)
        if "error" not in doc:
            docs.append(doc)
    return docs


# ── Cross-References ─────────────────────────────────────────────


def compute_structural_refs(node_id, tree):
    """Parent/child/sibling cross-refs from tree structure.

    Args:
        node_id: the node to compute refs for
        tree: nodes_by_id dict

    Returns:
        dict: {parent, children, siblings}
    """
    node = tree.get(node_id, {})
    parent = node.get("parent")
    children = node.get("children", [])

    # Siblings = other children of same parent
    siblings = []
    if parent and parent in tree:
        parent_node = tree[parent]
        siblings = [c for c in parent_node.get("children", []) if c != node_id]

    return {
        "parent": parent,
        "children": children,
        "siblings": siblings,
    }


def compute_code_refs(node_id, ownership=None):
    """Find nodes sharing files with this node → code_shared edges.

    Args:
        node_id: the node to check
        ownership: ownership map

    Returns:
        list[dict]: [{node, files}] for nodes sharing files
    """
    if ownership is None:
        ownership = OWNERSHIP_MAP

    my_files = set(ownership.get(node_id, []))
    if not my_files:
        return []

    refs = []
    for other_id, other_files in ownership.items():
        if other_id == node_id:
            continue
        shared = my_files & set(other_files)
        if shared:
            refs.append({
                "node": other_id,
                "files": sorted(shared),
            })

    return refs


def compute_semantic_refs(node_id, all_docs):
    """Vector similarity > 0.7 → semantic edges.

    Uses sentence-transformers if available, otherwise returns empty.

    Args:
        node_id: the node to check
        all_docs: list of all doc artifacts

    Returns:
        list[dict]: [{node, similarity}] for semantically similar nodes
    """
    try:
        from ptc.embeddings import embed_text, _load_config
        config = _load_config()
        if not config["enabled"]:
            return []
    except ImportError:
        return []

    # Find this doc's text
    my_doc = None
    for d in all_docs:
        if d.get("node_id") == node_id:
            my_doc = d
            break
    if not my_doc:
        return []

    my_text = _doc_to_text(my_doc)
    my_embedding = embed_text(my_text)
    if my_embedding is None:
        return []

    refs = []
    for d in all_docs:
        if d.get("node_id") == node_id:
            continue
        other_text = _doc_to_text(d)
        other_embedding = embed_text(other_text)
        if other_embedding is None:
            continue

        # Cosine similarity
        sim = _cosine_similarity(my_embedding, other_embedding)
        if sim > 0.7:
            refs.append({
                "node": d["node_id"],
                "similarity": round(sim, 3),
            })

    # Sort by similarity descending
    refs.sort(key=lambda r: r["similarity"], reverse=True)
    return refs


def _doc_to_text(doc):
    """Convert doc artifact to searchable text."""
    parts = [
        doc.get("title", ""),
        doc.get("description", ""),
        " ".join(doc.get("what_it_does", [])),
        " ".join(doc.get("key_concepts", [])),
        " ".join(doc.get("entry_points", [])),
        " ".join(doc.get("owned_files", [])),
    ]
    return " ".join(parts)


def _cosine_similarity(a, b):
    """Compute cosine similarity between two vectors."""
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = sum(x * x for x in a) ** 0.5
    norm_b = sum(x * x for x in b) ** 0.5
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)


def build_cross_refs(node_id, tree, ownership=None, all_docs=None):
    """Merge all three ref types into a single cross_refs dict.

    Args:
        node_id: the node
        tree: nodes_by_id dict
        ownership: ownership map
        all_docs: list of all docs (for semantic refs)

    Returns:
        dict: {structural, code_shared, semantic}
    """
    structural = compute_structural_refs(node_id, tree)
    code_shared = compute_code_refs(node_id, ownership)
    semantic = compute_semantic_refs(node_id, all_docs or [])

    return {
        "structural": structural,
        "code_shared": code_shared,
        "semantic": semantic,
    }


# ── Full Interconnection Graph ───────────────────────────────────


def full_interconnect(tree, all_docs):
    """Build complete bidirectional graph (THE CIRCLE).

    Args:
        tree: nodes_by_id dict
        all_docs: list of all doc artifacts

    Returns:
        dict: {nodes: [...], edges: [...]}
    """
    docs_by_id = {d["node_id"]: d for d in all_docs}
    edges = []

    for doc in all_docs:
        nid = doc["node_id"]
        refs = build_cross_refs(nid, tree, OWNERSHIP_MAP, all_docs)
        doc["cross_refs"] = refs

        # Structural edges
        if refs["structural"].get("parent"):
            edges.append({
                "from": nid,
                "to": refs["structural"]["parent"],
                "type": "structural",
                "relation": "parent",
            })
        for child in refs["structural"].get("children", []):
            edges.append({
                "from": nid,
                "to": child,
                "type": "structural",
                "relation": "child",
            })

        # Code-shared edges
        for cs in refs.get("code_shared", []):
            edges.append({
                "from": nid,
                "to": cs["node"],
                "type": "code_shared",
                "files": cs["files"],
            })

        # Semantic edges
        for sem in refs.get("semantic", []):
            edges.append({
                "from": nid,
                "to": sem["node"],
                "type": "semantic",
                "similarity": sem["similarity"],
            })

    # Make bidirectional
    edges = make_bidirectional(edges)

    # Build node list for graph
    nodes = []
    for doc in all_docs:
        nodes.append({
            "id": doc["node_id"],
            "title": doc["title"],
            "scale": doc.get("scale", ""),
            "is_stale": doc.get("staleness", {}).get("is_stale", False),
        })

    return {"nodes": nodes, "edges": edges}


def make_bidirectional(edges):
    """Every A->B gets B->A. Deduplicate by (from, to, type).

    Args:
        edges: list of edge dicts

    Returns:
        list[dict]: deduplicated bidirectional edges
    """
    seen = set()
    result = []

    for e in edges:
        key = (e["from"], e["to"], e["type"])
        reverse_key = (e["to"], e["from"], e["type"])
        if key not in seen:
            seen.add(key)
            result.append(e)
        if reverse_key not in seen:
            seen.add(reverse_key)
            reverse = dict(e)
            reverse["from"] = e["to"]
            reverse["to"] = e["from"]
            if e.get("relation") == "parent":
                reverse["relation"] = "child"
            elif e.get("relation") == "child":
                reverse["relation"] = "parent"
            result.append(reverse)

    return result


# ── Staleness Tracking ───────────────────────────────────────────


def check_staleness(doc):
    """Recompute file hash for a doc, compare to stored hash.

    Args:
        doc: doc artifact dict

    Returns:
        bool: True if stale (files changed since doc was generated)
    """
    owned_files = doc.get("owned_files", [])
    current_hash = compute_file_hash(owned_files)
    stored_hash = doc.get("staleness", {}).get("source_hash", "")
    return current_hash != stored_hash


def check_all_stale():
    """Check all docs in MongoDB for staleness.

    Returns:
        list[dict]: [{node_id, title, is_stale, current_hash, stored_hash}]
    """
    docs = _load_all_docs()
    results = []
    for doc in docs:
        is_stale = check_staleness(doc)
        results.append({
            "node_id": doc["node_id"],
            "title": doc.get("title", ""),
            "is_stale": is_stale,
        })
    return results


def propagate_staleness(node_id, graph):
    """Flag connected docs when source changes.

    Follows all edges from the stale node and marks connected nodes
    as potentially stale (needs review).

    Args:
        node_id: the stale node
        graph: interconnection graph from full_interconnect()

    Returns:
        list[str]: node_ids flagged as potentially affected
    """
    affected = set()
    edges = graph.get("edges", [])

    # Direct connections
    for e in edges:
        if e["from"] == node_id:
            affected.add(e["to"])
        elif e["to"] == node_id:
            affected.add(e["from"])

    affected.discard(node_id)
    return sorted(affected)


# ── Refresh ──────────────────────────────────────────────────────


def refresh_doc(node_id, tree_path=None):
    """Regenerate one stale doc + re-embed + re-interconnect.

    Args:
        node_id: the node to refresh

    Returns:
        dict: refreshed doc artifact
    """
    tree = load_tree(tree_path)
    doc = generate_doc(node_id, tree)
    if "error" in doc:
        return doc

    # Add cross-refs
    all_docs = _load_all_docs()
    doc["cross_refs"] = build_cross_refs(node_id, tree, OWNERSHIP_MAP, all_docs)

    # Store
    store_doc(doc)

    # Embed
    embed_doc(doc)

    return doc


def refresh_all_stale(tree_path=None):
    """Regenerate all stale docs.

    Returns:
        dict: {refreshed: int, total: int, stale_nodes: [...]}
    """
    docs = _load_all_docs()
    tree = load_tree(tree_path)
    stale_nodes = []
    refreshed = 0

    for doc in docs:
        if check_staleness(doc):
            stale_nodes.append(doc["node_id"])
            new_doc = generate_doc(doc["node_id"], tree)
            if "error" not in new_doc:
                new_doc["cross_refs"] = build_cross_refs(
                    doc["node_id"], tree, OWNERSHIP_MAP, docs
                )
                store_doc(new_doc)
                embed_doc(new_doc)
                refreshed += 1

    return {
        "refreshed": refreshed,
        "total": len(docs),
        "stale_nodes": stale_nodes,
    }


# ── Storage ──────────────────────────────────────────────────────


def store_doc(doc):
    """Dual-store to MongoDB + IPFS (fire-and-forget).

    Args:
        doc: doc artifact dict
    """
    # MongoDB store
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        doc_json = json.dumps(doc, default=str)
        try:
            subprocess.Popen(
                ["node", store_js, "put", "docs", doc_json],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass

    # IPFS store (if available)
    try:
        from ptc.ipfs import dual_store
        dual_store(
            name=doc.get("node_id", "unknown"),
            artifact_type="doc",
            content=doc,
            project=doc.get("project", "claude-cage"),
        )
    except ImportError:
        pass

    # Local backup
    _store_local(doc)


def _store_local(doc):
    """Store doc as local JSON file."""
    docs_dir = os.path.join(CAGE_ROOT, "docs")
    os.makedirs(docs_dir, exist_ok=True)
    node_id = doc.get("node_id", "unknown").replace(":", "-")
    filepath = os.path.join(docs_dir, f"{node_id}.json")
    with open(filepath, "w") as f:
        json.dump(doc, f, indent=2, default=str)


# ── Embedding ────────────────────────────────────────────────────


def embed_doc(doc):
    """Background embedding via sentence-transformers.

    Args:
        doc: doc artifact dict
    """
    try:
        from ptc.embeddings import embed_and_store
        text = _doc_to_text(doc)
        embed_and_store("embeddings", doc["node_id"], text, {
            "source_type": "doc",
            "node_id": doc["node_id"],
            "title": doc.get("title", ""),
        })
    except ImportError:
        pass


# ── Search ───────────────────────────────────────────────────────


def search_docs(query, limit=10):
    """Semantic search across all docs.

    Uses $vectorSearch if available, falls back to text search.

    Args:
        query: search query text
        limit: max results

    Returns:
        list[dict]: matching docs with scores
    """
    # Try semantic search first
    try:
        from ptc.embeddings import semantic_search
        results = semantic_search("docs", query, limit)
        if results:
            return results
    except ImportError:
        pass

    # Fallback: text search via MongoDB
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        try:
            result = subprocess.run(
                ["node", store_js, "search", "docs", query, str(limit)],
                capture_output=True, text=True, timeout=15,
            )
            if result.returncode == 0 and result.stdout.strip():
                return json.loads(result.stdout)
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            pass

    # Final fallback: local file search
    return _search_local(query, limit)


def _search_local(query, limit=10):
    """Search local doc files by keyword matching."""
    docs_dir = os.path.join(CAGE_ROOT, "docs")
    if not os.path.isdir(docs_dir):
        return []

    query_lower = query.lower()
    results = []
    for f in sorted(os.listdir(docs_dir)):
        if not f.endswith(".json"):
            continue
        try:
            with open(os.path.join(docs_dir, f)) as fh:
                doc = json.load(fh)
            text = _doc_to_text(doc).lower()
            if query_lower in text:
                results.append({
                    "node_id": doc.get("node_id"),
                    "title": doc.get("title"),
                    "description": doc.get("description", "")[:200],
                    "score": text.count(query_lower) / max(len(text.split()), 1),
                })
        except (json.JSONDecodeError, KeyError):
            pass

    results.sort(key=lambda r: r.get("score", 0), reverse=True)
    return results[:limit]


# ── Getters ──────────────────────────────────────────────────────


def get_doc(node_id):
    """Fetch doc from MongoDB or local storage.

    Args:
        node_id: tree node ID

    Returns:
        dict or None
    """
    # Try MongoDB
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        try:
            query = json.dumps({"node_id": node_id})
            result = subprocess.run(
                ["node", store_js, "get", "docs", query, "1"],
                capture_output=True, text=True, timeout=10,
            )
            if result.returncode == 0 and result.stdout.strip():
                docs = json.loads(result.stdout)
                if isinstance(docs, list) and docs:
                    return docs[0]
                elif isinstance(docs, dict) and docs.get("node_id"):
                    return docs
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            pass

    # Fallback: local file
    docs_dir = os.path.join(CAGE_ROOT, "docs")
    filename = f"{node_id.replace(':', '-')}.json"
    filepath = os.path.join(docs_dir, filename)
    if os.path.exists(filepath):
        with open(filepath) as f:
            return json.load(f)

    return None


def get_graph():
    """Return full interconnection graph as JSON.

    Returns:
        dict: {nodes, edges}
    """
    tree = load_tree()
    all_docs = _load_all_docs()
    if not all_docs:
        return {"nodes": [], "edges": [], "message": "No docs generated yet"}
    return full_interconnect(tree, all_docs)


def get_status():
    """Coverage stats: total/documented/stale/fresh.

    Returns:
        dict: {total, documented, stale, fresh, undocumented}
    """
    tree = load_tree()
    total = len(tree)
    all_docs = _load_all_docs()
    documented = len(all_docs)
    documented_ids = {d["node_id"] for d in all_docs}

    stale = 0
    for doc in all_docs:
        if check_staleness(doc):
            stale += 1

    fresh = documented - stale
    undocumented = total - documented
    undocumented_nodes = sorted(set(tree.keys()) - documented_ids)

    return {
        "total": total,
        "documented": documented,
        "stale": stale,
        "fresh": fresh,
        "undocumented": undocumented,
        "undocumented_nodes": undocumented_nodes,
    }


# ── Internal Helpers ─────────────────────────────────────────────


def _load_all_docs():
    """Load all docs from MongoDB or local storage."""
    # Try MongoDB first
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if os.path.exists(store_js):
        try:
            result = subprocess.run(
                ["node", store_js, "get", "docs", "{}", "200"],
                capture_output=True, text=True, timeout=15,
            )
            if result.returncode == 0 and result.stdout.strip():
                docs = json.loads(result.stdout)
                if isinstance(docs, list) and docs:
                    return docs
                elif isinstance(docs, dict) and docs.get("node_id"):
                    return [docs]
        except (subprocess.TimeoutExpired, json.JSONDecodeError):
            pass

    # Fallback: local files
    return _load_local_docs()


def _load_local_docs():
    """Load all docs from local docs/ directory."""
    docs_dir = os.path.join(CAGE_ROOT, "docs")
    if not os.path.isdir(docs_dir):
        return []

    docs = []
    for f in sorted(os.listdir(docs_dir)):
        if f.endswith(".json"):
            try:
                with open(os.path.join(docs_dir, f)) as fh:
                    docs.append(json.load(fh))
            except (json.JSONDecodeError, KeyError):
                pass
    return docs


# ── CLI Entry Point ──────────────────────────────────────────────


def main():
    """CLI interface for documentation operations."""
    if len(sys.argv) < 2:
        print("Usage: python3 -m ptc.docs <command> [args]")
        print("Commands:")
        print("  generate <node_id>    Generate doc for one node")
        print("  generate-all          Generate docs for all nodes")
        print("  check-stale           Check all docs for staleness")
        print("  refresh [node_id]     Regenerate stale doc(s)")
        print("  interconnect          Compute full graph (the circle)")
        print("  search <query> [N]    Semantic search across docs")
        print("  show <node_id>        Display doc with cross-refs")
        print("  graph                 Output interconnection graph JSON")
        print("  status                Coverage + staleness stats")
        sys.exit(1)

    command = sys.argv[1]

    if command == "generate":
        if len(sys.argv) < 3:
            print("Usage: python3 -m ptc.docs generate <node_id>")
            sys.exit(1)
        node_id = sys.argv[2]
        tree = load_tree()
        doc = generate_doc(node_id, tree)
        if "error" in doc:
            print(f"Error: {doc['error']}")
            sys.exit(1)
        # Compute cross-refs
        all_docs = _load_all_docs()
        doc["cross_refs"] = build_cross_refs(node_id, tree, OWNERSHIP_MAP, all_docs)
        store_doc(doc)
        embed_doc(doc)
        print(f"Generated: {doc['node_id']}")
        print(f"  Title:    {doc['title']}")
        print(f"  Files:    {len(doc['owned_files'])}")
        print(f"  Concepts: {', '.join(doc['key_concepts'][:5])}")
        print(f"  Hash:     {doc['content_hash'][:20]}")
        refs = doc.get("cross_refs", {})
        struct = refs.get("structural", {})
        print(f"  Parent:   {struct.get('parent', 'none')}")
        print(f"  Children: {len(struct.get('children', []))}")
        print(f"  Siblings: {len(struct.get('siblings', []))}")
        print(f"  Code-shared: {len(refs.get('code_shared', []))}")
        print(f"  Semantic: {len(refs.get('semantic', []))}")

    elif command == "generate-all":
        tree = load_tree()
        all_docs = generate_all()
        # Compute cross-refs for all
        for doc in all_docs:
            doc["cross_refs"] = build_cross_refs(
                doc["node_id"], tree, OWNERSHIP_MAP, all_docs
            )
            store_doc(doc)
            embed_doc(doc)
        print(f"Generated {len(all_docs)} docs")
        for doc in all_docs:
            refs = doc.get("cross_refs", {})
            code_shared = len(refs.get("code_shared", []))
            semantic = len(refs.get("semantic", []))
            print(f"  {doc['node_id']:25s} files={len(doc['owned_files'])} code_shared={code_shared} semantic={semantic}")

    elif command == "check-stale":
        results = check_all_stale()
        if not results:
            print("No docs found. Run: python3 -m ptc.docs generate-all")
            sys.exit(0)
        stale_count = sum(1 for r in results if r["is_stale"])
        fresh_count = sum(1 for r in results if not r["is_stale"])
        print(f"Staleness check: {stale_count} stale, {fresh_count} fresh")
        for r in results:
            icon = "STALE" if r["is_stale"] else "fresh"
            print(f"  [{icon:5s}] {r['node_id']:25s} {r.get('title', '')}")

    elif command == "refresh":
        if len(sys.argv) >= 3:
            node_id = sys.argv[2]
            doc = refresh_doc(node_id)
            if "error" in doc:
                print(f"Error: {doc['error']}")
                sys.exit(1)
            print(f"Refreshed: {doc['node_id']}")
        else:
            result = refresh_all_stale()
            print(f"Refreshed: {result['refreshed']}/{result['total']}")
            if result["stale_nodes"]:
                for nid in result["stale_nodes"]:
                    print(f"  refreshed: {nid}")

    elif command == "interconnect":
        tree = load_tree()
        all_docs = _load_all_docs()
        if not all_docs:
            print("No docs found. Run: python3 -m ptc.docs generate-all")
            sys.exit(1)
        graph = full_interconnect(tree, all_docs)
        # Store each doc with updated cross-refs
        docs_by_id = {d["node_id"]: d for d in all_docs}
        for doc in all_docs:
            store_doc(doc)
        print(f"Interconnection complete: {len(graph['nodes'])} nodes, {len(graph['edges'])} edges")
        # Edge type breakdown
        by_type = {}
        for e in graph["edges"]:
            t = e.get("type", "unknown")
            by_type[t] = by_type.get(t, 0) + 1
        for t, count in sorted(by_type.items()):
            print(f"  {t}: {count} edges")

    elif command == "search":
        if len(sys.argv) < 3:
            print("Usage: python3 -m ptc.docs search <query> [limit]")
            sys.exit(1)
        query = sys.argv[2]
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 10
        results = search_docs(query, limit)
        if not results:
            print("No results")
        else:
            for r in results:
                score = r.get("score", "?")
                if isinstance(score, float):
                    score = f"{score:.3f}"
                print(f"  [{score}] {r.get('node_id', r.get('title', '?'))}: {r.get('description', '')[:80]}")

    elif command == "show":
        if len(sys.argv) < 3:
            print("Usage: python3 -m ptc.docs show <node_id>")
            sys.exit(1)
        doc = get_doc(sys.argv[2])
        if not doc:
            print(f"Doc not found: {sys.argv[2]}")
            sys.exit(1)
        print(json.dumps(doc, indent=2, default=str))

    elif command == "graph":
        graph = get_graph()
        print(json.dumps(graph, indent=2, default=str))

    elif command == "status":
        status = get_status()
        print(f"Documentation Circle Status")
        print(f"  Total nodes:    {status['total']}")
        print(f"  Documented:     {status['documented']}")
        print(f"  Fresh:          {status['fresh']}")
        print(f"  Stale:          {status['stale']}")
        print(f"  Undocumented:   {status['undocumented']}")
        if status["undocumented_nodes"]:
            print(f"  Missing docs:   {', '.join(status['undocumented_nodes'][:10])}")

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
