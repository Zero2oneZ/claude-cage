#!/usr/bin/env bash
# lifecycle.sh — Container lifecycle control: max sessions, idle reap, memory reap, GC

# ── enforce max sessions ─────────────────────────────────────────
# Returns 0 if a new session can be launched, 1 if at limit
lifecycle_enforce_max() {
    local max="${CAGE_CFG[max_sessions]:-5}"
    local running
    running=$(docker ps -q --filter "label=managed-by=claude-cage" 2>/dev/null | wc -l)

    if (( running >= max )); then
        echo "==> Session limit reached ($running/$max running)."
        echo "    Attempting to reap idle sessions..."
        lifecycle_reap_idle
        # Recount
        running=$(docker ps -q --filter "label=managed-by=claude-cage" 2>/dev/null | wc -l)
        if (( running >= max )); then
            echo "Error: still at session limit ($running/$max). Stop a session first." >&2
            echo "  claude-cage list      — show sessions" >&2
            echo "  claude-cage stop <n>  — stop a session" >&2
            echo "  claude-cage reap      — auto-stop idle sessions" >&2
            return 1
        fi
        echo "==> Freed slot. Proceeding."
    fi
    return 0
}

# ── reap idle containers ─────────────────────────────────────────
# Idle = CPU usage < 1% sustained. Checks idle_timeout from label or config.
lifecycle_reap_idle() {
    local default_timeout="${CAGE_CFG[idle_timeout]:-60}"
    local now
    now=$(date +%s)
    local reaped=0

    local containers
    containers=$(docker ps --filter "label=managed-by=claude-cage" --format '{{.Names}}' 2>/dev/null)
    [[ -z "$containers" ]] && return 0

    while IFS= read -r container; do
        [[ -z "$container" ]] && continue
        local name="${container#cage-}"

        # Get idle timeout from label or default
        local timeout
        timeout=$(docker inspect -f '{{index .Config.Labels "cage.idle_timeout"}}' "$container" 2>/dev/null)
        timeout="${timeout:-$default_timeout}"

        # Get container start time
        local started_at
        started_at=$(docker inspect -f '{{.State.StartedAt}}' "$container" 2>/dev/null) || continue
        local started_epoch
        started_epoch=$(date -d "$started_at" +%s 2>/dev/null) || continue

        local uptime_min=$(( (now - started_epoch) / 60 ))

        # Skip if container hasn't been up long enough
        if (( uptime_min < timeout )); then
            continue
        fi

        # Check CPU usage — if < 1%, consider idle
        local cpu_pct
        cpu_pct=$(docker stats --no-stream --format '{{.CPUPerc}}' "$container" 2>/dev/null | tr -d '%')
        # Handle empty or non-numeric
        if [[ -z "$cpu_pct" ]]; then
            continue
        fi
        # Compare as integer (truncate decimal)
        local cpu_int="${cpu_pct%%.*}"
        cpu_int="${cpu_int:-0}"

        if (( cpu_int < 1 )); then
            echo "  Reaping idle: $name (up ${uptime_min}m, CPU ${cpu_pct}%)"
            docker stop "$container" 2>/dev/null || true
            session_set_status "$name" "stopped"
            mongo_log "lifecycle" "reap:idle" \
                "{\"session\":\"$name\",\"uptime_min\":$uptime_min,\"cpu\":\"$cpu_pct\"}"
            ((reaped++)) || true
        fi
    done <<< "$containers"

    if (( reaped > 0 )); then
        echo "==> Reaped $reaped idle session(s)."
    else
        echo "==> No idle sessions to reap."
    fi
}

# ── reap memory-heavy containers ─────────────────────────────────
lifecycle_reap_memory() {
    local warn_pct="${CAGE_CFG[memory_warn]:-80}"
    local kill_pct="${CAGE_CFG[memory_kill]:-95}"
    local reaped=0

    local containers
    containers=$(docker ps --filter "label=managed-by=claude-cage" --format '{{.Names}}' 2>/dev/null)
    [[ -z "$containers" ]] && return 0

    while IFS= read -r container; do
        [[ -z "$container" ]] && continue
        local name="${container#cage-}"

        local mem_pct
        mem_pct=$(docker stats --no-stream --format '{{.MemPerc}}' "$container" 2>/dev/null | tr -d '%')
        [[ -z "$mem_pct" ]] && continue

        local mem_int="${mem_pct%%.*}"
        mem_int="${mem_int:-0}"

        if (( mem_int >= kill_pct )); then
            echo "  KILL: $name using ${mem_pct}% memory (limit: ${kill_pct}%)"
            docker stop "$container" 2>/dev/null || true
            session_set_status "$name" "stopped"
            mongo_log "lifecycle" "reap:memory" \
                "{\"session\":\"$name\",\"mem_pct\":\"$mem_pct\",\"action\":\"kill\"}"
            ((reaped++)) || true
        elif (( mem_int >= warn_pct )); then
            echo "  WARN: $name using ${mem_pct}% memory (warn at ${warn_pct}%)"
            mongo_log "lifecycle" "memory:warn" \
                "{\"session\":\"$name\",\"mem_pct\":\"$mem_pct\"}"
        fi
    done <<< "$containers"

    if (( reaped > 0 )); then
        echo "==> Stopped $reaped memory-heavy session(s)."
    else
        echo "==> No memory-heavy sessions to stop."
    fi
}

