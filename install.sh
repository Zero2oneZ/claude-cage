#!/usr/bin/env bash
# install.sh — claude-cage installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Zero2oneZ/claude-cage/main/install.sh | bash
#   or:  ./install.sh [--prefix /usr/local] [--no-build] [--uninstall]
set -euo pipefail

# ── Defaults ─────────────────────────────────────────────────────
PREFIX="${PREFIX:-/usr/local}"
CAGE_DIR=""
NO_BUILD=false
UNINSTALL=false
SKIP_DEPS=false
VERBOSE=false

# ── Colors ───────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

# ── Helpers ──────────────────────────────────────────────────────
info()  { echo -e "${BLUE}[INFO]${RESET}  $*"; }
ok()    { echo -e "${GREEN}[OK]${RESET}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${RESET}  $*"; }
fail()  { echo -e "${RED}[FAIL]${RESET}  $*" >&2; }
fatal() { fail "$@"; exit 1; }

banner() {
    echo -e "${CYAN}${BOLD}"
    cat <<'ART'
      ╔═══════════════════════════════════════════╗
      ║                                           ║
      ║         ┌─┐┬  ┌─┐┬ ┬┌┬┐┌─┐              ║
      ║         │  │  ├─┤│ │ ││├┤               ║
      ║         └─┘┴─┘┴ ┴└─┘─┴┘└─┘              ║
      ║           ┌─┐┌─┐┌─┐┌─┐                   ║
      ║           │  ├─┤│ ┬├┤                     ║
      ║           └─┘┴ ┴└─┘└─┘                   ║
      ║                                           ║
      ║    Dockerized Sandbox for Claude CLI       ║
      ║                                           ║
      ╚═══════════════════════════════════════════╝
ART
    echo -e "${RESET}"
}

spinner() {
    local pid=$1
    local msg="${2:-Working...}"
    local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
    local i=0
    while kill -0 "$pid" 2>/dev/null; do
        echo -ne "\r  ${CYAN}${frames[$i]}${RESET} $msg"
        i=$(( (i + 1) % ${#frames[@]} ))
        sleep 0.1
    done
    wait "$pid"
    local status=$?
    echo -ne "\r"
    return $status
}

# ── Argument parsing ─────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)     PREFIX="$2"; shift 2 ;;
        --dir)        CAGE_DIR="$2"; shift 2 ;;
        --no-build)   NO_BUILD=true; shift ;;
        --skip-deps)  SKIP_DEPS=true; shift ;;
        --uninstall)  UNINSTALL=true; shift ;;
        --verbose)    VERBOSE=true; shift ;;
        -h|--help)
            echo "Usage: install.sh [options]"
            echo ""
            echo "Options:"
            echo "  --prefix <path>   Install prefix (default: /usr/local)"
            echo "  --dir <path>      claude-cage source directory (default: auto-detect)"
            echo "  --no-build        Skip building Docker images"
            echo "  --skip-deps       Skip dependency checks"
            echo "  --uninstall       Remove claude-cage installation"
            echo "  --verbose         Show detailed output"
            echo "  -h, --help        Show this help"
            exit 0
            ;;
        *) fatal "Unknown option: $1" ;;
    esac
done

# ── Auto-detect source directory ─────────────────────────────────
if [[ -z "$CAGE_DIR" ]]; then
    if [[ -f "$(pwd)/bin/claude-cage" ]]; then
        CAGE_DIR="$(pwd)"
    elif [[ -f "$(dirname "${BASH_SOURCE[0]}")/bin/claude-cage" ]]; then
        CAGE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    else
        fatal "Cannot find claude-cage source. Run from the project root or use --dir <path>"
    fi
fi

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/claude-cage"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/claude-cage"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/claude-cage"
COMPLETIONS_DIR="$PREFIX/share/bash-completion/completions"

# ── Uninstall ────────────────────────────────────────────────────
if $UNINSTALL; then
    banner
    info "Uninstalling claude-cage..."

    rm -f "$BIN_DIR/claude-cage"         && ok "Removed $BIN_DIR/claude-cage"        || true
    rm -rf "$SHARE_DIR"                  && ok "Removed $SHARE_DIR"                  || true
    rm -f "$COMPLETIONS_DIR/claude-cage" && ok "Removed shell completions"           || true

    echo ""
    info "User data preserved at: $CONFIG_DIR"
    info "User sessions at:       $DATA_DIR"
    info "To remove everything:   rm -rf $CONFIG_DIR $DATA_DIR"
    echo ""

    # Offer to remove Docker resources
    read -rp "Remove Docker images and volumes? [y/N] " yn
    if [[ "$yn" =~ ^[Yy]$ ]]; then
        docker rmi claude-cage-cli:latest claude-cage-desktop:latest 2>/dev/null && ok "Removed images" || true
        docker volume ls -q --filter name=cage- 2>/dev/null | xargs -r docker volume rm 2>/dev/null && ok "Removed volumes" || true
        docker network rm cage-filtered 2>/dev/null && ok "Removed network" || true
    fi

    ok "Uninstall complete."
    exit 0
