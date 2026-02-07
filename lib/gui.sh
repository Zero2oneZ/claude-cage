#!/usr/bin/env bash
# gui.sh — Interactive TUI screens for claude-cage
# Depends on: tui.sh, config.sh, docker.sh, session.sh, sandbox.sh

GUI_CURRENT_SCREEN="dashboard"
GUI_SELECTED_IDX=0
GUI_SESSION_LIST=()
GUI_TOAST_MSG=""
GUI_TOAST_TYPE=""

# ═══════════════════════════════════════════════════════════════════
# Main entry point
# ═══════════════════════════════════════════════════════════════════
gui_main() {
    config_load_default
    tui_init

    while $TUI_RUNNING; do
        tui_get_size

        case "$GUI_CURRENT_SCREEN" in
            dashboard)   gui_screen_dashboard ;;
            new_session) gui_screen_new_session ;;
            session)     gui_screen_session_detail ;;
            config)      gui_screen_config ;;
            help)        gui_screen_help ;;
        esac
    done

    tui_cleanup
}

# ═══════════════════════════════════════════════════════════════════
# Header — shown on every screen
# ═══════════════════════════════════════════════════════════════════
gui_draw_header() {
    local title="${1:-Dashboard}"

    # Top bar
    tui_cursor_to 1 1
    echo -ne "${BG_BLUE}${C_WHITE}${C_BOLD}"
    printf "%-${TUI_COLS}s" ""
    tui_cursor_to 1 3
    echo -ne "◈ claude-cage"
    tui_cursor_to 1 $((TUI_COLS - 20))
    echo -ne "v${CAGE_VERSION:-0.1.0}"
    echo -ne "${C_RESET}"

    # Screen title
    tui_cursor_to 2 1
    echo -ne "${BG_BBLUE}${C_WHITE}"
    printf "%-${TUI_COLS}s" ""
    tui_cursor_to 2 3
    echo -ne "${C_BOLD}${title}${C_RESET}"

    # Separator
    tui_hline 3 1 "$TUI_COLS" "─" "$C_GRAY"
}

