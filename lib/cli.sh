#!/usr/bin/env bash
# cli.sh — Command implementations and help text

cmd_help() {
    cat <<'HELPTEXT'
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
    init        Initialize a new project with tree infrastructure
    tree        Inspect and operate on a project tree
    ptc         Pass-Through Coordination — intent down, artifacts up
    train       Training data extraction and LoRA pipeline
    design      Architect-mode blueprint system
    docs        Circular documentation system — the perfect circle
    ipfs        IPFS content-addressed storage
    vsearch     Semantic vector search across everything
    porkbun     Domain management via Porkbun API
    icons       Design assets from Noun Project
    fork        Git federation — bidirectional forking with sovereignty
    hf          Hugging Face Hub — models, datasets, inference
    gui         Launch interactive TUI dashboard
    web         Launch web dashboard (http://localhost:5000)
    observe     Show observability dashboard for running sessions
    version     Print version
    help        Show this help message

INIT
    claude-cage init <directory> [--name <project-name>]

TREE
    claude-cage tree show [tree.json]              Show tree hierarchy
    claude-cage tree count [tree.json]             Count nodes
    claude-cage tree node <tree.json> <node-id>    Get node details
    claude-cage tree blast-radius <tree.json> <targets>  Calculate blast radius
    claude-cage tree route <tree.json> <intent>    Route intent through tree
    claude-cage tree seed <tree.json> [project]    Seed tree into MongoDB

PTC (Pass-Through Coordination)
    claude-cage ptc run "intent" [--tree path] [--live]   Run full PTC cycle
    claude-cage ptc exec <node> "task" [--live]           Execute at a specific leaf
    claude-cage ptc leaves [tree.json]                    Show all worker nodes
    claude-cage ptc tree [tree.json]                      Show tree structure

DESIGN (Architect Mode)
    claude-cage design create "intent"           Create a new blueprint
    claude-cage design list [--status X]         List all blueprints
    claude-cage design show <blueprint-id>       Show blueprint detail
    claude-cage design build <id> [--live]       Decompose to PTC builders
    claude-cage design verify <id>               Verify against acceptance criteria
    claude-cage design search "query"            Vector search blueprints

DOCS (Circular Documentation)
    claude-cage docs status                      Coverage + staleness stats
    claude-cage docs generate <node_id>          Generate doc for one node
    claude-cage docs generate-all                Generate docs for all nodes
    claude-cage docs check                       Check all docs for staleness
    claude-cage docs refresh [node_id]           Regenerate stale doc(s)
    claude-cage docs interconnect                Compute full bidirectional graph
    claude-cage docs search "query" [N]          Semantic search across docs
    claude-cage docs show <node_id>              Display doc with cross-refs
    claude-cage docs graph                       Output interconnection graph JSON

IPFS
    claude-cage ipfs status                      Check IPFS daemon connectivity
    claude-cage ipfs migrate                     Backfill IPFS CIDs for artifacts

VECTOR SEARCH
    claude-cage vsearch "query"                  Semantic search across everything

DOMAINS (Porkbun)
    claude-cage porkbun ping                     Test API connectivity
    claude-cage porkbun check <domain>           Check domain availability
    claude-cage porkbun domains                  List account domains
    claude-cage porkbun dns <domain>             Show DNS records
    claude-cage porkbun dns-create <domain> <type> <content> [name]
    claude-cage porkbun dns-delete <domain> <id> Delete DNS record
    claude-cage porkbun ssl <domain>             Get free SSL bundle
    claude-cage porkbun pricing [tld]            TLD pricing
    claude-cage porkbun forward <domain> <url>   URL forwarding

ICONS (Noun Project)
    claude-cage icons search <query> [--limit N] Search icons
    claude-cage icons get <id>                   Icon metadata
    claude-cage icons download <id> <path>       Download icon to file
    claude-cage icons batch <query> <dir>        Batch download icons
    claude-cage icons collections <query>        Search collections
    claude-cage icons usage                      API usage limits

FEDERATION (Git Sovereignty)
    claude-cage fork init <upstream-url> <dir> [--name n]  Create a fork
    claude-cage fork branch <dir> <url> <branch>           Branch mode
    claude-cage fork pull <dir> [--nodes n1,n2]            Sync from upstream
    claude-cage fork push <dir> [--nodes n1,n2]            Push as PR
    claude-cage fork status <dir>                          Ahead/behind status
    claude-cage fork verify <dir>                          Verify tree trust
    claude-cage fork diff <tree-a> <tree-b>                Structural diff
    claude-cage fork forks                                 List known forks

HUGGING FACE
    claude-cage hf status                        Token + cache status
    claude-cage hf download <repo> [--files f]   Download model/dataset
    claude-cage hf embed <text>                  Get embedding
    claude-cage hf chat <message> [--model m]    Chat completion
    claude-cage hf search <query>                Search models
    claude-cage hf upload <repo> <path>          Upload to Hub
    claude-cage hf repo-create <name>            Create Hub repo
    claude-cage hf cache                         Cache status

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

HELPTEXT
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

    # API key is optional — Claude Max users authenticate via `claude login` inside the container
    if [[ -z "$api_key" ]]; then
        echo "==> No API key set. Use 'claude login' inside the container to authenticate with your Max subscription."
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

# ── Init: scaffold a new project with tree infrastructure ──────
cmd_init() {
    local project_dir=""
    local project_name=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name) project_name="$2"; shift 2 ;;
            *)      project_dir="$1"; shift ;;
        esac
    done

    if [[ -z "$project_dir" ]]; then
        echo "Usage: claude-cage init <directory> [--name <project-name>]" >&2
        exit 1
    fi

    # Default name from directory basename
    if [[ -z "$project_name" ]]; then
        project_name="$(basename "$project_dir")"
    fi

    # Create directory if it doesn't exist
    mkdir -p "$project_dir"
    project_dir="$(cd "$project_dir" && pwd)"

    echo "==> Initializing project: $project_name"
    echo "    Directory: $project_dir"

    # Initialize tree
    tree_init "$project_dir" "$project_name"

    echo ""
    echo "==> Project '$project_name' initialized."
    echo "    Edit tree.json to add your nodes."
    echo "    View tree: claude-cage tree $project_dir"
}

