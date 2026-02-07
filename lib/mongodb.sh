#!/usr/bin/env bash
# mongodb.sh — MongoDB fire-and-forget storage layer
# Key/value store: just write, don't wait.
# All writes are backgrounded and disowned — zero blocking.

MONGO_STORE="${CAGE_ROOT}/mongodb/store.js"
MONGO_ENV="${CAGE_ROOT}/mongodb/.env"
MONGO_READY=false

# ── init: load .env, check deps, auto-install ──────────────────
mongo_init() {
    # Source .env into environment
    if [[ -f "$MONGO_ENV" ]]; then
        set -a
        # shellcheck disable=SC1090
        source "$MONGO_ENV" 2>/dev/null || true
        set +a
    fi

    # Bail if store.js or node missing
    if [[ ! -f "$MONGO_STORE" ]]; then
        return 0
    fi
    if ! command -v node &>/dev/null; then
        return 0
    fi

    # Auto-install node_modules on first run
    if [[ ! -d "${CAGE_ROOT}/mongodb/node_modules" ]]; then
        (cd "${CAGE_ROOT}/mongodb" && npm install --silent 2>/dev/null) &
        disown 2>/dev/null
        return 0
    fi

    MONGO_READY=true
}

# ── put: fire-and-forget insert into any collection ────────────
# Usage: mongo_put <collection> <json_string>
mongo_put() {
    $MONGO_READY || return 0
    local collection="$1" doc="$2"
    ( node "$MONGO_STORE" put "$collection" "$doc" >/dev/null 2>&1 ) &
    disown 2>/dev/null
}

# ── log: fire-and-forget structured event ──────────────────────
# Usage: mongo_log <type> <key> [value_json]
mongo_log() {
    $MONGO_READY || return 0
    local type="$1" key="$2" value="${3:-'{}'}"
    ( CAGE_PROJECT="${CAGE_PROJECT:-claude-cage}" \
      node "$MONGO_STORE" log "$type" "$key" "$value" >/dev/null 2>&1 ) &
    disown 2>/dev/null
}

# ── get: synchronous query (returns JSON array) ────────────────
# Usage: mongo_get <collection> [query_json] [limit]
mongo_get() {
    $MONGO_READY || { echo "[]"; return 0; }
    local collection="$1" query="${2:-'{}'}" limit="${3:-10}"
    node "$MONGO_STORE" get "$collection" "$query" "$limit" 2>/dev/null || echo "[]"
}

# ── ping: test connectivity (synchronous) ──────────────────────
mongo_ping() {
    $MONGO_READY || { echo '{"ok":0,"error":"not initialized"}'; return 1; }
    node "$MONGO_STORE" ping 2>/dev/null
}

# ── convenience: log a session lifecycle event ─────────────────
# Usage: mongo_log_session <event> <session_name> [metadata_json]
mongo_log_session() {
    local event="$1" session="$2" meta="${3:-'{}'}"
    mongo_log "session" "${event}:${session}" "$meta"
}

# ── convenience: log a CLI command ─────────────────────────────
# Usage: mongo_log_command <command> [arg1] [arg2] ...
mongo_log_command() {
    local cmd="$1"
    shift
    local args_json="[]"
    if command -v jq &>/dev/null && [[ $# -gt 0 ]]; then
        args_json=$(printf '%s\n' "$@" | jq -R -s 'split("\n") | map(select(. != ""))' 2>/dev/null || echo '[]')
    fi
    mongo_log "command" "$cmd" "{\"args\":$args_json}"
}

# ── convenience: store an artifact (code, config, output) ──────
# Usage: mongo_store_artifact <name> <type> <content>
mongo_store_artifact() {
    $MONGO_READY || return 0
    local name="$1" atype="$2" content="$3"
    local doc
    if command -v jq &>/dev/null; then
        doc=$(jq -n \
            --arg name "$name" \
            --arg type "$atype" \
            --arg content "$content" \
            --arg project "${CAGE_PROJECT:-claude-cage}" \
            '{name:$name,type:$type,content:$content,project:$project}')
    else
        # fallback: manual JSON (content truncated to avoid escaping issues)
        local safe_content="${content//\"/\\\"}"
        safe_content="${safe_content:0:4096}"
        doc="{\"name\":\"$name\",\"type\":\"$atype\",\"content\":\"$safe_content\",\"project\":\"${CAGE_PROJECT:-claude-cage}\"}"
    fi
    mongo_put "artifacts" "$doc"
}

# ── convenience: log for a specific project context ────────────
# Usage: mongo_log_project <project_name> <type> <key> [value_json]
mongo_log_project() {
    local project="$1" type="$2" key="$3" value="${4:-'{}'}"
    ( CAGE_PROJECT="$project" \
      node "$MONGO_STORE" log "$type" "$key" "$value" >/dev/null 2>&1 ) &
    disown 2>/dev/null
}
