"""ptc/nounproject.py — Noun Project API v2 design asset client.

OAuth 1.0a HMAC-SHA1 auth built from stdlib (hmac, hashlib, urllib.parse).
Base URL: https://api.thenounproject.com/v2/

Config: NOUNPROJECT_KEY, NOUNPROJECT_SECRET, NOUNPROJECT_ENABLED
"""

import base64
import hashlib
import hmac
import json
import os
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from urllib.parse import quote, urlencode, urlparse, parse_qs
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BASE_URL = "https://api.thenounproject.com/v2"


# ── Configuration ──────────────────────────────────────────────


def _load_config():
    """Load Noun Project config from environment."""
    return {
        "enabled": os.environ.get("NOUNPROJECT_ENABLED", "false").lower() in ("true", "1", "yes"),
        "key": os.environ.get("NOUNPROJECT_KEY", ""),
        "secret": os.environ.get("NOUNPROJECT_SECRET", ""),
    }


def _np_available():
    """Check if Noun Project API credentials are configured."""
    config = _load_config()
    if not config["enabled"]:
        return False, "NOUNPROJECT_ENABLED is not set"
    if not config["key"] or not config["secret"]:
        return False, "NOUNPROJECT_KEY or NOUNPROJECT_SECRET not set"
    return True, "ok"


