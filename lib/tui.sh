#!/usr/bin/env bash
# tui.sh — Terminal UI rendering engine for claude-cage
# Pure bash ANSI-based TUI: no ncurses, no dialog dependency.

# ── Terminal state ───────────────────────────────────────────────
TUI_ROWS=0
TUI_COLS=0
TUI_RUNNING=false
TUI_NEEDS_REDRAW=true

# ── Color palette ────────────────────────────────────────────────
C_RESET='\033[0m'
C_BOLD='\033[1m'
C_DIM='\033[2m'
C_ITALIC='\033[3m'
C_UNDERLINE='\033[4m'
C_BLINK='\033[5m'
C_INVERSE='\033[7m'

# Foreground
C_BLACK='\033[30m'
C_RED='\033[31m'
C_GREEN='\033[32m'
C_YELLOW='\033[33m'
C_BLUE='\033[34m'
C_MAGENTA='\033[35m'
C_CYAN='\033[36m'
C_WHITE='\033[37m'
C_GRAY='\033[90m'

# Bright foreground
C_BRED='\033[91m'
C_BGREEN='\033[92m'
C_BYELLOW='\033[93m'
C_BBLUE='\033[94m'
C_BMAGENTA='\033[95m'
C_BCYAN='\033[96m'
C_BWHITE='\033[97m'

# Background
BG_BLACK='\033[40m'
BG_RED='\033[41m'
BG_GREEN='\033[42m'
BG_YELLOW='\033[43m'
BG_BLUE='\033[44m'
BG_MAGENTA='\033[45m'
BG_CYAN='\033[46m'
BG_WHITE='\033[47m'
BG_GRAY='\033[100m'
BG_BBLUE='\033[104m'

# ── Cursor control ───────────────────────────────────────────────
tui_cursor_to()    { echo -ne "\033[${1};${2}H"; }
tui_cursor_up()    { echo -ne "\033[${1:-1}A"; }
tui_cursor_down()  { echo -ne "\033[${1:-1}B"; }
tui_cursor_right() { echo -ne "\033[${1:-1}C"; }
tui_cursor_left()  { echo -ne "\033[${1:-1}D"; }
tui_cursor_hide()  { echo -ne "\033[?25l"; }
tui_cursor_show()  { echo -ne "\033[?25h"; }
tui_cursor_save()  { echo -ne "\033[s"; }
tui_cursor_restore() { echo -ne "\033[u"; }

# ── Screen control ───────────────────────────────────────────────
tui_clear()        { echo -ne "\033[2J\033[H"; }
tui_clear_line()   { echo -ne "\033[2K"; }
tui_clear_below()  { echo -ne "\033[J"; }

tui_get_size() {
    if [[ -t 0 ]]; then
        TUI_ROWS=$(tput lines 2>/dev/null || echo 24)
        TUI_COLS=$(tput cols 2>/dev/null || echo 80)
    else
        TUI_ROWS=24
        TUI_COLS=80
    fi
}

# ── Initialization / Cleanup ────────────────────────────────────
tui_init() {
    TUI_RUNNING=true
    tui_get_size

    # Save terminal state
    tput smcup 2>/dev/null || true   # alternate screen buffer
    tui_cursor_hide
    tui_clear
    stty -echo -icanon 2>/dev/null || true

    # Handle resize
    trap 'tui_on_resize' WINCH
    # Handle exit
    trap 'tui_cleanup' EXIT INT TERM
}

tui_cleanup() {
    TUI_RUNNING=false
    tui_cursor_show
    tput rmcup 2>/dev/null || true   # restore screen buffer
    stty echo icanon 2>/dev/null || true
    echo ""
}

tui_on_resize() {
    tui_get_size
    TUI_NEEDS_REDRAW=true
}

