"""ptc/ipfs.py — Content-addressed artifact storage.

Dual-write: MongoDB for fast query + IPFS for permanent addressing.
Hash computed at creation. IPFS add is fire-and-forget (never blocks CLI).

Three tiers:
  1. Local IPFS node (ipfs daemon running on host)
  2. Pinata (remote pinning, no local node needed)
  3. Hash-only fallback (graceful degradation — compute hash, skip IPFS)
"""

import hashlib
import json
import os
import subprocess
import time
from datetime import datetime, timezone
from urllib.request import urlopen, Request
from urllib.error import URLError

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ── Configuration ──────────────────────────────────────────────


def _load_config():
    """Load IPFS config from environment or defaults."""
    return {
        "enabled": os.environ.get("IPFS_ENABLED", "false").lower() in ("true", "1", "yes"),
        "api": os.environ.get("IPFS_API", "http://localhost:5001"),
        "gateway": os.environ.get("IPFS_GATEWAY", "http://localhost:8080"),
        "pin_service": os.environ.get("IPFS_PIN_SERVICE", "local"),
        "pinata_jwt": os.environ.get("IPFS_PINATA_JWT", ""),
    }


# ── Content Hashing ────────────────────────────────────────────


def content_hash(content):
    """SHA-256 hash of content. Always computed, regardless of IPFS availability.

    Args:
        content: str or bytes

    Returns:
        str: "sha256:<hex digest>"
    """
    if isinstance(content, str):
        content = content.encode("utf-8")
    digest = hashlib.sha256(content).hexdigest()
    return f"sha256:{digest}"


# ── IPFS Operations ────────────────────────────────────────────


def ipfs_available():
    """Check if IPFS API is reachable."""
    config = _load_config()
    if not config["enabled"]:
        return False
    try:
        req = Request(f"{config['api']}/api/v0/id", method="POST")
        with urlopen(req, timeout=3) as resp:
            return resp.status == 200
    except (URLError, OSError):
        return False


def ipfs_add(content, pin=True):
    """Add content to IPFS, return CID.

    Fire-and-forget when called from store — this function itself
    is synchronous for direct use.

    Args:
        content: str or bytes
        pin: whether to pin after adding

    Returns:
        str: IPFS CID (e.g., "QmXyz...") or None if IPFS unavailable
    """
    config = _load_config()
    if not config["enabled"]:
        return None

    if isinstance(content, str):
        content = content.encode("utf-8")

    try:
        import urllib.request
        import io

        # Multipart form data for IPFS add API
        boundary = f"----CageBoundary{int(time.time())}"
        body = (
            f"--{boundary}\r\n"
            f'Content-Disposition: form-data; name="file"; filename="artifact"\r\n'
            f"Content-Type: application/octet-stream\r\n\r\n"
        ).encode() + content + f"\r\n--{boundary}--\r\n".encode()

        req = Request(
            f"{config['api']}/api/v0/add?pin={'true' if pin else 'false'}",
            data=body,
            headers={"Content-Type": f"multipart/form-data; boundary={boundary}"},
            method="POST",
        )
        with urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read())
            return result.get("Hash")
    except (URLError, OSError, json.JSONDecodeError, KeyError):
        return None


def ipfs_get(cid):
    """Retrieve content from IPFS by CID.

    Args:
        cid: IPFS content identifier

    Returns:
        bytes or None
    """
    config = _load_config()
    gateway = config["gateway"]
    try:
        req = Request(f"{gateway}/ipfs/{cid}")
        with urlopen(req, timeout=30) as resp:
            return resp.read()
    except (URLError, OSError):
        return None


def ipfs_pin(cid, service=None):
    """Pin CID to ensure persistence.

    Args:
        cid: IPFS content identifier
        service: "local" | "pinata" (overrides config)

    Returns:
        bool: success
    """
    config = _load_config()
    service = service or config["pin_service"]

    if service == "pinata":
        return _pin_pinata(cid, config)
    else:
        return _pin_local(cid, config)


def _pin_local(cid, config):
    """Pin to local IPFS node."""
    try:
        req = Request(f"{config['api']}/api/v0/pin/add?arg={cid}", method="POST")
        with urlopen(req, timeout=30) as resp:
            return resp.status == 200
    except (URLError, OSError):
        return False


def _pin_pinata(cid, config):
    """Pin via Pinata remote pinning service."""
    jwt = config.get("pinata_jwt", "")
    if not jwt:
        return False
    try:
        data = json.dumps({"hashToPin": cid}).encode()
        req = Request(
            "https://api.pinata.cloud/pinning/pinByHash",
            data=data,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {jwt}",
            },
            method="POST",
        )
        with urlopen(req, timeout=30) as resp:
            return resp.status == 200
    except (URLError, OSError):
        return False


# ── Dual-Write Storage ─────────────────────────────────────────


