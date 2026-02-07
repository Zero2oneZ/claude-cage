"""ptc/git_ops.py — Git process pipeline.

Git branches are organic separations of concern FOR THE USER (human-readable).
The databases (MongoDB + Vector DB) are the REAL navigation — git is the friendly layer.

Branch conventions:
  design/<blueprint-id>  — architectural designs
  build/<blueprint-id>/<task-id> — builder execution
  verify/<blueprint-id>  — verification results
  main                   — merged verified work

Every commit gets embedded in the vector DB for semantic navigation.
"""

import json
import os
import subprocess
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


def _current_branch():
    """Get current git branch name."""
    ok, out, _ = _git("rev-parse", "--abbrev-ref", "HEAD")
    return out if ok else None


def _branch_exists(branch_name):
    """Check if a branch exists locally."""
    ok, _, _ = _git("rev-parse", "--verify", branch_name)
    return ok


def _stash_if_dirty():
    """Stash uncommitted changes if working tree is dirty. Returns True if stashed."""
    ok, out, _ = _git("status", "--porcelain")
    if ok and out:
        _git("stash", "push", "-m", "claude-cage: auto-stash for branch switch")
        return True
    return False


def _unstash():
    """Pop stash if there is one."""
    _git("stash", "pop")


# ── Branch Management ─────────────────────────────────────────


def git_branch_for_blueprint(blueprint_id):
    """Create or checkout a branch for a blueprint.

    Convention: design/<blueprint-id>
    Safe: stashes dirty work, creates branch if needed, returns to original branch info.

    Args:
        blueprint_id: e.g., "blueprint:ipfs-storage"

    Returns:
        dict: {branch, created, previous_branch}
    """
    # Sanitize blueprint ID for branch name
    branch_name = f"design/{blueprint_id.replace(':', '-').replace(' ', '-')}"
    previous = _current_branch()
    created = False

    if not _branch_exists(branch_name):
        ok, _, err = _git("checkout", "-b", branch_name)
        if not ok:
            return {"error": f"Failed to create branch: {err}", "branch": None}
        created = True
    else:
        if _current_branch() != branch_name:
            stashed = _stash_if_dirty()
            ok, _, err = _git("checkout", branch_name)
            if not ok:
                if stashed:
                    _unstash()
                return {"error": f"Failed to checkout branch: {err}", "branch": None}

    return {
        "branch": branch_name,
        "created": created,
        "previous_branch": previous,
    }


def git_build_branch(blueprint_id, task_id):
    """Create a build branch for a specific task within a blueprint.

    Convention: build/<blueprint-id>/<task-id>

    Args:
        blueprint_id: parent blueprint
        task_id: the specific builder task

    Returns:
        dict: {branch, created}
    """
    bp_slug = blueprint_id.replace(":", "-").replace(" ", "-")
    task_slug = task_id.replace(":", "-").replace(" ", "-")
    branch_name = f"build/{bp_slug}/{task_slug}"

    if not _branch_exists(branch_name):
        # Create from the design branch if it exists
        design_branch = f"design/{bp_slug}"
        base = design_branch if _branch_exists(design_branch) else "main"
        ok, _, err = _git("checkout", "-b", branch_name, base)
        if not ok:
            return {"error": f"Failed to create build branch: {err}"}
        return {"branch": branch_name, "created": True}

    ok, _, _ = _git("checkout", branch_name)
    return {"branch": branch_name, "created": False}


# ── Commit Operations ─────────────────────────────────────────


def git_commit_artifact(artifact, message=None):
    """Commit an artifact change with auto-generated message.

    Args:
        artifact: dict with {name, type, files, hash}
        message: custom commit message (auto-generated if None)

    Returns:
        dict: {sha, message, branch} or {error}
    """
    name = artifact.get("name", "artifact")
    atype = artifact.get("type", "unknown")
    files = artifact.get("files", [])
    chash = artifact.get("hash", "")

    if not message:
        message = f"artifact({atype}): {name}"
        if chash:
            message += f" [{chash[:16]}]"

    # Stage specified files, or all changes if no files specified
    if files:
        for f in files:
            filepath = os.path.join(CAGE_ROOT, f)
            if os.path.exists(filepath):
                _git("add", f)
    else:
        _git("add", "-A")

    # Check if there's anything to commit
    ok, status, _ = _git("diff", "--cached", "--quiet")
    if ok:
        return {"sha": None, "message": "nothing to commit", "branch": _current_branch()}

    ok, out, err = _git("commit", "-m", message)
    if not ok:
        return {"error": f"Commit failed: {err}"}

    # Get the commit SHA
    ok, sha, _ = _git("rev-parse", "HEAD")

    # Embed the commit for vector search (fire-and-forget)
    if sha:
        try:
            from ptc.embeddings import embed_commit
            diff_ok, diff_out, _ = _git("diff", "--stat", "HEAD~1..HEAD")
            embed_commit(sha, message, diff_out if diff_ok else "")
        except ImportError:
            pass

    return {
        "sha": sha if ok else None,
        "message": message,
        "branch": _current_branch(),
    }