# ── Input handling ───────────────────────────────────────────────
# Read a single keypress (including arrow keys, escape sequences)
tui_read_key() {
    local key
    IFS= read -rsn1 key 2>/dev/null || return 1

    # Handle escape sequences (arrow keys, etc.)
    if [[ "$key" == $'\033' ]]; then
        local seq1 seq2
        read -rsn1 -t 0.05 seq1 2>/dev/null || true
        read -rsn1 -t 0.05 seq2 2>/dev/null || true

        if [[ "$seq1" == "[" ]]; then
            case "$seq2" in
                A) echo "UP"; return ;;
                B) echo "DOWN"; return ;;
                C) echo "RIGHT"; return ;;
                D) echo "LEFT"; return ;;
                H) echo "HOME"; return ;;
                F) echo "END"; return ;;
                Z) echo "SHIFT_TAB"; return ;;
            esac
            # Handle longer sequences (page up/down, etc.)
            if [[ "$seq2" =~ [0-9] ]]; then
                local seq3
                read -rsn1 -t 0.05 seq3 2>/dev/null || true
                case "${seq2}${seq3}" in
                    "5~") echo "PGUP"; return ;;
                    "6~") echo "PGDN"; return ;;
                    "3~") echo "DELETE"; return ;;
                esac
            fi
        fi
        echo "ESCAPE"
        return
    fi

    case "$key" in
        $'\n'|$'\r') echo "ENTER" ;;
        $'\t')       echo "TAB" ;;
        $'\177'|$'\b') echo "BACKSPACE" ;;
        ' ')         echo "SPACE" ;;
        q|Q)         echo "QUIT" ;;
        *)           echo "$key" ;;
    esac
}

# Read a line of text with visual feedback
tui_read_line() {
    local prompt="$1"
    local default="${2:-}"
    local row="$3"
    local col="$4"
    local max_width="${5:-40}"
    local value="$default"

    tui_cursor_show
    stty echo icanon 2>/dev/null || true

    tui_cursor_to "$row" "$col"
    echo -ne "${C_BOLD}${prompt}${C_RESET} "
    if [[ -n "$default" ]]; then
        echo -ne "${C_DIM}($default)${C_RESET} "
    fi

    local input
    read -r input

    stty -echo -icanon 2>/dev/null || true
    tui_cursor_hide

    if [[ -n "$input" ]]; then
        echo "$input"
    else
        echo "$default"
    fi
}

# ── Drawing primitives ───────────────────────────────────────────

# Print text at position
tui_print() {
    local row="$1" col="$2"
    shift 2
    tui_cursor_to "$row" "$col"
    echo -ne "$*"
}

# Print centered text
tui_print_center() {
    local row="$1"
    shift
    local text="$*"
    # Strip ANSI codes for length calculation
    local plain
    plain=$(echo -ne "$text" | sed 's/\x1b\[[0-9;]*m//g')
    local col=$(( (TUI_COLS - ${#plain}) / 2 ))
    (( col < 1 )) && col=1
    tui_cursor_to "$row" "$col"
    echo -ne "$text"
}

# Fill a row with a character
tui_fill_row() {
    local row="$1"
    local char="${2:- }"
    local color="${3:-}"
    tui_cursor_to "$row" 1
    echo -ne "$color"
    printf "%${TUI_COLS}s" "" | tr ' ' "$char"
    echo -ne "${C_RESET}"
}

# Draw a horizontal line
tui_hline() {
    local row="$1"
    local col="$2"
    local width="$3"
    local char="${4:-─}"
    local color="${5:-${C_GRAY}}"
    tui_cursor_to "$row" "$col"
    echo -ne "$color"
    printf "%0.s$char" $(seq 1 "$width")
    echo -ne "${C_RESET}"
}

# Draw a vertical line
tui_vline() {
    local row="$1"
    local col="$2"
    local height="$3"
    local char="${4:-│}"
    local color="${5:-${C_GRAY}}"
    for (( i = 0; i < height; i++ )); do
        tui_cursor_to $((row + i)) "$col"
        echo -ne "${color}${char}${C_RESET}"
    done
}

# ── Box drawing ──────────────────────────────────────────────────
# Draw a box with optional title
# tui_box row col width height [title] [color]
tui_box() {
    local row="$1" col="$2" width="$3" height="$4"
    local title="${5:-}"
    local color="${6:-${C_GRAY}}"

    local inner_w=$((width - 2))

    # Top border
    tui_cursor_to "$row" "$col"
    echo -ne "${color}╭$(printf '%0.s─' $(seq 1 $inner_w))╮${C_RESET}"

    # Title (if provided)
    if [[ -n "$title" ]]; then
        local title_pos=$(( col + 2 ))
        tui_cursor_to "$row" "$title_pos"
        echo -ne "${color}┤ ${C_BOLD}${C_WHITE}${title}${C_RESET}${color} ├${C_RESET}"
    fi

    # Sides
    for (( i = 1; i < height - 1; i++ )); do
        tui_cursor_to $((row + i)) "$col"
        echo -ne "${color}│${C_RESET}"
        tui_cursor_to $((row + i)) $((col + width - 1))
        echo -ne "${color}│${C_RESET}"
    done

    # Bottom border
    tui_cursor_to $((row + height - 1)) "$col"
    echo -ne "${color}╰$(printf '%0.s─' $(seq 1 $inner_w))╯${C_RESET}"
}

# Draw a filled box (clears interior)
tui_box_filled() {
    local row="$1" col="$2" width="$3" height="$4"
    local title="${5:-}"
    local color="${6:-${C_GRAY}}"
    local bg="${7:-}"

    tui_box "$row" "$col" "$width" "$height" "$title" "$color"

    # Fill interior
    local inner_w=$((width - 2))
    for (( i = 1; i < height - 1; i++ )); do
        tui_cursor_to $((row + i)) $((col + 1))
        echo -ne "${bg}$(printf '%*s' $inner_w '')${C_RESET}"
    done
}

# ── Status badge ─────────────────────────────────────────────────
tui_badge() {
    local text="$1"
    local color="${2:-${BG_BLUE}${C_WHITE}}"
    echo -ne "${color}${C_BOLD} ${text} ${C_RESET}"
}

tui_status_badge() {
    local status="$1"
    case "$status" in
        running)  tui_badge "RUNNING" "${BG_GREEN}${C_BLACK}" ;;
        stopped)  tui_badge "STOPPED" "${BG_GRAY}${C_WHITE}" ;;
        created)  tui_badge "CREATED" "${BG_BLUE}${C_WHITE}" ;;
        exited)   tui_badge "EXITED"  "${BG_YELLOW}${C_BLACK}" ;;
        removed)  tui_badge "REMOVED" "${BG_RED}${C_WHITE}" ;;
        *)        tui_badge "$status" "${BG_GRAY}${C_WHITE}" ;;
    esac
}

