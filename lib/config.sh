#!/usr/bin/env bash
# config.sh — Configuration loading and validation

CAGE_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/claude-cage"
CAGE_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/claude-cage"

# Default values (overridden by config file)
declare -A CAGE_CFG=(
    [mode]="cli"
    [network]="filtered"
    [cpus]="2"
    [memory]="4g"
    [desktop_port]="6080"
    [vnc_port]="5900"
    [image_cli]="claude-cage-cli:latest"
    [image_desktop]="claude-cage-desktop:latest"
    [log_level]="info"
    [session_dir]=""
    [persist]="true"
    [allowed_hosts]="api.anthropic.com,cdn.anthropic.com"
    [dns]="1.1.1.1"
    [read_only_root]="true"
    [seccomp_profile]="default"
    [max_sessions]="5"
)

config_default_path() {
    echo "$CAGE_ROOT/config/default.yaml"
}

config_load_default() {
    local default_cfg="$CAGE_ROOT/config/default.yaml"
    if [[ -f "$default_cfg" ]]; then
        _parse_yaml "$default_cfg"
    fi

    # User overrides
    local user_cfg="$CAGE_CONFIG_DIR/config.yaml"
    if [[ -f "$user_cfg" ]]; then
        _parse_yaml "$user_cfg"
    fi

    # Set derived values
    if [[ -z "${CAGE_CFG[session_dir]}" ]]; then
        CAGE_CFG[session_dir]="$CAGE_DATA_DIR/sessions"
    fi
}

config_load() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        echo "Error: config file not found: $file" >&2
        exit 1
    fi
    config_load_default
    _parse_yaml "$file"
}

# Minimal YAML parser (flat key: value only — no nested structures)
_parse_yaml() {
    local file="$1"
    while IFS= read -r line; do
        # Skip comments and blank lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" =~ ^[[:space:]]*$ ]] && continue

        # Match "key: value"
        if [[ "$line" =~ ^[[:space:]]*([a-zA-Z_][a-zA-Z0-9_]*)[[:space:]]*:[[:space:]]*(.*) ]]; then
            local key="${BASH_REMATCH[1]}"
            local val="${BASH_REMATCH[2]}"
            # Strip quotes
            val="${val%\"}"
            val="${val#\"}"
            val="${val%\'}"
            val="${val#\'}"
            # Trim trailing whitespace
            val="${val%"${val##*[![:space:]]}"}"
            CAGE_CFG[$key]="$val"
        fi
    done < "$file"
}

config_validate() {
    local valid=true

    # Validate mode
    if [[ "${CAGE_CFG[mode]}" != "cli" && "${CAGE_CFG[mode]}" != "desktop" ]]; then
        echo "Error: config 'mode' must be 'cli' or 'desktop'" >&2
        valid=false
    fi

    # Validate network
    local net="${CAGE_CFG[network]}"
    if [[ "$net" != "none" && "$net" != "host" && "$net" != "filtered" ]]; then
        echo "Error: config 'network' must be 'none', 'host', or 'filtered'" >&2
        valid=false
    fi

    # Validate numeric fields
    if ! [[ "${CAGE_CFG[cpus]}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
        echo "Error: config 'cpus' must be a number" >&2
        valid=false
    fi

    if ! [[ "${CAGE_CFG[memory]}" =~ ^[0-9]+[gmkGMK]?$ ]]; then
        echo "Error: config 'memory' must be a size (e.g. 4g, 512m)" >&2
        valid=false
    fi

    $valid
}

config_print() {
    echo "claude-cage configuration:"
    echo "---"
    for key in $(echo "${!CAGE_CFG[@]}" | tr ' ' '\n' | sort); do
        # Mask API keys
        if [[ "$key" == *"key"* || "$key" == *"secret"* ]]; then
            echo "  $key: ****"
        else
            echo "  $key: ${CAGE_CFG[$key]}"
        fi
    done
}

config_get() {
    echo "${CAGE_CFG[$1]:-$2}"
}
