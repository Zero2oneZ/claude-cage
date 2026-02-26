#!/usr/bin/env bash
# ============================================================
# GentlyOS Headless Bootstrap
# ============================================================
# Master script - runs all setup stages on a headless machine.
# Usage:
#   sudo bash bootstrap.sh              # Full setup
#   sudo bash bootstrap.sh --stage 3    # Resume from stage 3
#   sudo bash bootstrap.sh --dry-run    # Show what would run
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="/var/log/gentlyos-setup"
STAMP_DIR="/var/lib/gentlyos-setup"

# --- Colors (safe for headless logging) ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${GREEN}[$(date '+%H:%M:%S')]${NC} $*" | tee -a "$LOG_DIR/bootstrap.log"; }
warn() { echo -e "${YELLOW}[$(date '+%H:%M:%S')] WARN:${NC} $*" | tee -a "$LOG_DIR/bootstrap.log"; }
err()  { echo -e "${RED}[$(date '+%H:%M:%S')] ERROR:${NC} $*" | tee -a "$LOG_DIR/bootstrap.log"; }
banner() {
    echo -e "${CYAN}"
    echo "============================================================"
    echo "  $*"
    echo "============================================================"
    echo -e "${NC}"
}

# --- Pre-flight checks ---
if [[ $EUID -ne 0 ]]; then
    err "This script must be run as root (use sudo)"
    exit 1
fi

# --- Parse arguments ---
START_STAGE=1
DRY_RUN=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --stage)   START_STAGE="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        --help|-h)
            echo "Usage: sudo bash bootstrap.sh [--stage N] [--dry-run]"
            echo "Stages: 1=system, 2=user/ssh, 3=network, 4=python, 5=ollama, 6=gentlyos, 7=services"
            exit 0 ;;
        *) err "Unknown argument: $1"; exit 1 ;;
    esac
done

# --- Load config ---
CONFIG_FILE="$SCRIPT_DIR/config.env"
if [[ ! -f "$CONFIG_FILE" ]]; then
    err "Config file not found: $CONFIG_FILE"
    err "Copy config.env.example to config.env and edit it first."
    exit 1
fi
# shellcheck source=config.env
source "$CONFIG_FILE"
log "Loaded config from $CONFIG_FILE"

# --- Setup directories ---
mkdir -p "$LOG_DIR" "$STAMP_DIR"

# --- Stage runner ---
STAGES=(
    "01-system-base.sh:System Base Packages"
    "02-user-ssh.sh:User Account & SSH"
    "03-network.sh:Ethernet & Networking"
    "04-python-pytorch.sh:Python & PyTorch"
    "05-ollama.sh:Ollama & Models"
    "06-gentlyos.sh:GentlyOS Build & Install"
    "07-services.sh:Services & API Gateway"
)

run_stage() {
    local stage_num=$1
    local stage_file stage_name
    stage_file="$(echo "${STAGES[$((stage_num-1))]}" | cut -d: -f1)"
    stage_name="$(echo "${STAGES[$((stage_num-1))]}" | cut -d: -f2)"
    local stage_path="$SCRIPT_DIR/$stage_file"
    local stamp_file="$STAMP_DIR/stage-${stage_num}.done"

    if [[ -f "$stamp_file" && "$START_STAGE" -ne "$stage_num" ]]; then
        log "Stage $stage_num ($stage_name) already completed, skipping."
        return 0
    fi

    banner "Stage $stage_num / ${#STAGES[@]}: $stage_name"

    if [[ ! -f "$stage_path" ]]; then
        err "Stage script not found: $stage_path"
        return 1
    fi

    if [[ $DRY_RUN -eq 1 ]]; then
        log "[DRY RUN] Would execute: $stage_path"
        return 0
    fi

    # Export all config vars so stage scripts can use them
    export GENTLY_USER GENTLY_PASSWORD GENTLY_PASSWORD_PROMPT
    export SSH_PORT SSH_ALLOW_PASSWORD SSH_AUTHORIZED_KEY
    export NETWORK_MODE STATIC_IP STATIC_GATEWAY STATIC_NETMASK STATIC_DNS ETH_INTERFACE
    export OLLAMA_MODEL OLLAMA_EXTRA_MODELS OLLAMA_HOST OLLAMA_PORT
    export PYTHON_VERSION PYTORCH_COMPUTE PYTORCH_VENV_PATH
    export GENTLYOS_INSTALL_MODE GENTLYOS_REPO_PATH
    export WEB_PORT API_PORT ENABLE_FIREWALL
    export LOG_DIR SCRIPT_DIR

    if bash "$stage_path" 2>&1 | tee -a "$LOG_DIR/stage-${stage_num}.log"; then
        touch "$stamp_file"
        log "Stage $stage_num ($stage_name) completed successfully."
    else
        err "Stage $stage_num ($stage_name) FAILED. Check $LOG_DIR/stage-${stage_num}.log"
        err "Fix the issue, then resume with: sudo bash bootstrap.sh --stage $stage_num"
        exit 1
    fi
}

# --- Main ---
banner "GentlyOS Headless Setup"
log "Starting from stage $START_STAGE"
log "Config: user=$GENTLY_USER ip=$STATIC_IP ollama_model=$OLLAMA_MODEL"

for i in $(seq "$START_STAGE" "${#STAGES[@]}"); do
    run_stage "$i"
done

banner "Setup Complete"
log ""
log "Access your headless GentlyOS node:"
log "  SSH:         ssh ${GENTLY_USER}@${STATIC_IP} -p ${SSH_PORT}"
log "  Ollama API:  http://${STATIC_IP}:${OLLAMA_PORT}"
log "  GentlyOS UI: http://${STATIC_IP}:${WEB_PORT}"
log "  Unified API: http://${STATIC_IP}:${API_PORT}"
log ""
log "From your 2nd PC, connect ethernet and set your IP to ${STATIC_IP%.*}.2/24"
log "Full logs: $LOG_DIR/"