fi

# ── Install ──────────────────────────────────────────────────────
banner
echo -e "  ${DIM}Installing to: $PREFIX${RESET}"
echo ""

# ── Step 1: Platform detection ───────────────────────────────────
info "Detecting platform..."
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  ok "Platform: Linux ($ARCH)" ;;
    Darwin) ok "Platform: macOS ($ARCH)" ;;
    *)      warn "Unsupported OS: $OS — installation may not work correctly" ;;
esac

# ── Step 2: Dependency checks ───────────────────────────────────
if ! $SKIP_DEPS; then
    info "Checking dependencies..."

    deps_ok=true

    # Docker
    if command -v docker &>/dev/null; then
        docker_version=$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo "unknown")
        ok "Docker: $docker_version"
    else
        fail "Docker is not installed."
        echo -e "    Install: ${CYAN}https://docs.docker.com/get-docker/${RESET}"
        deps_ok=false
    fi

    # Docker Compose
    if docker compose version &>/dev/null 2>&1; then
        compose_version=$(docker compose version --short 2>/dev/null || echo "unknown")
        ok "Docker Compose: $compose_version"
    elif command -v docker-compose &>/dev/null; then
        compose_version=$(docker-compose version --short 2>/dev/null || echo "unknown")
        ok "Docker Compose (standalone): $compose_version"
        warn "Consider upgrading to Docker Compose v2 plugin"
    else
        fail "Docker Compose is not installed."
        echo -e "    Install: ${CYAN}https://docs.docker.com/compose/install/${RESET}"
        deps_ok=false
    fi

    # Docker daemon running
    if docker info &>/dev/null 2>&1; then
        ok "Docker daemon: running"
    else
        fail "Docker daemon is not running or you lack permissions."
        echo "    Try: sudo systemctl start docker"
        echo "    Or:  sudo usermod -aG docker $USER  (then re-login)"
        deps_ok=false
    fi

    # bash >= 4 (for associative arrays)
    bash_version="${BASH_VERSINFO[0]}"
    if (( bash_version >= 4 )); then
        ok "Bash: $BASH_VERSION"
    else
        fail "Bash 4+ required (found $BASH_VERSION)"
        deps_ok=false
    fi

    # git (optional but useful)
    if command -v git &>/dev/null; then
        ok "Git: $(git --version | awk '{print $3}')"
    else
        warn "Git not found (optional but recommended)"
    fi

    # jq (optional)
    if command -v jq &>/dev/null; then
        ok "jq: $(jq --version 2>/dev/null)"
    else
        warn "jq not found (optional — used for JSON output)"
    fi

    if ! $deps_ok; then
        fatal "Missing required dependencies. Install them and re-run."
    fi

    echo ""
fi

# ── Step 3: Create directories ──────────────────────────────────
info "Creating directories..."

mkdir -p "$BIN_DIR"
mkdir -p "$SHARE_DIR"/{lib,docker/cli,docker/desktop,security,config}
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"/{sessions,logs}
mkdir -p "$COMPLETIONS_DIR" 2>/dev/null || true

ok "Directories created"

# ── Step 4: Copy files ──────────────────────────────────────────
info "Installing files..."

# Core binary
cp "$CAGE_DIR/bin/claude-cage" "$BIN_DIR/claude-cage"
chmod +x "$BIN_DIR/claude-cage"

