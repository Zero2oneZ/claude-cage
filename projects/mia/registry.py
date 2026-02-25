"""registry.py — MongoDB CID registry for mia vaults.

All operations go through `node mongodb/store.js` (same pattern as lib/mongodb.sh).
Tracks IPFS CIDs, project lineage (parent_cid chains), and pin metadata.
"""

import json
import os
import subprocess
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
STORE_JS = os.path.join(CAGE_ROOT, "mongodb", "store.js")
COLLECTION = "mia_registry"


def _node_store(*args, timeout=15):
    """Call node store.js with arguments.

    Returns:
        str: stdout on success, None on failure
    """
    if not os.path.exists(STORE_JS):
        return None
    try:
        result = subprocess.run(
            ["node", STORE_JS] + list(args),
            capture_output=True, text=True, timeout=timeout,
            cwd=CAGE_ROOT,
        )
        if result.returncode == 0:
            return result.stdout.strip()
        return None
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None


def register_pin(cid, scope, project=None, os_target=None, keys=None, parent_cid=None):
    """Register an IPFS pin in the mia_registry collection.

    Args:
        cid: IPFS CID (from Pinata)
        scope: "global" or project name
        project: project name (for project vaults)
        os_target: OS tag (linux, darwin, etc.)
        keys: list of key names in this vault
        parent_cid: CID of the global vault this was spawned from

    Returns:
        bool: True if registered successfully
    """
    doc = {
        "cid": cid,
        "scope": scope,
        "project": project,
        "os": os_target,
        "keys": keys or [],
        "parent_cid": parent_cid,
        "pinned_at": datetime.now(timezone.utc).isoformat(),
        "_ts": datetime.now(timezone.utc).isoformat(),
    }

    result = _node_store("put", COLLECTION, json.dumps(doc))
    if result is not None:
        # Also log as event
        _node_store(
            "log", "mia:pin", f"{scope}:{cid[:16]}",
            json.dumps({"cid": cid, "scope": scope, "project": project}),
        )
        return True
    return False


def list_all():
    """List all registered CIDs.

    Returns:
        list[dict]: registry entries sorted by timestamp
    """
    result = _node_store("get", COLLECTION, "{}", "100")
    if not result:
        return []
    try:
        docs = json.loads(result)
        if isinstance(docs, dict):
            docs = [docs]
        return docs
    except json.JSONDecodeError:
        return []


def get_by_scope(scope):
    """Get the latest CID for a scope (global or project name).

    Returns:
        dict or None
    """
    query = json.dumps({"scope": scope})
    result = _node_store("get", COLLECTION, query, "1")
    if not result:
        return None
    try:
        docs = json.loads(result)
        if isinstance(docs, list):
            return docs[0] if docs else None
        return docs
    except json.JSONDecodeError:
        return None


def get_chain(project):
    """Get the CID chain for a project (project → parent global).

    Returns:
        list[dict]: chain from project vault back to global vault
    """
    chain = []

    # Get project entry
    entry = get_by_scope(project)
    if entry:
        chain.append(entry)
        # Follow parent_cid to global
        parent_cid = entry.get("parent_cid")
        if parent_cid:
            query = json.dumps({"cid": parent_cid})
            result = _node_store("get", COLLECTION, query, "1")
            if result:
                try:
                    parent = json.loads(result)
                    if isinstance(parent, list):
                        parent = parent[0] if parent else None
                    if parent:
                        chain.append(parent)
                except json.JSONDecodeError:
                    pass

    return chain


def is_available():
    """Check if MongoDB store is reachable.

    Returns:
        bool
    """
    result = _node_store("ping")
    return result is not None and "pong" in (result or "").lower()


def format_table(entries):
    """Format registry entries as a readable table.

    Args:
        entries: list of registry dicts

    Returns:
        str: formatted table
    """
    if not entries:
        return "  (empty)"

    lines = []
    lines.append(f"  {'SCOPE':<14} {'CID':<52} {'KEYS':<6} {'PINNED':<20} {'PARENT'}")
    lines.append(f"  {'─' * 14} {'─' * 52} {'─' * 6} {'─' * 20} {'─' * 16}")

    for e in entries:
        cid = e.get("cid", "?")
        scope = e.get("scope", "?")
        keys = str(len(e.get("keys", [])))
        pinned = e.get("pinned_at", "?")[:19]
        parent = (e.get("parent_cid") or "—")[:16]
        lines.append(f"  {scope:<14} {cid:<52} {keys:<6} {pinned:<20} {parent}")

    return "\n".join(lines)
