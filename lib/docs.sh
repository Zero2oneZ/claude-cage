#!/usr/bin/env bash
# docs.sh — Bash wrappers for the Circular Documentation System
# Documentation as code. Bidirectional. Staleness-tracked. One circle.

docs_generate() {
    local node_id="$1"
    if [[ -z "$node_id" ]]; then
        echo "Usage: docs_generate <node_id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs generate "$node_id"
}

docs_generate_all() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs generate-all
}

docs_check_stale() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs check-stale
}

docs_refresh() {
    local node_id="${1:-}"
    if [[ -n "$node_id" ]]; then
        CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
            python3 -m ptc.docs refresh "$node_id"
    else
        CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
            python3 -m ptc.docs refresh
    fi
}

docs_interconnect() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs interconnect
}

docs_search() {
    local query="$1"
    local limit="${2:-10}"
    if [[ -z "$query" ]]; then
        echo "Usage: docs_search <query> [limit]" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs search "$query" "$limit"
}

docs_show() {
    local node_id="$1"
    if [[ -z "$node_id" ]]; then
        echo "Usage: docs_show <node_id>" >&2
        return 1
    fi
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs show "$node_id"
}

docs_status() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs status
}

docs_graph() {
    CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
        python3 -m ptc.docs graph
}