# ── Progress bar ─────────────────────────────────────────────────
tui_progress() {
    local row="$1" col="$2" width="$3" percent="$4"
    local label="${5:-}"
    local filled=$(( width * percent / 100 ))
    local empty=$(( width - filled ))
    local color

    if (( percent < 50 )); then
        color="$C_GREEN"
    elif (( percent < 80 )); then
        color="$C_YELLOW"
    else
        color="$C_RED"
    fi

    tui_cursor_to "$row" "$col"
    echo -ne "${color}"
    printf "%0.s█" $(seq 1 $((filled > 0 ? filled : 0))) 2>/dev/null || true
    echo -ne "${C_GRAY}"
    printf "%0.s░" $(seq 1 $((empty > 0 ? empty : 0))) 2>/dev/null || true
    echo -ne "${C_RESET}"

    if [[ -n "$label" ]]; then
        echo -ne " ${C_DIM}${label}${C_RESET}"
    fi
}

# ── Table rendering ──────────────────────────────────────────────
# tui_table_header row col col_widths... -- headers...
tui_table_header() {
    local row="$1" col="$2"
    shift 2

    local -a widths=()
    while [[ "$1" != "--" ]]; do
        widths+=("$1")
        shift
    done
    shift  # consume --

    local x="$col"
    local i=0
    for header in "$@"; do
        tui_cursor_to "$row" "$x"
        echo -ne "${C_BOLD}${C_CYAN}$(printf "%-${widths[$i]}s" "$header")${C_RESET}"
        x=$(( x + widths[i] ))
        (( i++ ))
    done

    # Underline
    local total_w=0
    for w in "${widths[@]}"; do
        total_w=$((total_w + w))
    done
    tui_hline $((row + 1)) "$col" "$total_w" "─" "$C_GRAY"
}

# tui_table_row row col selected col_widths... -- values...
tui_table_row() {
    local row="$1" col="$2" selected="$3"
    shift 3

    local -a widths=()
    while [[ "$1" != "--" ]]; do
        widths+=("$1")
        shift
    done
    shift  # consume --

    # Highlight selected row
    if [[ "$selected" == "true" ]]; then
        tui_cursor_to "$row" "$col"
        echo -ne "${BG_BBLUE}${C_WHITE}$(printf "%*s" $(( ${widths[*]/%/+}0 )) '')${C_RESET}"
    fi

    local x="$col"
    local i=0
    local prefix=""
    if [[ "$selected" == "true" ]]; then
        prefix="${BG_BBLUE}${C_WHITE}${C_BOLD}"
    fi

    for val in "$@"; do
        tui_cursor_to "$row" "$x"
        echo -ne "${prefix}$(printf "%-${widths[$i]}s" "$val")${C_RESET}"
        x=$(( x + widths[i] ))
        (( i++ ))
    done
}

