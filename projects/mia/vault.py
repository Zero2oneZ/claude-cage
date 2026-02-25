"""vault.py — SOPS + age encryption for .env secrets.

Parses .env files (including multi-line RSA keys), converts to YAML,
encrypts via SOPS with age, and spawns per-project vaults from subsets.
"""

import json
import os
import shutil
import subprocess
import tempfile
import yaml

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
SECRETS_DIR = os.path.join(CAGE_ROOT, ".secrets")
SOPS_AGE_KEY_FILE = os.path.expanduser("~/.config/sops/age/keys.txt")

# ── Predefined key groups for spawn shortcuts ──────────────────

PROJECT_KEY_GROUPS = {
    "livepeer": [
        "LIVEPEER_API_KEY", "LIVEPEER_PRIVATEKEY", "LIVEPEER_SIGNING_KEY",
        "LIVEPEER_PRIVATE_SIGNING_KEY", "LIVEPEER_PUBLIC_SIGNING_KEY",
    ],
    "twilio": [
        "TWILIO_ACCOUNT_SID", "TWILIO_AUTH_TOKEN",
        "TWILIO_DEV_NUMBER", "TWILIO_RECOVERY",
    ],
    "porkbun": ["PORKBUN_API_KEY", "PORKBUN_SECRET"],
    "pinata": ["PINATA_API_KEY", "PINATA_API_SECRET", "PINATA_JWT"],
    "nounproject": ["THE_NOUN_PROJECT_API", "THE_NOUN_PROJECT_SECRET"],
    "io-training": ["IO_TRAINING_API"],
    "coolify": ["COOLIFY_PUBLIC_SSH_KEY", "COOLIFY_PRIVATE_SSH_KEY"],
}


# ── .env Parsing ──────────────────────────────────────────────


def parse_env_file(path):
    """Parse a .env file, handling multi-line quoted values (RSA keys).

    Returns:
        dict: {KEY: value} with quotes stripped and multi-line values joined.
    """
    env = {}
    with open(path, "r") as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i].rstrip("\n")

        # Skip blanks and comments
        if not line.strip() or line.strip().startswith("#"):
            i += 1
            continue

        # Must have = to be a key=value line
        if "=" not in line:
            i += 1
            continue

        # Handle KEY-"value" (malformed, seen in COOLIFY_PRIVATE_SSH_KEY-)
        # First check if this is a KEY-"value" line (dash instead of equals)
        import re
        dash_match = re.match(r'^([A-Z_]+)-(".*"?)$', line) or re.match(r"^([A-Z_]+)-('.*'?)$", line)
        if dash_match:
            key = dash_match.group(1).strip()
            val = dash_match.group(2).strip()
        else:
            key_part, _, val_part = line.partition("=")
            key = key_part.strip()
            val = val_part.strip()

        # Check if value starts with a quote
        if val.startswith('"') or val.startswith("'"):
            quote_char = val[0]
            other_quote = "'" if quote_char == '"' else '"'
            # Strip trailing duplicate quotes (e.g., IO_TRAINING_API="...TsA"")
            while len(val) > 2 and val[-1] == val[-2] and val[-1] in '"\'':
                val = val[:-1]
            # Check if it closes on same line (either matching quote or mismatched)
            if val.endswith(quote_char) and len(val) > 1:
                env[key] = val[1:-1]
            elif val.endswith(other_quote) and len(val) > 1:
                # Mismatched quotes (e.g., PINATA_JWT='...") — treat as single-line
                env[key] = val[1:-1]
            else:
                # Multi-line value — accumulate until closing quote
                collected = [val[1:]]  # strip opening quote
                i += 1
                while i < len(lines):
                    mline = lines[i].rstrip("\n")
                    if mline.endswith(quote_char) or mline.endswith(other_quote):
                        collected.append(mline[:-1])  # strip closing quote
                        break
                    collected.append(mline)
                    i += 1
                env[key] = "\n".join(collected)
        else:
            env[key] = val

        i += 1

    return env


# ── YAML Conversion ──────────────────────────────────────────


def env_to_yaml(env_dict, metadata=None):
    """Convert env dict to SOPS-friendly YAML structure.

    Structure:
        _metadata: (plaintext — not encrypted by SOPS regex)
            created_at, source, key_count, keys
        secrets:   (encrypted by SOPS --encrypted-regex '^secrets$')
            KEY1: value1
            KEY2: value2

    Returns:
        str: YAML string
    """
    from datetime import datetime, timezone

    doc = {
        "_metadata": metadata or {
            "created_at": datetime.now(timezone.utc).isoformat(),
            "source": ".env.gently.global",
            "key_count": len(env_dict),
            "keys": sorted(env_dict.keys()),
        },
        "secrets": dict(sorted(env_dict.items())),
    }
    return yaml.dump(doc, default_flow_style=False, allow_unicode=True, width=120)