def dual_store(name, artifact_type, content, project="claude-cage"):
    """Store artifact to MongoDB + IPFS simultaneously.

    MongoDB gets the full document (for query/search).
    IPFS gets the content (for permanent addressing).
    Both share the same SHA-256 hash as the bridge.

    The IPFS add runs in a background subprocess — never blocks.

    Args:
        name: artifact name
        artifact_type: code|config|doc|output|decision|trace|blueprint|design
        content: the artifact content (str or dict)
        project: project identifier

    Returns:
        dict: {hash, cid, storage}
    """
    # Always compute hash (cheap)
    content_str = content if isinstance(content, str) else json.dumps(content)
    chash = content_hash(content_str)

    # MongoDB store (fire-and-forget, existing pattern)
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    config = _load_config()
    storage = "ipfs" if config["enabled"] else "mongodb"

    doc = json.dumps({
        "name": name,
        "type": artifact_type,
        "content": content_str[:50000],
        "project": project,
        "hash": chash,
        "storage": storage,
        "ipfs_cid": None,
        "_ts": datetime.now(timezone.utc).isoformat(),
    })

    if os.path.exists(store_js):
        try:
            subprocess.Popen(
                ["node", store_js, "put", "artifacts", doc],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except Exception:
            pass

    # IPFS add (fire-and-forget background)
    cid = None
    if config["enabled"]:
        try:
            # Background the IPFS add so it never blocks
            subprocess.Popen(
                [
                    "python3", "-c",
                    f"from ptc.ipfs import _ipfs_add_and_update; "
                    f"_ipfs_add_and_update({json.dumps(name)}, {json.dumps(content_str[:50000])}, {json.dumps(chash)})"
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env={**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT},
            )
        except Exception:
            pass

    return {
        "hash": chash,
        "cid": cid,
        "storage": storage,
    }


def _ipfs_add_and_update(name, content, chash):
    """Background worker: add to IPFS, update MongoDB with CID.

    Called via subprocess from dual_store(). Not meant to be called directly.
    """
    cid = ipfs_add(content)
    if cid:
        # Update MongoDB document with the CID
        store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
        if os.path.exists(store_js):
            update_doc = json.dumps({
                "hash": chash,
                "ipfs_cid": cid,
                "storage": "ipfs",
                "ipfs_pinned_at": datetime.now(timezone.utc).isoformat(),
            })
            try:
                subprocess.Popen(
                    ["node", store_js, "log", "ipfs:pinned", name, update_doc],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            except Exception:
                pass


# ── Migration ──────────────────────────────────────────────────


def migrate_existing(collection="artifacts", batch_size=100):
    """Backfill IPFS CIDs for existing MongoDB artifacts.

    Reads artifacts from MongoDB, computes hash, adds to IPFS if enabled.
    Idempotent — skips artifacts that already have a hash.

    Returns:
        dict: {processed, hashed, ipfs_added, skipped, errors}
    """
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return {"error": "store.js not found"}

    stats = {"processed": 0, "hashed": 0, "ipfs_added": 0, "skipped": 0, "errors": 0}

    try:
        result = subprocess.run(
            ["node", store_js, "get", collection, "{}", str(batch_size)],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            return {"error": result.stderr.strip()}

        docs = json.loads(result.stdout)
        if not isinstance(docs, list):
            docs = [docs]

        for doc in docs:
            stats["processed"] += 1
            content = doc.get("content", "")
            existing_hash = doc.get("hash")

            if existing_hash and doc.get("storage") == "ipfs":
                stats["skipped"] += 1
                continue

            # Compute hash
            chash = content_hash(content)
            stats["hashed"] += 1

            # Add to IPFS if enabled
            config = _load_config()
            if config["enabled"] and content:
                cid = ipfs_add(content)
                if cid:
                    stats["ipfs_added"] += 1

    except (subprocess.TimeoutExpired, json.JSONDecodeError, Exception) as e:
        stats["errors"] += 1
        stats["last_error"] = str(e)

    return stats


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for IPFS operations."""
    import sys

    if len(sys.argv) < 2:
        print("Usage: python -m ptc.ipfs <command> [args]")
        print("Commands: status, add <file>, get <cid>, pin <cid>, migrate")
        sys.exit(1)

    command = sys.argv[1]

    if command == "status":
        config = _load_config()
        available = ipfs_available()
        print(f"IPFS enabled: {config['enabled']}")
        print(f"IPFS API:     {config['api']}")
        print(f"IPFS gateway: {config['gateway']}")
        print(f"Pin service:  {config['pin_service']}")
        print(f"Available:    {available}")

    elif command == "add":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.ipfs add <file>")
            sys.exit(1)
        with open(sys.argv[2], "rb") as f:
            content = f.read()
        chash = content_hash(content)
        print(f"Hash: {chash}")
        cid = ipfs_add(content)
        if cid:
            print(f"CID:  {cid}")
        else:
            print("CID:  (IPFS not available — hash-only mode)")

    elif command == "get":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.ipfs get <cid>")
            sys.exit(1)
        data = ipfs_get(sys.argv[2])
        if data:
            sys.stdout.buffer.write(data)
        else:
            print("Error: could not retrieve CID", file=sys.stderr)
            sys.exit(1)

    elif command == "pin":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.ipfs pin <cid>")
            sys.exit(1)
        ok = ipfs_pin(sys.argv[2])
        print(f"Pinned: {ok}")

    elif command == "migrate":
        stats = migrate_existing()
        print(json.dumps(stats, indent=2))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