# ═══════════════════════════════════════════════════════════════════
# Dashboard — main screen with session list & stats
# ═══════════════════════════════════════════════════════════════════
gui_screen_dashboard() {
    tui_clear
    gui_draw_header "Dashboard"

    # ── Stats row ────────────────────────────────────────────────
    local running=0 stopped=0 total=0
    gui_refresh_sessions
    total=${#GUI_SESSION_LIST[@]}

    for entry in "${GUI_SESSION_LIST[@]+"${GUI_SESSION_LIST[@]}"}"; do
        local status
        status=$(echo "$entry" | cut -d'|' -f3)
        case "$status" in
            running) (( running++ )) ;;
            stopped|exited) (( stopped++ )) ;;
        esac
    done

    local stats_row=5
    local box_w=$(( (TUI_COLS - 8) / 3 ))
    (( box_w < 18 )) && box_w=18

    # Running box
    tui_box "$stats_row" 2 "$box_w" 4 "" "$C_GREEN"
    tui_print $((stats_row + 1)) 4 "${C_GREEN}${C_BOLD}${running}${C_RESET}"
    tui_print $((stats_row + 2)) 4 "${C_DIM}Running${C_RESET}"

    # Stopped box
    local col2=$(( 2 + box_w + 2 ))
    tui_box "$stats_row" "$col2" "$box_w" 4 "" "$C_YELLOW"
    tui_print $((stats_row + 1)) $((col2 + 2)) "${C_YELLOW}${C_BOLD}${stopped}${C_RESET}"
    tui_print $((stats_row + 2)) $((col2 + 2)) "${C_DIM}Stopped${C_RESET}"

    # Total box
    local col3=$(( col2 + box_w + 2 ))
    tui_box "$stats_row" "$col3" "$box_w" 4 "" "$C_CYAN"
    tui_print $((stats_row + 1)) $((col3 + 2)) "${C_CYAN}${C_BOLD}${total}${C_RESET}"
    tui_print $((stats_row + 2)) $((col3 + 2)) "${C_DIM}Total${C_RESET}"

    # ── Session table ────────────────────────────────────────────
    local table_start=10
    tui_print $table_start 2 "${C_BOLD}${C_WHITE}Sessions${C_RESET}"

    if (( total == 0 )); then
        tui_print $((table_start + 2)) 4 "${C_DIM}No sessions found. Press ${C_RESET}${C_BOLD}n${C_RESET}${C_DIM} to create one.${C_RESET}"
    else
        local col_w_name=22
        local col_w_mode=10
        local col_w_status=12
        local col_w_created=24

        tui_table_header $((table_start + 1)) 4 \
            "$col_w_name" "$col_w_mode" "$col_w_status" "$col_w_created" -- \
            "NAME" "MODE" "STATUS" "CREATED"

        local max_visible=$(( TUI_ROWS - table_start - 6 ))
        (( max_visible < 1 )) && max_visible=1

        # Ensure selected index is in bounds
        (( GUI_SELECTED_IDX >= total )) && GUI_SELECTED_IDX=$((total - 1))
        (( GUI_SELECTED_IDX < 0 )) && GUI_SELECTED_IDX=0

        local scroll_offset=0
        if (( GUI_SELECTED_IDX >= max_visible )); then
            scroll_offset=$(( GUI_SELECTED_IDX - max_visible + 1 ))
        fi

        local i=0
        for entry in "${GUI_SESSION_LIST[@]+"${GUI_SESSION_LIST[@]}"}"; do
            if (( i < scroll_offset )); then
                (( i++ ))
                continue
            fi
            if (( i - scroll_offset >= max_visible )); then
                break
            fi

            local name mode status created
            IFS='|' read -r name mode status created <<< "$entry"

            local display_row=$(( table_start + 3 + i - scroll_offset ))
            local selected_flag="false"
            (( i == GUI_SELECTED_IDX )) && selected_flag="true"

            # Format status with color
            local status_display="$status"

            tui_table_row "$display_row" 4 "$selected_flag" \
                "$col_w_name" "$col_w_mode" "$col_w_status" "$col_w_created" -- \
                "$name" "$mode" "$status_display" "$created"

            # Add colored status indicator
            tui_cursor_to "$display_row" $((4 + col_w_name + col_w_mode))
            if [[ "$selected_flag" == "true" ]]; then
                echo -ne "${BG_BBLUE}"
            fi
            case "$status" in
                running) echo -ne "${C_GREEN}● running${C_RESET}" ;;
                stopped) echo -ne "${C_YELLOW}○ stopped${C_RESET}" ;;
                exited)  echo -ne "${C_RED}○ exited${C_RESET}" ;;
                *)       echo -ne "${C_GRAY}○ $status${C_RESET}" ;;
            esac

            (( i++ ))
        done

        # Scroll indicator
        if (( total > max_visible )); then
            tui_print $((table_start + 3 + max_visible)) 4 \
                "${C_DIM}... $((total - max_visible - scroll_offset)) more (scroll with ↑↓)${C_RESET}"
        fi
    fi

    # ── Key hints ────────────────────────────────────────────────
    tui_keyhints $((TUI_ROWS)) \
        "n" "New" \
        "Enter" "Details" \
        "s" "Shell" \
        "x" "Stop" \
        "d" "Destroy" \
        "c" "Config" \
        "?" "Help" \
        "q" "Quit"

    # ── Handle input ─────────────────────────────────────────────
    local key
    key=$(tui_read_key)

    case "$key" in
        UP)
            (( GUI_SELECTED_IDX > 0 )) && (( GUI_SELECTED_IDX-- ))
            ;;
        DOWN)
            (( GUI_SELECTED_IDX < ${#GUI_SESSION_LIST[@]} - 1 )) && (( GUI_SELECTED_IDX++ )) || true
            ;;
        ENTER)
            if (( ${#GUI_SESSION_LIST[@]} > 0 )); then
                GUI_CURRENT_SCREEN="session"
            fi
            ;;
        n|N)
            GUI_CURRENT_SCREEN="new_session"
            ;;
        s)
            gui_action_shell
            ;;
        x)
            gui_action_stop
            ;;
        d)
            gui_action_destroy
            ;;
        c)
            GUI_CURRENT_SCREEN="config"
            ;;
        "?"|h)
            GUI_CURRENT_SCREEN="help"
            ;;
        r|R)
            gui_refresh_sessions
            ;;
        QUIT|ESCAPE)
            TUI_RUNNING=false
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════
# New Session wizard
# ═══════════════════════════════════════════════════════════════════
gui_screen_new_session() {
    local mode="cli"
    local name=""
    local network="filtered"
    local cpus="2"
    local memory="4g"
    local mount_path=""
    local ephemeral="no"
    local current_field=0
    local num_fields=7

    while true; do
        tui_clear
        gui_draw_header "New Session"

        local form_row=5
        local label_col=4
        local value_col=24

        tui_print $form_row $label_col "${C_BOLD}${C_WHITE}Create a new sandboxed Claude session${C_RESET}"
        tui_hline $((form_row + 1)) $label_col $((TUI_COLS - 8)) "─" "$C_GRAY"

        # Field rendering helper
        _field() {
            local idx="$1" row="$2" label="$3" value="$4" hint="${5:-}"
            local prefix="${C_WHITE}"
            local marker="  "
            if (( idx == current_field )); then
                prefix="${C_BCYAN}${C_BOLD}"
                marker="${C_BCYAN}▸ ${C_RESET}"
            fi
            tui_cursor_to "$row" $label_col
            echo -ne "${marker}${prefix}${label}:${C_RESET}"
            tui_cursor_to "$row" $value_col
            if (( idx == current_field )); then
                echo -ne "${BG_GRAY}${C_WHITE}${C_BOLD} ${value} ${C_RESET}"
            else
                echo -ne "${C_WHITE} ${value}${C_RESET}"
            fi
            if [[ -n "$hint" ]] && (( idx == current_field )); then
                echo -ne "  ${C_DIM}${hint}${C_RESET}"
            fi
        }

        local r=$((form_row + 3))
        _field 0 $((r))     "Mode"       "$mode"       "← → to toggle: cli | desktop"
        _field 1 $((r + 2)) "Name"       "${name:-<auto>}" "Enter to edit"
        _field 2 $((r + 4)) "Network"    "$network"    "← → to toggle: none | filtered | host"
        _field 3 $((r + 6)) "CPUs"       "$cpus"       "← → to adjust"
        _field 4 $((r + 8)) "Memory"     "$memory"     "← → to adjust"
        _field 5 $((r + 10)) "Mount"     "${mount_path:-<none>}" "Enter to set path"
        _field 6 $((r + 12)) "Ephemeral" "$ephemeral"  "← → yes | no"

        # Preview box
        local preview_row=$((r + 15))
        tui_print "$preview_row" $label_col "${C_BOLD}${C_CYAN}Preview command:${C_RESET}"
        local cmd="claude-cage start --mode $mode"
        [[ -n "$name" ]] && cmd+=" --name $name"
        cmd+=" --network $network --cpus $cpus --memory $memory"
        [[ -n "$mount_path" ]] && cmd+=" --mount $mount_path"
        [[ "$ephemeral" == "yes" ]] && cmd+=" --ephemeral"

        tui_print $((preview_row + 1)) $label_col "${C_DIM}\$ ${cmd}${C_RESET}"

        # Key hints
        tui_keyhints $((TUI_ROWS)) \
            "↑↓" "Navigate" \
            "←→" "Change value" \
            "Enter" "Edit/Confirm" \
            "F5" "Launch" \
            "Esc" "Cancel"

        # Also show launch hint
        tui_print $((TUI_ROWS - 2)) $label_col \
            "${C_GREEN}${C_BOLD}Press Enter on any field to edit, or L to launch session${C_RESET}"

        # Input
        local key
        key=$(tui_read_key)

        case "$key" in
            UP)
                (( current_field > 0 )) && (( current_field-- ))
                ;;
            DOWN)
                (( current_field < num_fields - 1 )) && (( current_field++ ))
                ;;
            LEFT)
                case $current_field in
                    0) [[ "$mode" == "desktop" ]] && mode="cli" || mode="desktop" ;;
                    2)
                        case "$network" in
                            host)     network="filtered" ;;
                            filtered) network="none" ;;
                            none)     network="host" ;;
                        esac ;;
                    3)
                        local n="${cpus%.*}"
                        (( n > 1 )) && cpus="$((n - 1))"
                        ;;
                    4)
                        local n="${memory%g}"
                        (( n > 1 )) && memory="$((n - 1))g"
                        ;;
                    6) [[ "$ephemeral" == "yes" ]] && ephemeral="no" || ephemeral="yes" ;;
                esac
                ;;
            RIGHT)
                case $current_field in
                    0) [[ "$mode" == "cli" ]] && mode="desktop" || mode="cli" ;;
                    2)
                        case "$network" in
                            none)     network="filtered" ;;
                            filtered) network="host" ;;
                            host)     network="none" ;;
                        esac ;;
                    3)
                        local n="${cpus%.*}"
                        (( n < 16 )) && cpus="$((n + 1))"
                        ;;
                    4)
                        local n="${memory%g}"
                        (( n < 64 )) && memory="$((n + 1))g"
                        ;;
                    6) [[ "$ephemeral" == "no" ]] && ephemeral="yes" || ephemeral="no" ;;
                esac
                ;;
            ENTER)
                case $current_field in
                    1) # Edit name
                        tui_cursor_show
                        stty echo icanon 2>/dev/null || true
                        tui_cursor_to $((r + 2)) $value_col
                        echo -ne "${C_RESET}                              "
                        tui_cursor_to $((r + 2)) $value_col
                        echo -ne "> "
                        read -r name
                        stty -echo -icanon 2>/dev/null || true
                        tui_cursor_hide
                        ;;
                    5) # Edit mount path
                        tui_cursor_show
                        stty echo icanon 2>/dev/null || true
                        tui_cursor_to $((r + 10)) $value_col
                        echo -ne "${C_RESET}                              "
                        tui_cursor_to $((r + 10)) $value_col
                        echo -ne "> "
                        read -r mount_path
                        stty -echo -icanon 2>/dev/null || true
                        tui_cursor_hide
                        ;;
                esac
                ;;
            l|L)
                # Launch session
                gui_launch_session "$mode" "$name" "$network" "$cpus" "$memory" "$mount_path" "$ephemeral"
                GUI_CURRENT_SCREEN="dashboard"
                return
                ;;
            ESCAPE|QUIT)
                GUI_CURRENT_SCREEN="dashboard"
                return
                ;;
        esac
    done
}

