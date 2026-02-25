"""pinata.py — Pinata HTTP client for IPFS pinning.

Stdlib-only (urllib). Follows the pattern from ptc/ipfs.py.
Pins encrypted vault files to IPFS via Pinata's pinFileToIPFS endpoint.
"""

import json
import os
import time
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

PINATA_API_BASE = "https://api.pinata.cloud"
PINATA_GATEWAY = "https://gateway.pinata.cloud"


def _load_jwt():
    """Load Pinata JWT from environment or .env file.

    Returns:
        str: JWT token or empty string
    """
    jwt = os.environ.get("PINATA_JWT", "")
    if jwt:
        return jwt

    # Try loading from .secrets/.env.gently.global
    env_path = os.path.join(CAGE_ROOT, ".secrets", ".env.gently.global")
    if os.path.exists(env_path):
        with open(env_path, "r") as f:
            for line in f:
                line = line.strip()
                if line.startswith("PINATA_JWT="):
                    val = line.split("=", 1)[1].strip()
                    # Strip quotes (handle mismatched quote pairs too)
                    if len(val) >= 2 and val[0] in "'\"" and val[-1] in "'\"":
                        val = val[1:-1]
                    return val
    return ""


def _build_multipart(file_path, pin_name, metadata=None):
    """Build multipart/form-data body for pinFileToIPFS.

    Args:
        file_path: path to file to upload
        pin_name: display name for the pin
        metadata: optional dict of key-value metadata

    Returns:
        tuple: (body_bytes, content_type_header)
    """
    boundary = f"----MiaBoundary{int(time.time() * 1000)}"
    parts = []

    # File part
    filename = os.path.basename(file_path)
    with open(file_path, "rb") as f:
        file_data = f.read()

    parts.append(
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'
        f"Content-Type: application/octet-stream\r\n\r\n"
    )
    parts.append(file_data)
    parts.append(b"\r\n")

    # pinataMetadata part
    meta = {"name": pin_name}
    if metadata:
        meta["keyvalues"] = metadata
    meta_json = json.dumps(meta)
    parts.append(
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="pinataMetadata"\r\n'
        f"Content-Type: application/json\r\n\r\n"
        f"{meta_json}\r\n"
    )

    # Closing boundary
    parts.append(f"--{boundary}--\r\n")

    # Assemble
    body = b""
    for part in parts:
        if isinstance(part, str):
            body += part.encode("utf-8")
        else:
            body += part

    content_type = f"multipart/form-data; boundary={boundary}"
    return body, content_type


def pin_file(file_path, pin_name=None, metadata=None):
    """Pin a file to IPFS via Pinata's pinFileToIPFS endpoint.

    Args:
        file_path: path to file to upload
        pin_name: display name (defaults to filename)
        metadata: optional dict of key-value metadata

    Returns:
        dict: {"IpfsHash": "Qm...", "PinSize": N, "Timestamp": "..."}

    Raises:
        RuntimeError: on auth failure or upload error
    """
    jwt = _load_jwt()
    if not jwt:
        raise RuntimeError("No Pinata JWT found. Set PINATA_JWT env var or add to .secrets/.env.gently.global")

    pin_name = pin_name or os.path.basename(file_path)
    body, content_type = _build_multipart(file_path, pin_name, metadata)

    req = Request(
        f"{PINATA_API_BASE}/pinning/pinFileToIPFS",
        data=body,
        headers={
            "Content-Type": content_type,
            "Authorization": f"Bearer {jwt}",
        },
        method="POST",
    )

    try:
        with urlopen(req, timeout=60) as resp:
            result = json.loads(resp.read())
            return result
    except HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"Pinata upload failed ({e.code}): {body}")
    except URLError as e:
        raise RuntimeError(f"Pinata connection failed: {e.reason}")


def get_from_gateway(cid):
    """Retrieve content from Pinata's IPFS gateway.

    Args:
        cid: IPFS content identifier (Qm... or bafy...)

    Returns:
        bytes: raw content

    Raises:
        RuntimeError: on retrieval failure
    """
    url = f"{PINATA_GATEWAY}/ipfs/{cid}"
    req = Request(url)

    try:
        with urlopen(req, timeout=60) as resp:
            return resp.read()
    except HTTPError as e:
        raise RuntimeError(f"Gateway retrieval failed ({e.code}): {cid}")
    except URLError as e:
        raise RuntimeError(f"Gateway connection failed: {e.reason}")


def test_auth():
    """Verify Pinata JWT validity.

    Returns:
        dict: {"authenticated": True/False, "error": str or None}
    """
    jwt = _load_jwt()
    if not jwt:
        return {"authenticated": False, "error": "No JWT configured"}

    req = Request(
        f"{PINATA_API_BASE}/data/testAuthentication",
        headers={"Authorization": f"Bearer {jwt}"},
        method="GET",
    )

    try:
        with urlopen(req, timeout=10) as resp:
            result = json.loads(resp.read())
            return {"authenticated": True, "message": result.get("message", "ok")}
    except HTTPError as e:
        return {"authenticated": False, "error": f"HTTP {e.code}"}
    except URLError as e:
        return {"authenticated": False, "error": str(e.reason)}
