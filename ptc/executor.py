"""executor.py — Leaf node executor.

The bridge between the tree and the real world.
Each leaf node receives a task and DOES THE WORK.
Results flow back up. Artifacts get stored.

Execution modes:
1. claude   — Invoke Claude Code to do the work (default)
2. shell    — Run a shell command
3. inspect  — Read files, analyze, report
4. compose  — Combine multiple outputs

The executor doesn't know about the tree.
It receives a task, executes it, returns results.
The engine handles the tree coordination.
"""

import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


CAGE_ROOT = os.environ.get("CAGE_ROOT", str(Path(__file__).parent.parent))


def execute(task):
    """Execute a leaf-level task. Returns output dict.

    The task contains:
    - node_id, node_name, scale
    - intent: what needs to be done
    - files: relevant source files
    - functions: relevant function names
    - rules: constraints to apply
    - lineage: path from root to this node
    """
    intent = task.get("intent", "")
    files = task.get("files", [])
    functions = task.get("functions", [])
    rules = task.get("rules", [])
    node_name = task.get("node_name", "unknown")

    # Determine execution mode from intent and node metadata
    mode = _detect_mode(task)

    if mode == "inspect":
        return _execute_inspect(task)
    elif mode == "shell":
        return _execute_shell(task)
    elif mode == "claude":
        return _execute_claude(task)
    elif mode == "compose":
        return _execute_compose(task)
    else:
        return _execute_plan(task)


def _detect_mode(task):
    """Detect execution mode from task context."""
    intent = task.get("intent", "").lower()
    files = task.get("files", [])

    # Keywords that suggest inspection/analysis
    inspect_words = ["show", "list", "check", "verify", "audit", "status", "inspect", "read"]
    if any(w in intent for w in inspect_words):
        return "inspect"

    # Keywords that suggest shell execution
    shell_words = ["build", "run", "install", "deploy", "start", "stop", "restart"]
    if any(w in intent for w in shell_words):
        return "shell"

    # Keywords that suggest Claude should do the work
    claude_words = ["create", "add", "implement", "fix", "refactor", "write", "update", "modify"]
    if any(w in intent for w in claude_words):
        return "claude"

    # Default to planning
    return "plan"


def _execute_inspect(task):
    """Inspect mode: read files, analyze, report."""
    files = task.get("files", [])
    results = {"mode": "inspect", "inspected": []}

    for f in files:
        fpath = os.path.join(CAGE_ROOT, f)
        if os.path.exists(fpath):
            stat = os.stat(fpath)
            results["inspected"].append({
                "file": f,
                "exists": True,
                "size": stat.st_size,
                "modified": datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc).isoformat(),
            })
        else:
            results["inspected"].append({
                "file": f,
                "exists": False,
            })

    results["summary"] = f"Inspected {len(results['inspected'])} files for {task['node_name']}"
    return results


def _execute_shell(task):
    """Shell mode: run a command and capture output."""
    intent = task.get("intent", "")
    node_id = task.get("node_id", "")

    # Build a safe command based on the intent and node context
    # We don't blindly execute — we construct known-safe commands
    command = _intent_to_command(task)

    if not command:
        return {
            "mode": "shell",
            "status": "skipped",
            "reason": f"Could not construct safe command for: {intent}",
        }

    try:
        result = subprocess.run(
            command,
            shell=True,
            capture_output=True,
            text=True,
            timeout=30,
            cwd=CAGE_ROOT,
        )
        return {
            "mode": "shell",
            "command": command,
            "exit_code": result.returncode,
            "stdout": result.stdout[:5000] if result.stdout else "",
            "stderr": result.stderr[:2000] if result.stderr else "",
        }
    except subprocess.TimeoutExpired:
        return {
            "mode": "shell",
            "command": command,
            "status": "timeout",
            "error": "Command timed out after 30s",
        }
    except Exception as e:
        return {
            "mode": "shell",
            "command": command,
            "status": "error",
            "error": str(e),
        }


def _intent_to_command(task):
    """Convert a task intent into a safe shell command.

    Only known patterns are allowed. Unknown intents return None.
    """
    intent = task.get("intent", "").lower()
    node_id = task.get("node_id", "")
    files = task.get("files", [])

    # Build commands
    if "build" in intent and ("docker" in intent or "image" in intent or "cli" in node_id):
        if "desktop" in intent or "desktop" in node_id:
            return "make build-desktop"
        return "make build-cli"

    # Status commands
    if "status" in intent or "check" in intent:
        return "make status"

    # Verify sandbox
    if "verify" in intent and "sandbox" in intent:
        return "make verify-sandbox"

    # Mongo commands
    if "mongo" in intent and "ping" in intent:
        return "make mongo-ping"
    if "mongo" in intent and "status" in intent:
        return "make mongo-status"

    # Tree commands
    if "tree" in intent and "show" in intent:
        return "make tree"

    return None


def _execute_claude(task):
    """Claude mode: invoke Claude Code to do the work.

    This is where the magic happens — Claude is instructed
    with the full node context and does the actual coding/work.
    """
    intent = task.get("intent", "")
    files = task.get("files", [])
    functions = task.get("functions", [])
    rules = task.get("rules", [])
    lineage = task.get("lineage", [])

    # Build the instruction for Claude
    instruction = _build_claude_instruction(task)

    # For now, return the instruction as the plan
    # In live mode, this would invoke `claude` CLI
    return {
        "mode": "claude",
        "instruction": instruction,
        "context": {
            "files": files,
            "functions": functions,
            "rules": [r["name"] for r in rules],
            "lineage": " → ".join(lineage),
        },
        "ready_for_execution": True,
    }


def _build_claude_instruction(task):
    """Build a structured instruction for Claude Code.

    This is the prompt that would be sent to Claude
    when executing in live mode.
    """
    parts = []

    # Context
    parts.append(f"## Task: {task['intent']}")
    parts.append(f"## Node: {task['node_name']} ({task['node_id']})")
    parts.append(f"## Scale: {task['scale']}")

    # Lineage — where this node sits in the tree
    if task.get("lineage"):
        parts.append(f"## Lineage: {' → '.join(task['lineage'])}")

    # Files to work with
    if task.get("files"):
        parts.append(f"## Files: {', '.join(task['files'])}")

    # Functions in scope
    if task.get("functions"):
        parts.append(f"## Functions: {', '.join(task['functions'])}")

    # Rules (constraints)
    if task.get("rules"):
        parts.append("## Rules:")
        for r in task["rules"]:
            parts.append(f"  - {r['name']}: IF {r.get('condition', '?')} THEN {r.get('action', '?')}")

    # Escalation path
    esc = task.get("escalation", {})
    if esc.get("target"):
        parts.append(f"## Escalation: → {esc['target']} if risk >= {esc.get('threshold', '?')}")

    return "\n".join(parts)


def _execute_compose(task):
    """Compose mode: combine multiple inputs into a single output."""
    return {
        "mode": "compose",
        "intent": task.get("intent", ""),
        "composed_from": task.get("lineage", []),
        "summary": f"Composition point for {task['node_name']}",
    }


def _execute_plan(task):
    """Plan mode: return what WOULD be done without doing it."""
    return {
        "mode": "plan",
        "intent": task.get("intent", ""),
        "node": task.get("node_id", ""),
        "files": task.get("files", []),
        "functions": task.get("functions", []),
        "rules_applied": [r["name"] for r in task.get("rules", [])],
        "summary": f"Planning: {task.get('intent', '')}",
    }