# ═══════════════════════════════════════════════════════════════════
# Session detail screen
# ═══════════════════════════════════════════════════════════════════
gui_screen_session_detail() {
    if (( ${#GUI_SESSION_LIST[@]} == 0 )); then
        GUI_CURRENT_SCREEN="dashboard"
        return
    fi

    local entry="${GUI_SESSION_LIST[$GUI_SELECTED_IDX]}"
    local name mode status created
    IFS='|' read -r name mode status created <<< "$entry"

    while true; do
        tui_clear
        gui_draw_header "Session: $name"

        local r=5
        local label_col=4
        local val_col=22

        # Session info
        tui_print $r $label_col "${C_BOLD}${C_WHITE}Session Information${C_RESET}"
        tui_hline $((r + 1)) $label_col $((TUI_COLS - 8)) "─" "$C_GRAY"

        tui_print $((r + 3)) $label_col "${C_CYAN}Name:${C_RESET}"
        tui_print $((r + 3)) $val_col "${C_WHITE}${C_BOLD}$name${C_RESET}"

        tui_print $((r + 4)) $label_col "${C_CYAN}Mode:${C_RESET}"
        tui_print $((r + 4)) $val_col "$mode"

        tui_print $((r + 5)) $label_col "${C_CYAN}Status:${C_RESET}"
        tui_cursor_to $((r + 5)) $val_col
        tui_status_badge "$status"

        tui_print $((r + 6)) $label_col "${C_CYAN}Created:${C_RESET}"
        tui_print $((r + 6)) $val_col "$created"

        tui_print $((r + 7)) $label_col "${C_CYAN}Container:${C_RESET}"
        tui_print $((r + 7)) $val_col "cage-${name}"

        # Docker details (if running)
        if [[ "$status" == "running" ]]; then
            local container_name="cage-${name}"

            tui_print $((r + 9)) $label_col "${C_BOLD}${C_WHITE}Container Details${C_RESET}"
            tui_hline $((r + 10)) $label_col $((TUI_COLS - 8)) "─" "$C_GRAY"

            local image mem_limit cpu_limit
            image=$(docker inspect -f '{{.Config.Image}}' "$container_name" 2>/dev/null || echo "N/A")
            mem_limit=$(docker inspect -f '{{.HostConfig.Memory}}' "$container_name" 2>/dev/null || echo "0")
            cpu_limit=$(docker inspect -f '{{.HostConfig.NanoCpus}}' "$container_name" 2>/dev/null || echo "0")

            tui_print $((r + 12)) $label_col "${C_CYAN}Image:${C_RESET}"
            tui_print $((r + 12)) $val_col "$image"

            tui_print $((r + 13)) $label_col "${C_CYAN}Memory limit:${C_RESET}"
            if [[ "$mem_limit" != "0" ]]; then
                local mem_gb=$(echo "scale=1; $mem_limit / 1073741824" | bc 2>/dev/null || echo "N/A")
                tui_print $((r + 13)) $val_col "${mem_gb} GB"
            else
                tui_print $((r + 13)) $val_col "unlimited"
            fi

            tui_print $((r + 14)) $label_col "${C_CYAN}CPU limit:${C_RESET}"
            if [[ "$cpu_limit" != "0" ]]; then
                local cpus_display=$(echo "scale=1; $cpu_limit / 1000000000" | bc 2>/dev/null || echo "N/A")
                tui_print $((r + 14)) $val_col "${cpus_display} cores"
            else
                tui_print $((r + 14)) $val_col "unlimited"
            fi

            # Security checks
            tui_print $((r + 16)) $label_col "${C_BOLD}${C_WHITE}Security Status${C_RESET}"
            tui_hline $((r + 17)) $label_col $((TUI_COLS - 8)) "─" "$C_GRAY"

            local ro caps sec_opts
            ro=$(docker inspect -f '{{.HostConfig.ReadonlyRootfs}}' "$container_name" 2>/dev/null || echo "false")
            caps=$(docker inspect -f '{{.HostConfig.CapDrop}}' "$container_name" 2>/dev/null || echo "[]")
            sec_opts=$(docker inspect -f '{{.HostConfig.SecurityOpt}}' "$container_name" 2>/dev/null || echo "[]")

            local check_row=$((r + 19))
            if [[ "$ro" == "true" ]]; then
                tui_print $check_row $label_col "${C_GREEN}✓${C_RESET} Read-only root filesystem"
            else
                tui_print $check_row $label_col "${C_RED}✗${C_RESET} Root filesystem is writable"
            fi

            if [[ "$caps" == *"ALL"* ]]; then
                tui_print $((check_row + 1)) $label_col "${C_GREEN}✓${C_RESET} All capabilities dropped"
            else
                tui_print $((check_row + 1)) $label_col "${C_YELLOW}!${C_RESET} Some capabilities retained"
            fi

            if [[ "$sec_opts" == *"no-new-privileges"* ]]; then
                tui_print $((check_row + 2)) $label_col "${C_GREEN}✓${C_RESET} no-new-privileges enforced"
            else
                tui_print $((check_row + 2)) $label_col "${C_RED}✗${C_RESET} no-new-privileges NOT set"
            fi

            if [[ "$sec_opts" == *"seccomp"* ]]; then
                tui_print $((check_row + 3)) $label_col "${C_GREEN}✓${C_RESET} Seccomp profile active"
            else
                tui_print $((check_row + 3)) $label_col "${C_YELLOW}!${C_RESET} Default seccomp"
            fi
        fi

        # Desktop URL
        if [[ "$mode" == "desktop" && "$status" == "running" ]]; then
            local port
            port=$(docker inspect -f '{{(index (index .NetworkSettings.Ports "6080/tcp") 0).HostPort}}' "cage-${name}" 2>/dev/null || echo "6080")
            tui_print $((TUI_ROWS - 4)) $label_col \
                "${C_GREEN}${C_BOLD}Desktop URL: ${C_UNDERLINE}http://localhost:${port}${C_RESET}"
        fi

        # Key hints
        tui_keyhints $((TUI_ROWS)) \
            "s" "Shell" \
            "l" "Logs" \
            "x" "Stop" \
            "S" "Start" \
            "d" "Destroy" \
            "Esc" "Back"

        local key
        key=$(tui_read_key)

        case "$key" in
            s)
                if [[ "$status" == "running" ]]; then
                    tui_cleanup
                    docker exec -it "cage-${name}" /bin/bash
                    tui_init
                fi
                ;;
            l)
                tui_cleanup
                echo "==> Logs for cage-${name} (Ctrl+C to exit)"
                docker logs -f "cage-${name}" 2>&1 || true
                tui_init
                ;;
            x)
                if [[ "$status" == "running" ]]; then
                    docker stop "cage-${name}" >/dev/null 2>&1
                    session_set_status "$name" "stopped"
                    status="stopped"
                fi
                ;;
            S)
                if [[ "$status" != "running" ]]; then
                    docker start "cage-${name}" >/dev/null 2>&1
                    session_set_status "$name" "running"
                    status="running"
                fi
                ;;
            d)
                if tui_confirm "Destroy session '$name' permanently?"; then
                    docker rm -f "cage-${name}" >/dev/null 2>&1 || true
                    docker volume rm "cage-data-${name}" >/dev/null 2>&1 || true
                    session_remove "$name"
                    gui_refresh_sessions
                    GUI_SELECTED_IDX=0
                    GUI_CURRENT_SCREEN="dashboard"
                    return
                fi
                ;;
            ESCAPE|QUIT)
                GUI_CURRENT_SCREEN="dashboard"
                gui_refresh_sessions
                return
                ;;
        esac
    done
}