# ── garbage collect dead containers + orphan volumes ─────────────
lifecycle_gc() {
    local removed=0
    local volumes_removed=0

    # Find stopped/exited/dead cage containers
    local dead
    dead=$(docker ps -a --filter "label=managed-by=claude-cage" \
        --filter "status=exited" --filter "status=dead" \
        --format '{{.Names}}' 2>/dev/null)

    if [[ -n "$dead" ]]; then
        while IFS= read -r container; do
            [[ -z "$container" ]] && continue
            local name="${container#cage-}"
            echo "  Removing: $container"
            docker rm "$container" 2>/dev/null || true
            # Clean up data volume if it exists
            docker volume rm "cage-data-${name}" 2>/dev/null && ((volumes_removed++)) || true
            # Clean session metadata
            local meta_dir="${CAGE_CFG[session_dir]:-$CAGE_DATA_DIR/sessions}/$name"
            if [[ -d "$meta_dir" ]]; then
                rm -rf "$meta_dir"
            fi
            mongo_log "lifecycle" "gc:container" "{\"session\":\"$name\"}"
            ((removed++)) || true
        done <<< "$dead"
    fi

    # Also clean created (never started) containers
    local created
    created=$(docker ps -a --filter "label=managed-by=claude-cage" \
        --filter "status=created" \
        --format '{{.Names}}' 2>/dev/null)

    if [[ -n "$created" ]]; then
        while IFS= read -r container; do
            [[ -z "$container" ]] && continue
            local name="${container#cage-}"
            echo "  Removing (never started): $container"
            docker rm "$container" 2>/dev/null || true
            docker volume rm "cage-data-${name}" 2>/dev/null && ((volumes_removed++)) || true
            ((removed++)) || true
        done <<< "$created"
    fi

    # Prune dangling cage volumes (cage-data-* without a container)
    local all_volumes
    all_volumes=$(docker volume ls -q --filter "name=cage-data-" 2>/dev/null)
    if [[ -n "$all_volumes" ]]; then
        while IFS= read -r vol; do
            [[ -z "$vol" ]] && continue
            local vol_name="${vol#cage-data-}"
            # Check if any container (running or stopped) uses this volume
            local in_use
            in_use=$(docker ps -a --filter "name=cage-${vol_name}" -q 2>/dev/null)
            if [[ -z "$in_use" ]]; then
                echo "  Removing orphan volume: $vol"
                docker volume rm "$vol" 2>/dev/null && ((volumes_removed++)) || true
            fi
        done <<< "$all_volumes"
    fi

    echo "==> GC: removed $removed container(s), $volumes_removed volume(s)."
    mongo_log "lifecycle" "gc:complete" \
        "{\"containers\":$removed,\"volumes\":$volumes_removed}"
}

# ── summary: one line per container ──────────────────────────────
lifecycle_summary() {
    local containers
    containers=$(docker ps --filter "label=managed-by=claude-cage" --format '{{.Names}}' 2>/dev/null)

    if [[ -z "$containers" ]]; then
        echo "No running cage sessions."
        return
    fi

    printf "%-22s %-14s %-8s %-10s %-10s\n" "SESSION" "PROJECT" "MEM %" "UPTIME" "CPU %"
    printf "%-22s %-14s %-8s %-10s %-10s\n" "-------" "-------" "-----" "------" "-----"

    local now
    now=$(date +%s)

    while IFS= read -r container; do
        [[ -z "$container" ]] && continue
        local name="${container#cage-}"

        local project
        project=$(docker inspect -f '{{index .Config.Labels "cage.project"}}' "$container" 2>/dev/null)
        project="${project:--}"

        local mem_pct
        mem_pct=$(docker stats --no-stream --format '{{.MemPerc}}' "$container" 2>/dev/null)
        mem_pct="${mem_pct:-?}"

        local cpu_pct
        cpu_pct=$(docker stats --no-stream --format '{{.CPUPerc}}' "$container" 2>/dev/null)
        cpu_pct="${cpu_pct:-?}"

        local started_at
        started_at=$(docker inspect -f '{{.State.StartedAt}}' "$container" 2>/dev/null)
        local uptime="?"
        if [[ -n "$started_at" ]]; then
            local started_epoch
            started_epoch=$(date -d "$started_at" +%s 2>/dev/null)
            if [[ -n "$started_epoch" ]]; then
                local diff=$(( now - started_epoch ))
                if (( diff < 3600 )); then
                    uptime="$(( diff / 60 ))m"
                else
                    uptime="$(( diff / 3600 ))h$(( (diff % 3600) / 60 ))m"
                fi
            fi
        fi

        printf "%-22s %-14s %-8s %-10s %-10s\n" "$name" "$project" "$mem_pct" "$uptime" "$cpu_pct"
    done <<< "$containers"
}
