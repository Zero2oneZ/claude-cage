#!/usr/bin/env bash
# observability.sh — Session observability, metrics, and health tracking
# Pattern: Anthropic cookbook observability + cost tracking patterns
# Tracks container resource usage, session health, and operational metrics

OBS_COLLECTION="metrics"

# ── snapshot: capture current container metrics ─────────────
# Usage: obs_snapshot <session_name>
obs_snapshot() {
    local session="$1"
    local container="cage-${session}"

    # Check container exists
    if ! docker inspect "$container" &>/dev/null; then
        return 1
    fi

    # Gather metrics
    local cpu mem net pids status
    cpu=$(docker stats --no-stream --format "{{.CPUPerc}}" "$container" 2>/dev/null || echo "0%")
    mem=$(docker stats --no-stream --format "{{.MemUsage}}" "$container" 2>/dev/null || echo "0B/0B")
    net=$(docker stats --no-stream --format "{{.NetIO}}" "$container" 2>/dev/null || echo "0B/0B")
    pids=$(docker stats --no-stream --format "{{.PIDs}}" "$container" 2>/dev/null || echo "0")
    status=$(docker inspect -f '{{.State.Status}}' "$container" 2>/dev/null || echo "unknown")

    local now
    now="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

    local doc
    if command -v jq &>/dev/null; then
        doc=$(jq -n \
            --arg session "$session" \
            --arg container "$container" \
            --arg cpu "$cpu" \
            --arg mem "$mem" \
            --arg net "$net" \
            --arg pids "$pids" \
            --arg status "$status" \
            --arg ts "$now" \
            '{
                session: $session,
                container: $container,
                cpu: $cpu,
                memory: $mem,
                network_io: $net,
                pids: $pids,
                status: $status,
                _ts: $ts,
                _type: "container_metrics"
            }')
    else
        doc="{\"session\":\"$session\",\"cpu\":\"$cpu\",\"memory\":\"$mem\",\"status\":\"$status\",\"_ts\":\"$now\"}"
    fi

    # Fire-and-forget to MongoDB
    mongo_put "$OBS_COLLECTION" "$doc"

    echo "$doc"
}

# ── health: quick health check for a session ────────────────
# Usage: obs_health <session_name>
# Returns: healthy, degraded, or unhealthy
obs_health() {
    local session="$1"
    local container="cage-${session}"
    local health="healthy"
    local issues=()

    # Check container is running
    local status
    status=$(docker inspect -f '{{.State.Status}}' "$container" 2>/dev/null) || {
        echo "unhealthy:not_found"
        return 1
    }
    [[ "$status" != "running" ]] && { health="unhealthy"; issues+=("status:$status"); }

    # Check OOM kill
    local oom
    oom=$(docker inspect -f '{{.State.OOMKilled}}' "$container" 2>/dev/null)
    [[ "$oom" == "true" ]] && { health="unhealthy"; issues+=("oom_killed"); }

    # Check restart count
    local restarts
    restarts=$(docker inspect -f '{{.RestartCount}}' "$container" 2>/dev/null || echo 0)
    (( restarts > 3 )) && { health="degraded"; issues+=("restarts:$restarts"); }

    # Check memory usage (warn above 80%)
    local mem_pct
    mem_pct=$(docker stats --no-stream --format "{{.MemPerc}}" "$container" 2>/dev/null | tr -d '%')
    if [[ -n "$mem_pct" ]]; then
        local mem_int=${mem_pct%.*}
        (( mem_int > 90 )) && { health="unhealthy"; issues+=("mem:${mem_pct}%"); }
        (( mem_int > 80 && mem_int <= 90 )) && { [[ "$health" == "healthy" ]] && health="degraded"; issues+=("mem_warn:${mem_pct}%"); }
    fi

    local result="${health}"
    if (( ${#issues[@]} > 0 )); then
        result="${health}:$(IFS=,; echo "${issues[*]}")"
    fi

    echo "$result"

    # Log health check
    mongo_log "health" "${session}" "{\"status\":\"$health\",\"issues\":\"${issues[*]}\"}"
}

# ── dashboard: show metrics for all running sessions ────────
obs_dashboard() {
    echo "CAGE OBSERVABILITY DASHBOARD"
    echo "════════════════════════════════════════════════════"
    printf "%-18s %-8s %-8s %-15s %-6s %-10s\n" "SESSION" "STATUS" "CPU" "MEMORY" "PIDs" "HEALTH"
    printf "%-18s %-8s %-8s %-15s %-6s %-10s\n" "-------" "------" "---" "------" "----" "------"

    local containers
    containers=$(docker ps --filter "label=managed-by=claude-cage" --format "{{.Names}}" 2>/dev/null)

    if [[ -z "$containers" ]]; then
        echo "  (no running sessions)"
        return
    fi

    while IFS= read -r container; do
        local session="${container#cage-}"
        local cpu mem pids status health

        cpu=$(docker stats --no-stream --format "{{.CPUPerc}}" "$container" 2>/dev/null || echo "-")
        mem=$(docker stats --no-stream --format "{{.MemUsage}}" "$container" 2>/dev/null || echo "-")
        pids=$(docker stats --no-stream --format "{{.PIDs}}" "$container" 2>/dev/null || echo "-")
        status=$(docker inspect -f '{{.State.Status}}' "$container" 2>/dev/null || echo "?")
        health=$(obs_health "$session" 2>/dev/null | cut -d: -f1)

        printf "%-18s %-8s %-8s %-15s %-6s %-10s\n" "$session" "$status" "$cpu" "$mem" "$pids" "$health"
    done <<< "$containers"

    echo "════════════════════════════════════════════════════"

    # MongoDB stats if available
    if $MONGO_READY; then
        local event_count artifact_count
        event_count=$(node "$MONGO_STORE" count events 2>/dev/null | grep -o '"count":[0-9]*' | grep -o '[0-9]*' || echo "?")
        artifact_count=$(node "$MONGO_STORE" count artifacts 2>/dev/null | grep -o '"count":[0-9]*' | grep -o '[0-9]*' || echo "?")
        echo "MongoDB: ${event_count} events, ${artifact_count} artifacts"
    fi
}

# ── log_timing: log operation duration ──────────────────────
# Usage: obs_log_timing <operation> <start_epoch> [metadata_json]
obs_log_timing() {
    local operation="$1" start="$2" meta="${3:-'{}'}"
    local end
    end=$(date +%s)
    local duration=$(( end - start ))

    mongo_log "timing" "$operation" "{\"duration_s\":$duration,\"meta\":$meta}"
}

# ── events_summary: aggregate event stats from MongoDB ──────
obs_events_summary() {
    if ! $MONGO_READY; then
        echo "MongoDB not available"
        return 1
    fi

    echo "EVENT SUMMARY"
    echo "─────────────────────────────"

    # Get distinct event types
    local types
    types=$(node "$MONGO_STORE" distinct events type 2>/dev/null)
    if [[ -n "$types" ]]; then
        echo "Event types: $types"
    fi

    # Get counts per type using aggregation
    local agg
    agg=$(node "$MONGO_STORE" aggregate events '[{"$group":{"_id":"$type","count":{"$sum":1}}},{"$sort":{"count":-1}}]' 2>/dev/null)
    if [[ -n "$agg" ]]; then
        echo ""
        printf "%-20s %s\n" "TYPE" "COUNT"
        printf "%-20s %s\n" "----" "-----"
        echo "$agg" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    for item in data:
        print(f\"  {item['_id']:<18} {item['count']}\")
except: pass
" 2>/dev/null || echo "$agg"
    fi
}
