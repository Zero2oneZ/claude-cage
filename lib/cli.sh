#!/usr/bin/env bash
# cli.sh — Command implementations and help text

cmd_help() {
    cat <<'USAGE'
claude-cage — Dockerized sandbox for Claude CLI & Claude Desktop

USAGE
    claude-cage <command> [options]

COMMANDS
    start       Launch a new sandboxed Claude session
    stop        Stop a running session
    shell       Open a shell inside a running session
    status      Show status of sessions
    logs        Stream logs from a session
    list        List all sessions (running and stopped)
    destroy     Remove a session and its data
    build       Build container images
    config      Show or validate configuration
    gui         Launch interactive TUI dashboard
    version     Print version
    help        Show this help message

START OPTIONS
    claude-cage start [options]

    --mode <cli|desktop>    Run mode (default: cli)
    --name <name>           Session name (default: auto-generated)
    --mount <path>          Mount host directory into sandbox (repeatable)
    --network <policy>      Network policy: none | host | filtered (default: filtered)
    --cpus <n>              CPU limit (default: 2)
    --memory <size>         Memory limit (default: 4g)
    --gpu                   Enable GPU passthrough
    --port <host:container> Expose additional port (repeatable)
    --env <KEY=VAL>         Pass environment variable (repeatable)
    --config <file>         Use custom config file
    --api-key <key>         Anthropic API key (or set ANTHROPIC_API_KEY)
    --ephemeral             Destroy session on exit
    --no-persist            Do not persist session state between restarts

EXAMPLES
    # Start Claude CLI with current directory mounted
    claude-cage start --mode cli --mount .

    # Start Claude Desktop accessible via browser
    claude-cage start --mode desktop --port 6080:6080

    # Fully isolated session with no network
    claude-cage start --mode cli --network none --ephemeral

    # Resource-constrained session
    claude-cage start --mode cli --cpus 1 --memory 2g

USAGE
}

cmd_start() {
    local mode="cli"
    local name=""
    local network="filtered"
    local cpus="2"
    local memory="4g"
    local gpu=false
    local ephemeral=false
    local persist=true
    local config_file=""
    local api_key="${ANTHROPIC_API_KEY:-}"
    local -a mounts=()
    local -a ports=()
    local -a env_vars=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --mode)       mode="$2"; shift 2 ;;
            --name)       name="$2"; shift 2 ;;
            --mount)      mounts+=("$2"); shift 2 ;;
            --network)    network="$2"; shift 2 ;;
            --cpus)       cpus="$2"; shift 2 ;;
            --memory)     memory="$2"; shift 2 ;;
            --gpu)        gpu=true; shift ;;
            --port)       ports+=("$2"); shift 2 ;;
            --env)        env_vars+=("$2"); shift 2 ;;
            --config)     config_file="$2"; shift 2 ;;
            --api-key)    api_key="$2"; shift 2 ;;
            --ephemeral)  ephemeral=true; shift ;;
            --no-persist) persist=false; shift ;;
            *)
                echo "Error: unknown option '$1'" >&2
                exit 1
                ;;
        esac
    done

    # Validate mode
    if [[ "$mode" != "cli" && "$mode" != "desktop" ]]; then
        echo "Error: --mode must be 'cli' or 'desktop'" >&2
        exit 1
    fi

    # Validate network policy
    if [[ "$network" != "none" && "$network" != "host" && "$network" != "filtered" ]]; then
        echo "Error: --network must be 'none', 'host', or 'filtered'" >&2
        exit 1
    fi

    # Require API key
    if [[ -z "$api_key" ]]; then
        echo "Error: Anthropic API key required." >&2
        echo "Set ANTHROPIC_API_KEY or pass --api-key <key>" >&2
        exit 1
    fi

    # Load config
    if [[ -n "$config_file" ]]; then
        config_load "$config_file"
    else
        config_load_default
    fi

    # Generate session name if not provided
    if [[ -z "$name" ]]; then
        name="$(session_generate_name)"
    fi

    echo "==> Starting claude-cage session: $name (mode=$mode)"

    # Create session record
    session_create "$name" "$mode"

    # Resolve mount paths to absolute
    local -a abs_mounts=()
    for m in "${mounts[@]+"${mounts[@]}"}"; do
        abs_mounts+=("$(cd "$m" 2>/dev/null && pwd || echo "$m")")
    done

    # Build sandbox flags
    local -a sandbox_flags
    sandbox_flags=($(sandbox_build_flags "$network" "$cpus" "$memory" "$gpu"))

    # Launch container
    docker_run_session \
        "$name" \
        "$mode" \
        "$api_key" \
        "$network" \
        "$cpus" \
        "$memory" \
        "$gpu" \
        "$ephemeral" \
        "$persist" \
        abs_mounts \
        ports \
        env_vars

    # Post-launch info
    if [[ "$mode" == "desktop" ]]; then
        local vnc_port="${CAGE_DESKTOP_PORT:-6080}"
        echo "==> Claude Desktop available at: http://localhost:${vnc_port}"
    fi

    echo "==> Session '$name' is running."
    echo "    Attach:  claude-cage shell --name $name"
    echo "    Logs:    claude-cage logs --name $name"
    echo "    Stop:    claude-cage stop --name $name"
}