# Libraries
for f in "$CAGE_DIR"/lib/*.sh; do
    cp "$f" "$SHARE_DIR/lib/"
done

# Docker files
cp "$CAGE_DIR/docker/cli/Dockerfile" "$SHARE_DIR/docker/cli/"
cp "$CAGE_DIR/docker/desktop/Dockerfile" "$SHARE_DIR/docker/desktop/"
cp "$CAGE_DIR/docker/desktop/entrypoint-desktop.sh" "$SHARE_DIR/docker/desktop/"
cp "$CAGE_DIR/docker/desktop/openbox-rc.xml" "$SHARE_DIR/docker/desktop/"

# Security profiles
cp "$CAGE_DIR/security/seccomp-default.json" "$SHARE_DIR/security/"
cp "$CAGE_DIR/security/apparmor-profile" "$SHARE_DIR/security/"

# Default config
cp "$CAGE_DIR/config/default.yaml" "$SHARE_DIR/config/"

# Docker Compose
cp "$CAGE_DIR/docker-compose.yml" "$SHARE_DIR/"

# User config (only if not existing — don't overwrite user customizations)
if [[ ! -f "$CONFIG_DIR/config.yaml" ]]; then
    cp "$CAGE_DIR/config/default.yaml" "$CONFIG_DIR/config.yaml"
    ok "Created user config at $CONFIG_DIR/config.yaml"
fi

# Patch CAGE_ROOT in the installed binary to point to SHARE_DIR
sed -i "s|CAGE_ROOT=.*|CAGE_ROOT=\"$SHARE_DIR\"|" "$BIN_DIR/claude-cage" 2>/dev/null || \
    sed -i '' "s|CAGE_ROOT=.*|CAGE_ROOT=\"$SHARE_DIR\"|" "$BIN_DIR/claude-cage" 2>/dev/null || true

ok "Files installed"

# ── Step 5: Shell completions ───────────────────────────────────
info "Installing shell completions..."

cat > "$COMPLETIONS_DIR/claude-cage" 2>/dev/null <<'COMP' || warn "Could not install completions"
_claude_cage() {
    local cur prev commands
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    commands="start stop shell status logs list destroy build config gui version help"

    case "$prev" in
        claude-cage)
            COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
            return 0
            ;;
        start)
            COMPREPLY=( $(compgen -W "--mode --name --mount --network --cpus --memory --gpu --port --env --config --api-key --ephemeral --no-persist" -- "$cur") )
            return 0
            ;;
        --mode)
            COMPREPLY=( $(compgen -W "cli desktop" -- "$cur") )
            return 0
            ;;
        --network)
            COMPREPLY=( $(compgen -W "none filtered host" -- "$cur") )
            return 0
            ;;
        stop|shell|status|logs|destroy)
            COMPREPLY=( $(compgen -W "--name --all --force --follow" -- "$cur") )
            return 0
            ;;
        build)
            COMPREPLY=( $(compgen -W "cli desktop all" -- "$cur") )
            return 0
            ;;
        config)
            COMPREPLY=( $(compgen -W "show validate path" -- "$cur") )
            return 0
            ;;
        --mount)
            COMPREPLY=( $(compgen -d -- "$cur") )
            return 0
            ;;
    esac
}
complete -F _claude_cage claude-cage
COMP

ok "Bash completions installed"

# ── Step 6: Build Docker images ─────────────────────────────────
if ! $NO_BUILD; then
    echo ""
    info "Building Docker images..."
    echo -e "  ${DIM}(skip with --no-build)${RESET}"
    echo ""

    info "Building CLI image..."
    if $VERBOSE; then
        docker build -t claude-cage-cli:latest -f "$SHARE_DIR/docker/cli/Dockerfile" "$SHARE_DIR/docker/cli/"
    else
        docker build -t claude-cage-cli:latest -f "$SHARE_DIR/docker/cli/Dockerfile" "$SHARE_DIR/docker/cli/" > /tmp/cage-build-cli.log 2>&1 &
        if spinner $! "Building claude-cage-cli:latest..."; then
            ok "CLI image built"
        else
            fail "CLI image build failed. See /tmp/cage-build-cli.log"
        fi
    fi

    info "Building Desktop image..."
    if $VERBOSE; then
        docker build -t claude-cage-desktop:latest -f "$SHARE_DIR/docker/desktop/Dockerfile" "$SHARE_DIR/docker/desktop/"
    else
        docker build -t claude-cage-desktop:latest -f "$SHARE_DIR/docker/desktop/Dockerfile" "$SHARE_DIR/docker/desktop/" > /tmp/cage-build-desktop.log 2>&1 &
        if spinner $! "Building claude-cage-desktop:latest..."; then
            ok "Desktop image built"
        else
            fail "Desktop image build failed. See /tmp/cage-build-desktop.log"
        fi
    fi
else
    warn "Skipping image build (--no-build). Run 'claude-cage build' later."
fi

# ── Step 7: Verify installation ─────────────────────────────────
echo ""
info "Verifying installation..."

if command -v claude-cage &>/dev/null; then
    ok "claude-cage is on PATH"
else
    warn "claude-cage not found on PATH."
    warn "Add to your shell profile:"
    echo -e "    export PATH=\"$BIN_DIR:\$PATH\""
fi

# ── Done ─────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}  ✓ Installation complete!${RESET}"
echo ""
echo -e "  ${BOLD}Quick start:${RESET}"
echo -e "    ${CYAN}export ANTHROPIC_API_KEY=sk-ant-...${RESET}"
echo -e "    ${CYAN}claude-cage start --mode cli --mount .${RESET}"
echo ""
echo -e "  ${BOLD}Interactive GUI:${RESET}"
echo -e "    ${CYAN}claude-cage gui${RESET}"
echo ""
echo -e "  ${BOLD}Paths:${RESET}"
echo -e "    Binary:  $BIN_DIR/claude-cage"
echo -e "    Data:    $SHARE_DIR"
echo -e "    Config:  $CONFIG_DIR/config.yaml"
echo ""
