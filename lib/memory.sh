#!/usr/bin/env bash
# memory.sh — Session memory compaction and persistence
# Pattern: Background compaction from Anthropic cookbook
# Stores session context summaries in MongoDB for cross-session learning

MEMORY_DIR="${CAGE_DATA_DIR:-$HOME/.local/share/claude-cage}/memory"

# ── init: ensure memory directory exists ────────────────────
memory_init() {
    mkdir -p "$MEMORY_DIR"
}

# ── save: persist session context to disk and MongoDB ───────
# Usage: memory_save <session_name> <context_json>
memory_save() {
    local session="$1" context="$2"
    local mem_file="$MEMORY_DIR/${session}.json"

    # Write to local file
    echo "$context" > "$mem_file"

    # Fire-and-forget to MongoDB
    mongo_put "memory" "$context"
    mongo_log "memory" "save:${session}" "{\"size\":${#context}}"
}

# ── load: retrieve session memory from disk ─────────────────
# Usage: memory_load <session_name>
memory_load() {
    local session="$1"
    local mem_file="$MEMORY_DIR/${session}.json"

    if [[ -f "$mem_file" ]]; then
        cat "$mem_file"
    else
        echo "{}"
    fi
}

# ── compact: summarize and compact session history ──────────
# Stores a compacted summary of session activity in MongoDB
# Usage: memory_compact <session_name>
memory_compact() {
    local session="$1"
    local mem_file="$MEMORY_DIR/${session}.json"

    # Collect session metadata
    local meta_dir
    meta_dir="$(_sessions_dir 2>/dev/null)/$session"
    local metadata="{}"
    if [[ -f "$meta_dir/metadata" ]]; then
        local name mode status created
        name=$(grep "^name=" "$meta_dir/metadata" 2>/dev/null | cut -d= -f2)
        mode=$(grep "^mode=" "$meta_dir/metadata" 2>/dev/null | cut -d= -f2)
        status=$(grep "^status=" "$meta_dir/metadata" 2>/dev/null | cut -d= -f2)
        created=$(grep "^created=" "$meta_dir/metadata" 2>/dev/null | cut -d= -f2)
        metadata="{\"name\":\"$name\",\"mode\":\"$mode\",\"status\":\"$status\",\"created\":\"$created\"}"
    fi

    # Collect recent events from MongoDB for this session
    local events="[]"
    if $MONGO_READY; then
        events=$(mongo_get "events" "{\"key\":{\"\$regex\":\"$session\"}}" 50 2>/dev/null || echo "[]")
    fi

    # Build compacted memory document
    local compact_doc
    if command -v jq &>/dev/null; then
        compact_doc=$(jq -n \
            --arg session "$session" \
            --arg ts "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
            --argjson meta "$metadata" \
            --argjson events "$events" \
            '{
                session: $session,
                compacted_at: $ts,
                metadata: $meta,
                event_count: ($events | length),
                events_summary: ($events | map({type, key, _ts})),
                _project: "claude-cage"
            }')
    else
        compact_doc="{\"session\":\"$session\",\"compacted_at\":\"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\",\"_project\":\"claude-cage\"}"
    fi

    # Save compacted memory
    echo "$compact_doc" > "$mem_file"
    mongo_put "memory" "$compact_doc"
    mongo_log "memory" "compact:${session}" "{\"event_count\":$(echo "$events" | grep -c '"_id"' 2>/dev/null || echo 0)}"
}

# ── list: show all saved session memories ───────────────────
memory_list() {
    echo "SAVED SESSION MEMORIES:"
    printf "%-25s %-12s %-20s\n" "SESSION" "SIZE" "MODIFIED"
    printf "%-25s %-12s %-20s\n" "-------" "----" "--------"

    if [[ -d "$MEMORY_DIR" ]]; then
        for f in "$MEMORY_DIR"/*.json; do
            [[ -f "$f" ]] || continue
            local name
            name=$(basename "$f" .json)
            local size
            size=$(wc -c < "$f" 2>/dev/null || echo 0)
            local modified
            modified=$(stat -c '%y' "$f" 2>/dev/null | cut -d. -f1 || echo "unknown")
            printf "%-25s %-12s %-20s\n" "$name" "${size}B" "$modified"
        done
    fi
}

# ── clean: remove old session memories ──────────────────────
# Usage: memory_clean [days_old]  (default: 30)
memory_clean() {
    local days="${1:-30}"
    local count=0

    if [[ -d "$MEMORY_DIR" ]]; then
        while IFS= read -r -d '' f; do
            rm -f "$f"
            ((count++))
        done < <(find "$MEMORY_DIR" -name "*.json" -mtime "+$days" -print0 2>/dev/null)
    fi

    if (( count > 0 )); then
        echo "Cleaned $count memory files older than $days days"
        mongo_log "memory" "clean" "{\"removed\":$count,\"threshold_days\":$days}"
    fi
}

# ── search: find sessions by content in memory ──────────────
# Usage: memory_search <pattern>
memory_search() {
    local pattern="$1"

    if $MONGO_READY; then
        # Search MongoDB for matching memory docs
        node "$MONGO_STORE" search memory "$pattern" 10 2>/dev/null
    else
        # Fallback: grep local files
        grep -rl "$pattern" "$MEMORY_DIR" 2>/dev/null | while read -r f; do
            echo "$(basename "$f" .json)"
        done
    fi
}