# ── Tree: inspect and operate on a project tree ────────────────
cmd_tree() {
    local action="${1:-show}"
    shift || true
    local tree_path="${1:-./tree.json}"

    case "$action" in
        show)
            tree_show "$tree_path"
            ;;
        load|count)
            local count
            count=$(tree_load "$tree_path")
            echo "Nodes: $count"
            ;;
        node)
            local node_id="${2:-}"
            if [[ -z "$node_id" ]]; then
                echo "Usage: claude-cage tree node <tree.json> <node-id>" >&2
                exit 1
            fi
            tree_node "$tree_path" "$node_id"
            ;;
        blast-radius)
            local targets="${2:-}"
            if [[ -z "$targets" ]]; then
                echo "Usage: claude-cage tree blast-radius <tree.json> <target1,target2,...>" >&2
                exit 1
            fi
            tree_blast_radius "$tree_path" "$targets"
            ;;
        route)
            local intent="${2:-}"
            if [[ -z "$intent" ]]; then
                echo "Usage: claude-cage tree route <tree.json> <intent keywords>" >&2
                exit 1
            fi
            tree_route "$tree_path" "$intent"
            ;;
        seed)
            local project="${2:-$(basename "$(dirname "$tree_path")")}"
            tree_seed "$tree_path" "$project"
            ;;
        *)
            echo "Usage: claude-cage tree <show|count|node|blast-radius|route|seed> [tree.json] [args]" >&2
            exit 1
            ;;
    esac
}

# ── Web: launch the web dashboard ─────────────────────────────
cmd_web() {
    local port="${CAGE_WEB_PORT:-5000}"
    echo "==> Starting claude-cage web dashboard"
    echo "    http://localhost:${port}"
    python3 "$CAGE_ROOT/web/app.py"
}

