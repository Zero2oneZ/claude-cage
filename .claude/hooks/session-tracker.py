#!/usr/bin/env python3
"""PostToolUse hook: tracks session lifecycle events.

Detects docker run/stop/rm commands and logs session transitions to MongoDB.
Also tracks file writes to session-related paths.
"""

import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path

STORE_JS = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "mongodb", "store.js")
AUDIT_DIR = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "audit")
AUDIT_FILE = os.path.join(AUDIT_DIR, "session_log.json")
MAX_ENTRIES = 100

# Patterns that indicate session lifecycle commands
SESSION_PATTERNS = [
    (r"docker\s+run\b.*cage", "session:start"),
    (r"docker\s+stop\b.*cage", "session:stop"),
    (r"docker\s+rm\b.*cage", "session:destroy"),
    (r"docker\s+exec\b.*cage", "session:attach"),
    (r"claude-cage\s+start", "session:start"),
    (r"claude-cage\s+stop", "session:stop"),
    (r"claude-cage\s+destroy", "session:destroy"),
    (r"claude-cage\s+shell", "session:attach"),
    (r"docker\s+build\b.*cage", "build:image"),
    (r"make\s+build", "build:image"),
    (r"make\s+run", "session:start"),
    (r"make\s+stop", "session:stop"),
]


def detect_event(command):
    """Match command against session lifecycle patterns."""
    for pattern, event_type in SESSION_PATTERNS:
        if re.search(pattern, command, re.IGNORECASE):
            return event_type
    return None


def extract_session_name(command):
    """Try to extract session name from command."""
    match = re.search(r"cage-([a-z]+-[a-z]+-[0-9a-f]{4})", command)
    if match:
        return match.group(1)
    match = re.search(r"--name\s+(\S+)", command)
    if match:
        return match.group(1)
    return None


def main():
    try:
        input_data = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        return

    tool_name = input_data.get("tool_name", "")
    tool_input = input_data.get("tool_input", {})

    command = ""
    if tool_name == "Bash":
        command = tool_input.get("command", "")
    elif tool_name == "Write":
        file_path = tool_input.get("file_path", "")
        if "claude-cage" in file_path or "session" in file_path:
            command = f"write:{file_path}"

    if not command:
        return

    event_type = detect_event(command)
    if not event_type:
        return

    session_name = extract_session_name(command)

    entry = {
        "timestamp": datetime.now().isoformat(),
        "event": event_type,
        "session": session_name,
        "command": command[:500],
        "tool": tool_name,
    }

    # Fire-and-forget to MongoDB
    if os.path.exists(STORE_JS):
        try:
            key = f"{event_type}:{session_name or 'unknown'}"
            subprocess.Popen(
                ["node", STORE_JS, "log", "session", key, json.dumps(entry)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass

    # Local audit log
    try:
        Path(AUDIT_DIR).mkdir(parents=True, exist_ok=True)
        entries = []
        if os.path.exists(AUDIT_FILE):
            with open(AUDIT_FILE) as f:
                entries = json.load(f)
        entries.append(entry)
        entries = entries[-MAX_ENTRIES:]
        with open(AUDIT_FILE, "w") as f:
            json.dump(entries, f, indent=2)
    except Exception:
        pass


if __name__ == "__main__":
    main()
