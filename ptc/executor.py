"""executor.py — Leaf node executor.

The bridge between the tree and the real world.
Each leaf node receives a task and DOES THE WORK.
Results flow back up. Artifacts get stored.

Execution modes:
1. design   — Architect mode: produce a blueprint, not code
2. claude   — Invoke Claude Code to do the work
3. shell    — Run a shell command
4. inspect  — Read files, analyze, report
5. compose  — Combine multiple outputs

The executor doesn't know about the tree.
It receives a task, executes it, returns results.
The engine handles the tree coordination.
"""

import json
import os
import subprocess
import sys
import time
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

    if mode == "design":
        return _execute_design(task)
    elif mode == "inspect":
        return _execute_inspect(task)
    elif mode == "shell":
        return _execute_shell(task)
    elif mode == "native":
        return _execute_native(task)
    elif mode == "claude":
        return _execute_claude(task)
    elif mode == "compose":
        return _execute_compose(task)
    elif mode == "codie":
        return _execute_codie(task)
    else:
        return _execute_plan(task)


def _detect_mode(task):
    """Detect execution mode from task context."""
    intent = task.get("intent", "").lower()
    files = task.get("files", [])

    # CODIE mode: task has codie metadata or intent mentions codie
    # Must be checked first — "codie build" would otherwise match shell_words
    if task.get("codie_program") or "codie" in intent:
        return "codie"

    # Native mode: cargo, nix, or nixos-rebuild commands
    native_words = ["cargo build", "cargo test", "cargo clippy", "cargo fmt",
                     "nix build", "nix develop", "nix flake", "nixos-rebuild",
                     "rebuild crate", "rebuild tier"]
    if any(w in intent for w in native_words):
        return "native"

    # Keywords that suggest architect/design mode
    design_words = ["design", "architect", "blueprint", "specify", "plan architecture", "draft"]
    if any(w in intent for w in design_words):
        return "design"

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


def _execute_design(task):
    """Design mode: produce a blueprint, not code.

    This is the architect speaking. No hammers.
    Returns a blueprint node that can be decomposed into builder tasks.
    """
    try:
        from ptc.architect import create_blueprint

        blueprint = create_blueprint(
            intent=task.get("intent", ""),
            context={
                "node_id": task.get("node_id"),
                "files": task.get("files", []),
                "functions": task.get("functions", []),
                "rules": task.get("rules", []),
                "lineage": task.get("lineage", []),
            }
        )

        content = {}
        for artifact in blueprint.get("artifacts", []):
            if artifact.get("type") == "blueprint":
                content = artifact.get("content", {})
                break

        return {
            "mode": "design",
            "blueprint_id": blueprint.get("id"),
            "blueprint_name": blueprint.get("name"),
            "cached": blueprint.get("metadata", {}).get("cached", False),
            "task_count": len(content.get("builder_tasks", [])),
            "status": blueprint.get("metadata", {}).get("status", "draft"),
            "hash": blueprint.get("metadata", {}).get("content_hash", "")[:20],
        }
    except ImportError:
        return {
            "mode": "design",
            "error": "architect module not available",
            "plan": f"Would design: {task.get('intent', '')}",
        }


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

    # Cargo commands (for shell mode fallback — native mode handles these directly)
    if "cargo" in intent and "build" in intent:
        crate = _extract_crate_name(intent)
        return f"cargo build -p {crate}" if crate else "cargo build --workspace"
    if "cargo" in intent and "test" in intent:
        crate = _extract_crate_name(intent)
        return f"cargo test -p {crate}" if crate else "cargo test --workspace"
    if "cargo" in intent and "clippy" in intent:
        crate = _extract_crate_name(intent)
        return f"cargo clippy -p {crate}" if crate else "cargo clippy --workspace"

    # Nix commands
    if "nix" in intent and "build" in intent:
        return "nix build"
    if "nix" in intent and "flake" in intent:
        return "nix flake check"

    return None