# ── PTC: Pass-Through Coordination ─────────────────────────────
cmd_ptc() {
    local action="${1:-}"
    shift || true

    case "$action" in
        run)
            # claude-cage ptc run "intent" [--tree path] [--target node] [--live] [--json]
            local intent=""
            local tree_path="${CAGE_ROOT}/tree.json"
            local target=""
            local extra_args=()

            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --tree)   tree_path="$2"; shift 2 ;;
                    --target) target="$2"; shift 2 ;;
                    --live)   extra_args+=(--live); shift ;;
                    --json)   extra_args+=(--json); shift ;;
                    -v|--verbose) extra_args+=(--verbose); shift ;;
                    *)        intent="$1"; shift ;;
                esac
            done

            if [[ -z "$intent" ]]; then
                echo "Usage: claude-cage ptc run \"intent\" [--tree path] [--target node] [--live]" >&2
                exit 1
            fi

            local -a cmd=(python3 -m ptc.engine --tree "$tree_path" --intent "$intent")
            [[ -n "$target" ]] && cmd+=(--target "$target")
            cmd+=("${extra_args[@]}")

            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" "${cmd[@]}"
            ;;
        exec)
            # claude-cage ptc exec <node-id> "task" [--tree path] [--live]
            local node_id=""
            local task=""
            local tree_path="${CAGE_ROOT}/tree.json"
            local extra_args=()

            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --tree) tree_path="$2"; shift 2 ;;
                    --live) extra_args+=(--live); shift ;;
                    --json) extra_args+=(--json); shift ;;
                    *)
                        if [[ -z "$node_id" ]]; then
                            node_id="$1"
                        else
                            task="$1"
                        fi
                        shift
                        ;;
                esac
            done

            if [[ -z "$node_id" || -z "$task" ]]; then
                echo "Usage: claude-cage ptc exec <node-id> \"task\" [--tree path] [--live]" >&2
                exit 1
            fi

            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.engine --tree "$tree_path" --node "$node_id" --task "$task" "${extra_args[@]}"
            ;;
        leaves)
            # claude-cage ptc leaves [--tree path]
            local tree_path="${1:-${CAGE_ROOT}/tree.json}"
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.engine --tree "$tree_path" --show-leaves
            ;;
        tree)
            # claude-cage ptc tree [--tree path]
            local tree_path="${1:-${CAGE_ROOT}/tree.json}"
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.engine --tree "$tree_path" --show-tree
            ;;
        help|"")
            cat <<'PTCHELP'
PTC — Pass-Through Coordination

  Intent flows DOWN. Artifacts flow UP. One pattern. Every scale.

USAGE
    claude-cage ptc run "intent" [--tree path] [--target node] [--live] [-v]
    claude-cage ptc exec <node-id> "task" [--tree path] [--live]
    claude-cage ptc leaves [tree.json]
    claude-cage ptc tree [tree.json]

EXAMPLES
    # Dry run — see what would happen
    claude-cage ptc run "add GPU monitoring"

    # Live execution — leaves do the work
    claude-cage ptc run "fix security sandbox" --live

    # Target a specific department
    claude-cage ptc run "update auth" --target dept:security

    # Execute directly at a leaf node
    claude-cage ptc exec capt:sandbox "verify all flags" --live

    # Use a different tree
    claude-cage ptc run "implement wire protocol" --tree gentlyos/tree.json

    # Show all workers
    claude-cage ptc leaves

PTCHELP
            ;;
        *)
            echo "Error: unknown ptc action '$action'" >&2
            echo "Run 'claude-cage ptc help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── Train: training data extraction and LoRA pipeline ──────────
cmd_train() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        extract)
            # claude-cage train extract [--source local|mongodb] [--output dir]
            local source="local"
            local output=""
            local extra_args=()

            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --source) source="$2"; shift 2 ;;
                    --output|-o) output="$2"; shift 2 ;;
                    *) extra_args+=("$1"); shift ;;
                esac
            done

            [[ -z "$output" ]] && output="${CAGE_ROOT}/training/datasets/latest"

            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.training extract --source "$source" --output "$output" "${extra_args[@]}"
            ;;
        pipeline)
            # claude-cage train pipeline [--tree path] [--model name]
            local tree_path="${CAGE_ROOT}/tree.json"
            local model=""
            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --tree) tree_path="$2"; shift 2 ;;
                    --model) model="$2"; shift 2 ;;
                    *) shift ;;
                esac
            done

            local -a cmd=(python3 -m ptc.lora pipeline --tree "$tree_path")
            [[ -n "$model" ]] && cmd+=(--model "$model")

            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" "${cmd[@]}"
            ;;
        stack)
            # claude-cage train stack [--tree path]
            local tree_path="${1:-${CAGE_ROOT}/tree.json}"
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.lora stack --tree "$tree_path"
            ;;
        preview)
            # claude-cage train preview [--trace file]
            local trace="${1:-}"
            if [[ -z "$trace" ]]; then
                # Find most recent trace
                trace=$(ls -t "${CAGE_ROOT}/training/traces/"*.json 2>/dev/null | head -1)
                if [[ -z "$trace" ]]; then
                    echo "No traces found. Run: claude-cage ptc run \"intent\" --json > training/traces/trace.json" >&2
                    exit 1
                fi
            fi
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.training preview --trace "$trace"
            ;;
        stats)
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.training stats --source local
            ;;
        help|"")
            cat <<'TRAINHELP'
