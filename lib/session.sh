#!/usr/bin/env bash
# session.sh — Session lifecycle and metadata management

_sessions_dir() {
    local dir="${CAGE_CFG[session_dir]:-$CAGE_DATA_DIR/sessions}"
    mkdir -p "$dir"
    echo "$dir"
}

session_generate_name() {
    # Generate a human-friendly session name: adjective-noun-XXXX
    local adjectives=("swift" "calm" "bold" "keen" "warm" "cool" "bright" "deep"
                      "fair" "glad" "pure" "wise" "vast" "safe" "lean" "clear")
    local nouns=("fox" "owl" "elk" "ray" "bay" "oak" "gem" "arc"
                 "key" "pen" "dot" "fin" "orb" "cap" "rod" "hub")

    local adj="${adjectives[$((RANDOM % ${#adjectives[@]}))]}"
    local noun="${nouns[$((RANDOM % ${#nouns[@]}))]}"
    local suffix
    suffix=$(printf '%04x' $((RANDOM % 65536)))

    echo "${adj}-${noun}-${suffix}"
}

session_create() {
    local name="$1"
    local mode="$2"
    local dir
    dir="$(_sessions_dir)/$name"
    mkdir -p "$dir"

    # Write session metadata
    cat > "$dir/metadata" <<EOF
name=$name
mode=$mode
status=running
created=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
container=cage-${name}
EOF
}

session_set_status() {
    local name="$1"
    local status="$2"
    local meta
    meta="$(_sessions_dir)/$name/metadata"

    if [[ -f "$meta" ]]; then
        # Update status line in-place
        if grep -q "^status=" "$meta"; then
            sed -i "s/^status=.*/status=$status/" "$meta"
        else
            echo "status=$status" >> "$meta"
        fi
    fi
}

session_get_status() {
    local name="$1"
    local meta
    meta="$(_sessions_dir)/$name/metadata"

    if [[ -f "$meta" ]]; then
        grep "^status=" "$meta" 2>/dev/null | cut -d= -f2
    else
        # Check docker directly
        local container_status
        container_status=$(docker inspect -f '{{.State.Status}}' "cage-${name}" 2>/dev/null) || echo "unknown"
        echo "$container_status"
    fi
}

session_list_all() {
    local dir
    dir="$(_sessions_dir)"

    # Also check for docker containers we might have lost track of
    local running_containers
    running_containers=$(docker ps --filter "label=managed-by=claude-cage" --format '{{.Names}}' 2>/dev/null | sed 's/^cage-//')

    local shown=()

    # Sessions from metadata
    if [[ -d "$dir" ]]; then
        for session_dir in "$dir"/*/; do
            [[ -d "$session_dir" ]] || continue
            local meta="$session_dir/metadata"
            [[ -f "$meta" ]] || continue

            local name mode status created
            name=$(grep "^name=" "$meta" 2>/dev/null | cut -d= -f2)
            mode=$(grep "^mode=" "$meta" 2>/dev/null | cut -d= -f2)
            created=$(grep "^created=" "$meta" 2>/dev/null | cut -d= -f2)

            # Reconcile status with docker
            local docker_status
            docker_status=$(docker inspect -f '{{.State.Status}}' "cage-${name}" 2>/dev/null) || docker_status="removed"
            status="$docker_status"
            session_set_status "$name" "$status"

            printf "%-20s %-10s %-10s %-20s\n" "$name" "$mode" "$status" "$created"
            shown+=("$name")
        done
    fi

    # Containers without metadata (orphans)
    for c in $running_containers; do
        local is_shown=false
        for s in "${shown[@]+"${shown[@]}"}"; do
            [[ "$s" == "$c" ]] && is_shown=true
        done
        if ! $is_shown; then
            local c_mode
            c_mode=$(docker inspect -f '{{index .Config.Labels "cage.mode"}}' "cage-${c}" 2>/dev/null || echo "?")
            printf "%-20s %-10s %-10s %-20s\n" "$c" "$c_mode" "running" "(orphan)"
        fi
    done
}

session_most_recent() {
    local dir
    dir="$(_sessions_dir)"

    # Find most recently created running session
    local latest=""
    local latest_time=0

    if [[ -d "$dir" ]]; then
        for session_dir in "$dir"/*/; do
            [[ -d "$session_dir" ]] || continue
            local meta="$session_dir/metadata"
            [[ -f "$meta" ]] || continue

            local name
            name=$(grep "^name=" "$meta" 2>/dev/null | cut -d= -f2)

            # Check if actually running
            if docker inspect -f '{{.State.Running}}' "cage-${name}" 2>/dev/null | grep -q true; then
                local mtime
                mtime=$(stat -c %Y "$meta" 2>/dev/null || echo 0)
                if (( mtime > latest_time )); then
                    latest_time=$mtime
                    latest="$name"
                fi
            fi
        done
    fi

    echo "$latest"
}

session_remove() {
    local name="$1"
    local dir
    dir="$(_sessions_dir)/$name"
    if [[ -d "$dir" ]]; then
        rm -rf "$dir"
    fi
}