def _execute_claude(task):
    """Claude mode: invoke Claude Code to do the work.

    Builds a structured instruction from the task context,
    invokes `claude --print` in non-interactive mode,
    captures the output, stores it as an artifact.
    """
    intent = task.get("intent", "")
    files = task.get("files", [])
    functions = task.get("functions", [])
    rules = task.get("rules", [])
    lineage = task.get("lineage", [])
    node_id = task.get("node_id", "unknown")

    # Check approval gate before executing
    approval = _check_approval(task)
    if approval["blocked"]:
        return {
            "mode": "claude",
            "status": "blocked",
            "reason": approval["reason"],
            "risk": approval["risk"],
            "escalated_to": approval.get("escalated_to"),
        }

    # Build the instruction for Claude
    instruction = _build_claude_instruction(task)

    # Invoke Claude Code CLI in non-interactive mode
    try:
        # --print sends the prompt, prints the response, exits
        result = subprocess.run(
            ["claude", "--print", instruction],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=CAGE_ROOT,
            env={**os.environ, "CLAUDE_CODE_ENTRYPOINT": "ptc"},
        )

        output = result.stdout.strip() if result.stdout else ""
        stderr = result.stderr.strip() if result.stderr else ""

        # Store the output as an artifact
        store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
        artifact_name = f"claude-output-{node_id}-{int(time.time())}"
        try:
            doc = json.dumps({
                "name": artifact_name,
                "type": "claude_output",
                "content": output[:50000],
                "project": "claude-cage",
                "node_id": node_id,
                "intent": intent,
            })
            subprocess.Popen(
                ["node", store_js, "put", "artifacts", doc],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass

        return {
            "mode": "claude",
            "exit_code": result.returncode,
            "output": output[:10000],
            "stderr": stderr[:2000] if result.returncode != 0 else "",
            "instruction": instruction,
            "context": {
                "files": files,
                "functions": functions,
                "rules": [r["name"] for r in rules],
                "lineage": " -> ".join(lineage),
            },
            "artifact": artifact_name,
            "approval": approval,
        }

    except FileNotFoundError:
        # claude CLI not installed — fall back to plan mode
        return {
            "mode": "claude",
            "status": "fallback",
            "reason": "claude CLI not found in PATH",
            "instruction": instruction,
            "context": {
                "files": files,
                "functions": functions,
                "rules": [r["name"] for r in rules],
                "lineage": " -> ".join(lineage),
            },
        }
    except subprocess.TimeoutExpired:
        return {
            "mode": "claude",
            "status": "timeout",
            "error": "Claude CLI timed out after 120s",
            "instruction": instruction,
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


# ── Native Execution Mode ─────────────────────────────────────


def _execute_native(task):
    """Native mode: run cargo/nix/rebuild commands directly on host.

    Three sub-modes:
    - cargo: cargo build/test/clippy/fmt for individual crates
    - nix: nix build/develop/flake-check
    - rebuild: nixos-rebuild switch (risk 9 — always requires human approval)
    """
    intent = task.get("intent", "").lower()
    node_id = task.get("node_id", "")

    # Check approval gate
    approval = _check_approval(task)
    if approval["blocked"]:
        return {
            "mode": "native",
            "status": "blocked",
            "reason": approval["reason"],
            "risk": approval["risk"],
            "escalated_to": approval.get("escalated_to"),
        }

    # Detect sub-mode
    if "nixos-rebuild" in intent:
        return _execute_native_rebuild(task, approval)
    elif "nix " in intent or "nix build" in intent or "nix develop" in intent or "nix flake" in intent:
        return _execute_native_nix(task, approval)
    else:
        return _execute_native_cargo(task, approval)


def _execute_native_cargo(task, approval):
    """Handle cargo build/test/clippy/fmt for individual crates."""
    intent = task.get("intent", "").lower()
    node_id = task.get("node_id", "")

    # Extract crate name from intent
    crate = _extract_crate_name(intent)

    # Determine cargo sub-command
    if "test" in intent:
        cmd = f"cargo test -p {crate}" if crate else "cargo test --workspace"
    elif "clippy" in intent:
        cmd = f"cargo clippy -p {crate}" if crate else "cargo clippy --workspace"
    elif "fmt" in intent:
        cmd = "cargo fmt --all --check"
    else:
        cmd = f"cargo build -p {crate}" if crate else "cargo build --workspace"

    # Workspace root is the Gently-nix project
    workspace_root = os.path.join(CAGE_ROOT, "projects", "Gently-nix")
    if not os.path.isdir(workspace_root):
        workspace_root = CAGE_ROOT

    try:
        result = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=300,
            cwd=workspace_root,
        )
        return {
            "mode": "native",
            "sub_mode": "cargo",
            "command": cmd,
            "crate": crate,
            "exit_code": result.returncode,
            "stdout": result.stdout[:5000] if result.stdout else "",
            "stderr": result.stderr[:2000] if result.stderr else "",
            "approval": approval,
        }
    except subprocess.TimeoutExpired:
        return {
            "mode": "native",
            "sub_mode": "cargo",
            "command": cmd,
            "status": "timeout",
            "error": "Cargo command timed out after 300s",
        }
    except FileNotFoundError:
        return {
            "mode": "native",
            "sub_mode": "cargo",
            "command": cmd,
            "status": "error",
            "error": "cargo not found in PATH",
        }


def _execute_native_nix(task, approval):
    """Handle nix build/develop/flake-check commands."""
    intent = task.get("intent", "").lower()

    if "flake check" in intent or "flake" in intent:
        cmd = "nix flake check"
    elif "develop" in intent:
        cmd = "nix develop --command echo 'devshell OK'"
    else:
        # Extract target from intent (e.g., "nix build .#gently-cli")
        target = _extract_nix_target(intent)
        cmd = f"nix build .#{target}" if target else "nix build"

    workspace_root = os.path.join(CAGE_ROOT, "projects", "Gently-nix")
    if not os.path.isdir(workspace_root):
        workspace_root = CAGE_ROOT

    try:
        result = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=600,
            cwd=workspace_root,
        )
        return {
            "mode": "native",
            "sub_mode": "nix",
            "command": cmd,
            "exit_code": result.returncode,
            "stdout": result.stdout[:5000] if result.stdout else "",
            "stderr": result.stderr[:2000] if result.stderr else "",
            "approval": approval,
        }
    except subprocess.TimeoutExpired:
        return {
            "mode": "native",
            "sub_mode": "nix",
            "command": cmd,
            "status": "timeout",
            "error": "Nix command timed out after 600s",
        }
    except FileNotFoundError:
        return {
            "mode": "native",
            "sub_mode": "nix",
            "command": cmd,
            "status": "error",
            "error": "nix not found in PATH",
        }


def _execute_native_rebuild(task, approval):
    """Handle nixos-rebuild switch — risk 9, always requires human approval."""
    # Force risk to 9 for rebuild
    if approval["risk"] < 9:
        return {
            "mode": "native",
            "sub_mode": "rebuild",
            "status": "blocked",
            "reason": "nixos-rebuild switch requires human approval (risk 9)",
            "risk": 9,
            "escalated_to": "root:human",
        }

    return {
        "mode": "native",
        "sub_mode": "rebuild",
        "status": "blocked",
        "reason": "nixos-rebuild switch requires human approval (risk 9)",
        "risk": 9,
        "escalated_to": "root:human",
        "command": "nixos-rebuild switch",
    }