TRAIN — Training Data Extraction & LoRA Pipeline

  Every PTC trace IS a chain of thought. Extract. Train. Stack. Grow.

USAGE
    claude-cage train extract [--source local|mongodb] [--output dir]
    claude-cage train pipeline [--tree path] [--model name]
    claude-cage train stack [--tree path]
    claude-cage train preview [trace.json]
    claude-cage train stats

FLOW
    1. Run PTC to generate traces:
       claude-cage ptc run "intent" --json > training/traces/my-trace.json

    2. Extract training data (Alpaca, ShareGPT, CoT formats):
       claude-cage train extract

    3. Generate LoRA pipeline (configs for all adapters):
       claude-cage train pipeline

    4. See stacking order (base → scale → department → captain):
       claude-cage train stack

    5. Train adapters (on your 3090s):
       python3 training/scripts/train_ptc_base.py

TRAINHELP
            ;;
        *)
            echo "Error: unknown train action '$action'" >&2
            echo "Run 'claude-cage train help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── Observe: show observability dashboard ──────────────────────
cmd_observe() {
    echo "OBSERVABILITY DASHBOARD"
    echo "═══════════════════════════════════════"
    echo ""
    docker ps --filter "label=managed-by=claude-cage" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null || echo "  (no containers running)"
    echo ""
    echo "RESOURCE USAGE:"
    docker stats --no-stream --filter "label=managed-by=claude-cage" --format "  {{.Name}}: CPU={{.CPUPerc}} MEM={{.MemUsage}} NET={{.NetIO}}" 2>/dev/null || echo "  (no containers running)"
}

# ── Design: architect-mode blueprint system ────────────────────
cmd_design() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        create)
            local intent="$*"
            if [[ -z "$intent" ]]; then
                echo "Usage: claude-cage design create \"intent\"" >&2
                exit 1
            fi
            architect_create "$intent"
            ;;
        list)
            local status_filter=""
            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --status) status_filter="$2"; shift 2 ;;
                    *) shift ;;
                esac
            done
            architect_list "$status_filter"
            ;;
        show)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage design show <blueprint-id>" >&2
                exit 1
            fi
            architect_show "$1"
            ;;
        build)
            local bp_id="${1:-}"
            shift || true
            if [[ -z "$bp_id" ]]; then
                echo "Usage: claude-cage design build <blueprint-id> [--live]" >&2
                exit 1
            fi
            # Get tasks from blueprint, then run them through PTC
            CAGE_ROOT="$CAGE_ROOT" PYTHONPATH="$CAGE_ROOT" \
                python3 -m ptc.architect tasks "$bp_id"
            ;;
        verify)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage design verify <blueprint-id>" >&2
                exit 1
            fi
            architect_validate "$1"
            ;;
        search)
            local query="$*"
            if [[ -z "$query" ]]; then
                echo "Usage: claude-cage design search \"query\"" >&2
                exit 1
            fi
            architect_search "$query"
            ;;
        cache)
            local sub="${1:-stats}"
            case "$sub" in
                stats)
                    echo "BLUEPRINT CACHE"
                    echo "═══════════════════════════════════════"
                    architect_list
                    ;;
                *)
                    echo "Usage: claude-cage design cache stats" >&2
                    exit 1
                    ;;
            esac
            ;;
        help|"")
            cat <<'DESIGNHELP'
DESIGN — Architect Mode Blueprint System

  Claude designs. PTC decomposes. Builders execute.
  The architect doesn't pick up hammers. The architect designs them.

USAGE
    claude-cage design create "intent"            Create a new blueprint
    claude-cage design list [--status X]          List all blueprints
    claude-cage design show <blueprint-id>        Show blueprint detail
    claude-cage design build <id> [--live]        Decompose to PTC builders
    claude-cage design verify <id>                Check results vs acceptance
    claude-cage design search "query"             Vector search blueprints
    claude-cage design cache stats                Show cache statistics

