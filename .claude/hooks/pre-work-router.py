#!/usr/bin/env python3
"""PreToolUse hook: routes file changes through the tree BEFORE they happen.

Reads tool call JSON from stdin. If Write/Edit on a code file:
  - Maps file to tree node via path matching
  - Runs PTC dry-run to show routing decision
  - Logs coordination:phase INTAKE to MongoDB
  - Prints one-line routing summary (never blocks)
"""

import json
import os
import subprocess
import sys
from pathlib import Path

CAGE_ROOT = os.environ.get("CLAUDE_PROJECT_DIR", os.environ.get("CAGE_ROOT", "."))
STORE_JS = os.path.join(CAGE_ROOT, "mongodb", "store.js")

# File extensions that trigger routing
CODE_EXTS = {".sh", ".py", ".js", ".rs", ".ts", ".jsx", ".tsx", ".toml", ".yml", ".yaml"}

# Skip patterns (don't route docs, configs, hooks themselves)
SKIP_PATTERNS = [
    ".claude/",
    "audit/",
    "docs/",
    "CLAUDE.md",
    "README.md",
    ".gitignore",
    "node_modules/",
    "target/",
    ".env",
]

# File path → tree node prefix mapping
PATH_MAP = [
    ("projects/test-apps-rust/", "capt:build"),
    ("projects/test-apps/", "capt:build"),
    ("lib/docker.sh", "capt:infra"),
    ("lib/sandbox.sh", "dept:security"),
    ("lib/session.sh", "capt:state"),
    ("lib/cli.sh", "capt:api"),
    ("lib/config.sh", "capt:state"),
    ("lib/mongodb.sh", "capt:context"),
    ("lib/tree.sh", "capt:ptc"),
    ("lib/observability.sh", "capt:audit"),
    ("lib/lifecycle.sh", "capt:state"),
    ("lib/memory.sh", "capt:memory"),
    ("ptc/engine.py", "capt:ptc"),
    ("ptc/executor.py", "capt:exec"),
    ("ptc/docs.py", "capt:context"),
    ("ptc/architect.py", "capt:build"),
    ("ptc/ipfs.py", "capt:p2p"),
    ("ptc/embeddings.py", "capt:alexandria"),
    ("cage-web/src/codie_parser.rs", "capt:codie"),
    ("cage-web/src/routes/codie.rs", "capt:codie"),
    ("cage-web/src/routes/sessions.rs", "capt:state"),
    ("cage-web/src/routes/gentlyos.rs", "dept:orchestration"),
    ("cage-web/src/main.rs", "capt:api"),
    ("cage-web/src/subprocess.rs", "dept:runtime"),
    ("cage-web/", "dept:interface"),
    ("web/app.py", "capt:api"),
    ("web/templates/", "capt:ux"),
    ("docker/", "dept:runtime"),
    ("security/", "dept:security"),
    ("mongodb/", "capt:context"),
    ("gentlyos/", "dept:orchestration"),
    ("tree.json", "dept:orchestration"),
    ("Makefile", "capt:build"),
    ("bin/claude-cage", "capt:api"),
]


def resolve_node(file_path):
    """Map a file path to its tree node."""
    rel = os.path.relpath(file_path, CAGE_ROOT) if os.path.isabs(file_path) else file_path
    for prefix, node_id in PATH_MAP:
        if rel.startswith(prefix) or rel == prefix:
            return node_id
    return None


def should_route(file_path):
    """Check if this file change warrants routing."""
    rel = os.path.relpath(file_path, CAGE_ROOT) if os.path.isabs(file_path) else file_path
    # Skip non-code files
    if Path(rel).suffix not in CODE_EXTS:
        return False
    # Skip patterns
    for pattern in SKIP_PATTERNS:
        if rel.startswith(pattern):
            return False
    return True


def log_phase(phase, action, meta=None):
    """Fire-and-forget log to MongoDB."""
    if not os.path.exists(STORE_JS):
        return
    value = json.dumps(meta or {})
    try:
        subprocess.Popen(
            ["node", STORE_JS, "log", "coordination:phase", f"{phase}:{action}", value],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


def main():
    try:
        input_data = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        return

    tool_name = input_data.get("tool_name", "")
    if tool_name not in ("Write", "Edit"):
        return

    tool_input = input_data.get("tool_input", {})
    file_path = tool_input.get("file_path", "")

    if not file_path or not should_route(file_path):
        return

    node_id = resolve_node(file_path)
    rel_path = os.path.relpath(file_path, CAGE_ROOT) if os.path.isabs(file_path) else file_path

    # Log INTAKE phase
    log_phase("INTAKE", f"modify:{rel_path}", {
        "file": rel_path,
        "node": node_id,
        "tool": tool_name,
    })

    # Print routing summary to stderr (visible in Claude Code)
    if node_id:
        print(f"[TREE] {rel_path} -> {node_id}", file=sys.stderr)
    else:
        print(f"[TREE] {rel_path} -> (unowned file)", file=sys.stderr)


if __name__ == "__main__":
    main()
