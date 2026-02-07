#!/usr/bin/env bash
# architect.sh — Bash wrappers for architect-mode operations
# Blueprints, IPFS, vector search, git process pipeline

# ── Design (Blueprints) ───────────────────────────────────────

architect_create() {
    local intent="$*"
    if [[ -z "$intent" ]]; then
        echo "Usage: architect_create <intent>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect create "$intent"
}

architect_list() {
    local status_filter=""
    [[ -n "$1" ]] && status_filter="--status $1"
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect list $status_filter
}

architect_show() {
    local bp_id="$1"
    if [[ -z "$bp_id" ]]; then
        echo "Usage: architect_show <blueprint-id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect show "$bp_id"
}

architect_tasks() {
    local bp_id="$1"
    if [[ -z "$bp_id" ]]; then
        echo "Usage: architect_tasks <blueprint-id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect tasks "$bp_id"
}

architect_validate() {
    local bp_id="$1"
    if [[ -z "$bp_id" ]]; then
        echo "Usage: architect_validate <blueprint-id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect validate "$bp_id"
}

architect_search() {
    local query="$*"
    if [[ -z "$query" ]]; then
        echo "Usage: architect_search <query>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.architect search "$query"
}

# ── IPFS ───────────────────────────────────────────────────────

ipfs_status() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.ipfs status
}

ipfs_migrate() {
    echo "==> Migrating existing artifacts to IPFS..."
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.ipfs migrate
}

# ── Vector Search ──────────────────────────────────────────────

vsearch() {
    local query="$*"
    if [[ -z "$query" ]]; then
        echo "Usage: vsearch <query>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.embeddings search "$query"
}

embed_all() {
    echo "==> Embedding all artifacts..."
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.embeddings embed-all
}

# ── Git Ops ────────────────────────────────────────────────────

git_branches() {
    local pattern="${1:-}"
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.git_ops branches "$pattern"
}

git_log_node() {
    local node_id="$1"
    if [[ -z "$node_id" ]]; then
        echo "Usage: git_log_node <node-id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.git_ops log-node "$node_id"
}
