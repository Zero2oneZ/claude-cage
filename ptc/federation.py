"""ptc/federation.py — Git sovereignty + bidirectional forking.

The critical piece. Bidirectional git forking where the tree doesn't break
and sovereignty doesn't lose trust. Fork decides. Always.

Core concepts:
  - Upstream = parent repo (claude-cage core)
  - Fork = independent repo (user's project)
  - Manifest = federation.json declaring the relationship
  - Tree integrity = every fork has its own tree.json referencing upstream nodes
  - Sovereignty = the fork controls what it accepts

Config: No env vars required. Uses git + gh CLI.
"""

import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ── Helpers ────────────────────────────────────────────────────


def _git(*args, cwd=None):
    """Run a git command, return (success, stdout, stderr)."""
    cwd = cwd or CAGE_ROOT
    try:
        result = subprocess.run(
            ["git"] + list(args),
            capture_output=True, text=True, timeout=30, cwd=cwd,
        )
        return result.returncode == 0, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return False, "", "timeout"
    except FileNotFoundError:
        return False, "", "git not found"


def _mongo_log(event_type, key, value=None):
    """Fire-and-forget MongoDB event log."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return
    doc = json.dumps({
        "type": event_type, "key": key, "value": value,
        "_ts": datetime.now(timezone.utc).isoformat(), "_source": "federation",
    })
    try:
        subprocess.Popen(
            ["node", store_js, "log", event_type, key, doc],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


# ── Manifest ───────────────────────────────────────────────────


def _load_manifest(directory):
    """Load federation.json from directory.

    Args:
        directory: project directory containing federation.json

    Returns:
        dict or None
    """
    path = os.path.join(directory, "federation.json")
    if not os.path.exists(path):
        return None
    with open(path) as f:
        return json.load(f)


def _save_manifest(directory, manifest):
    """Write federation.json to directory."""
    path = os.path.join(directory, "federation.json")
    with open(path, "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")


def _default_manifest(upstream_url, fork_url, upstream_branch="main", fork_branch="main"):
    """Create a default federation manifest."""
    now = datetime.now(timezone.utc).isoformat()
    return {
        "upstream": {
            "url": upstream_url,
            "branch": upstream_branch,
            "tree": "tree.json",
            "last_sync": now,
            "sync_sha": None,
        },
        "fork": {
            "url": fork_url,
            "branch": fork_branch,
            "tree": "tree.json",
            "created": now,
        },
        "sync_policy": {
            "auto_pull": False,
            "auto_push": False,
            "pull_nodes": [],
            "push_nodes": [],
            "reject_nodes": [],
            "conflict_resolution": "fork_wins",
        },
        "trust": {
            "upstream_verified": False,
            "upstream_tree_hash": None,
            "fork_tree_hash": None,
            "last_verified": None,
        },
    }


# ── Tree Hashing & Integrity ──────────────────────────────────


def _tree_hash(tree_path):
    """SHA-256 hash of tree.json content.

    Args:
        tree_path: path to tree.json

    Returns:
        str: "sha256:<hex>" or None
    """
    if not os.path.exists(tree_path):
        return None
    with open(tree_path, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
    return f"sha256:{digest}"


def _load_tree(tree_path):
    """Load tree.json, return parsed dict or None."""
    if not os.path.exists(tree_path):
        return None
    with open(tree_path) as f:
        return json.load(f)


def _verify_tree_integrity(local_tree, upstream_tree):
    """Check node consistency between local and upstream trees.

    Verifies:
    - No broken parent/child references
    - All referenced parents exist
    - Children arrays are consistent

    Args:
        local_tree: parsed local tree.json
        upstream_tree: parsed upstream tree.json

    Returns:
        dict: {valid, issues}
    """
    issues = []

    for tree, label in [(local_tree, "local"), (upstream_tree, "upstream")]:
        if tree is None:
            continue
        nodes = tree.get("nodes", [])
        node_ids = {n["id"] for n in nodes}

        for node in nodes:
            # Check parent exists (if not root)
            parent = node.get("parent")
            if parent and parent not in node_ids:
                issues.append(f"{label}: node {node['id']} references missing parent {parent}")

            # Check children exist
            for child_id in node.get("children", []):
                if child_id not in node_ids:
                    issues.append(f"{label}: node {node['id']} references missing child {child_id}")

    return {"valid": len(issues) == 0, "issues": issues}


# ── Tree Diff/Merge ────────────────────────────────────────────


def diff_trees(tree_a, tree_b):
    """Structural diff between two trees.

    Args:
        tree_a: first tree dict (or path)
        tree_b: second tree dict (or path)

    Returns:
        dict: {added, removed, modified, unchanged}
    """
    if isinstance(tree_a, str):
        tree_a = _load_tree(tree_a)
    if isinstance(tree_b, str):
        tree_b = _load_tree(tree_b)

    if tree_a is None or tree_b is None:
        return {"error": "Could not load one or both trees"}

    nodes_a = {n["id"]: n for n in tree_a.get("nodes", [])}
    nodes_b = {n["id"]: n for n in tree_b.get("nodes", [])}

    ids_a = set(nodes_a.keys())
    ids_b = set(nodes_b.keys())

    added = sorted(ids_b - ids_a)
    removed = sorted(ids_a - ids_b)
    common = ids_a & ids_b

    modified = []
    unchanged = []
    for nid in sorted(common):
        a_json = json.dumps(nodes_a[nid], sort_keys=True)
        b_json = json.dumps(nodes_b[nid], sort_keys=True)
        if a_json != b_json:
            modified.append(nid)
        else:
            unchanged.append(nid)

    return {
        "added": added,
        "removed": removed,
        "modified": modified,
        "unchanged": unchanged,
        "summary": f"+{len(added)} -{len(removed)} ~{len(modified)} ={len(unchanged)}",
    }


def merge_trees(local_tree, upstream_tree, policy):
    """Merge upstream changes into local tree, respecting sovereignty.

    Args:
        local_tree: parsed local tree dict
        upstream_tree: parsed upstream tree dict
        policy: sync_policy from federation.json

    Returns:
        dict: {merged_tree, changes_applied, changes_rejected}
    """
    pull_nodes = set(policy.get("pull_nodes", []))
    reject_nodes = set(policy.get("reject_nodes", []))
    resolution = policy.get("conflict_resolution", "fork_wins")

    diff = diff_trees(local_tree, upstream_tree)
    if "error" in diff:
        return {"error": diff["error"]}

    local_nodes = {n["id"]: n for n in local_tree.get("nodes", [])}
    upstream_nodes = {n["id"]: n for n in upstream_tree.get("nodes", [])}

    applied = []
    rejected = []

    # Process added nodes from upstream
    for nid in diff["added"]:
        if nid in reject_nodes:
            rejected.append({"node": nid, "action": "add", "reason": "reject_policy"})
            continue
        if pull_nodes and nid not in pull_nodes:
            # If pull_nodes is specified, only pull listed nodes
            rejected.append({"node": nid, "action": "add", "reason": "not_in_pull_nodes"})
            continue
        local_nodes[nid] = upstream_nodes[nid]
        applied.append({"node": nid, "action": "add"})

    # Process modified nodes
    for nid in diff["modified"]:
        if nid in reject_nodes:
            rejected.append({"node": nid, "action": "modify", "reason": "reject_policy"})
            continue
        if pull_nodes and nid not in pull_nodes:
            rejected.append({"node": nid, "action": "modify", "reason": "not_in_pull_nodes"})
            continue
        if resolution == "fork_wins":
            rejected.append({"node": nid, "action": "modify", "reason": "fork_wins"})
        elif resolution == "upstream_wins":
            local_nodes[nid] = upstream_nodes[nid]
            applied.append({"node": nid, "action": "modify"})
        else:
            # manual — skip, flag for review
            rejected.append({"node": nid, "action": "modify", "reason": "manual_review"})

    # Build merged tree
    merged = dict(local_tree)
    merged["nodes"] = list(local_nodes.values())

    return {
        "merged_tree": merged,
        "changes_applied": applied,
        "changes_rejected": rejected,
    }


def resolve_conflicts(local_tree, upstream_tree, resolution):
    """Resolve conflicts between trees.

    Args:
        local_tree: parsed local tree
        upstream_tree: parsed upstream tree
        resolution: "fork_wins" | "upstream_wins" | "manual"

    Returns:
        dict: resolved tree
    """
    policy = {"conflict_resolution": resolution, "pull_nodes": [], "reject_nodes": []}
    return merge_trees(local_tree, upstream_tree, policy)


# ── Fork Operations ───────────────────────────────────────────


def init_fork(directory, upstream_url, name=None):
    """Create a fork: clone upstream, create independent tree, push to new repo.

    Args:
        directory: local directory for the fork
        upstream_url: git URL of upstream repo
        name: project name (defaults to directory basename)

    Returns:
        dict: {directory, manifest, tree_path} or {error}
    """
    name = name or os.path.basename(os.path.abspath(directory))

    # Clone upstream
    if not os.path.exists(directory):
        ok, out, err = _git("clone", upstream_url, directory, cwd=os.path.dirname(os.path.abspath(directory)) or ".")
        if not ok:
            return {"error": f"Clone failed: {err}"}
    elif not os.path.exists(os.path.join(directory, ".git")):
        return {"error": f"{directory} exists but is not a git repo"}

    # Get upstream SHA
    ok, sha, _ = _git("rev-parse", "HEAD", cwd=directory)

    # Create federation.json
    fork_url = ""  # Will be filled after repo creation
    manifest = _default_manifest(upstream_url, fork_url)
    manifest["upstream"]["sync_sha"] = sha if ok else None

    # Hash upstream tree
    tree_path = os.path.join(directory, "tree.json")
    if os.path.exists(tree_path):
        manifest["trust"]["upstream_tree_hash"] = _tree_hash(tree_path)
        manifest["trust"]["upstream_verified"] = True
        manifest["trust"]["last_verified"] = datetime.now(timezone.utc).isoformat()

    _save_manifest(directory, manifest)

    # Commit federation.json
    _git("add", "federation.json", cwd=directory)
    _git("commit", "-m", f"federation: init fork from {upstream_url}", cwd=directory)

    # Hash fork tree
    manifest["trust"]["fork_tree_hash"] = _tree_hash(tree_path)
    _save_manifest(directory, manifest)

    _mongo_log("federation:fork-init", name, upstream_url)

    return {
        "directory": os.path.abspath(directory),
        "manifest": manifest,
        "tree_path": tree_path,
        "name": name,
    }


def init_branch(directory, upstream_url, branch):
    """Branch mode: shared repo, separate branch.

    Args:
        directory: project directory
        upstream_url: git URL
        branch: branch name to create

    Returns:
        dict: {branch, manifest} or {error}
    """
    ok, _, err = _git("checkout", "-b", branch, cwd=directory)
    if not ok:
        return {"error": f"Branch creation failed: {err}"}

    fork_url = upstream_url  # Same repo
    manifest = _default_manifest(upstream_url, fork_url, fork_branch=branch)
    _save_manifest(directory, manifest)

    _git("add", "federation.json", cwd=directory)
    _git("commit", "-m", f"federation: init branch {branch}", cwd=directory)

    _mongo_log("federation:branch-init", branch, upstream_url)
    return {"branch": branch, "manifest": manifest}


# ── Sync Operations ───────────────────────────────────────────


def sync_pull(directory, nodes=None):
    """Pull from upstream, filtered by policy.

    Protocol:
    1. git fetch upstream main
    2. Load upstream tree.json from fetched ref
    3. diff_trees() -> changed nodes
    4. Filter by sync_policy.pull_nodes
    5. Merge accepted node changes + update local tree.json
    6. Reject reject_nodes silently
    7. verify_tree_integrity() — no broken parent/child refs
    8. Update trust hashes + last_sync
    9. Commit
    10. Log to MongoDB

    Args:
        directory: project directory with federation.json
        nodes: optional list of specific nodes to pull

    Returns:
        dict: {pulled, rejected, integrity} or {error}
    """
    manifest = _load_manifest(directory)
    if not manifest:
        return {"error": "No federation.json found"}

    upstream_url = manifest["upstream"]["url"]
    upstream_branch = manifest["upstream"]["branch"]

    # Ensure upstream remote exists
    ok, remotes, _ = _git("remote", cwd=directory)
    if "upstream" not in remotes:
        _git("remote", "add", "upstream", upstream_url, cwd=directory)

    # Fetch upstream
    ok, _, err = _git("fetch", "upstream", upstream_branch, cwd=directory)
    if not ok:
        return {"error": f"Fetch failed: {err}"}

    # Get upstream tree from fetched ref
    ok, upstream_tree_content, _ = _git(
        "show", f"upstream/{upstream_branch}:tree.json", cwd=directory
    )
    if not ok:
        return {"error": "Could not read upstream tree.json"}

    try:
        upstream_tree = json.loads(upstream_tree_content)
    except json.JSONDecodeError:
        return {"error": "Invalid upstream tree.json"}

    # Load local tree
    local_tree_path = os.path.join(directory, manifest["fork"].get("tree", "tree.json"))
    local_tree = _load_tree(local_tree_path)
    if not local_tree:
        return {"error": "Local tree.json not found"}

    # Override pull_nodes if specified
    policy = dict(manifest["sync_policy"])
    if nodes:
        policy["pull_nodes"] = nodes

    # Merge
    merge_result = merge_trees(local_tree, upstream_tree, policy)
    if "error" in merge_result:
        return merge_result

    # Verify integrity
    integrity = _verify_tree_integrity(merge_result["merged_tree"], None)
    if not integrity["valid"]:
        return {
            "error": "Tree integrity check failed after merge",
            "issues": integrity["issues"],
            "changes_applied": merge_result["changes_applied"],
        }

    # Write merged tree
    with open(local_tree_path, "w") as f:
        json.dump(merge_result["merged_tree"], f, indent=2)
        f.write("\n")

    # Update manifest
    ok, sha, _ = _git("rev-parse", f"upstream/{upstream_branch}", cwd=directory)
    manifest["upstream"]["sync_sha"] = sha if ok else None
    manifest["upstream"]["last_sync"] = datetime.now(timezone.utc).isoformat()
    manifest["trust"]["upstream_tree_hash"] = _tree_hash(local_tree_path)
    manifest["trust"]["fork_tree_hash"] = _tree_hash(local_tree_path)
    manifest["trust"]["last_verified"] = datetime.now(timezone.utc).isoformat()
    _save_manifest(directory, manifest)

    # Commit
    _git("add", "tree.json", "federation.json", cwd=directory)
    sync_sha_short = (sha[:8] if sha else "unknown") if ok else "unknown"
    _git("commit", "-m", f"federation: sync from upstream [{sync_sha_short}]", cwd=directory)

    _mongo_log("federation:pull", directory, json.dumps({
        "applied": len(merge_result["changes_applied"]),
        "rejected": len(merge_result["changes_rejected"]),
    }))

    return {
        "pulled": merge_result["changes_applied"],
        "rejected": merge_result["changes_rejected"],
        "integrity": integrity,
        "sync_sha": sha if ok else None,
    }


def sync_push(directory, nodes=None):
    """Push local changes to upstream as a PR (via gh CLI).

    Args:
        directory: project directory
        nodes: optional list of specific nodes to push

    Returns:
        dict: {branch, pr_url} or {error}
    """
    manifest = _load_manifest(directory)
    if not manifest:
        return {"error": "No federation.json found"}

    push_nodes = nodes or manifest["sync_policy"].get("push_nodes", [])
    if not push_nodes:
        return {"error": "No push_nodes configured in sync_policy"}

    # Create a PR branch
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    pr_branch = f"federation/push-{timestamp}"

    ok, _, err = _git("checkout", "-b", pr_branch, cwd=directory)
    if not ok:
        return {"error": f"Branch creation failed: {err}"}

    _git("add", "tree.json", cwd=directory)
    _git("commit", "-m", f"federation: push nodes {', '.join(push_nodes)}", cwd=directory)

    # Push branch
    ok, _, err = _git("push", "upstream", pr_branch, cwd=directory)
    if not ok:
        return {"error": f"Push failed: {err}"}

    _mongo_log("federation:push", directory, json.dumps({"nodes": push_nodes, "branch": pr_branch}))
    return {"branch": pr_branch, "nodes": push_nodes}


def sync_status(directory):
    """Show ahead/behind/diverged status relative to upstream.

    Args:
        directory: project directory

    Returns:
        dict: {ahead, behind, diverged, last_sync, sync_sha}
    """
    manifest = _load_manifest(directory)
    if not manifest:
        return {"error": "No federation.json found"}

    upstream_branch = manifest["upstream"]["branch"]

    # Fetch to get latest
    _git("fetch", "upstream", upstream_branch, cwd=directory)

    # Count ahead/behind
    ok, out, _ = _git(
        "rev-list", "--left-right", "--count",
        f"HEAD...upstream/{upstream_branch}", cwd=directory
    )

    ahead, behind = 0, 0
    if ok and out:
        parts = out.split()
        if len(parts) >= 2:
            ahead = int(parts[0])
            behind = int(parts[1])

    # Tree diff
    ok, upstream_tree_content, _ = _git(
        "show", f"upstream/{upstream_branch}:tree.json", cwd=directory
    )
    tree_diff = None
    if ok:
        try:
            upstream_tree = json.loads(upstream_tree_content)
            local_tree = _load_tree(os.path.join(directory, "tree.json"))
            if local_tree:
                tree_diff = diff_trees(local_tree, upstream_tree)
        except json.JSONDecodeError:
            pass

    return {
        "ahead": ahead,
        "behind": behind,
        "diverged": ahead > 0 and behind > 0,
        "last_sync": manifest["upstream"].get("last_sync"),
        "sync_sha": manifest["upstream"].get("sync_sha"),
        "tree_diff": tree_diff,
    }


# ── Trust Verification ────────────────────────────────────────


def verify_trust(directory):
    """Re-verify tree hashes and trust chain.

    Args:
        directory: project directory

    Returns:
        dict: {verified, upstream_hash, fork_hash, match}
    """
    manifest = _load_manifest(directory)
    if not manifest:
        return {"error": "No federation.json found"}

    local_tree_path = os.path.join(directory, manifest["fork"].get("tree", "tree.json"))
    current_hash = _tree_hash(local_tree_path)
    stored_hash = manifest["trust"].get("fork_tree_hash")

    # Fetch and check upstream
    upstream_branch = manifest["upstream"]["branch"]
    _git("fetch", "upstream", upstream_branch, cwd=directory)
    ok, upstream_content, _ = _git(
        "show", f"upstream/{upstream_branch}:tree.json", cwd=directory
    )
    upstream_hash = None
    if ok:
        upstream_hash = f"sha256:{hashlib.sha256(upstream_content.encode('utf-8')).hexdigest()}"

    stored_upstream = manifest["trust"].get("upstream_tree_hash")
    upstream_match = upstream_hash == stored_upstream if upstream_hash and stored_upstream else None

    # Update manifest
    manifest["trust"]["fork_tree_hash"] = current_hash
    if upstream_hash:
        manifest["trust"]["upstream_tree_hash"] = upstream_hash
    manifest["trust"]["last_verified"] = datetime.now(timezone.utc).isoformat()
    manifest["trust"]["upstream_verified"] = upstream_match is not False
    _save_manifest(directory, manifest)

    _mongo_log("federation:verify", directory)

    return {
        "verified": upstream_match is not False and current_hash == stored_hash,
        "fork_hash": current_hash,
        "fork_hash_match": current_hash == stored_hash,
        "upstream_hash": upstream_hash,
        "upstream_hash_match": upstream_match,
        "last_verified": manifest["trust"]["last_verified"],
    }


# ── Repository Management ─────────────────────────────────────


def list_forks(directory=None):
    """List known forks from MongoDB.

    Returns:
        list: fork records
    """
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return []
    try:
        result = subprocess.run(
            ["node", store_js, "get", "events", '{"type":"federation:fork-init"}', "50"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode == 0 and result.stdout.strip():
            return json.loads(result.stdout)
    except Exception:
        pass
    return []


def create_github_repo(name, private=True):
    """Create a GitHub repo via gh CLI.

    Args:
        name: repo name
        private: whether repo is private

    Returns:
        dict: {url, name} or {error}
    """
    visibility = "--private" if private else "--public"
    try:
        result = subprocess.run(
            ["gh", "repo", "create", name, visibility, "--confirm"],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode == 0:
            url = result.stdout.strip()
            return {"url": url, "name": name}
        return {"error": result.stderr.strip()}
    except FileNotFoundError:
        return {"error": "gh CLI not found"}
    except subprocess.TimeoutExpired:
        return {"error": "Timeout creating repo"}


def setup_remotes(directory, upstream_url, fork_url):
    """Configure git remotes.

    Args:
        directory: project directory
        upstream_url: upstream repo URL
        fork_url: fork repo URL
    """
    _git("remote", "set-url", "origin", fork_url, cwd=directory)
    ok, remotes, _ = _git("remote", cwd=directory)
    if "upstream" not in (remotes or ""):
        _git("remote", "add", "upstream", upstream_url, cwd=directory)
    else:
        _git("remote", "set-url", "upstream", upstream_url, cwd=directory)


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for federation operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.federation <command> [args]")
        print("Commands: fork <upstream-url> <dir> [--name n],")
        print("          branch <dir> <upstream-url> <branch>,")
        print("          pull <dir> [--nodes n1,n2],")
        print("          push <dir> [--nodes n1,n2],")
        print("          status <dir>,")
        print("          verify <dir>,")
        print("          diff <tree-a> <tree-b>,")
        print("          forks")
        sys.exit(1)

    command = sys.argv[1]

    if command == "fork":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.federation fork <upstream-url> <dir> [--name n]", file=sys.stderr)
            sys.exit(1)
        upstream_url = sys.argv[2]
        directory = sys.argv[3]
        name = None
        for i, arg in enumerate(sys.argv[4:], 4):
            if arg == "--name" and i + 1 < len(sys.argv):
                name = sys.argv[i + 1]
        result = init_fork(directory, upstream_url, name)
        print(json.dumps(result, indent=2, default=str))

    elif command == "branch":
        if len(sys.argv) < 5:
            print("Usage: python -m ptc.federation branch <dir> <upstream-url> <branch>", file=sys.stderr)
            sys.exit(1)
        result = init_branch(sys.argv[2], sys.argv[3], sys.argv[4])
        print(json.dumps(result, indent=2, default=str))

    elif command == "pull":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.federation pull <dir> [--nodes n1,n2]", file=sys.stderr)
            sys.exit(1)
        directory = sys.argv[2]
        nodes = None
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--nodes" and i + 1 < len(sys.argv):
                nodes = sys.argv[i + 1].split(",")
        result = sync_pull(directory, nodes)
        print(json.dumps(result, indent=2, default=str))

    elif command == "push":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.federation push <dir> [--nodes n1,n2]", file=sys.stderr)
            sys.exit(1)
        directory = sys.argv[2]
        nodes = None
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--nodes" and i + 1 < len(sys.argv):
                nodes = sys.argv[i + 1].split(",")
        result = sync_push(directory, nodes)
        print(json.dumps(result, indent=2, default=str))

    elif command == "status":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.federation status <dir>", file=sys.stderr)
            sys.exit(1)
        result = sync_status(sys.argv[2])
        print(json.dumps(result, indent=2, default=str))

    elif command == "verify":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.federation verify <dir>", file=sys.stderr)
            sys.exit(1)
        result = verify_trust(sys.argv[2])
        print(json.dumps(result, indent=2, default=str))

    elif command == "diff":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.federation diff <tree-a> <tree-b>", file=sys.stderr)
            sys.exit(1)
        result = diff_trees(sys.argv[2], sys.argv[3])
        print(json.dumps(result, indent=2))

    elif command == "forks":
        result = list_forks()
        print(json.dumps(result, indent=2, default=str))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