# ── Age Key Management ───────────────────────────────────────


def get_age_pubkey():
    """Read age public key from the SOPS key file.

    Returns:
        str: age public key (age1...) or None
    """
    if not os.path.exists(SOPS_AGE_KEY_FILE):
        return None
    with open(SOPS_AGE_KEY_FILE, "r") as f:
        for line in f:
            line = line.strip()
            if line.startswith("# public key:"):
                return line.split(":", 1)[1].strip()
    return None


def init_age_key():
    """Generate age keypair at SOPS default location if not present.

    Returns:
        tuple: (pubkey, key_file_path) or raises RuntimeError
    """
    if os.path.exists(SOPS_AGE_KEY_FILE):
        pubkey = get_age_pubkey()
        if pubkey:
            return pubkey, SOPS_AGE_KEY_FILE

    # Create directory
    os.makedirs(os.path.dirname(SOPS_AGE_KEY_FILE), exist_ok=True)

    # Generate keypair
    result = subprocess.run(
        ["age-keygen"],
        capture_output=True, text=True, timeout=10,
    )
    if result.returncode != 0:
        raise RuntimeError(f"age-keygen failed: {result.stderr}")

    # Write key file
    with open(SOPS_AGE_KEY_FILE, "w") as f:
        f.write(result.stdout)
    os.chmod(SOPS_AGE_KEY_FILE, 0o600)

    # Parse pubkey from output
    pubkey = None
    for line in result.stderr.splitlines() + result.stdout.splitlines():
        if line.startswith("# public key:"):
            pubkey = line.split(":", 1)[1].strip()
        elif line.startswith("Public key:"):
            pubkey = line.split(":", 1)[1].strip()
    if not pubkey:
        # Try reading back from the file
        pubkey = get_age_pubkey()

    if not pubkey:
        raise RuntimeError("Could not extract public key from age-keygen output")

    return pubkey, SOPS_AGE_KEY_FILE


def init_sops_config(pubkey):
    """Generate .secrets/.sops.yaml targeting vault.*.enc.yaml files.

    Args:
        pubkey: age public key string
    """
    sops_config = {
        "creation_rules": [
            {
                "path_regex": r"vault\..*\.enc\.yaml$",
                "age": pubkey,
                "encrypted_regex": "^secrets$",
            }
        ]
    }
    config_path = os.path.join(SECRETS_DIR, ".sops.yaml")
    os.makedirs(SECRETS_DIR, exist_ok=True)
    with open(config_path, "w") as f:
        yaml.dump(sops_config, f, default_flow_style=False)
    return config_path


# ── SOPS Encrypt / Decrypt ───────────────────────────────────


def sops_encrypt_file(input_path, output_path, pubkey):
    """Encrypt a YAML file with SOPS + age.

    Only the 'secrets' key is encrypted (--encrypted-regex '^secrets$').
    Metadata remains plaintext for inspection.

    Args:
        input_path: path to plaintext YAML
        output_path: path to write encrypted YAML
        pubkey: age public key

    Returns:
        str: output path on success

    Raises:
        RuntimeError: on SOPS failure
    """
    env = os.environ.copy()
    env["SOPS_AGE_KEY_FILE"] = SOPS_AGE_KEY_FILE

    result = subprocess.run(
        [
            "sops", "encrypt",
            "--age", pubkey,
            "--encrypted-regex", "^secrets$",
            "--input-type", "yaml",
            "--output-type", "yaml",
            "--output", output_path,
            input_path,
        ],
        capture_output=True, text=True, timeout=30, env=env,
    )
    if result.returncode != 0:
        raise RuntimeError(f"sops encrypt failed: {result.stderr}")
    return output_path


def sops_decrypt_file(path):
    """Decrypt a SOPS-encrypted YAML file.

    Returns the decrypted content as a dict. NEVER writes to disk.

    Args:
        path: path to encrypted YAML

    Returns:
        dict: decrypted YAML content
    """
    env = os.environ.copy()
    env["SOPS_AGE_KEY_FILE"] = SOPS_AGE_KEY_FILE

    result = subprocess.run(
        ["sops", "decrypt", "--input-type", "yaml", "--output-type", "yaml", path],
        capture_output=True, text=True, timeout=30, env=env,
    )
    if result.returncode != 0:
        raise RuntimeError(f"sops decrypt failed: {result.stderr}")
    return yaml.safe_load(result.stdout)


# ── Encrypt Global Vault ─────────────────────────────────────