EXAMPLES
    # Design a new feature
    claude-cage design create "add webhook notification system"

    # Find existing designs
    claude-cage design search "webhook"

    # Check what a blueprint will build
    claude-cage design show blueprint:add-webhook-abc123

    # Send to builders
    claude-cage design build blueprint:add-webhook-abc123 --live

DESIGNHELP
            ;;
        *)
            echo "Error: unknown design action '$action'" >&2
            echo "Run 'claude-cage design help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── Docs: circular documentation system ────────────────────────
cmd_docs() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        status)
            docs_status
            ;;
        generate)
            local node_id="${1:-}"
            if [[ -z "$node_id" ]]; then
                echo "Usage: claude-cage docs generate <node_id>" >&2
                exit 1
            fi
            docs_generate "$node_id"
            ;;
        generate-all)
            docs_generate_all
            ;;
        check|check-stale)
            docs_check_stale
            ;;
        refresh)
            local node_id="${1:-}"
            docs_refresh "$node_id"
            ;;
        interconnect)
            docs_interconnect
            ;;
        search)
            local query="${1:-}"
            local limit="${2:-10}"
            if [[ -z "$query" ]]; then
                echo "Usage: claude-cage docs search \"query\" [limit]" >&2
                exit 1
            fi
            docs_search "$query" "$limit"
            ;;
        show)
            local node_id="${1:-}"
            if [[ -z "$node_id" ]]; then
                echo "Usage: claude-cage docs show <node_id>" >&2
                exit 1
            fi
            docs_show "$node_id"
            ;;
        graph)
            docs_graph
            ;;
        help|"")
            cat <<'DOCSHELP'
DOCS — Circular Documentation System

  Documentation as code. Bidirectional. Staleness-tracked. One circle.
  Change one side, the other knows.

USAGE
    claude-cage docs status                      Coverage + staleness stats
    claude-cage docs generate <node_id>          Generate doc for one node
    claude-cage docs generate-all                Generate docs for all nodes
    claude-cage docs check                       Check all docs for staleness
    claude-cage docs refresh [node_id]           Regenerate stale doc(s)
    claude-cage docs interconnect                Compute full bidirectional graph
    claude-cage docs search "query" [N]          Semantic search across docs
    claude-cage docs show <node_id>              Display doc with cross-refs
    claude-cage docs graph                       Output interconnection graph JSON

EXAMPLES
    # See coverage
    claude-cage docs status

    # Generate all docs with cross-refs
    claude-cage docs generate-all

    # Build the circle (interconnection graph)
    claude-cage docs interconnect

    # Check what drifted
    claude-cage docs check

    # Search docs semantically
    claude-cage docs search "container isolation"

DOCSHELP
            ;;
        *)
            echo "Error: unknown docs action '$action'" >&2
            echo "Run 'claude-cage docs help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── IPFS: content-addressed storage ────────────────────────────
cmd_ipfs() {
    local action="${1:-status}"
    shift || true

    case "$action" in
        status)  ipfs_status ;;
        migrate) ipfs_migrate ;;
        help|"")
            echo "IPFS — Content-Addressed Artifact Storage"
            echo ""
            echo "  claude-cage ipfs status    Check IPFS daemon connectivity"
            echo "  claude-cage ipfs migrate   Backfill IPFS CIDs for existing artifacts"
            ;;
        *)
            echo "Error: unknown ipfs action '$action'" >&2
            exit 1
            ;;
    esac
}

# ── Vector Search: semantic search across everything ───────────
cmd_vsearch() {
    local query="$*"
    if [[ -z "$query" ]]; then
        echo "Usage: claude-cage vsearch \"query\"" >&2
        echo ""
        echo "Semantic search across all artifacts, traces, commits, and blueprints."
        exit 1
    fi
    vsearch "$query"
}

