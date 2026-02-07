"""workspace_manager.py — Project/workspace CRUD.

Manages a workspace index at ~/.config/claude-cage/workspaces.json.
Workspaces are directories on the host that get bind-mounted into containers.
"""

import json
import os
from datetime import datetime, timezone

WORKSPACES_FILE = os.path.expanduser("~/.config/claude-cage/workspaces.json")
DEFAULT_PROJECT_ROOT = os.path.expanduser("~/projects")


def _load():
    """Load workspaces index from disk."""
    if os.path.isfile(WORKSPACES_FILE):
        with open(WORKSPACES_FILE) as f:
            return json.load(f)
    return []


def _save(workspaces):
    """Persist workspaces index to disk."""
    os.makedirs(os.path.dirname(WORKSPACES_FILE), exist_ok=True)
    with open(WORKSPACES_FILE, "w") as f:
        json.dump(workspaces, f, indent=2)


def list_workspaces():
    """Return all registered workspaces."""
    workspaces = _load()
    # Annotate with existence check
    for w in workspaces:
        w["exists"] = os.path.isdir(w.get("path", ""))
    return workspaces


def add_workspace(path):
    """Register an existing directory as a workspace.

    Returns (success, workspace_dict_or_error).
    """
    path = os.path.abspath(os.path.expanduser(path))
    if not os.path.isdir(path):
        return False, f"Directory does not exist: {path}"

    workspaces = _load()
    # Check for duplicate
    for w in workspaces:
        if w.get("path") == path:
            return False, f"Already registered: {path}"

    now = datetime.now(timezone.utc).isoformat()
    entry = {
        "path": path,
        "name": os.path.basename(path),
        "created": now,
        "last_used": now,
    }
    workspaces.append(entry)
    _save(workspaces)
    return True, entry


def create_workspace(name, parent=None):
    """Create a new project directory and register it.

    Returns (success, workspace_dict_or_error).
    """
    if not name:
        return False, "Name is required"

    # Sanitize name
    safe_name = "".join(c for c in name if c.isalnum() or c in "-_.")
    if not safe_name:
        return False, "Invalid project name"

    parent = parent or DEFAULT_PROJECT_ROOT
    parent = os.path.abspath(os.path.expanduser(parent))

    project_path = os.path.join(parent, safe_name)
    if os.path.exists(project_path):
        return False, f"Already exists: {project_path}"

    os.makedirs(project_path, exist_ok=True)
    return add_workspace(project_path)


def remove_workspace(path):
    """Unregister a workspace (does NOT delete the directory).

    Returns (success, message).
    """
    path = os.path.abspath(os.path.expanduser(path))
    workspaces = _load()
    before = len(workspaces)
    workspaces = [w for w in workspaces if w.get("path") != path]
    if len(workspaces) == before:
        return False, f"Not found: {path}"
    _save(workspaces)
    return True, f"Removed: {path}"


def touch_workspace(path):
    """Update last_used timestamp for a workspace."""
    path = os.path.abspath(os.path.expanduser(path))
    workspaces = _load()
    for w in workspaces:
        if w.get("path") == path:
            w["last_used"] = datetime.now(timezone.utc).isoformat()
            break
    _save(workspaces)


def browse_directory(path=None):
    """List subdirectories of a path (dirs only, restricted to $HOME).

    Returns list of dicts: [{name, path, type}, ...]
    """
    home = os.path.expanduser("~")
    if path is None:
        path = home
    path = os.path.abspath(os.path.expanduser(path))

    # Security: restrict to home directory tree
    if not path.startswith(home):
        return []

    if not os.path.isdir(path):
        return []

    entries = []
    try:
        for name in sorted(os.listdir(path)):
            full = os.path.join(path, name)
            if os.path.isdir(full) and not name.startswith("."):
                entries.append({
                    "name": name,
                    "path": full,
                    "type": "dir",
                })
    except PermissionError:
        pass

    return entries
