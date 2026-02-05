#!/usr/bin/env bash
# docker.sh — Docker lifecycle management for cage sessions

CAGE_LABEL="managed-by=claude-cage"

docker_check() {
    if ! command -v docker &>/dev/null; then
        echo "Error: Docker is not installed or not in PATH." >&2
        exit 1
    fi
    if ! docker info &>/dev/null 2>&1; then
        echo "Error: Docker daemon is not running or you lack permissions." >&2
        echo "Try: sudo systemctl start docker" >&2
        exit 1
    fi
}

docker_build_cli() {
    echo "==> Building Claude CLI image..."
    docker build \
        -t "$(config_get image_cli claude-cage-cli:latest)" \
        -f "$CAGE_ROOT/docker/cli/Dockerfile" \
        "$CAGE_ROOT/docker/cli"
    echo "==> CLI image built successfully."
}

docker_build_desktop() {
    echo "==> Building Claude Desktop image..."
    docker build \
        -t "$(config_get image_desktop claude-cage-desktop:latest)" \
        -f "$CAGE_ROOT/docker/desktop/Dockerfile" \
        "$CAGE_ROOT/docker/desktop"
    echo "==> Desktop image built successfully."
}

docker_run_session() {
    local name="$1"
    local mode="$2"
    local api_key="$3"
    local network="$4"
    local cpus="$5"
    local memory="$6"
    local gpu="$7"
    local ephemeral="$8"
    local persist="$9"

    # Array arguments passed by name
    local -n _mounts="${10}"
    local -n _ports="${11}"
    local -n _env_vars="${12}"

    docker_check

    # Ensure filtered network exists
    if [[ "$network" == "filtered" ]]; then
        sandbox_create_network
    fi

    local container_name="cage-${name}"
    local image

    if [[ "$mode" == "cli" ]]; then
        image="$(config_get image_cli claude-cage-cli:latest)"
    else
        image="$(config_get image_desktop claude-cage-desktop:latest)"
    fi

    # Build docker run command
    local -a cmd=(docker run)

    # Detach or interactive
    if [[ "$mode" == "cli" ]]; then
        cmd+=(-it)
    else
        cmd+=(-d)
    fi

    # Container naming and labels
    cmd+=(--name "$container_name")
    cmd+=(--label "$CAGE_LABEL")
    cmd+=(--label "cage.mode=$mode")
    cmd+=(--label "cage.session=$name")
    cmd+=(--hostname "cage-${name}")

    # Auto-remove if ephemeral
    if [[ "$ephemeral" == "true" ]]; then
        cmd+=(--rm)
    fi

    # Restart policy for desktop
    if [[ "$mode" == "desktop" && "$ephemeral" != "true" ]]; then
        cmd+=(--restart unless-stopped)
    fi

    # Sandbox flags
    local -a sandbox_flags
    sandbox_flags=($(sandbox_build_flags "$network" "$cpus" "$memory" "$gpu"))
    cmd+=("${sandbox_flags[@]}")

    # API key
    cmd+=(-e "ANTHROPIC_API_KEY=$api_key")

    # Additional environment variables
    for ev in "${_env_vars[@]+"${_env_vars[@]}"}"; do
        cmd+=(-e "$ev")
    done

    # Persistent volume for session data
    if [[ "$persist" == "true" ]]; then
        local session_vol="cage-data-${name}"
        cmd+=(-v "${session_vol}:/home/cageuser/.claude")
    fi

    # Host directory mounts (bind mounts, read-write under /workspace)
    for m in "${_mounts[@]+"${_mounts[@]}"}"; do
        local mount_target="/workspace/$(basename "$m")"
        cmd+=(-v "${m}:${mount_target}:rw")
    done

    # Port mappings
    for p in "${_ports[@]+"${_ports[@]}"}"; do
        cmd+=(-p "$p")
    done

    # Desktop-specific ports
    if [[ "$mode" == "desktop" ]]; then
        local desktop_port
        desktop_port="$(config_get desktop_port 6080)"
        cmd+=(-p "${desktop_port}:6080")
    fi

    # Working directory
    cmd+=(-w /workspace)

    # Image
    cmd+=("$image")

    echo "==> Launching container: $container_name"
    "${cmd[@]}"

    # Post-launch: apply network filtering
    if [[ "$network" == "filtered" && "$mode" == "desktop" ]]; then
        sandbox_apply_network_filter "$container_name"
    fi

    # Verify sandbox
    if [[ "$mode" == "desktop" ]]; then
        sandbox_verify "$container_name"
    fi
}

docker_stop_session() {
    local name="$1"
    local container_name="cage-${name}"
    docker stop "$container_name" 2>/dev/null || true
}

docker_stop_all() {
    echo "==> Stopping all claude-cage sessions..."
    local containers
    containers=$(docker ps -q --filter "label=$CAGE_LABEL" 2>/dev/null)
    if [[ -n "$containers" ]]; then
        echo "$containers" | xargs docker stop
        echo "==> All sessions stopped."
    else
        echo "==> No running sessions found."
    fi
}

docker_destroy_session() {
    local name="$1"
    local force="$2"
    local container_name="cage-${name}"

    if [[ "$force" == "true" ]]; then
        docker rm -f "$container_name" 2>/dev/null || true
    else
        docker stop "$container_name" 2>/dev/null || true
        docker rm "$container_name" 2>/dev/null || true
    fi

    # Remove persistent volume
    docker volume rm "cage-data-${name}" 2>/dev/null || true
}

docker_exec_session() {
    local name="$1"
    shift
    local container_name="cage-${name}"
    docker exec -it "$container_name" "$@"
}

docker_inspect_session() {
    local name="$1"
    local container_name="cage-${name}"

    local status
    status=$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null) || {
        echo "Session '$name' not found."
        return 1
    }

    local started
    started=$(docker inspect -f '{{.State.StartedAt}}' "$container_name" 2>/dev/null)
    local image
    image=$(docker inspect -f '{{.Config.Image}}' "$container_name" 2>/dev/null)
    local mem
    mem=$(docker inspect -f '{{.HostConfig.Memory}}' "$container_name" 2>/dev/null)
    local cpus
    cpus=$(docker inspect -f '{{.HostConfig.NanoCpus}}' "$container_name" 2>/dev/null)

    echo "Session: $name"
    echo "  Status:   $status"
    echo "  Image:    $image"
    echo "  Started:  $started"
    echo "  Memory:   $mem bytes"
    echo "  CPUs:     $(echo "scale=1; ${cpus:-0}/1000000000" | bc 2>/dev/null || echo "N/A")"

    # Sandbox verification
    sandbox_verify "$container_name"
}
