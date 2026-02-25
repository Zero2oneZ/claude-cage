#!/usr/bin/env python3
"""mia.py — SOPS + age encrypted secrets manager with IPFS pinning.

CLI entry point. All secrets are encrypted with SOPS (age backend),
pinned to IPFS via Pinata, and registered in MongoDB.

Usage:
    python3 projects/mia/mia.py <command> [args]

Commands:
    init                          Generate age keypair + .sops.yaml
    encrypt                       Encrypt .env → vault.global.enc.yaml
    decrypt                       Decrypt vault to stdout (memory only)
    pin                           Pin encrypted vault to Pinata IPFS
    spawn <project> [--keys K1,K2] [--os linux]
                                  Create per-project vault
    list                          Show CID registry
    pull <cid>                    Retrieve encrypted file from IPFS
    status                        Health check (age, sops, pinata, mongo)
"""

import argparse
import json
import os
import sys

# Ensure project root is importable
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPT_DIR)

from vault import (
    SECRETS_DIR, PROJECT_KEY_GROUPS,
    check_tools, init_age_key, init_sops_config,
    encrypt_global, decrypt_global, get_age_pubkey,
    spawn_project_vault, parse_env_file,
)
from pinata import pin_file, test_auth, get_from_gateway
from registry import (
    register_pin, list_all, get_chain, is_available as mongo_available,
    format_table,
)


# ── Commands ─────────────────────────────────────────────────


def cmd_init(args):
    """Generate age keypair and .sops.yaml configuration."""
    print("mia init — setting up encryption")
    print()

    # 1. Age keypair
    try:
        pubkey, key_path = init_age_key()
        print(f"  age key:    {key_path}")
        print(f"  public key: {pubkey}")
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        print("  Install age: sudo apt install age", file=sys.stderr)
        return 1

    # 2. SOPS config
    try:
        config_path = init_sops_config(pubkey)
        print(f"  sops config: {config_path}")
    except Exception as e:
        print(f"  ERROR writing .sops.yaml: {e}", file=sys.stderr)
        return 1

    print()
    print("  Ready. Run 'mia encrypt' to encrypt your secrets.")
    return 0


def cmd_encrypt(args):
    """Encrypt .env.gently.global to vault.global.enc.yaml."""
    print("mia encrypt — encrypting global secrets")
    print()

    env_path = os.path.join(SECRETS_DIR, ".env.gently.global")
    if not os.path.exists(env_path):
        print(f"  ERROR: {env_path} not found", file=sys.stderr)
        return 1

    # Parse to show key count
    env_dict = parse_env_file(env_path)
    print(f"  source: {env_path}")
    print(f"  keys:   {len(env_dict)} ({', '.join(sorted(env_dict.keys())[:5])}...)")

    try:
        output = encrypt_global(env_path)
        print(f"  output: {output}")
        size = os.path.getsize(output)
        print(f"  size:   {size:,} bytes")
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    print()
    print("  Encrypted. Run 'mia pin' to pin to IPFS.")
    return 0


def cmd_decrypt(args):
    """Decrypt vault and print secrets to stdout. Never writes to disk."""
    vault_path = os.path.join(SECRETS_DIR, "vault.global.enc.yaml")

    # Allow specifying a project vault
    if args.project:
        vault_path = os.path.join(SECRETS_DIR, args.project, f"vault.{args.project}.enc.yaml")

    if not os.path.exists(vault_path):
        print(f"  ERROR: {vault_path} not found", file=sys.stderr)
        return 1

    try:
        secrets = decrypt_global(vault_path)
        for key in sorted(secrets.keys()):
            val = secrets[key]
            # Truncate long values for display
            if len(str(val)) > 80:
                display = str(val)[:77] + "..."
            else:
                display = val
            print(f"  {key}={display}")
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    return 0