def _mongo_log(event_type, key, value=None):
    """Fire-and-forget MongoDB event log."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return
    doc = json.dumps({
        "type": event_type, "key": key, "value": value,
        "_ts": datetime.now(timezone.utc).isoformat(), "_source": "nounproject",
    })
    try:
        subprocess.Popen(
            ["node", store_js, "log", event_type, key, doc],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


# ── OAuth 1.0a ─────────────────────────────────────────────────


def _percent_encode(s):
    """RFC 5849 percent-encode."""
    return quote(str(s), safe="")


def _oauth_header(method, url, params=None):
    """Build OAuth 1.0a Authorization header with HMAC-SHA1.

    Args:
        method: HTTP method (GET/POST)
        url: full URL (without query string for signing)
        params: query parameters dict

    Returns:
        str: Authorization header value
    """
    config = _load_config()
    consumer_key = config["key"]
    consumer_secret = config["secret"]

    oauth_params = {
        "oauth_consumer_key": consumer_key,
        "oauth_nonce": uuid.uuid4().hex,
        "oauth_signature_method": "HMAC-SHA1",
        "oauth_timestamp": str(int(time.time())),
        "oauth_version": "1.0",
    }

    # Combine oauth params and query params for signature base
    all_params = dict(oauth_params)
    if params:
        all_params.update(params)

    # Sort and encode parameters
    sorted_params = sorted(all_params.items())
    param_string = "&".join(f"{_percent_encode(k)}={_percent_encode(v)}" for k, v in sorted_params)

    # Parse URL to get base URL without query string
    parsed = urlparse(url)
    base_url = f"{parsed.scheme}://{parsed.netloc}{parsed.path}"

    # Signature base string
    sig_base = f"{method.upper()}&{_percent_encode(base_url)}&{_percent_encode(param_string)}"

    # Signing key (consumer_secret&token_secret — no token for 2-legged OAuth)
    signing_key = f"{_percent_encode(consumer_secret)}&"

    # HMAC-SHA1 signature
    hashed = hmac.new(signing_key.encode("utf-8"), sig_base.encode("utf-8"), hashlib.sha1)
    signature = base64.b64encode(hashed.digest()).decode("utf-8")

    oauth_params["oauth_signature"] = signature

    # Build Authorization header
    auth_parts = ", ".join(f'{_percent_encode(k)}="{_percent_encode(v)}"' for k, v in sorted(oauth_params.items()))
    return f"OAuth {auth_parts}"


# ── HTTP Client ────────────────────────────────────────────────


def _np_get(endpoint, params=None):
    """GET from Noun Project API with OAuth header.

    Args:
        endpoint: API path after /v2/
        params: query parameters dict

    Returns:
        dict: parsed JSON response
    """
    url = f"{BASE_URL}/{endpoint}"
    if params:
        url_with_params = f"{url}?{urlencode(params)}"
    else:
        url_with_params = url
        params = {}

    auth = _oauth_header("GET", url, params)
    req = Request(url_with_params, headers={"Authorization": auth, "Accept": "application/json"})

    try:
        with urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if hasattr(e, "read") else ""
        return {"error": f"HTTP {e.code}", "detail": body[:500]}
    except (URLError, OSError) as e:
        return {"error": str(e)}
    except json.JSONDecodeError:
        return {"error": "Invalid JSON response"}


# ── API Functions ──────────────────────────────────────────────


def search_icons(query, limit=20, page=1, include_svg=False):
    """Search for icons.

    Args:
        query: search term
        limit: results per page (max 50)
        page: page number
        include_svg: include SVG thumbnail data

    Returns:
        dict: search results with icons list
    """
    params = {"query": query, "limit": str(limit), "page": str(page)}
    if include_svg:
        params["include_svg"] = "1"
    result = _np_get("icon", params)
    _mongo_log("nounproject:search", query, str(limit))
    return result


def get_icon(icon_id):
    """Get icon metadata by ID.

    Args:
        icon_id: numeric icon ID

    Returns:
        dict: icon details
    """
    return _np_get(f"icon/{icon_id}")


def download_icon(icon_id, filetype="svg", color="000000", size=200):
    """Download icon as base64-encoded data.

    Args:
        icon_id: numeric icon ID
        filetype: svg or png
        color: hex color (without #)
        size: pixel size (for png)

    Returns:
        dict: {data, filetype, size} or {error}
    """
    params = {"filetype": filetype, "color": color}
    if filetype == "png":
        params["size"] = str(size)

    url = f"{BASE_URL}/icon/{icon_id}/download"
    url_with_params = f"{url}?{urlencode(params)}"
    auth = _oauth_header("GET", url, params)
    req = Request(url_with_params, headers={"Authorization": auth, "Accept": "application/json"})

    try:
        with urlopen(req, timeout=30) as resp:
            data = resp.read()
            try:
                result = json.loads(data.decode("utf-8"))
                return result
            except json.JSONDecodeError:
                encoded = base64.b64encode(data).decode("utf-8")
                return {"data": encoded, "filetype": filetype, "size": len(data)}
    except HTTPError as e:
        return {"error": f"HTTP {e.code}"}
    except (URLError, OSError) as e:
        return {"error": str(e)}


def autocomplete(query):
    """Get search suggestions.

    Args:
        query: partial search term

    Returns:
        dict: suggestions list
    """
    return _np_get("autocomplete", {"query": query})


def search_collections(query, limit=20, page=1):
    """Search icon collections.

    Args:
        query: search term
        limit: results per page
        page: page number

    Returns:
        dict: collections list
    """
    params = {"query": query, "limit": str(limit), "page": str(page)}
    return _np_get("collection", params)


def get_collection(collection_id):
    """Get collection detail with icons.

    Args:
        collection_id: numeric collection ID

    Returns:
        dict: collection with icons
    """
    return _np_get(f"collection/{collection_id}")


def get_usage():
    """Get API usage limits and current usage."""
    return _np_get("usage")


def save_icon(icon_id, output_path, filetype="svg", color="000000", size=200):
    """Download icon and write to disk.

    Args:
        icon_id: numeric icon ID
        output_path: local file path
        filetype: svg or png
        color: hex color
        size: pixel size (png only)

    Returns:
        dict: {path, size} or {error}
    """
    result = download_icon(icon_id, filetype, color, size)
    if "error" in result:
        return result

    data = result.get("data")
    if not data:
        return {"error": "No data returned"}

    try:
        decoded = base64.b64decode(data)
    except Exception:
        decoded = data.encode("utf-8") if isinstance(data, str) else data

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "wb") as f:
        f.write(decoded)

    _mongo_log("nounproject:download", str(icon_id), output_path)
    return {"path": output_path, "size": len(decoded)}


def batch_download(query, output_dir, limit=10, filetype="svg", color="000000"):
    """Download multiple icons matching a query.

    Args:
        query: search term
        output_dir: directory to save icons
        limit: max icons to download
        filetype: svg or png
        color: hex color

    Returns:
        dict: {downloaded, failed, icons}
    """
    results = search_icons(query, limit=limit)
    icons = results.get("icons", [])
    if not icons:
        return {"downloaded": 0, "failed": 0, "icons": [], "error": results.get("error")}

    os.makedirs(output_dir, exist_ok=True)
    downloaded = []
    failed = 0

    for icon in icons[:limit]:
        icon_id = icon.get("id")
        if not icon_id:
            failed += 1
            continue
        ext = filetype
        path = os.path.join(output_dir, f"{icon_id}.{ext}")
        result = save_icon(icon_id, path, filetype, color)
        if "error" in result:
            failed += 1
        else:
            downloaded.append({"id": icon_id, "path": path, "size": result["size"]})

    _mongo_log("nounproject:batch", query, str(len(downloaded)))
    return {"downloaded": len(downloaded), "failed": failed, "icons": downloaded}


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for Noun Project operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.nounproject <command> [args]")
        print("Commands: search <query> [--limit N], get <id>,")
        print("          download <id> <path> [--type svg|png] [--color hex],")
        print("          batch <query> <dir> [--limit N] [--type svg|png],")
        print("          collections <query>, collection <id>,")
        print("          autocomplete <query>, usage")
        sys.exit(1)

    command = sys.argv[1]

    if command == "search":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.nounproject search <query> [--limit N]", file=sys.stderr)
            sys.exit(1)
        query = sys.argv[2]
        limit = 20
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--limit" and i + 1 < len(sys.argv):
                limit = int(sys.argv[i + 1])
        result = search_icons(query, limit=limit)
        print(json.dumps(result, indent=2))

    elif command == "get":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.nounproject get <id>", file=sys.stderr)
            sys.exit(1)
        result = get_icon(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "download":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.nounproject download <id> <path> [--type svg|png]", file=sys.stderr)
            sys.exit(1)
        icon_id = sys.argv[2]
        path = sys.argv[3]
        filetype = "svg"
        color = "000000"
        for i, arg in enumerate(sys.argv[4:], 4):
            if arg == "--type" and i + 1 < len(sys.argv):
                filetype = sys.argv[i + 1]
            elif arg == "--color" and i + 1 < len(sys.argv):
                color = sys.argv[i + 1]
        result = save_icon(icon_id, path, filetype, color)
        print(json.dumps(result, indent=2))

    elif command == "batch":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.nounproject batch <query> <dir> [--limit N]", file=sys.stderr)
            sys.exit(1)
        query = sys.argv[2]
        output_dir = sys.argv[3]
        limit = 10
        filetype = "svg"
        for i, arg in enumerate(sys.argv[4:], 4):
            if arg == "--limit" and i + 1 < len(sys.argv):
                limit = int(sys.argv[i + 1])
            elif arg == "--type" and i + 1 < len(sys.argv):
                filetype = sys.argv[i + 1]
        result = batch_download(query, output_dir, limit, filetype)
        print(json.dumps(result, indent=2))

    elif command == "collections":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.nounproject collections <query>", file=sys.stderr)
            sys.exit(1)
        result = search_collections(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "collection":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.nounproject collection <id>", file=sys.stderr)
            sys.exit(1)
        result = get_collection(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "autocomplete":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.nounproject autocomplete <query>", file=sys.stderr)
            sys.exit(1)
        result = autocomplete(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "usage":
        result = get_usage()
        print(json.dumps(result, indent=2))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