# ═══════════════════════════════════════════════════════════════════
# Config viewer screen
# ═══════════════════════════════════════════════════════════════════
gui_screen_config() {
    tui_clear
    gui_draw_header "Configuration"

    local r=5
    local label_col=4
    local val_col=30

    tui_print $r $label_col "${C_BOLD}${C_WHITE}Current Configuration${C_RESET}"
    tui_hline $((r + 1)) $label_col $((TUI_COLS - 8)) "─" "$C_GRAY"

    local row=$((r + 3))
    for key in $(echo "${!CAGE_CFG[@]}" | tr ' ' '\n' | sort); do
        local val="${CAGE_CFG[$key]}"

        # Mask sensitive values
        if [[ "$key" == *"key"* || "$key" == *"secret"* ]]; then
            val="********"
        fi

        tui_print "$row" $label_col "${C_CYAN}${key}:${C_RESET}"
        tui_print "$row" $val_col "${C_WHITE}${val}${C_RESET}"
        (( row++ ))

        if (( row >= TUI_ROWS - 4 )); then
            tui_print "$row" $label_col "${C_DIM}... (terminal too small for all entries)${C_RESET}"
            break
        fi
    done

    tui_print $((TUI_ROWS - 3)) $label_col \
        "${C_DIM}Config file: $(config_default_path)${C_RESET}"
    tui_print $((TUI_ROWS - 2)) $label_col \
        "${C_DIM}User config: ${CAGE_CONFIG_DIR}/config.yaml${C_RESET}"

    tui_keyhints $((TUI_ROWS)) \
        "Esc" "Back" \
        "q" "Quit"

    local key
    key=$(tui_read_key)

    case "$key" in
        ESCAPE|QUIT)
            GUI_CURRENT_SCREEN="dashboard"
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════
# Help screen
# ═══════════════════════════════════════════════════════════════════
gui_screen_help() {
    tui_clear
    gui_draw_header "Help"

    local r=5
    local c=4

    tui_print $r $c "${C_BOLD}${C_WHITE}Keyboard Shortcuts${C_RESET}"
    tui_hline $((r + 1)) $c $((TUI_COLS - 8)) "─" "$C_GRAY"

    local row=$((r + 3))
    _help_key() {
        tui_print "$row" $c "${C_CYAN}${C_BOLD}$(printf '%-14s' "$1")${C_RESET} ${C_WHITE}$2${C_RESET}"
        (( row++ ))
    }

    _help_key "↑ / ↓"        "Navigate session list"
    _help_key "Enter"         "View session details"
    _help_key "n"             "Create new session"
    _help_key "s"             "Open shell in selected session"
    _help_key "x"             "Stop selected session"
    _help_key "d"             "Destroy selected session"
    _help_key "r"             "Refresh session list"
    _help_key "c"             "View configuration"
    _help_key "?"             "Show this help"
    _help_key "q / Esc"       "Quit / Go back"

    (( row += 2 ))
    tui_print $row $c "${C_BOLD}${C_WHITE}Session Wizard${C_RESET}"
    tui_hline $((row + 1)) $c $((TUI_COLS - 8)) "─" "$C_GRAY"
    (( row += 3 ))

    _help_key "← / →"        "Toggle field values"
    _help_key "Enter"         "Edit text fields"
    _help_key "L"             "Launch session"
    _help_key "Esc"           "Cancel"

    (( row += 2 ))
    tui_print $row $c "${C_BOLD}${C_WHITE}Session Detail${C_RESET}"
    tui_hline $((row + 1)) $c $((TUI_COLS - 8)) "─" "$C_GRAY"
    (( row += 3 ))

    _help_key "s"             "Attach shell (exits TUI temporarily)"
    _help_key "l"             "Follow logs (Ctrl+C to return)"
    _help_key "x"             "Stop session"
    _help_key "S"             "Start stopped session"
    _help_key "d"             "Destroy session"

    tui_keyhints $((TUI_ROWS)) \
        "Esc" "Back" \
        "q" "Quit"

    local key
    key=$(tui_read_key)

    case "$key" in
        ESCAPE|QUIT)
            GUI_CURRENT_SCREEN="dashboard"
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════
# Actions
# ═══════════════════════════════════════════════════════════════════