def git_commit_trace(trace):
    """Commit a PTC trace result.

    Writes the trace to training/traces/ and commits.

    Args:
        trace: PTC execution trace dict

    Returns:
        dict: {sha, message, branch, trace_file}
    """
    run_id = trace.get("run_id", "unknown")
    intent = trace.get("intent", "unknown intent")

    # Write trace file
    trace_dir = os.path.join(CAGE_ROOT, "training", "traces")
    os.makedirs(trace_dir, exist_ok=True)
    trace_file = os.path.join(trace_dir, f"{run_id}.json")

    with open(trace_file, "w") as f:
        json.dump(trace, f, indent=2)

    # Commit
    rel_path = os.path.relpath(trace_file, CAGE_ROOT)
    _git("add", rel_path)

    message = f"trace: {intent} ({trace.get('tasks_completed', 0)}/{trace.get('tasks_decomposed', 0)} tasks)"

    ok, _, err = _git("commit", "-m", message)
    if not ok:
        return {"error": f"Commit failed: {err}"}

    ok, sha, _ = _git("rev-parse", "HEAD")

    return {
        "sha": sha if ok else None,
        "message": message,
        "branch": _current_branch(),
        "trace_file": rel_path,
    }


# ── History Navigation ────────────────────────────────────────


def git_log_for_node(node_id, tree_path=None, limit=20):
    """Get commits that affected files owned by a node.

    Args:
        node_id: tree node identifier
        tree_path: path to tree.json (default: CAGE_ROOT/tree.json)
        limit: max commits to return

    Returns:
        list[dict]: [{sha, message, date, files}]
    """
    tree_path = tree_path or os.path.join(CAGE_ROOT, "tree.json")

    # Load tree to find node's files
    try:
        with open(tree_path) as f:
            tree = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return []

    node_files = []
    for node in tree.get("nodes", []):
        if node.get("id") == node_id:
            node_files = node.get("metadata", {}).get("files", [])
            break

    if not node_files:
        return []

    # Get commits touching these files
    commits = []
    ok, out, _ = _git("log", f"--max-count={limit}", "--format=%H|%s|%aI", "--", *node_files)
    if ok and out:
        for line in out.split("\n"):
            parts = line.split("|", 2)
            if len(parts) >= 3:
                commits.append({
                    "sha": parts[0],
                    "message": parts[1],
                    "date": parts[2],
                    "node_id": node_id,
                    "files": node_files,
                })

    return commits


def git_diff_blueprint(blueprint_id):
    """Show what changed since a blueprint branch was created.

    Args:
        blueprint_id: e.g., "blueprint:ipfs-storage"

    Returns:
        dict: {branch, files_changed, insertions, deletions, diff_stat}
    """
    branch_name = f"design/{blueprint_id.replace(':', '-').replace(' ', '-')}"

    if not _branch_exists(branch_name):
        return {"error": f"Branch {branch_name} not found"}

    # Diff from where the branch diverged from main
    ok, out, _ = _git("diff", "--stat", f"main...{branch_name}")
    if not ok:
        return {"error": "Could not compute diff"}

    # Parse stat output
    lines = out.strip().split("\n") if out.strip() else []
    files_changed = max(0, len(lines) - 1)  # Last line is summary

    return {
        "branch": branch_name,
        "files_changed": files_changed,
        "diff_stat": out,
    }


def git_branches(pattern=None):
    """List branches, optionally filtered by pattern.

    Args:
        pattern: glob pattern (e.g., "design/*", "build/*")

    Returns:
        list[dict]: [{name, current, last_commit}]
    """
    args = ["branch", "--format=%(refname:short)|%(HEAD)|%(objectname:short)|%(subject)"]
    if pattern:
        args.append(f"--list={pattern}")

    ok, out, _ = _git(*args)
    if not ok or not out:
        return []

    branches = []
    for line in out.split("\n"):
        parts = line.split("|", 3)
        if len(parts) >= 4:
            branches.append({
                "name": parts[0],
                "current": parts[1] == "*",
                "sha": parts[2],
                "message": parts[3],
            })

    return branches


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for git operations."""
    import sys

    if len(sys.argv) < 2:
        print("Usage: python -m ptc.git_ops <command> [args]")
        print("Commands: branches [pattern], log-node <node-id>, diff <blueprint-id>")
        sys.exit(1)

    command = sys.argv[1]

    if command == "branches":
        pattern = sys.argv[2] if len(sys.argv) > 2 else None
        branches = git_branches(pattern)
        for b in branches:
            marker = "*" if b["current"] else " "
            print(f"  {marker} {b['name']} ({b['sha']}) {b['message']}")
        if not branches:
            print("  (no matching branches)")

    elif command == "log-node":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.git_ops log-node <node-id>")
            sys.exit(1)
        commits = git_log_for_node(sys.argv[2])
        for c in commits:
            print(f"  {c['sha'][:8]} {c['date'][:10]} {c['message']}")
        if not commits:
            print(f"  (no commits for node {sys.argv[2]})")

    elif command == "diff":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.git_ops diff <blueprint-id>")
            sys.exit(1)
        result = git_diff_blueprint(sys.argv[2])
        if "error" in result:
            print(f"  Error: {result['error']}")
        else:
            print(result.get("diff_stat", "(no changes)"))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
