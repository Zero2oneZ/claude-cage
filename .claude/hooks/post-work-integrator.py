#!/usr/bin/env python3
"""PostToolUse hook: auto-generates docs after file changes.

Fires after Write/Edit. Maps modified file to tree node, triggers
doc regeneration (fire-and-forget), logs INTEGRATE phase.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

CAGE_ROOT = os.environ.get("CLAUDE_PROJECT_DIR", os.environ.get("CAGE_ROOT", "."))
STORE_JS = os.path.join(CAGE_ROOT, "mongodb", "store.js")

# Same code extension filter as pre-work-router
CODE_EXTS = {".sh", ".py", ".js", ".rs", ".ts", ".jsx", ".tsx", ".toml", ".yml", ".yaml"}
SKIP_PATTERNS = [".claude/", "audit/", "node_modules/", "target/", ".env"]

# Reverse lookup: file prefix → node IDs to regenerate
REGEN_MAP = {
    "projects/test-apps/kv-store": ["capt:kv-store", "project:test-apps"],
    "projects/test-apps/chat-server": ["capt:chat-server", "project:test-apps"],
    "projects/test-apps/task-api": ["capt:task-api", "project:test-apps"],
    "projects/test-apps-rust/src/kv_store": ["capt:kv-store-rs", "project:test-apps-rust"],
    "projects/test-apps-rust/src/chat_server": ["capt:chat-server-rs", "project:test-apps-rust"],
    "projects/test-apps-rust/src/task_api": ["capt:task-api-rs", "project:test-apps-rust"],
    "lib/docker.sh": ["capt:docker", "dept:runtime"],
    "lib/sandbox.sh": ["capt:sandbox", "capt:network", "dept:security"],
    "lib/session.sh": ["capt:lifecycle", "dept:sessions"],
    "lib/cli.sh": ["capt:cli-parse"],
    "lib/mongodb.sh": ["capt:mongodb"],
    "lib/tree.sh": ["capt:tree-ops", "capt:scaffold"],
    "ptc/engine.py": ["capt:engine"],
    "ptc/executor.py": ["capt:executor"],
    "ptc/docs.py": ["capt:docs"],
    "web/app.py": ["capt:flask"],
    "tree.json": ["dept:tree", "root:cage"],
}


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

    if not file_path:
        return

    rel = os.path.relpath(file_path, CAGE_ROOT) if os.path.isabs(file_path) else file_path

    # Skip non-code / skip patterns
    if Path(rel).suffix not in CODE_EXTS:
        return
    for pat in SKIP_PATTERNS:
        if rel.startswith(pat):
            return

    # Find nodes to regenerate
    nodes_to_regen = set()
    for prefix, node_ids in REGEN_MAP.items():
        if rel.startswith(prefix) or rel == prefix:
            nodes_to_regen.update(node_ids)

    if not nodes_to_regen:
        return

    # Fire-and-forget: regenerate docs for affected nodes
    for node_id in nodes_to_regen:
        try:
            env = os.environ.copy()
            env["CAGE_ROOT"] = CAGE_ROOT
            env["PYTHONPATH"] = CAGE_ROOT
            subprocess.Popen(
                ["python3", "-m", "ptc.docs", "generate", node_id],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=env,
            )
        except Exception:
            pass

    # Log INTEGRATE phase
    if os.path.exists(STORE_JS):
        meta = json.dumps({"file": rel, "nodes": list(nodes_to_regen)})
        try:
            subprocess.Popen(
                ["node", STORE_JS, "log", "coordination:phase", f"INTEGRATE:doc-regen", meta],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass


if __name__ == "__main__":
    main()