def encrypt_global(env_path=None, output_path=None):
    """Encrypt .env.gently.global to vault.global.enc.yaml.

    Uses a temp file for the plaintext YAML intermediate, cleaned up in finally.

    Returns:
        str: path to encrypted vault
    """
    env_path = env_path or os.path.join(SECRETS_DIR, ".env.gently.global")
    output_path = output_path or os.path.join(SECRETS_DIR, "vault.global.enc.yaml")

    pubkey = get_age_pubkey()
    if not pubkey:
        raise RuntimeError("No age key found. Run 'mia init' first.")

    env_dict = parse_env_file(env_path)
    yaml_content = env_to_yaml(env_dict)

    # Write plaintext to temp, encrypt, clean up
    tmp_fd, tmp_path = tempfile.mkstemp(suffix=".yaml", prefix="mia_")
    try:
        with os.fdopen(tmp_fd, "w") as f:
            f.write(yaml_content)
        sops_encrypt_file(tmp_path, output_path, pubkey)
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)

    return output_path


# ── Decrypt (memory only) ────────────────────────────────────


def decrypt_global(vault_path=None):
    """Decrypt global vault, return secrets dict. Never writes to disk.

    Returns:
        dict: {KEY: value, ...}
    """
    vault_path = vault_path or os.path.join(SECRETS_DIR, "vault.global.enc.yaml")
    doc = sops_decrypt_file(vault_path)
    return doc.get("secrets", {})


# ── Spawn Project Vault ──────────────────────────────────────


def spawn_project_vault(project, keys=None, os_target=None, global_vault=None):
    """Create a per-project encrypted vault with a subset of keys.

    Decrypts global vault → filters to requested keys → re-encrypts.

    Args:
        project: project name (e.g., "twilio", "livepeer")
        keys: list of key names, or None to use PROJECT_KEY_GROUPS
        os_target: optional OS tag for metadata (e.g., "linux", "darwin")
        global_vault: path to global vault (default: .secrets/vault.global.enc.yaml)

    Returns:
        str: path to project vault
    """
    global_vault = global_vault or os.path.join(SECRETS_DIR, "vault.global.enc.yaml")
    pubkey = get_age_pubkey()
    if not pubkey:
        raise RuntimeError("No age key found. Run 'mia init' first.")

    # Resolve keys
    if keys is None:
        keys = PROJECT_KEY_GROUPS.get(project)
        if keys is None:
            raise ValueError(
                f"Unknown project '{project}'. Known: {', '.join(PROJECT_KEY_GROUPS.keys())}. "
                f"Or pass --keys explicitly."
            )

    # Decrypt global
    all_secrets = decrypt_global(global_vault)

    # Filter
    project_secrets = {}
    missing = []
    for k in keys:
        if k in all_secrets:
            project_secrets[k] = all_secrets[k]
        else:
            missing.append(k)

    if missing:
        print(f"  Warning: keys not found in global vault: {', '.join(missing)}")

    if not project_secrets:
        raise ValueError(f"No matching keys found for project '{project}'")

    # Build metadata
    from datetime import datetime, timezone
    metadata = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "project": project,
        "source": "vault.global.enc.yaml",
        "key_count": len(project_secrets),
        "keys": sorted(project_secrets.keys()),
    }
    if os_target:
        metadata["os"] = os_target

    yaml_content = env_to_yaml(project_secrets, metadata=metadata)

    # Output path
    project_dir = os.path.join(SECRETS_DIR, project)
    os.makedirs(project_dir, exist_ok=True)
    output_path = os.path.join(project_dir, f"vault.{project}.enc.yaml")

    tmp_fd, tmp_path = tempfile.mkstemp(suffix=".yaml", prefix=f"mia_{project}_")
    try:
        with os.fdopen(tmp_fd, "w") as f:
            f.write(yaml_content)
        sops_encrypt_file(tmp_path, output_path, pubkey)
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)

    return output_path


# ── Status Checks ────────────────────────────────────────────


def check_tools():
    """Check if age and sops are installed.

    Returns:
        dict: {age: bool, sops: bool, age_key: bool, sops_config: bool}
    """
    status = {}

    # age
    status["age"] = shutil.which("age") is not None

    # sops
    status["sops"] = shutil.which("sops") is not None

    # age key
    status["age_key"] = os.path.exists(SOPS_AGE_KEY_FILE)
    status["age_pubkey"] = get_age_pubkey() if status["age_key"] else None

    # .sops.yaml
    status["sops_config"] = os.path.exists(os.path.join(SECRETS_DIR, ".sops.yaml"))

    # global vault
    status["global_vault"] = os.path.exists(os.path.join(SECRETS_DIR, "vault.global.enc.yaml"))

    # env file
    status["env_file"] = os.path.exists(os.path.join(SECRETS_DIR, ".env.gently.global"))

    return status
