"""ptc/porkbun.py — Porkbun API v3 domain management.

Stdlib-only HTTP client. All 27 Porkbun endpoints are POST with JSON body auth.
Base URL: https://api.porkbun.com/api/json/v3/

Config: PORKBUN_API_KEY, PORKBUN_SECRET_KEY, PORKBUN_ENABLED
"""

import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BASE_URL = "https://api.porkbun.com/api/json/v3"


# ── Configuration ──────────────────────────────────────────────


def _load_config():
    """Load Porkbun config from environment."""
    return {
        "enabled": os.environ.get("PORKBUN_ENABLED", "false").lower() in ("true", "1", "yes"),
        "api_key": os.environ.get("PORKBUN_API_KEY", ""),
        "secret_key": os.environ.get("PORKBUN_SECRET_KEY", ""),
    }


def _porkbun_available():
    """Check if Porkbun API credentials are configured."""
    config = _load_config()
    if not config["enabled"]:
        return False, "PORKBUN_ENABLED is not set"
    if not config["api_key"] or not config["secret_key"]:
        return False, "PORKBUN_API_KEY or PORKBUN_SECRET_KEY not set"
    return True, "ok"


def _mongo_log(event_type, key, value=None):
    """Fire-and-forget MongoDB event log."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return
    doc = json.dumps({
        "type": event_type,
        "key": key,
        "value": value,
        "_ts": datetime.now(timezone.utc).isoformat(),
        "_source": "porkbun",
    })
    try:
        subprocess.Popen(
            ["node", store_js, "log", event_type, key, doc],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


# ── HTTP Client ────────────────────────────────────────────────


def _porkbun_post(endpoint, extra_data=None, auth=True, retries=2):
    """POST to Porkbun API with auth injection and retry on 503.

    Args:
        endpoint: API path after /api/json/v3/
        extra_data: additional JSON body fields
        auth: whether to inject API keys
        retries: retry count on 503

    Returns:
        dict: parsed JSON response
    """
    url = f"{BASE_URL}/{endpoint}"
    body = {}
    if auth:
        config = _load_config()
        body["apikey"] = config["api_key"]
        body["secretapikey"] = config["secret_key"]
    if extra_data:
        body.update(extra_data)

    data = json.dumps(body).encode("utf-8")
    req = Request(url, data=data, headers={"Content-Type": "application/json"}, method="POST")

    for attempt in range(retries + 1):
        try:
            with urlopen(req, timeout=30) as resp:
                result = json.loads(resp.read().decode("utf-8"))
                return result
        except HTTPError as e:
            if e.code == 503 and attempt < retries:
                import time
                time.sleep(1 * (attempt + 1))
                continue
            body_text = e.read().decode("utf-8", errors="replace") if hasattr(e, "read") else ""
            return {"status": "ERROR", "message": f"HTTP {e.code}: {body_text[:500]}"}
        except (URLError, OSError) as e:
            return {"status": "ERROR", "message": str(e)}
        except json.JSONDecodeError:
            return {"status": "ERROR", "message": "Invalid JSON response"}

    return {"status": "ERROR", "message": "Max retries exceeded"}


# ── API Functions ──────────────────────────────────────────────


def ping():
    """Test API connectivity."""
    result = _porkbun_post("ping")
    _mongo_log("porkbun:ping", "ping", result.get("status"))
    return result


def get_pricing(tld=None):
    """Get TLD pricing. No auth required for default pricing."""
    if tld:
        return _porkbun_post(f"pricing/get/{tld}", auth=False)
    return _porkbun_post("pricing/get", auth=False)


def check_domain(domain):
    """Check domain availability and price."""
    result = _porkbun_post(f"domain/checkDomain/{domain}")
    _mongo_log("porkbun:check", domain, result.get("status"))
    return result


def register_domain(domain, years=1):
    """Register a domain."""
    result = _porkbun_post(f"domain/register/{domain}", {"years": years})
    _mongo_log("porkbun:register", domain, result.get("status"))
    return result


def list_domains():
    """List all domains in account."""
    result = _porkbun_post("domain/listAll")
    _mongo_log("porkbun:list", "domains")
    return result


def get_dns(domain):
    """Get all DNS records for a domain."""
    return _porkbun_post(f"dns/retrieve/{domain}")


def create_dns(domain, record_type, content, name="", ttl=600):
    """Create a DNS record.

    Args:
        domain: domain name
        record_type: A, AAAA, CNAME, MX, TXT, NS, SRV, CAA
        content: record value
        name: subdomain (empty for root)
        ttl: time-to-live in seconds
    """
    data = {"type": record_type, "content": content, "ttl": str(ttl)}
    if name:
        data["name"] = name
    result = _porkbun_post(f"dns/create/{domain}", data)
    _mongo_log("porkbun:dns-create", f"{domain}/{record_type}", name)
    return result


def edit_dns(domain, record_id, record_type, content, name="", ttl=600):
    """Edit an existing DNS record."""
    data = {"type": record_type, "content": content, "ttl": str(ttl)}
    if name:
        data["name"] = name
    result = _porkbun_post(f"dns/edit/{domain}/{record_id}", data)
    _mongo_log("porkbun:dns-edit", f"{domain}/{record_id}")
    return result


def delete_dns(domain, record_id):
    """Delete a DNS record."""
    result = _porkbun_post(f"dns/delete/{domain}/{record_id}")
    _mongo_log("porkbun:dns-delete", f"{domain}/{record_id}")
    return result


def get_nameservers(domain):
    """Get current nameservers for a domain."""
    return _porkbun_post(f"domain/getNs/{domain}")


def update_nameservers(domain, ns_list):
    """Set nameservers for a domain.

    Args:
        ns_list: list of nameserver hostnames
    """
    data = {"ns": ns_list}
    result = _porkbun_post(f"domain/updateNs/{domain}", data)
    _mongo_log("porkbun:ns-update", domain)
    return result


def get_ssl(domain):
    """Get free SSL certificate bundle for a domain."""
    result = _porkbun_post(f"ssl/retrieve/{domain}")
    _mongo_log("porkbun:ssl", domain)
    return result


def add_forward(domain, location, forward_type="temporary", include_path="no"):
    """Add URL forwarding for a domain.

    Args:
        domain: domain name
        location: target URL
        forward_type: "temporary" (302) or "permanent" (301)
        include_path: "yes" or "no"
    """
    data = {"location": location, "type": forward_type, "includePath": include_path}
    result = _porkbun_post(f"domain/addUrlForward/{domain}", data)
    _mongo_log("porkbun:forward-add", domain, location)
    return result


def get_forwards(domain):
    """List URL forwards for a domain."""
    return _porkbun_post(f"domain/getUrlForwarding/{domain}")


def delete_forward(domain, forward_id):
    """Remove a URL forward."""
    result = _porkbun_post(f"domain/deleteUrlForward/{domain}/{forward_id}")
    _mongo_log("porkbun:forward-delete", f"{domain}/{forward_id}")
    return result


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for Porkbun operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.porkbun <command> [args]")
        print("Commands: ping, check <domain>, domains, dns <domain>,")
        print("          dns-create <domain> <type> <content> [name] [ttl],")
        print("          dns-delete <domain> <id>, ssl <domain>,")
        print("          pricing [tld], forward <domain> <url> [type],")
        print("          forwards <domain>, forward-delete <domain> <id>,")
        print("          nameservers <domain>")
        sys.exit(1)

    command = sys.argv[1]

    if command == "ping":
        result = ping()
        print(json.dumps(result, indent=2))

    elif command == "check":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.porkbun check <domain>", file=sys.stderr)
            sys.exit(1)
        result = check_domain(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "domains":
        result = list_domains()
        print(json.dumps(result, indent=2))

    elif command == "dns":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.porkbun dns <domain>", file=sys.stderr)
            sys.exit(1)
        result = get_dns(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "dns-create":
        if len(sys.argv) < 5:
            print("Usage: python -m ptc.porkbun dns-create <domain> <type> <content> [name] [ttl]", file=sys.stderr)
            sys.exit(1)
        domain = sys.argv[2]
        rtype = sys.argv[3]
        content = sys.argv[4]
        name = sys.argv[5] if len(sys.argv) > 5 else ""
        ttl = int(sys.argv[6]) if len(sys.argv) > 6 else 600
        result = create_dns(domain, rtype, content, name, ttl)
        print(json.dumps(result, indent=2))

    elif command == "dns-delete":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.porkbun dns-delete <domain> <record-id>", file=sys.stderr)
            sys.exit(1)
        result = delete_dns(sys.argv[2], sys.argv[3])
        print(json.dumps(result, indent=2))

    elif command == "ssl":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.porkbun ssl <domain>", file=sys.stderr)
            sys.exit(1)
        result = get_ssl(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "pricing":
        tld = sys.argv[2] if len(sys.argv) > 2 else None
        result = get_pricing(tld)
        print(json.dumps(result, indent=2))

    elif command == "forward":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.porkbun forward <domain> <url> [type]", file=sys.stderr)
            sys.exit(1)
        ftype = sys.argv[4] if len(sys.argv) > 4 else "temporary"
        result = add_forward(sys.argv[2], sys.argv[3], ftype)
        print(json.dumps(result, indent=2))

    elif command == "forwards":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.porkbun forwards <domain>", file=sys.stderr)
            sys.exit(1)
        result = get_forwards(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "forward-delete":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.porkbun forward-delete <domain> <id>", file=sys.stderr)
            sys.exit(1)
        result = delete_forward(sys.argv[2], sys.argv[3])
        print(json.dumps(result, indent=2))

    elif command == "nameservers":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.porkbun nameservers <domain>", file=sys.stderr)
            sys.exit(1)
        result = get_nameservers(sys.argv[2])
        print(json.dumps(result, indent=2))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