# ── Porkbun: domain management ────────────────────────────────
cmd_porkbun() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        ping)       porkbun_ping ;;
        check)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage porkbun check <domain>" >&2; exit 1
            fi
            porkbun_check "$1"
            ;;
        domains)    porkbun_domains ;;
        dns)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage porkbun dns <domain>" >&2; exit 1
            fi
            porkbun_dns "$1"
            ;;
        dns-create) porkbun_dns_create "$@" ;;
        dns-delete)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage porkbun dns-delete <domain> <id>" >&2; exit 1
            fi
            porkbun_dns_delete "$1" "$2"
            ;;
        ssl)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage porkbun ssl <domain>" >&2; exit 1
            fi
            porkbun_ssl "$1"
            ;;
        pricing)    porkbun_pricing "${1:-}" ;;
        forward)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage porkbun forward <domain> <url> [type]" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.porkbun forward "$@"
            ;;
        forwards)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage porkbun forwards <domain>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.porkbun forwards "$1"
            ;;
        nameservers)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage porkbun nameservers <domain>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.porkbun nameservers "$1"
            ;;
        help|"")
            cat <<'PORKBUNHELP'
PORKBUN — Domain Management

USAGE
    claude-cage porkbun ping                     Test API connectivity
    claude-cage porkbun check <domain>           Check domain availability
    claude-cage porkbun domains                  List account domains
    claude-cage porkbun dns <domain>             Show DNS records
    claude-cage porkbun dns-create <domain> <type> <content> [name] [ttl]
    claude-cage porkbun dns-delete <domain> <id> Delete DNS record
    claude-cage porkbun ssl <domain>             Get free SSL bundle
    claude-cage porkbun pricing [tld]            TLD pricing
    claude-cage porkbun forward <domain> <url>   Add URL forwarding
    claude-cage porkbun forwards <domain>        List URL forwards
    claude-cage porkbun nameservers <domain>     Show nameservers

ENV
    PORKBUN_ENABLED=true
    PORKBUN_API_KEY=<your key>
    PORKBUN_SECRET_KEY=<your secret>
PORKBUNHELP
            ;;
        *)
            echo "Error: unknown porkbun action '$action'" >&2
            echo "Run 'claude-cage porkbun help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── Icons: Noun Project design assets ─────────────────────────
cmd_icons() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        search)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage icons search <query> [--limit N]" >&2; exit 1
            fi
            np_search "$@"
            ;;
        get)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage icons get <id>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.nounproject get "$1"
            ;;
        download)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage icons download <id> <path> [--type svg|png]" >&2; exit 1
            fi
            np_download "$@"
            ;;
        batch)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage icons batch <query> <dir> [--limit N]" >&2; exit 1
            fi
            np_batch "$@"
            ;;
        collections)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage icons collections <query>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.nounproject collections "$1"
            ;;
        autocomplete)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage icons autocomplete <query>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.nounproject autocomplete "$1"
            ;;
        usage)
            np_usage
            ;;
        help|"")
            cat <<'ICONSHELP'
ICONS — Noun Project Design Assets

USAGE
    claude-cage icons search <query> [--limit N]    Search icons
    claude-cage icons get <id>                      Icon metadata
    claude-cage icons download <id> <path>          Download icon file
    claude-cage icons batch <query> <dir> [--limit] Batch download
    claude-cage icons collections <query>           Search collections
    claude-cage icons autocomplete <query>          Search suggestions
    claude-cage icons usage                         API usage limits

ENV
    NOUNPROJECT_ENABLED=true
    NOUNPROJECT_KEY=<your key>
    NOUNPROJECT_SECRET=<your secret>
ICONSHELP
            ;;
        *)
            echo "Error: unknown icons action '$action'" >&2
            echo "Run 'claude-cage icons help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── Fork: git federation with sovereignty ─────────────────────
cmd_fork() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        init)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage fork init <upstream-url> <dir> [--name n]" >&2; exit 1
            fi
            federation_fork "$@"
            ;;
        branch)
            if [[ -z "${1:-}" || -z "${2:-}" || -z "${3:-}" ]]; then
                echo "Usage: claude-cage fork branch <dir> <upstream-url> <branch>" >&2; exit 1
            fi
            federation_branch "$@"
            ;;
        pull)
            federation_pull "${1:-.}" "${@:2}"
            ;;
        push)
            federation_push "${1:-.}" "${@:2}"
            ;;
        status)
            federation_status "${1:-.}"
            ;;
        verify)
            federation_verify "${1:-.}"
            ;;
        diff)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage fork diff <tree-a> <tree-b>" >&2; exit 1
            fi
            federation_diff "$1" "$2"
            ;;
        forks)
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.federation forks
            ;;
        help|"")
            cat <<'FORKHELP'
