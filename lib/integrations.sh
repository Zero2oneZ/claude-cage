#!/usr/bin/env bash
# integrations.sh — Bash wrappers for the four integration pillars
# Porkbun (domains), Noun Project (icons), Federation (git sovereignty), Hugging Face (ML hub)

# ── Porkbun (Domains) ────────────────────────────────────────

porkbun_ping() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun ping
}

porkbun_check() {
    local domain="$1"
    if [[ -z "$domain" ]]; then
        echo "Usage: porkbun_check <domain>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun check "$domain"
}

porkbun_domains() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun domains
}

porkbun_dns() {
    local domain="$1"
    if [[ -z "$domain" ]]; then
        echo "Usage: porkbun_dns <domain>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun dns "$domain"
}

porkbun_dns_create() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun dns-create "$@"
}

porkbun_dns_delete() {
    local domain="$1"
    local record_id="$2"
    if [[ -z "$domain" || -z "$record_id" ]]; then
        echo "Usage: porkbun_dns_delete <domain> <record-id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun dns-delete "$domain" "$record_id"
}

porkbun_ssl() {
    local domain="$1"
    if [[ -z "$domain" ]]; then
        echo "Usage: porkbun_ssl <domain>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun ssl "$domain"
}

porkbun_pricing() {
    local tld="${1:-}"
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.porkbun pricing $tld
}

# ── Noun Project (Icons) ─────────────────────────────────────

np_search() {
    local query="$*"
    if [[ -z "$query" ]]; then
        echo "Usage: np_search <query>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.nounproject search "$query"
}

np_download() {
    local icon_id="$1"
    local path="$2"
    if [[ -z "$icon_id" || -z "$path" ]]; then
        echo "Usage: np_download <icon-id> <path>" >&2
        return 1
    fi
    shift 2
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.nounproject download "$icon_id" "$path" "$@"
}

np_batch() {
    local query="$1"
    local dir="$2"
    if [[ -z "$query" || -z "$dir" ]]; then
        echo "Usage: np_batch <query> <dir> [--limit N]" >&2
        return 1
    fi
    shift 2
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.nounproject batch "$query" "$dir" "$@"
}

np_usage() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.nounproject usage
}

# ── Federation (Git Sovereignty) ──────────────────────────────

federation_fork() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation fork "$@"
}

federation_branch() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation branch "$@"
}

federation_pull() {
    local dir="${1:-.}"
    shift 2>/dev/null || true
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation pull "$dir" "$@"
}

federation_push() {
    local dir="${1:-.}"
    shift 2>/dev/null || true
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation push "$dir" "$@"
}

federation_status() {
    local dir="${1:-.}"
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation status "$dir"
}

federation_verify() {
    local dir="${1:-.}"
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation verify "$dir"
}

federation_diff() {
    local tree_a="$1"
    local tree_b="$2"
    if [[ -z "$tree_a" || -z "$tree_b" ]]; then
        echo "Usage: federation_diff <tree-a> <tree-b>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.federation diff "$tree_a" "$tree_b"
}

# ── Hugging Face (ML Hub) ────────────────────────────────────

hf_status() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface status
}

hf_download() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface download "$@"
}

hf_embed() {
    local text="$*"
    if [[ -z "$text" ]]; then
        echo "Usage: hf_embed <text>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface embed "$text"
}

hf_chat() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface chat "$@"
}

hf_search() {
    local query="$*"
    if [[ -z "$query" ]]; then
        echo "Usage: hf_search <query>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface search "$query"
}

hf_upload() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface upload "$@"
}

hf_cache() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.huggingface cache
}
