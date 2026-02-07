"""session_manager.py — Session lifecycle and metadata management.

Port of lib/session.sh. Reads/writes the same metadata format at
~/.local/share/claude-cage/sessions/<name>/metadata so the CLI and
web dashboard coexist seamlessly.
"""

import os
import random
import subprocess
from datetime import datetime, timezone

ADJECTIVES = [
    "swift", "calm", "bold", "keen", "warm", "cool", "bright", "deep",
    "fair", "glad", "pure", "wise", "vast", "safe", "lean", "clear",
]
NOUNS = [
    "fox", "owl", "elk", "ray", "bay", "oak", "gem", "arc",
    "key", "pen", "dot", "fin", "orb", "cap", "rod", "hub",
]


def _sessions_dir(cfg):
    """Return (and ensure exists) the sessions metadata directory."""
    d = cfg.get("session_dir") or os.path.expanduser(
        "~/.local/share/claude-cage/sessions"
    )
    os.makedirs(d, exist_ok=True)
    return d


def generate_name():
    """Generate a human-friendly session name: adjective-noun-XXXX."""
    adj = random.choice(ADJECTIVES)
    noun = random.choice(NOUNS)
    suffix = f"{random.randint(0, 0xFFFF):04x}"
    return f"{adj}-{noun}-{suffix}"


def create(cfg, name, mode):
    """Create session metadata. Returns the metadata dict."""
    d = os.path.join(_sessions_dir(cfg), name)
    os.makedirs(d, exist_ok=True)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    meta = {
        "name": name,
        "mode": mode,
        "status": "running",
        "created": now,
        "container": f"cage-{name}",
    }
    _write_metadata(d, meta)
    return meta


def _write_metadata(directory, meta):
    path = os.path.join(directory, "metadata")
    with open(path, "w") as f:
        for k, v in meta.items():
            f.write(f"{k}={v}\n")


def _read_metadata(directory):
    path = os.path.join(directory, "metadata")
    meta = {}
    if os.path.isfile(path):
        with open(path) as f:
            for line in f:
                line = line.strip()
                if "=" in line:
                    k, v = line.split("=", 1)
                    meta[k] = v
    return meta


def set_status(cfg, name, status):
    """Update the status field in session metadata."""
    d = os.path.join(_sessions_dir(cfg), name)
    meta_path = os.path.join(d, "metadata")
    if not os.path.isfile(meta_path):
        return
    meta = _read_metadata(d)
    meta["status"] = status
    _write_metadata(d, meta)


def get_status(cfg, name):
    """Get session status from metadata, fall back to Docker inspect."""
    d = os.path.join(_sessions_dir(cfg), name)
    meta = _read_metadata(d)
    if meta.get("status"):
        return meta["status"]
    # Fall back to docker
    try:
        result = subprocess.run(
            ["docker", "inspect", "-f", "{{.State.Status}}", f"cage-{name}"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return "unknown"


def list_all(cfg):
    """List all sessions from metadata + orphan containers.

    Returns list of dicts: [{name, mode, status, created}, ...]
    """
    sessions_dir = _sessions_dir(cfg)
    sessions = []
    seen_names = set()

    # Sessions from metadata
    if os.path.isdir(sessions_dir):
        for entry in os.listdir(sessions_dir):
            d = os.path.join(sessions_dir, entry)
            if not os.path.isdir(d):
                continue
            meta = _read_metadata(d)
            if not meta.get("name"):
                continue
            name = meta["name"]
            seen_names.add(name)

            # Reconcile with Docker
            docker_status = _docker_container_status(name)
            status = docker_status or "removed"
            set_status(cfg, name, status)

            sessions.append({
                "name": name,
                "mode": meta.get("mode", "?"),
                "status": status,
                "created": meta.get("created", ""),
            })

    # Orphan containers (running but no metadata)
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", "label=managed-by=claude-cage",
             "--format", "{{.Names}}"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            for line in result.stdout.strip().splitlines():
                cname = line.strip()
                if cname.startswith("cage-"):
                    sname = cname[5:]
                else:
                    sname = cname
                if sname not in seen_names:
                    mode = _docker_label(cname, "cage.mode") or "?"
                    sessions.append({
                        "name": sname,
                        "mode": mode,
                        "status": "running",
                        "created": "(orphan)",
                    })
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    return sessions


def remove(cfg, name):
    """Delete session metadata directory."""
    d = os.path.join(_sessions_dir(cfg), name)
    if os.path.isdir(d):
        import shutil
        shutil.rmtree(d)


def get_metadata(cfg, name):
    """Read full metadata for a session."""
    d = os.path.join(_sessions_dir(cfg), name)
    return _read_metadata(d)


def _docker_container_status(name):
    """Query Docker for container status. Returns status string or None."""
    try:
        result = subprocess.run(
            ["docker", "inspect", "-f", "{{.State.Status}}", f"cage-{name}"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return None


def _docker_label(container_name, label):
    """Get a Docker label value from a container."""
    try:
        result = subprocess.run(
            ["docker", "inspect", "-f",
             f'{{{{index .Config.Labels "{label}"}}}}', container_name],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            val = result.stdout.strip()
            return val if val and val != "<no value>" else None
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return None