def cmd_pin(args):
    """Pin encrypted vault to Pinata IPFS."""
    print("mia pin — pinning encrypted vault to IPFS")
    print()

    vault_path = os.path.join(SECRETS_DIR, "vault.global.enc.yaml")
    scope = "global"
    project = None

    if args.project:
        scope = args.project
        project = args.project
        vault_path = os.path.join(SECRETS_DIR, args.project, f"vault.{args.project}.enc.yaml")

    if not os.path.exists(vault_path):
        print(f"  ERROR: {vault_path} not found", file=sys.stderr)
        print(f"  Run 'mia encrypt' first.", file=sys.stderr)
        return 1

    print(f"  file:  {vault_path}")
    print(f"  scope: {scope}")

    # Read vault metadata for key list
    import yaml
    with open(vault_path, "r") as f:
        doc = yaml.safe_load(f)
    keys = doc.get("_metadata", {}).get("keys", [])

    # Pin to Pinata
    try:
        pin_name = f"mia-vault-{scope}"
        metadata = {"scope": scope, "type": "sops-encrypted-vault"}
        if project:
            metadata["project"] = project

        result = pin_file(vault_path, pin_name=pin_name, metadata=metadata)
        cid = result["IpfsHash"]
        pin_size = result.get("PinSize", "?")
        print(f"  CID:   {cid}")
        print(f"  size:  {pin_size} bytes")
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    # Register in MongoDB
    parent_cid = None
    if project:
        # Look up global vault CID as parent
        from registry import get_by_scope
        global_entry = get_by_scope("global")
        if global_entry:
            parent_cid = global_entry.get("cid")

    ok = register_pin(
        cid=cid,
        scope=scope,
        project=project,
        keys=keys,
        parent_cid=parent_cid,
    )
    if ok:
        print(f"  mongo: registered in mia_registry")
    else:
        print(f"  mongo: registration failed (MongoDB may be unavailable)")

    print()
    print(f"  Pinned. CID: {cid}")
    return 0


def cmd_spawn(args):
    """Create a per-project encrypted vault."""
    project = args.project
    print(f"mia spawn — creating vault for '{project}'")
    print()

    # Resolve keys
    keys = None
    if args.keys:
        keys = [k.strip() for k in args.keys.split(",")]
        print(f"  keys:    {', '.join(keys)}")
    elif project in PROJECT_KEY_GROUPS:
        keys = PROJECT_KEY_GROUPS[project]
        print(f"  group:   {project} ({len(keys)} keys)")
    else:
        print(f"  ERROR: Unknown project '{project}' and no --keys specified.", file=sys.stderr)
        print(f"  Known groups: {', '.join(sorted(PROJECT_KEY_GROUPS.keys()))}", file=sys.stderr)
        return 1

    if args.os:
        print(f"  os:      {args.os}")

    try:
        output = spawn_project_vault(
            project=project,
            keys=keys,
            os_target=args.os,
        )
        print(f"  output:  {output}")
        size = os.path.getsize(output)
        print(f"  size:    {size:,} bytes")
    except (RuntimeError, ValueError) as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    print()
    print(f"  Spawned. Run 'mia pin --project {project}' to pin to IPFS.")
    return 0


def cmd_list(args):
    """Show CID registry from MongoDB."""
    print("mia list — CID registry")
    print()

    if args.project:
        entries = get_chain(args.project)
        if entries:
            print(f"  Chain for '{args.project}':")
            print(format_table(entries))
        else:
            print(f"  No entries for '{args.project}'")
    else:
        entries = list_all()
        if entries:
            print(format_table(entries))
        else:
            print("  (empty — no pins registered yet)")

    return 0


def cmd_pull(args):
    """Retrieve encrypted file from IPFS gateway."""
    cid = args.cid
    print(f"mia pull — retrieving {cid}")
    print()

    try:
        data = get_from_gateway(cid)
        if args.output:
            with open(args.output, "wb") as f:
                f.write(data)
            print(f"  saved: {args.output} ({len(data):,} bytes)")
        else:
            # Print to stdout
            sys.stdout.buffer.write(data)
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    return 0