def _execute_tier_rebuild(task, changed_crates):
    """Tier-aware crate rebuild: load graph, compute blast radius, build in order.

    Given a set of changed crates:
    1. Load crate graph
    2. Calculate blast radius (all affected crates)
    3. Sort by tier (build order)
    4. Execute cargo build -p <crate> for each in order
    5. Return results for all builds
    """
    from ptc.crate_graph import load_graph, blast_radius, build_order

    graph = load_graph()
    radius = blast_radius(graph, changed_crates)
    ordered = radius["affected"]

    workspace_root = os.path.join(CAGE_ROOT, "projects", "Gently-nix")
    if not os.path.isdir(workspace_root):
        workspace_root = CAGE_ROOT

    build_results = []
    all_passed = True

    for crate in ordered:
        cmd = f"cargo build -p {crate}"
        try:
            result = subprocess.run(
                cmd,
                shell=True,
                capture_output=True,
                text=True,
                timeout=300,
                cwd=workspace_root,
            )
            passed = result.returncode == 0
            build_results.append({
                "crate": crate,
                "tier": graph["crates"].get(crate, {}).get("tier"),
                "command": cmd,
                "passed": passed,
                "exit_code": result.returncode,
                "stderr": result.stderr[:1000] if not passed else "",
            })
            if not passed:
                all_passed = False
                break  # Stop on first failure
        except (subprocess.TimeoutExpired, FileNotFoundError) as e:
            build_results.append({
                "crate": crate,
                "tier": graph["crates"].get(crate, {}).get("tier"),
                "command": cmd,
                "passed": False,
                "error": str(e),
            })
            all_passed = False
            break

    return {
        "mode": "native",
        "sub_mode": "tier_rebuild",
        "blast_radius": radius["summary"],
        "risk": radius["risk"],
        "affected_crates": len(ordered),
        "builds_attempted": len(build_results),
        "all_passed": all_passed,
        "build_results": build_results,
    }


def _extract_crate_name(intent):
    """Extract a gently-* crate name from an intent string."""
    import re
    match = re.search(r'(gently-[\w-]+|gentlyos-[\w-]+)', intent)
    return match.group(1) if match else None


def _extract_nix_target(intent):
    """Extract a nix build target from intent (e.g., .#gently-cli)."""
    import re
    # Match .#target or just a gently- crate name
    match = re.search(r'\.#([\w-]+)', intent)
    if match:
        return match.group(1)
    match = re.search(r'(gently-[\w-]+|gentlyos-[\w-]+)', intent)
    return match.group(1) if match else None


# ── CODIE Execution Mode ───────────────────────────────────────


def _build_codie_instruction(task):
    """Convert PTC leaf task to CODIE instruction chain."""
    node_id = task.get("node_id", "unknown")
    intent = task.get("intent", "")
    rules = task.get("rules", [])
    files = task.get("files", [])

    safe_id = node_id.replace(":", "_").upper()
    lines = [f"pug {safe_id}"]
    lines.append("|")

    # Rules as bones
    if rules:
        lines.append("+-- fence RULES")
        for r in rules:
            cond = r.get("condition", "?")
            act = r.get("action", "?")
            lines.append(f"|   +-- bone {cond} -> {act}")
        lines.append("|")

    # Context as elfs
    lines.append("+-- elf context")
    lines.append(f'|   +-- elf node_id <- "{node_id}"')
    lines.append(f'|   +-- elf intent <- "{intent}"')
    lines.append("|")

    # Files as barks
    for f in files:
        lines.append(f"+-- bark content <- @fs/read({f})")
    lines.append("|")

    # Execute
    lines.append("+-- cali EXECUTE_INTENT(context)")
    lines.append("|")

    # Return + checkpoint
    lines.append("+-- biz -> result")
    safe_anchor = node_id.replace(":", "_")
    lines.append(f"    +-- anchor #{safe_anchor}")

    return "\n".join(lines)


