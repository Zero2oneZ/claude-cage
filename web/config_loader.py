"""config_loader.py — Configuration loading and validation.

Port of lib/config.sh. Reads config/default.yaml then ~/.config/claude-cage/config.yaml
overrides using the same flat key:value YAML parser.
"""

import os
import re

DEFAULTS = {
    "mode": "cli",
    "network": "filtered",
    "cpus": "2",
    "memory": "4g",
    "desktop_port": "6080",
    "vnc_port": "5900",
    "image_cli": "claude-cage-cli:latest",
    "image_desktop": "claude-cage-desktop:latest",
    "log_level": "info",
    "session_dir": "",
    "persist": "true",
    "allowed_hosts": "api.anthropic.com,cdn.anthropic.com",
    "dns": "1.1.1.1",
    "read_only_root": "true",
    "seccomp_profile": "default",
    "max_sessions": "5",
}

_LINE_RE = re.compile(r"^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*(.*)")


def _parse_yaml_flat(filepath):
    """Minimal flat key:value YAML parser (no nested structures).

    Matches the bash _parse_yaml() in lib/config.sh exactly.
    """
    result = {}
    with open(filepath) as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.strip().startswith("#"):
                continue
            m = _LINE_RE.match(line)
            if m:
                key = m.group(1)
                val = m.group(2).strip()
                # Strip surrounding quotes
                if len(val) >= 2 and val[0] == val[-1] and val[0] in ('"', "'"):
                    val = val[1:-1]
                result[key] = val
    return result


def load_config(cage_root):
    """Load merged configuration: defaults -> default.yaml -> user overrides.

    Returns a dict with all config keys.
    """
    cfg = dict(DEFAULTS)

    # Project default config
    default_path = os.path.join(cage_root, "config", "default.yaml")
    if os.path.isfile(default_path):
        cfg.update(_parse_yaml_flat(default_path))

    # User overrides
    config_dir = os.environ.get(
        "XDG_CONFIG_HOME", os.path.expanduser("~/.config")
    )
    user_path = os.path.join(config_dir, "claude-cage", "config.yaml")
    if os.path.isfile(user_path):
        cfg.update(_parse_yaml_flat(user_path))

    # Derived values
    if not cfg.get("session_dir"):
        data_dir = os.environ.get(
            "XDG_DATA_HOME", os.path.expanduser("~/.local/share")
        )
        cfg["session_dir"] = os.path.join(data_dir, "claude-cage", "sessions")

    return cfg


def get(cfg, key, default=None):
    """Get a config value with optional fallback."""
    return cfg.get(key, default) or default


def validate(cfg):
    """Validate configuration values. Returns (ok: bool, errors: list[str])."""
    errors = []
    if cfg.get("mode") not in ("cli", "desktop"):
        errors.append("'mode' must be 'cli' or 'desktop'")
    if cfg.get("network") not in ("none", "host", "filtered"):
        errors.append("'network' must be 'none', 'host', or 'filtered'")
    if not re.match(r"^[0-9]+(\.[0-9]+)?$", cfg.get("cpus", "")):
        errors.append("'cpus' must be a number")
    if not re.match(r"^[0-9]+[gmkGMK]?$", cfg.get("memory", "")):
        errors.append("'memory' must be a size (e.g. 4g, 512m)")
    return len(errors) == 0, errors


def to_display(cfg):
    """Return config dict with sensitive values masked."""
    masked = {}
    for k, v in sorted(cfg.items()):
        if "key" in k.lower() or "secret" in k.lower():
            masked[k] = "********"
        else:
            masked[k] = v
    return masked