gui_refresh_sessions() {
    GUI_SESSION_LIST=()
    local dir="${CAGE_CFG[session_dir]:-$CAGE_DATA_DIR/sessions}"

    # From session metadata
    if [[ -d "$dir" ]]; then
        for session_dir in "$dir"/*/; do
            [[ -d "$session_dir" ]] || continue
            local meta="$session_dir/metadata"
            [[ -f "$meta" ]] || continue

            local name mode status created
            name=$(grep "^name=" "$meta" 2>/dev/null | cut -d= -f2)
            mode=$(grep "^mode=" "$meta" 2>/dev/null | cut -d= -f2)
            created=$(grep "^created=" "$meta" 2>/dev/null | cut -d= -f2)

            # Reconcile with Docker
            local docker_status
            docker_status=$(docker inspect -f '{{.State.Status}}' "cage-${name}" 2>/dev/null) || docker_status="removed"
            status="$docker_status"

            GUI_SESSION_LIST+=("${name}|${mode}|${status}|${created}")
        done
    fi

    # Orphan containers
    local running
    running=$(docker ps --filter "label=managed-by=claude-cage" --format '{{.Names}}' 2>/dev/null | sed 's/^cage-//' || true)
    for c in $running; do
        local found=false
        for entry in "${GUI_SESSION_LIST[@]+"${GUI_SESSION_LIST[@]}"}"; do
            [[ "$entry" == "${c}|"* ]] && found=true
        done
        if ! $found; then
            local c_mode
            c_mode=$(docker inspect -f '{{index .Config.Labels "cage.mode"}}' "cage-${c}" 2>/dev/null || echo "cli")
            GUI_SESSION_LIST+=("${c}|${c_mode}|running|(orphan)")
        fi
    done
}

gui_action_shell() {
    if (( ${#GUI_SESSION_LIST[@]} == 0 )); then
        return
    fi
    local entry="${GUI_SESSION_LIST[$GUI_SELECTED_IDX]}"
    local name status
    name=$(echo "$entry" | cut -d'|' -f1)
    status=$(echo "$entry" | cut -d'|' -f3)

    if [[ "$status" != "running" ]]; then
        return
    fi

    tui_cleanup
    docker exec -it "cage-${name}" /bin/bash
    tui_init
}

gui_action_stop() {
    if (( ${#GUI_SESSION_LIST[@]} == 0 )); then
        return
    fi
    local entry="${GUI_SESSION_LIST[$GUI_SELECTED_IDX]}"
    local name status
    name=$(echo "$entry" | cut -d'|' -f1)
    status=$(echo "$entry" | cut -d'|' -f3)

    if [[ "$status" == "running" ]]; then
        docker stop "cage-${name}" >/dev/null 2>&1 || true
        session_set_status "$name" "stopped"
        gui_refresh_sessions
    fi
}

gui_action_destroy() {
    if (( ${#GUI_SESSION_LIST[@]} == 0 )); then
        return
    fi
    local entry="${GUI_SESSION_LIST[$GUI_SELECTED_IDX]}"
    local name
    name=$(echo "$entry" | cut -d'|' -f1)

    if tui_confirm "Destroy session '$name'?"; then
        docker rm -f "cage-${name}" >/dev/null 2>&1 || true
        docker volume rm "cage-data-${name}" >/dev/null 2>&1 || true
        session_remove "$name"
        gui_refresh_sessions
        (( GUI_SELECTED_IDX > 0 )) && (( GUI_SELECTED_IDX-- ))
    fi
}

gui_launch_session() {
    local mode="$1" name="$2" network="$3" cpus="$4" memory="$5" mount_path="$6" ephemeral="$7"

    # Build command
    local -a args=(--mode "$mode" --network "$network" --cpus "$cpus" --memory "$memory")
    [[ -n "$name" ]] && args+=(--name "$name")
    [[ -n "$mount_path" ]] && args+=(--mount "$mount_path")
    [[ "$ephemeral" == "yes" ]] && args+=(--ephemeral)

    # Pass API key if available (optional — Max users authenticate via claude login)
    if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
        args+=(--api-key "$ANTHROPIC_API_KEY")
    fi

    if [[ "$mode" == "cli" ]]; then
        # CLI mode: exit TUI, run interactively
        tui_cleanup
        cmd_start "${args[@]}"
        echo ""
        read -rp "Press Enter to return to GUI..."
        tui_init
    else
        # Desktop mode: launch in background, stay in TUI
        tui_cleanup
        cmd_start "${args[@]}"
        tui_init
    fi

    gui_refresh_sessions
}