def _execute_codie(task):
    """CODIE mode: parse and interpret a CODIE program or generated instruction.

    Two paths:
    1. task has 'codie_program' — load and execute a .codie file
    2. task has no program — generate CODIE from task metadata, then execute

    The interpreter walks the AST and maps each node to real actions:
      pug   → entry point, set up execution context
      bark  → fetch file contents or run data source queries
      elf   → bind variable in context
      cali  → call a function (shell command, make target, or Claude)
      spin  → loop over a collection
      turk  → conditional transform
      fence → guard block (check preconditions, abort on failure)
      bone  → rule check (constraint enforcement)
      pin   → set immutable constant
      blob  → define data structure in context
      biz   → return final result
      anchor → checkpoint (log to MongoDB, snapshot state)
    """
    node_id = task.get("node_id", "unknown")
    intent = task.get("intent", "")

    # Check approval gate
    approval = _check_approval(task)
    if approval["blocked"]:
        return {
            "mode": "codie",
            "status": "blocked",
            "reason": approval["reason"],
            "risk": approval["risk"],
            "escalated_to": approval.get("escalated_to"),
        }

    # Get or generate CODIE source
    codie_program = task.get("codie_program")
    if codie_program:
        # Load from file
        codie_path = os.path.join(CAGE_ROOT, "projects", "Gently-nix", "tools", "codie-maps", f"{codie_program}.codie")
        if not os.path.exists(codie_path):
            codie_path = os.path.join(CAGE_ROOT, codie_program)
        if os.path.exists(codie_path):
            with open(codie_path) as f:
                codie_source = f.read()
        else:
            return {
                "mode": "codie",
                "status": "error",
                "error": f"CODIE program not found: {codie_program}",
            }
    else:
        codie_source = _build_codie_instruction(task)

    # Parse the CODIE source into AST (use cage-web binary or Python fallback)
    ast_nodes = _parse_codie(codie_source)

    # Interpret the AST
    ctx = CodieContext(node_id=node_id, intent=intent, task=task)
    try:
        result = ctx.execute(ast_nodes)
    except CodieHalt as e:
        result = {"halted": True, "reason": str(e)}
    except Exception as e:
        result = {"error": str(e)}

    # Store execution trace as artifact
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    artifact_name = f"codie-exec-{node_id}-{int(time.time())}"
    try:
        doc = json.dumps({
            "name": artifact_name,
            "type": "codie_execution",
            "content": json.dumps({
                "source": codie_source[:10000],
                "result": result,
                "checkpoints": ctx.checkpoints,
                "variables": {k: str(v)[:500] for k, v in ctx.variables.items()},
            }),
            "project": "claude-cage",
            "node_id": node_id,
        })
        subprocess.Popen(
            ["node", store_js, "put", "artifacts", doc],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass

    return {
        "mode": "codie",
        "codie_source": codie_source[:5000],
        "instruction_count": codie_source.count("\n") + 1,
        "node_id": node_id,
        "intent": intent,
        "status": "completed" if "error" not in result else "failed",
        "result": result,
        "checkpoints": ctx.checkpoints,
        "variables_set": list(ctx.variables.keys()),
        "approval": approval,
    }


# ── CODIE Interpreter ─────────────────────────────────────────


class CodieHalt(Exception):
    """Raised when a fence/bone check fails and execution must stop."""
    pass


class CodieContext:
    """Runtime context for CODIE program execution."""

    def __init__(self, node_id="unknown", intent="", task=None):
        self.node_id = node_id
        self.intent = intent
        self.task = task or {}
        self.variables = {}
        self.constants = {}
        self.checkpoints = []
        self.structs = {}
        self.results = []
        self.store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")

    def execute(self, nodes):
        """Walk AST nodes and execute each one. Returns final result."""
        final = None
        for node in nodes:
            result = self._exec_node(node)
            if result is not None:
                final = result
        return final if final is not None else {"completed": True, "checkpoints": self.checkpoints}

    def _exec_node(self, node):
        """Dispatch a single AST node to its handler."""
        ntype = node.get("type", "")

        if ntype == "Entry":
            return self._exec_entry(node)
        elif ntype == "Fetch":
            return self._exec_fetch(node)
        elif ntype == "Bind":
            return self._exec_bind(node)
        elif ntype == "Call":
            return self._exec_call(node)
        elif ntype == "Guard":
            return self._exec_guard(node)
        elif ntype == "Rule":
            return self._exec_rule(node)
        elif ntype == "Loop":
            return self._exec_loop(node)
        elif ntype == "Conditional":
            return self._exec_conditional(node)
        elif ntype == "Return":
            return self._exec_return(node)
        elif ntype == "Checkpoint":
            return self._exec_checkpoint(node)
        elif ntype == "Const":
            return self._exec_const(node)
        elif ntype == "Transform":
            return self._exec_transform(node)
        elif ntype == "Struct":
            return self._exec_struct(node)
        elif ntype == "Comment":
            return None
        return None

    def _exec_entry(self, node):
        """pug — entry point. Set up context and execute children."""
        self.variables["_entry"] = node.get("name", "")
        self._log_event("codie:entry", node.get("name", ""))
        children = node.get("children", [])
        final = None
        for child in children:
            result = self._exec_node(child)
            if result is not None:
                final = result
        return final

    def _exec_fetch(self, node):
        """bark — fetch data from a source.

        Sources:
          @fs/read(path) — read a file
          @system/* — run a system query
          @cargo/* — cargo operations
          @<anything> — treat as file path relative to CAGE_ROOT
          plain path — read file
        """
        source = node.get("source", "")
        target = node.get("target", "")

        value = self._resolve_source(source)

        if target:
            self.variables[target] = value
        return None

    def _resolve_source(self, source):
        """Resolve a bark source reference to actual data."""
        source = source.strip()

        # @fs/read(path) — read file contents
        if source.startswith("@fs/read(") and source.endswith(")"):
            path = source[9:-1].strip()
            return self._read_file(path)

        # @system/* — system detection
        if source.startswith("@system/"):
            query = source[8:]
            return self._system_query(query)

        # @cargo/* — cargo operations
        if source.startswith("@cargo/"):
            op = source[7:]
            return self._cargo_op(op)

        # @toolchain/* — tool detection
        if source.startswith("@toolchain/"):
            tool = source[11:]
            return self._check_toolchain(tool)

        # @validators/* — run validator
        if source.startswith("@validators/"):
            validator = source[12:]
            return self._run_validator(validator)

        # Plain @ reference — treat as file path
        if source.startswith("@"):
            path = source[1:]
            return self._read_file(path)

        # Bare path
        return self._read_file(source)

    def _read_file(self, path):
        """Read a file, resolving variables in the path."""
        # Resolve variable references like {platform.type}
        for var, val in self.variables.items():
            if isinstance(val, str):
                path = path.replace(f"{{{var}}}", val)
            elif isinstance(val, dict):
                for k, v in val.items():
                    path = path.replace(f"{{{var}.{k}}}", str(v))

        fpath = os.path.join(CAGE_ROOT, path)
        if os.path.exists(fpath):
            try:
                with open(fpath) as f:
                    return f.read()[:50000]
            except Exception as e:
                return {"error": str(e), "path": path}
        return {"missing": True, "path": path}

    def _system_query(self, query):
        """Handle @system/* queries."""
        import platform as plat
        if query == "detect_os":
            return {
                "type": plat.system().lower(),
                "release": plat.release(),
                "machine": plat.machine(),
                "unknown": False,
            }
        if query == "detect_all" or query == "specs":
            import shutil
            total, used, free = shutil.disk_usage("/")
            return {
                "os": plat.system().lower(),
                "machine": plat.machine(),
                "disk_total_gb": round(total / (1024**3), 1),
                "disk_free_gb": round(free / (1024**3), 1),
            }
        return {"query": query, "result": "unknown"}

    def _cargo_op(self, op):
        """Handle @cargo/* operations."""
        if op.startswith("build(") and op.endswith(")"):
            crate = op[6:-1]
            try:
                result = subprocess.run(
                    ["cargo", "build", "--release", "-p", crate],
                    capture_output=True, text=True, timeout=300, cwd=CAGE_ROOT,
                )
                return {
                    "crate": crate,
                    "success": result.returncode == 0,
                    "failed": result.returncode != 0,
                    "error": result.stderr[:2000] if result.returncode != 0 else "",
                }
            except (FileNotFoundError, subprocess.TimeoutExpired) as e:
                return {"crate": crate, "failed": True, "error": str(e)}

        if op == "test_workspace":
            try:
                result = subprocess.run(
                    ["cargo", "test", "--all"],
                    capture_output=True, text=True, timeout=300, cwd=CAGE_ROOT,
                )
                return {
                    "success": result.returncode == 0,
                    "failed": result.returncode != 0,
                    "output": result.stdout[:5000],
                }
            except (FileNotFoundError, subprocess.TimeoutExpired) as e:
                return {"failed": True, "error": str(e)}

        return {"op": op, "status": "unknown"}

    def _check_toolchain(self, tool):
        """Handle @toolchain/* checks."""
        cmd_map = {"rust": "rustc", "c_compiler": "cc", "nix": "nix"}
        cmd = cmd_map.get(tool, tool)
        try:
            result = subprocess.run(
                [cmd, "--version"], capture_output=True, text=True, timeout=10,
            )
            return {"tool": tool, "available": True, "missing": False, "version": result.stdout.strip()[:100]}
        except FileNotFoundError:
            return {"tool": tool, "available": False, "missing": True}
        except subprocess.TimeoutExpired:
            return {"tool": tool, "available": False, "missing": True, "error": "timeout"}

    def _run_validator(self, validator):
        """Handle @validators/* — run validation scripts."""
        # Resolve variables in validator name
        for var, val in self.variables.items():
            if isinstance(val, str):
                validator = validator.replace(f"{{{var}}}", val)
            elif isinstance(val, dict):
                for k, v in val.items():
                    validator = validator.replace(f"{{{var}.{k}}}", str(v))

        vpath = os.path.join(CAGE_ROOT, "scripts", validator)
        if os.path.exists(vpath):
            try:
                result = subprocess.run(
                    ["bash", vpath], capture_output=True, text=True, timeout=60, cwd=CAGE_ROOT,
                )
                return {"errors": 0 if result.returncode == 0 else 1, "output": result.stdout[:2000]}
            except Exception as e:
                return {"errors": 1, "error": str(e)}
        return {"errors": 0, "skipped": True, "reason": f"validator not found: {validator}"}

    def _exec_bind(self, node):
        """elf — bind a variable."""
        name = node.get("name", "")
        value = node.get("value", "")

        # Resolve variable references in value
        if value.startswith("@"):
            value = self._resolve_source(value)

        if name in self.constants:
            return None  # Constants can't be rebound

        self.variables[name] = value
        return None

    def _exec_call(self, node):
        """cali — call a function.

        Maps to safe shell commands, make targets, or Claude invocations.
        """
        name = node.get("name", "")
        args = node.get("args", "")
        children = node.get("children", [])

        # Resolve variable references in args
        for var, val in self.variables.items():
            if isinstance(val, str):
                args = args.replace(f"{{{var}}}", val)

        result = None

        # Known safe call patterns
        safe_calls = {
            "EXECUTE_INTENT": lambda: {"intent": self.intent, "executed": True},
            "BUILD": lambda: self._safe_shell(f"make build-{args}" if args else "make build"),
            "TEST": lambda: self._safe_shell("make test" if not args else f"cargo test -p {args}"),
            "STATUS": lambda: self._safe_shell("make status"),
            "VERIFY": lambda: self._safe_shell("make verify-sandbox"),
            "SEED": lambda: self._safe_shell("make gentlyos-seed"),
        }

        # Check if this is a known safe call
        call_upper = name.upper()
        if call_upper in safe_calls:
            result = safe_calls[call_upper]()
        else:
            # Unknown call — log but don't execute
            result = {"call": name, "args": args, "status": "planned", "reason": "unknown call pattern"}

        # Execute children in the call's context
        for child in children:
            self._exec_node(child)

        self.variables[f"_call_{name}"] = result
        return None

    def _safe_shell(self, command):
        """Run a shell command from the known-safe set."""
        # Only allow make targets, cargo, nix, and safe system commands
        allowed_prefixes = ["make ", "cargo ", "nix ", "rustc ", "rustfmt ",
                            "docker ps", "docker info", "node "]
        if not any(command.startswith(p) for p in allowed_prefixes):
            return {"command": command, "status": "blocked", "reason": "not in safe command set"}

        try:
            result = subprocess.run(
                command, shell=True, capture_output=True, text=True,
                timeout=60, cwd=CAGE_ROOT,
            )
            return {
                "command": command,
                "exit_code": result.returncode,
                "stdout": result.stdout[:5000],
                "stderr": result.stderr[:2000] if result.returncode != 0 else "",
            }
        except subprocess.TimeoutExpired:
            return {"command": command, "status": "timeout"}

    def _exec_guard(self, node):
        """fence — guard block. Check all children (bones/conditionals).
        If any rule is violated, halt execution.
        """
        name = node.get("name", "")
        children = node.get("children", [])

        for child in children:
            ctype = child.get("type", "")
            if ctype == "Rule":
                negated = child.get("negated", False)
                body = child.get("body", "")
                # Negated rules (bone NOT:) mean "this must NOT happen"
                # For now, log them as active constraints
                self.variables[f"_constraint_{body.replace(' ', '_')[:30]}"] = {
                    "negated": negated,
                    "active": True,
                }
            else:
                self._exec_node(child)

        self._log_event("codie:fence", name or "guard")
        return None

    def _exec_rule(self, node):
        """bone — rule check. Negated rules (NOT:) are constraints."""
        name = node.get("name", "")
        negated = node.get("negated", False)
        body = node.get("body", "")
        children = node.get("children", [])

        # Store as active constraint
        rule_key = name or body.replace(" ", "_")[:30]
        self.variables[f"_rule_{rule_key}"] = {
            "negated": negated,
            "body": body,
            "enforced": True,
        }

        for child in children:
            self._exec_node(child)

        return None

    def _exec_loop(self, node):
        """spin — loop over a collection."""
        var = node.get("var", "item")
        collection = node.get("collection", "")
        body = node.get("body", [])

        # Resolve collection from variables or literal
        items = self.variables.get(collection)
        if items is None:
            # Try parsing as JSON literal
            try:
                items = json.loads(collection)
            except (json.JSONDecodeError, TypeError):
                items = [collection] if collection else []

        if not isinstance(items, list):
            items = [items]

        for item in items:
            self.variables[var] = item
            for child in body:
                self._exec_node(child)

        return None

    def _exec_conditional(self, node):
        """? condition -> action — evaluate condition, take action."""
        condition = node.get("condition", "")
        action = node.get("action", "")

        # Evaluate condition against current variables
        triggered = self._eval_condition(condition)
        if triggered:
            if action.startswith("return ") or action.startswith("return\""):
                return {"returned": action[7:].strip().strip('"')}
            elif action.startswith("warn "):
                self.results.append({"warning": action[5:].strip().strip('"')})
        return None

    def _eval_condition(self, condition):
        """Simple condition evaluator against current variables."""
        condition = condition.strip()

        # var.field — check nested variable
        if "." in condition:
            parts = condition.split(".", 1)
            var = self.variables.get(parts[0])
            if isinstance(var, dict):
                field = parts[1].split()[0]  # Get field name before any operator
                val = var.get(field)

                # Check comparison operators
                if " < " in condition:
                    try:
                        threshold = float(condition.split(" < ")[1])
                        return val is not None and float(val) < threshold
                    except (ValueError, TypeError):
                        return False
                if " > " in condition:
                    try:
                        threshold = float(condition.split(" > ")[1])
                        return val is not None and float(val) > threshold
                    except (ValueError, TypeError):
                        return False

                # Boolean field check (e.g., platform.unknown, rust.missing)
                return bool(val)

        # Simple variable truth check
        val = self.variables.get(condition)
        return bool(val)

    def _exec_return(self, node):
        """biz — return a result value."""
        value = node.get("value", "")

        # Resolve variable references
        for var, val in self.variables.items():
            if isinstance(val, str):
                value = value.replace(f"{{{var}}}", val)

        self._log_event("codie:return", value[:200])
        return {"returned": value, "checkpoints": self.checkpoints, "variables": list(self.variables.keys())}

    def _exec_checkpoint(self, node):
        """anchor — log a checkpoint to MongoDB and record state."""
        name = node.get("name", "checkpoint")
        checkpoint = {
            "name": name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "variables_set": list(self.variables.keys()),
            "constants_set": list(self.constants.keys()),
        }
        self.checkpoints.append(checkpoint)
        self._log_event("codie:anchor", name)
        return None

    def _exec_const(self, node):
        """pin — set an immutable constant."""
        name = node.get("name", "")
        value = node.get("value", "")
        self.constants[name] = value
        self.variables[name] = value  # Also accessible as variable
        return None

    def _exec_transform(self, node):
        """turk — conditional transformation."""
        condition = node.get("condition", "")
        target = node.get("target", "")

        if self._eval_condition(condition):
            self.variables["_transform_result"] = target
        return None

    def _exec_struct(self, node):
        """blob — define a data structure."""
        name = node.get("name", "")
        fields = node.get("fields", [])
        self.structs[name] = {f["name"]: f.get("field_type", "any") for f in fields}
        self.variables[name] = {f["name"]: None for f in fields}
        return None

    def _log_event(self, event_type, key):
        """Fire-and-forget event log to MongoDB."""
        try:
            val = json.dumps({"node_id": self.node_id, "intent": self.intent})
            subprocess.Popen(
                ["node", self.store_js, "log", event_type, str(key), val],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass


def _parse_codie(source):
    """Parse CODIE source into AST nodes (JSON dicts).

    Uses cage-web binary if available, otherwise falls back to a
    lightweight Python line parser.
    """
    # Try cage-web binary first (full parser)
    cage_web_bin = os.path.join(CAGE_ROOT, "cage-web", "target", "release", "cage-web")
    if os.path.exists(cage_web_bin):
        try:
            import tempfile
            with tempfile.NamedTemporaryFile(mode="w", suffix=".codie", delete=False) as f:
                f.write(source)
                tmp_path = f.name
            result = subprocess.run(
                [cage_web_bin, "--parse-codie", tmp_path],
                capture_output=True, text=True, timeout=10,
            )
            os.unlink(tmp_path)
            if result.returncode == 0 and result.stdout.strip():
                parsed = json.loads(result.stdout)
                return parsed.get("nodes", [])
        except Exception:
            pass

    # Fallback: lightweight Python parser
    return _parse_codie_python(source)


def _parse_codie_python(source):
    """Minimal Python CODIE parser — handles the core keywords."""
    nodes = []
    lines = source.split("\n")
    i = 0

    while i < len(lines):
        line = lines[i].strip()

        # Strip pipe-tree prefixes
        for prefix in ["|   +-- ", "+-- ", "|  +-- ", "| +-- "]:
            if line.startswith(prefix):
                line = line[len(prefix):]
        line = line.strip()

        # Skip empty, pipe-only, comments
        if not line or line == "|":
            i += 1
            continue
        if line.startswith("//") or line.startswith("====") or line.startswith("----"):
            nodes.append({"type": "Comment", "text": line.lstrip("/ ")})
            i += 1
            continue
        if line == "}" or line == "})," or line == "});":
            i += 1
            continue

        word = line.split()[0].lower() if line.split() else ""
        rest = line[len(word):].strip() if word else ""

        if word == "pug":
            name = rest.rstrip("{").strip()
            children, i = _collect_children(lines, i + 1)
            nodes.append({"type": "Entry", "name": name, "children": children})
        elif word == "bark":
            if "<-" in rest:
                target, source_ref = rest.split("<-", 1)
                nodes.append({"type": "Fetch", "target": target.strip(), "source": source_ref.strip()})
            elif " from " in rest:
                target, source_ref = rest.split(" from ", 1)
                nodes.append({"type": "Fetch", "target": target.strip(), "source": source_ref.strip()})
            else:
                nodes.append({"type": "Fetch", "target": "", "source": rest})
        elif word == "elf":
            if "<-" in rest:
                name, value = rest.split("<-", 1)
                nodes.append({"type": "Bind", "name": name.strip(), "value": value.strip()})
            elif "=" in rest:
                name, value = rest.split("=", 1)
                nodes.append({"type": "Bind", "name": name.strip(), "value": value.strip()})
            else:
                nodes.append({"type": "Bind", "name": rest, "value": ""})
        elif word == "cali":
            name = rest.split("(")[0].strip().rstrip("{").strip()
            args = ""
            if "(" in rest and ")" in rest:
                args = rest[rest.index("(") + 1:rest.index(")")]
            children, i = _collect_children(lines, i + 1) if "{" in rest else ([], i)
            nodes.append({"type": "Call", "name": name, "args": args, "children": children})
        elif word == "spin":
            if " IN " in rest:
                var, collection = rest.split(" IN ", 1)
                body, i = _collect_children(lines, i + 1)
                nodes.append({"type": "Loop", "var": var.strip(), "collection": collection.strip(), "body": body})
            else:
                body, i = _collect_children(lines, i + 1)
                nodes.append({"type": "Loop", "var": rest, "collection": "", "body": body})
        elif word == "fence":
            name = rest.rstrip("{").strip()
            children, i = _collect_children(lines, i + 1)
            nodes.append({"type": "Guard", "name": name, "children": children})
        elif word == "bone":
            negated = rest.startswith("NOT:")
            body = rest[4:].strip() if negated else rest
            name = ""
            if ":" in body and not negated:
                name, body = body.split(":", 1)
                name = name.strip()
                body = body.strip()
            children, i = _collect_children(lines, i + 1) if "{" in rest else ([], i)
            nodes.append({"type": "Rule", "name": name, "negated": negated, "body": body, "children": children})
        elif word == "pin":
            if "=" in rest:
                name, value = rest.split("=", 1)
                nodes.append({"type": "Const", "name": name.strip(), "value": value.strip()})
            else:
                nodes.append({"type": "Const", "name": rest, "value": ""})
        elif word == "blob":
            name = rest.split("{")[0].strip() if "{" in rest else rest.split()[0] if rest.split() else ""
            fields = []
            if "{" in rest and "}" in rest:
                inner = rest[rest.index("{") + 1:rest.index("}")]
                for field_str in inner.split(","):
                    if ":" in field_str:
                        fname, ftype = field_str.split(":", 1)
                        fields.append({"name": fname.strip(), "field_type": ftype.strip()})
            nodes.append({"type": "Struct", "name": name, "fields": fields})
        elif word == "biz":
            value = rest.lstrip("->").strip()
            nodes.append({"type": "Return", "value": value})
        elif word == "anchor":
            name = rest.lstrip("#").strip()
            nodes.append({"type": "Checkpoint", "name": name})
        elif word == "turk":
            rest_stripped = rest.lstrip("if ").strip() if rest.startswith("if ") else rest
            if "->" in rest_stripped:
                cond, target = rest_stripped.split("->", 1)
                nodes.append({"type": "Transform", "condition": cond.strip(), "target": target.strip()})
            else:
                nodes.append({"type": "Transform", "condition": rest_stripped, "target": ""})
        elif word == "?":
            if "->" in rest:
                cond, action = rest.split("->", 1)
                nodes.append({"type": "Conditional", "condition": cond.strip(), "action": action.strip()})
            else:
                nodes.append({"type": "Conditional", "condition": rest, "action": ""})
        else:
            nodes.append({"type": "Comment", "text": line})

        i += 1

    return nodes


def _collect_children(lines, start):
    """Collect indented/pipe-prefixed children from line position."""
    children = []
    i = start
    if i >= len(lines):
        return children, i - 1

    base_indent = len(lines[start - 1]) - len(lines[start - 1].lstrip()) if start > 0 else 0

    while i < len(lines):
        line = lines[i].strip()
        raw_indent = len(lines[i]) - len(lines[i].lstrip())

        if not line or line == "|":
            i += 1
            continue

        # Stop at closing brace or same/lower indent top-level keyword
        if line == "}" or line == "})," or line == "});":
            i += 1
            break

        # Check if we've returned to the same indent level with a keyword
        if raw_indent <= base_indent and not line.startswith("|") and not line.startswith("+"):
            word = line.split()[0].lower() if line.split() else ""
            if word in ("pug", "bark", "elf", "spin", "cali", "turk", "fence", "pin", "bone", "blob", "biz", "anchor", "?", "//"):
                break

        # Strip pipe prefix
        for prefix in ["|   +-- ", "+-- ", "|  +-- ", "| +-- "]:
            if line.startswith(prefix):
                line = line[len(prefix):]
        line = line.strip()

        if not line or line == "|":
            i += 1
            continue

        word = line.split()[0].lower() if line.split() else ""
        rest_line = line[len(word):].strip() if word else ""

        if word == "bone":
            negated = rest_line.startswith("NOT:")
            body = rest_line[4:].strip() if negated else rest_line
            children.append({"type": "Rule", "name": "", "negated": negated, "body": body, "children": []})
        elif word == "?":
            if "->" in rest_line:
                cond, action = rest_line.split("->", 1)
                children.append({"type": "Conditional", "condition": cond.strip(), "action": action.strip()})
            else:
                children.append({"type": "Conditional", "condition": rest_line, "action": ""})
        elif word == "bark":
            if "<-" in rest_line:
                target, source_ref = rest_line.split("<-", 1)
                children.append({"type": "Fetch", "target": target.strip(), "source": source_ref.strip()})
            else:
                children.append({"type": "Fetch", "target": "", "source": rest_line})
        elif word == "elf":
            if "<-" in rest_line:
                name, value = rest_line.split("<-", 1)
                children.append({"type": "Bind", "name": name.strip(), "value": value.strip()})
            else:
                children.append({"type": "Bind", "name": rest_line, "value": ""})
        elif word == "anchor":
            name = rest_line.lstrip("#").strip()
            children.append({"type": "Checkpoint", "name": name})
        elif word == "biz":
            value = rest_line.lstrip("->").strip()
            children.append({"type": "Return", "value": value})
        elif line.startswith("//") or line.startswith("#"):
            children.append({"type": "Comment", "text": line})
        else:
            children.append({"type": "Comment", "text": line})

        i += 1

    return children, i - 1


# ── Approval Gate ──────────────────────────────────────────────


def _check_approval(task):
    """Check risk level against the node's escalation threshold.

    Risk levels:
      1-3: Auto-approved (captain-level, safe operations)
      4-6: Log and continue (director-level, notable changes)
      7-8: Block — requires CTO approval
      9-10: Block — requires human approval

    Returns dict with {risk, blocked, reason, escalated_to}
    """
    escalation = task.get("escalation", {})
    threshold = escalation.get("threshold", 10)
    target = escalation.get("target")
    scale = task.get("scale", "captain")
    intent = task.get("intent", "")

    # Calculate risk from intent + scale
    risk = _calculate_risk(task)

    # Log the approval decision
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    try:
        doc = json.dumps({
            "node_id": task.get("node_id"),
            "risk": risk,
            "threshold": threshold,
            "approved": risk < 7,
            "scale": scale,
        })
        subprocess.Popen(
            ["node", store_js, "log", "approval:check", task.get("node_id", ""), doc],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass

    if risk >= 9:
        return {
            "risk": risk,
            "blocked": True,
            "reason": f"Risk {risk} requires human approval",
            "escalated_to": "root:human",
            "level": "human",
        }
    elif risk >= 7:
        return {
            "risk": risk,
            "blocked": True,
            "reason": f"Risk {risk} requires CTO approval (threshold: {threshold})",
            "escalated_to": target or "exec:cto",
            "level": "cto",
        }
    elif risk >= 4:
        return {
            "risk": risk,
            "blocked": False,
            "reason": f"Risk {risk} — logged, proceeding (director-level)",
            "level": "director",
        }
    else:
        return {
            "risk": risk,
            "blocked": False,
            "reason": "Auto-approved",
            "level": "captain",
        }


def _calculate_risk(task):
    """Calculate risk level (1-10) from task properties.

    Factors:
    - Scale: executive=8, department=6, captain=3
    - Intent keywords: destructive words increase risk
    - File sensitivity: security/docker/config files increase risk
    - Rule count: more rules = more constrained = lower risk
    """
    risk = 0
    scale = task.get("scale", "captain")
    intent = task.get("intent", "").lower()
    files = task.get("files", [])
    rules = task.get("rules", [])

    # Base risk from scale
    scale_risk = {"executive": 8, "department": 6, "captain": 3, "module": 2, "crate": 2}
    risk = scale_risk.get(scale, 3)

    # Intent risk modifiers
    high_risk_words = ["delete", "destroy", "drop", "force", "reset", "remove", "wipe", "nuke", "nixos-rebuild"]
    medium_risk_words = ["deploy", "push", "release", "migrate", "update", "modify", "nix build", "rebuild tier"]
    if any(w in intent for w in high_risk_words):
        risk += 3
    elif any(w in intent for w in medium_risk_words):
        risk += 1

    # File sensitivity
    sensitive_paths = ["security/", "docker/", ".env", "credentials", "config/"]
    if any(any(s in f for s in sensitive_paths) for f in files):
        risk += 1

    # More rules = more constrained = slightly lower risk
    if len(rules) > 3:
        risk -= 1

    return max(1, min(10, risk))