FORK — Git Federation with Sovereignty

  Bidirectional forking where the tree doesn't break
  and sovereignty doesn't lose trust. Fork decides. Always.

USAGE
    claude-cage fork init <upstream-url> <dir> [--name n]  Create a fork
    claude-cage fork branch <dir> <url> <branch>           Branch mode
    claude-cage fork pull [dir] [--nodes n1,n2]            Sync from upstream
    claude-cage fork push [dir] [--nodes n1,n2]            Push as PR
    claude-cage fork status [dir]                          Ahead/behind status
    claude-cage fork verify [dir]                          Verify tree trust
    claude-cage fork diff <tree-a> <tree-b>                Structural tree diff
    claude-cage fork forks                                 List known forks

EXAMPLES
    # Fork claude-cage into a new project
    claude-cage fork init git@github.com:Zero2oneZ/claude-cage.git ./my-project --name my-project

    # Check sync status
    claude-cage fork status ./my-project

    # Pull security updates from upstream
    claude-cage fork pull ./my-project --nodes dept:security

    # Verify trust chain
    claude-cage fork verify ./my-project
FORKHELP
            ;;
        *)
            echo "Error: unknown fork action '$action'" >&2
            echo "Run 'claude-cage fork help' for usage." >&2
            exit 1
            ;;
    esac
}

# ── HF: Hugging Face Hub integration ─────────────────────────
cmd_hf() {
    local action="${1:-help}"
    shift || true

    case "$action" in
        status)     hf_status ;;
        download)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf download <repo> [--files f1,f2]" >&2; exit 1
            fi
            hf_download "$@"
            ;;
        embed)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf embed <text>" >&2; exit 1
            fi
            hf_embed "$@"
            ;;
        chat)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf chat <message> [--model m]" >&2; exit 1
            fi
            hf_chat "$@"
            ;;
        generate)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf generate <prompt> [--model m]" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.huggingface generate "$@"
            ;;
        search)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf search <query>" >&2; exit 1
            fi
            hf_search "$@"
            ;;
        datasets)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf datasets <query>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.huggingface datasets "$1"
            ;;
        upload)
            if [[ -z "${1:-}" || -z "${2:-}" ]]; then
                echo "Usage: claude-cage hf upload <repo> <path>" >&2; exit 1
            fi
            hf_upload "$@"
            ;;
        repo-create)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf repo-create <name> [--type model|dataset|space]" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.huggingface repo-create "$@"
            ;;
        repo-info)
            if [[ -z "${1:-}" ]]; then
                echo "Usage: claude-cage hf repo-info <repo>" >&2; exit 1
            fi
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.huggingface repo-info "$1"
            ;;
        cache)      hf_cache ;;
        cache-clean)
            CAGE_ROOT="${CAGE_ROOT}" PYTHONPATH="${CAGE_ROOT}" \
                python3 -m ptc.huggingface cache-clean "$@"
            ;;
        help|"")
            cat <<'HFHELP'
HF — Hugging Face Hub Integration

  Models, datasets, inference, embeddings. The ML backbone.

USAGE
    claude-cage hf status                        Token + cache status
    claude-cage hf download <repo> [--files f]   Download model/dataset
    claude-cage hf embed <text> [--model m]      Get embedding
    claude-cage hf chat <message> [--model m]    Chat completion
    claude-cage hf generate <prompt> [--model m] Text generation
    claude-cage hf search <query>                Search models
    claude-cage hf datasets <query>              Search datasets
    claude-cage hf upload <repo> <path>          Upload to Hub
    claude-cage hf repo-create <name>            Create Hub repo
    claude-cage hf repo-info <repo>              Repository metadata
    claude-cage hf cache                         Cache status
    claude-cage hf cache-clean [--days N]        Clean old cache

ENV
    HF_ENABLED=true
    HF_TOKEN=<your token>
    HF_CACHE_DIR=<optional cache path>
    HF_DEFAULT_EMBEDDING_MODEL=sentence-transformers/all-MiniLM-L6-v2
    HF_INFERENCE_PROVIDER=hf-inference
HFHELP
            ;;
        *)
            echo "Error: unknown hf action '$action'" >&2
            echo "Run 'claude-cage hf help' for usage." >&2
            exit 1
            ;;
    esac
}