cmd_stop() {
    local name=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name) name="$2"; shift 2 ;;
            --all)  docker_stop_all; return ;;
            *)      name="$1"; shift ;;
        esac
    done

    if [[ -z "$name" ]]; then
        echo "Error: session name required. Usage: claude-cage stop --name <name>" >&2
        exit 1
    fi

    echo "==> Stopping session: $name"
    docker_stop_session "$name"
    session_set_status "$name" "stopped"
    echo "==> Session '$name' stopped."
}

cmd_shell() {
    local name=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name) name="$2"; shift 2 ;;
            *)      name="$1"; shift ;;
        esac
    done

    if [[ -z "$name" ]]; then
        # Attach to most recent session
        name="$(session_most_recent)"
        if [[ -z "$name" ]]; then
            echo "Error: no running sessions found." >&2
            exit 1
        fi
    fi

    echo "==> Attaching to session: $name"
    docker_exec_session "$name" /bin/bash
}

cmd_status() {
    local name=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name) name="$2"; shift 2 ;;
            *)      name="$1"; shift ;;
        esac
    done

    if [[ -n "$name" ]]; then
        docker_inspect_session "$name"
    else
        cmd_list
    fi
}

cmd_logs() {
    local name=""
    local follow=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name)   name="$2"; shift 2 ;;
            -f|--follow) follow=true; shift ;;
            *)        name="$1"; shift ;;
        esac
    done

    if [[ -z "$name" ]]; then
        name="$(session_most_recent)"
        if [[ -z "$name" ]]; then
            echo "Error: no running sessions found." >&2
            exit 1
        fi
    fi

    if $follow; then
        docker logs -f "cage-${name}"
    else
        docker logs "cage-${name}"
    fi
}

cmd_list() {
    echo "SESSIONS:"
    printf "%-20s %-10s %-10s %-20s\n" "NAME" "MODE" "STATUS" "CREATED"
    printf "%-20s %-10s %-10s %-20s\n" "----" "----" "------" "-------"
    session_list_all
}

cmd_destroy() {
    local name=""
    local force=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name)  name="$2"; shift 2 ;;
            --force) force=true; shift ;;
            *)       name="$1"; shift ;;
        esac
    done

    if [[ -z "$name" ]]; then
        echo "Error: session name required." >&2
        exit 1
    fi

    echo "==> Destroying session: $name"
    docker_destroy_session "$name" "$force"
    session_remove "$name"
    echo "==> Session '$name' destroyed."
}

cmd_build() {
    local target="${1:-all}"
    case "$target" in
        cli)     docker_build_cli ;;
        desktop) docker_build_desktop ;;
        all)     docker_build_cli; docker_build_desktop ;;
        *)
            echo "Error: build target must be 'cli', 'desktop', or 'all'" >&2
            exit 1
            ;;
    esac
}

cmd_config() {
    local action="${1:-show}"
    case "$action" in
        show)
            config_load_default
            config_print
            ;;
        validate)
            config_load_default
            config_validate && echo "Configuration is valid."
            ;;
        path)
            echo "$(config_default_path)"
            ;;
        *)
            echo "Error: config action must be 'show', 'validate', or 'path'" >&2
            exit 1
            ;;
    esac
}