# ── Menu rendering ───────────────────────────────────────────────
# Renders a vertical menu and returns selected index
# tui_menu row col selected_idx items...
tui_menu_render() {
    local row="$1" col="$2" selected="$3"
    shift 3

    local i=0
    for item in "$@"; do
        tui_cursor_to $((row + i)) "$col"
        tui_clear_line
        if (( i == selected )); then
            echo -ne "  ${C_BCYAN}${C_BOLD}▸ ${item}${C_RESET}"
        else
            echo -ne "    ${C_WHITE}${item}${C_RESET}"
        fi
        (( i++ ))
    done
}

# ── Key hints bar ────────────────────────────────────────────────
tui_keyhints() {
    local row="$1"
    shift

    tui_cursor_to "$row" 1
    tui_clear_line

    local hints=""
    while [[ $# -gt 0 ]]; do
        local key="$1" desc="$2"
        shift 2
        hints+="${C_INVERSE}${C_WHITE} ${key} ${C_RESET} ${C_DIM}${desc}${C_RESET}  "
    done

    tui_cursor_to "$row" 2
    echo -ne "$hints"
}

# ── Notification / Toast ────────────────────────────────────────
tui_toast() {
    local msg="$1"
    local type="${2:-info}"
    local color

    case "$type" in
        success) color="${BG_GREEN}${C_BLACK}" ;;
        error)   color="${BG_RED}${C_WHITE}" ;;
        warn)    color="${BG_YELLOW}${C_BLACK}" ;;
        *)       color="${BG_BLUE}${C_WHITE}" ;;
    esac

    local row=$((TUI_ROWS - 2))
    local plain
    plain=$(echo -ne "$msg" | sed 's/\x1b\[[0-9;]*m//g')
    local width=$(( ${#plain} + 4 ))
    local col=$(( (TUI_COLS - width) / 2 ))
    (( col < 1 )) && col=1

    tui_cursor_to "$row" "$col"
    echo -ne "${color}${C_BOLD}  ${msg}  ${C_RESET}"

    # Auto-clear after a moment (non-blocking)
    ( sleep 2; tui_cursor_to "$row" "$col"; printf "%*s" "$width" ""; ) &
}

# ── Confirmation dialog ─────────────────────────────────────────
tui_confirm() {
    local msg="$1"
    local default="${2:-n}"

    local row=$(( TUI_ROWS / 2 - 2 ))
    local width=$(( ${#msg} + 20 ))
    (( width < 40 )) && width=40
    local col=$(( (TUI_COLS - width) / 2 ))

    tui_box_filled "$row" "$col" "$width" 5 "Confirm" "$C_YELLOW"

    tui_cursor_to $((row + 2)) $((col + 3))
    echo -ne "${C_WHITE}${msg}${C_RESET}"

    tui_cursor_to $((row + 3)) $((col + 3))
    if [[ "$default" == "y" ]]; then
        echo -ne "${C_DIM}[${C_RESET}${C_BOLD}Y${C_RESET}${C_DIM}/n]:${C_RESET} "
    else
        echo -ne "${C_DIM}[y/${C_RESET}${C_BOLD}N${C_RESET}${C_DIM}]:${C_RESET} "
    fi

    tui_cursor_show
    local key
    key=$(tui_read_key)
    tui_cursor_hide

    case "$key" in
        y|Y)     return 0 ;;
        n|N)     return 1 ;;
        ENTER)
            if [[ "$default" == "y" ]]; then
                return 0
            else
                return 1
            fi
            ;;
        *)       return 1 ;;
    esac
}

# ── Spinner ──────────────────────────────────────────────────────
TUI_SPINNER_PID=""

tui_spinner_start() {
    local row="$1" col="$2" msg="$3"
    local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
    (
        local i=0
        while true; do
            tui_cursor_to "$row" "$col"
            echo -ne "${C_CYAN}${frames[$i]}${C_RESET} ${msg}"
            i=$(( (i + 1) % ${#frames[@]} ))
            sleep 0.1
        done
    ) &
    TUI_SPINNER_PID=$!
}

tui_spinner_stop() {
    if [[ -n "$TUI_SPINNER_PID" ]]; then
        kill "$TUI_SPINNER_PID" 2>/dev/null || true
        wait "$TUI_SPINNER_PID" 2>/dev/null || true
        TUI_SPINNER_PID=""
    fi
}
