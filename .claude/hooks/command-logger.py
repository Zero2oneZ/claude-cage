#!/usr/bin/env python3
"""PostToolUse hook: logs Bash commands to MongoDB fire-and-forget store.

Reads tool call JSON from stdin, extracts command details, logs to MongoDB.
Keeps a local audit log as fallback.
"""

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path

STORE_JS = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "mongodb", "store.js")
AUDIT_DIR = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "audit")
AUDIT_FILE = os.path.join(AUDIT_DIR, "command_log.json")
MAX_ENTRIES = 200


def main():
    try:
        input_data = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        return

    tool_name = input_data.get("tool_name", "")
    if tool_name != "Bash":
        return

    tool_input = input_data.get("tool_input", {})
    command = tool_input.get("command", "")
    description = tool_input.get("description", "")

    if not command:
        return

    # Build log entry
    entry = {
        "timestamp": datetime.now().isoformat(),
        "command": command[:2000],  # truncate long commands
        "description": description,
        "tool": tool_name,
    }

    # Fire-and-forget to MongoDB
    if os.path.exists(STORE_JS):
        try:
            subprocess.Popen(
                ["node", STORE_JS, "log", "hook:bash", command[:200], json.dumps(entry)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass

    # Local audit log fallback
    try:
        Path(AUDIT_DIR).mkdir(parents=True, exist_ok=True)
        entries = []
        if os.path.exists(AUDIT_FILE):
            with open(AUDIT_FILE) as f:
                entries = json.load(f)
        entries.append(entry)
        entries = entries[-MAX_ENTRIES:]  # keep last N
        with open(AUDIT_FILE, "w") as f:
            json.dump(entries, f, indent=2)
    except Exception:
        pass


if __name__ == "__main__":
    main()