def cmd_status(args):
    """Health check for all mia dependencies."""
    print("mia status")
    print()

    status = check_tools()

    # age
    mark = "ok" if status["age"] else "MISSING — sudo apt install age"
    print(f"  age:         {mark}")

    # sops
    mark = "ok" if status["sops"] else "MISSING — see https://github.com/getsops/sops/releases"
    print(f"  sops:        {mark}")

    # age key
    if status["age_key"]:
        print(f"  age key:     {status['age_pubkey'][:20]}...")
    else:
        print(f"  age key:     NOT FOUND — run 'mia init'")

    # sops config
    mark = "ok" if status["sops_config"] else "NOT FOUND — run 'mia init'"
    print(f"  sops config: {mark}")

    # env file
    mark = "ok" if status["env_file"] else "NOT FOUND"
    print(f"  env file:    {mark}")

    # global vault
    mark = "ok" if status["global_vault"] else "NOT ENCRYPTED — run 'mia encrypt'"
    print(f"  global vault: {mark}")

    # Pinata
    print()
    auth = test_auth()
    if auth["authenticated"]:
        print(f"  pinata:      authenticated")
    else:
        print(f"  pinata:      {auth.get('error', 'not configured')}")

    # MongoDB
    if mongo_available():
        print(f"  mongodb:     connected")
        entries = list_all()
        print(f"  registry:    {len(entries)} CIDs registered")
    else:
        print(f"  mongodb:     not reachable")

    print()
    # Summary
    ok_count = sum([
        status["age"], status["sops"], status["age_key"],
        status["sops_config"], status["env_file"],
    ])
    total = 5
    if ok_count == total:
        print(f"  All {total} checks passed. Ready to encrypt.")
    else:
        print(f"  {ok_count}/{total} checks passed.")

    return 0


# ── Argument Parser ──────────────────────────────────────────


def build_parser():
    parser = argparse.ArgumentParser(
        prog="mia",
        description="SOPS + age encrypted secrets manager with IPFS pinning",
    )
    sub = parser.add_subparsers(dest="command", help="command")

    # init
    sub.add_parser("init", help="Generate age keypair + .sops.yaml")

    # encrypt
    sub.add_parser("encrypt", help="Encrypt .env → vault.global.enc.yaml")

    # decrypt
    p = sub.add_parser("decrypt", help="Decrypt vault to stdout (memory only)")
    p.add_argument("--project", "-p", help="Decrypt a project vault instead of global")

    # pin
    p = sub.add_parser("pin", help="Pin encrypted vault to Pinata IPFS")
    p.add_argument("--project", "-p", help="Pin a project vault instead of global")

    # spawn
    p = sub.add_parser("spawn", help="Create per-project vault")
    p.add_argument("project", help="Project name (e.g., twilio, livepeer)")
    p.add_argument("--keys", "-k", help="Comma-separated key names (overrides group)")
    p.add_argument("--os", help="OS target tag (linux, darwin)")

    # list
    p = sub.add_parser("list", help="Show CID registry")
    p.add_argument("--project", "-p", help="Show chain for a specific project")

    # pull
    p = sub.add_parser("pull", help="Retrieve encrypted file from IPFS")
    p.add_argument("cid", help="IPFS CID to retrieve")
    p.add_argument("--output", "-o", help="Save to file (default: stdout)")

    # status
    sub.add_parser("status", help="Health check")

    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    commands = {
        "init": cmd_init,
        "encrypt": cmd_encrypt,
        "decrypt": cmd_decrypt,
        "pin": cmd_pin,
        "spawn": cmd_spawn,
        "list": cmd_list,
        "pull": cmd_pull,
        "status": cmd_status,
    }

    handler = commands.get(args.command)
    if handler:
        return handler(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main() or 0)
