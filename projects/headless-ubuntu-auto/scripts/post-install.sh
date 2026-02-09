#!/usr/bin/env bash
###############################################################################
# post-install.sh — ML/AI Stack Provisioner for Ubuntu 24.04 + 2x RTX 3090
#
# Installs the full ML training and inference stack on a freshly-installed
# headless Ubuntu 24.04 Server with dual NVIDIA RTX 3090 GPUs (48GB total VRAM)
# and Intel integrated graphics.
#
# Usage:
#   sudo ./post-install.sh [--skip-reboot] [--stage N]
#
# Options:
#   --skip-reboot   Do not prompt for reboot after NVIDIA driver install
#   --stage N       Resume from stage N (skips all stages before N)
#
# Logs:    /var/log/headless-setup.log
# State:   /opt/headless-setup/.stage-complete
# Owner:   zero20nez
###############################################################################

set -euo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
readonly SCRIPT_VERSION="1.0.0"
readonly LOG_FILE="/var/log/headless-setup.log"
readonly STATE_DIR="/opt/headless-setup"
readonly STATE_FILE="${STATE_DIR}/.stage-complete"
readonly TARGET_USER="zero20nez"
readonly TOTAL_STAGES=19

readonly VENV_BASE="/data/venvs"
readonly ML_VENV="${VENV_BASE}/ml"
readonly MODEL_DIR="/data/models"
readonly TRAINING_DIR="/data/training"
readonly CHECKPOINT_DIR="/data/checkpoints"
readonly DATA_LOG_DIR="/data/logs"

# NVIDIA / CUDA versions
readonly NVIDIA_DRIVER_VER="550"
readonly CUDA_VER="12-4"
readonly CUDA_PATH="/usr/local/cuda-12.4"

# ---------------------------------------------------------------------------
# Color helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

log()     { echo -e "${CYAN}[$(date '+%H:%M:%S')]${NC} $*" | tee -a "$LOG_FILE"; }
info()    { echo -e "${BLUE}[INFO]${NC}  $*"  | tee -a "$LOG_FILE"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"  | tee -a "$LOG_FILE"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*" | tee -a "$LOG_FILE"; }
err()     { echo -e "${RED}[ERROR]${NC} $*"    | tee -a "$LOG_FILE"; }
header()  { echo -e "\n${BOLD}${BLUE}=== $* ===${NC}\n" | tee -a "$LOG_FILE"; }

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
SKIP_REBOOT=0
START_STAGE=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-reboot)
            SKIP_REBOOT=1
            shift
            ;;
        --stage)
            if [[ -z "${2:-}" ]] || ! [[ "$2" =~ ^[0-9]+$ ]]; then
                err "Option --stage requires a numeric argument (1-${TOTAL_STAGES})"
                exit 1
            fi
            START_STAGE="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: sudo $0 [--skip-reboot] [--stage N]"
            echo ""
            echo "  --skip-reboot   Suppress reboot prompt after driver install"
            echo "  --stage N       Resume from stage N (1-${TOTAL_STAGES})"
            exit 0
            ;;
        *)
            err "Unknown option: $1"
            echo "Usage: sudo $0 [--skip-reboot] [--stage N]"
            exit 1
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
if [[ $EUID -ne 0 ]]; then
    err "This script must be run as root (use sudo)."
    exit 1
fi

if ! grep -qi "ubuntu" /etc/os-release 2>/dev/null; then
    warn "This script is designed for Ubuntu 24.04. Detected a different OS."
fi

# ---------------------------------------------------------------------------
# State management helpers
# ---------------------------------------------------------------------------
mkdir -p "$STATE_DIR"
touch "$STATE_FILE"
touch "$LOG_FILE"

stage_done() {
    # Check whether stage $1 has already been completed
    grep -qx "STAGE_${1}_COMPLETE" "$STATE_FILE" 2>/dev/null
}

mark_stage() {
    # Record stage $1 as complete
    echo "STAGE_${1}_COMPLETE" >> "$STATE_FILE"
}

should_run_stage() {
    local stage_num="$1"
    # Skip if below the requested start stage
    if (( stage_num < START_STAGE )); then
        return 1
    fi
    # Skip if already completed
    if stage_done "$stage_num"; then
        info "Stage ${stage_num} already complete -- skipping."
        return 1
    fi
    return 0
}

run_stage() {
    # run_stage <number> <description> <function>
    local num="$1"
    local desc="$2"
    local func="$3"

    if ! should_run_stage "$num"; then
        return 0
    fi

    header "Stage ${num}/${TOTAL_STAGES}: ${desc}"
    log "Starting stage ${num}: ${desc}"

    if "$func"; then
        mark_stage "$num"
        ok "Stage ${num} complete: ${desc}"
    else
        err "Stage ${num} FAILED: ${desc}"
        err "Fix the issue, then re-run with:  sudo $0 --stage ${num}"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# Utility: safe apt-get wrapper (retries once on failure)
# ---------------------------------------------------------------------------
apt_install() {
    local attempt=1
    local max_attempts=2
    while (( attempt <= max_attempts )); do
        if DEBIAN_FRONTEND=noninteractive apt-get install -y "$@" 2>&1 | tee -a "$LOG_FILE"; then
            return 0
        fi
        warn "apt install failed (attempt ${attempt}/${max_attempts}), retrying in 5s..."
        sleep 5
        apt-get update -qq 2>&1 | tee -a "$LOG_FILE"
        (( attempt++ ))
    done
    err "apt install failed after ${max_attempts} attempts: $*"
    return 1
}

# ---------------------------------------------------------------------------
# Stage 1: System Update
# ---------------------------------------------------------------------------
stage_01_system_update() {
    info "Updating package lists..."
    apt-get update -y 2>&1 | tee -a "$LOG_FILE"

    info "Running full system upgrade..."
    DEBIAN_FRONTEND=noninteractive apt-get full-upgrade -y 2>&1 | tee -a "$LOG_FILE"

    info "Installing base packages..."
    apt_install \
        build-essential \
        linux-headers-generic \
        cmake \
        ninja-build \
        git \
        curl \
        wget \
        jq \
        unzip \
        software-properties-common

    info "Cleaning up apt cache..."
    apt-get autoremove -y 2>&1 | tee -a "$LOG_FILE"
    apt-get autoclean -y 2>&1 | tee -a "$LOG_FILE"
}

# ---------------------------------------------------------------------------
# Stage 2: NVIDIA Headless Driver
# ---------------------------------------------------------------------------
stage_02_nvidia_driver() {
    info "Blacklisting nouveau driver..."
    cat > /etc/modprobe.d/blacklist-nouveau.conf <<'NOUVEAU'
blacklist nouveau
options nouveau modeset=0
NOUVEAU
    info "Wrote /etc/modprobe.d/blacklist-nouveau.conf"

    info "Updating initramfs..."
    update-initramfs -u 2>&1 | tee -a "$LOG_FILE"

    info "Installing NVIDIA headless driver ${NVIDIA_DRIVER_VER} (dual 3090 setup)..."
    apt_install "nvidia-headless-${NVIDIA_DRIVER_VER}" "nvidia-utils-${NVIDIA_DRIVER_VER}"

    # Ensure CUDA workloads only see the NVIDIA GPUs, not the Intel iGPU
    info "Configuring CUDA_VISIBLE_DEVICES for dual 3090..."
    if ! grep -q "^CUDA_VISIBLE_DEVICES=" /etc/environment 2>/dev/null; then
        echo 'CUDA_VISIBLE_DEVICES=0,1' >> /etc/environment
    fi

    # Enable NVLink / peer-to-peer if both GPUs are on the same PCIe root
    info "Enabling GPU persistence mode via systemd..."
    cat > /etc/systemd/system/nvidia-persistenced.service.d/override.conf 2>/dev/null <<'NVPERSIST' || true
[Service]
ExecStart=
ExecStart=/usr/bin/nvidia-persistenced --user root --persistence-mode
NVPERSIST
    mkdir -p /etc/systemd/system/nvidia-persistenced.service.d 2>/dev/null || true

    warn "NVIDIA driver installed for 2x RTX 3090. A reboot is required to load the driver."
    warn "Continuing with remaining stages that do not require the GPU."
}

# ---------------------------------------------------------------------------
# Stage 3: CUDA 12.4 Toolkit
# ---------------------------------------------------------------------------
stage_03_cuda_toolkit() {
    info "Adding NVIDIA CUDA repository..."

    local keyring_url="https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb"
    local keyring_deb="/tmp/cuda-keyring.deb"

    wget -qO "$keyring_deb" "$keyring_url" 2>&1 | tee -a "$LOG_FILE"
    dpkg -i "$keyring_deb" 2>&1 | tee -a "$LOG_FILE"
    rm -f "$keyring_deb"

    info "Updating package lists with CUDA repo..."
    apt-get update -y 2>&1 | tee -a "$LOG_FILE"

    info "Installing CUDA Toolkit ${CUDA_VER}..."
    apt_install "cuda-toolkit-${CUDA_VER}"

    info "Configuring CUDA environment variables..."
    cat > /etc/profile.d/cuda.sh <<'CUDAENV'
export PATH=/usr/local/cuda-12.4/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda-12.4/lib64:${LD_LIBRARY_PATH:-}
CUDAENV
    chmod 644 /etc/profile.d/cuda.sh
    info "Wrote /etc/profile.d/cuda.sh"

    # Source it for the rest of this script
    export PATH="${CUDA_PATH}/bin:$PATH"
    export LD_LIBRARY_PATH="${CUDA_PATH}/lib64:${LD_LIBRARY_PATH:-}"
}

# ---------------------------------------------------------------------------
# Stage 4: cuDNN
# ---------------------------------------------------------------------------
stage_04_cudnn() {
    info "Installing cuDNN for CUDA 12..."
    apt_install libcudnn9-cuda-12
}

# ---------------------------------------------------------------------------
# Stage 5: Node.js 20 (for Claude CLI)
# ---------------------------------------------------------------------------
stage_05_nodejs() {
    info "Installing Node.js 20 via NodeSource..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - 2>&1 | tee -a "$LOG_FILE"
    apt_install nodejs

    local node_ver
    node_ver="$(node --version 2>/dev/null || echo 'unknown')"
    info "Node.js installed: ${node_ver}"
}

# ---------------------------------------------------------------------------
# Stage 6: Claude CLI
# ---------------------------------------------------------------------------
stage_06_claude_cli() {
    info "Installing Claude CLI globally via npm..."
    npm install -g @anthropic-ai/claude-code 2>&1 | tee -a "$LOG_FILE"

    if command -v claude &>/dev/null; then
        local claude_ver
        claude_ver="$(claude --version 2>/dev/null || echo 'unknown')"
        info "Claude CLI installed: ${claude_ver}"
    else
        warn "Claude CLI installed but 'claude' command not found in PATH."
        warn "It may be available after re-login or PATH update."
    fi
}

# ---------------------------------------------------------------------------
# Stage 7: Python 3.11
# ---------------------------------------------------------------------------
stage_07_python() {
    info "Adding deadsnakes PPA..."
    add-apt-repository -y ppa:deadsnakes/ppa 2>&1 | tee -a "$LOG_FILE"
    apt-get update -y 2>&1 | tee -a "$LOG_FILE"

    info "Installing Python 3.11..."
    apt_install python3.11 python3.11-venv python3.11-dev python3.11-distutils

    info "Installing pip for Python 3.11..."
    curl -sS https://bootstrap.pypa.io/get-pip.py | python3.11 2>&1 | tee -a "$LOG_FILE"

    info "Setting python3.11 as default python3 alternative..."
    update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 2>&1 | tee -a "$LOG_FILE"
    # If there is an existing python3 (e.g., 3.12 from Ubuntu 24.04), set priority
    # so 3.11 wins
    update-alternatives --set python3 /usr/bin/python3.11 2>&1 | tee -a "$LOG_FILE" || true

    info "Creating system-wide virtual-environments directory..."
    mkdir -p "$VENV_BASE"
    chown "${TARGET_USER}:${TARGET_USER}" "$VENV_BASE"

    local py_ver
    py_ver="$(python3.11 --version 2>/dev/null || echo 'unknown')"
    info "Python installed: ${py_ver}"
}

# ---------------------------------------------------------------------------
# Stage 8: PyTorch
# ---------------------------------------------------------------------------
stage_08_pytorch() {
    info "Creating ML virtual environment at ${ML_VENV}..."
    python3.11 -m venv "$ML_VENV" 2>&1 | tee -a "$LOG_FILE"

    info "Upgrading pip inside venv..."
    "${ML_VENV}/bin/pip" install --upgrade pip 2>&1 | tee -a "$LOG_FILE"

    info "Installing PyTorch (CUDA 12.4 wheels)..."
    "${ML_VENV}/bin/pip" install \
        torch torchvision torchaudio \
        --index-url https://download.pytorch.org/whl/cu124 \
        2>&1 | tee -a "$LOG_FILE"

    info "Verifying PyTorch import..."
    if "${ML_VENV}/bin/python" -c "import torch; print(f'PyTorch {torch.__version__} installed (CUDA compiled: {torch.version.cuda})')"; then
        ok "PyTorch import successful."
    else
        warn "PyTorch import failed. May work after reboot with NVIDIA driver loaded."
    fi
}

# ---------------------------------------------------------------------------
# Stage 9: vLLM
# ---------------------------------------------------------------------------
stage_09_vllm() {
    info "Installing vLLM in ML venv..."
    "${ML_VENV}/bin/pip" install vllm 2>&1 | tee -a "$LOG_FILE"

    if "${ML_VENV}/bin/python" -c "import vllm; print(f'vLLM {vllm.__version__}')" 2>/dev/null; then
        ok "vLLM import successful."
    else
        warn "vLLM installed but import verification skipped (may need GPU)."
    fi
}

# ---------------------------------------------------------------------------
# Stage 10: Ollama
# ---------------------------------------------------------------------------
stage_10_ollama() {
    info "Installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh 2>&1 | tee -a "$LOG_FILE"

    info "Configuring Ollama model storage directory..."
    mkdir -p /etc/systemd/system/ollama.service.d
    cat > /etc/systemd/system/ollama.service.d/override.conf <<'OLLAMACONF'
[Service]
Environment="OLLAMA_MODELS=/data/models/ollama"
Environment="OLLAMA_NUM_GPU=2"
Environment="CUDA_VISIBLE_DEVICES=0,1"
OLLAMACONF

    info "Creating model storage directories..."
    mkdir -p "${MODEL_DIR}/ollama"

    info "Reloading systemd daemon..."
    systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE"

    if command -v ollama &>/dev/null; then
        local ollama_ver
        ollama_ver="$(ollama --version 2>/dev/null || echo 'unknown')"
        info "Ollama installed: ${ollama_ver}"
    fi
}

# ---------------------------------------------------------------------------
# Stage 11: llama.cpp
# ---------------------------------------------------------------------------
stage_11_llama_cpp() {
    local llama_dir="/opt/llama.cpp"

    if [[ -d "$llama_dir" ]]; then
        info "llama.cpp directory already exists, pulling latest..."
        git -C "$llama_dir" pull 2>&1 | tee -a "$LOG_FILE"
    else
        info "Cloning llama.cpp..."
        git clone https://github.com/ggerganov/llama.cpp "$llama_dir" 2>&1 | tee -a "$LOG_FILE"
    fi

    # Ensure CUDA is on PATH for the build
    export PATH="${CUDA_PATH}/bin:$PATH"
    export LD_LIBRARY_PATH="${CUDA_PATH}/lib64:${LD_LIBRARY_PATH:-}"

    info "Building llama.cpp with CUDA support..."
    cmake -B "${llama_dir}/build" -S "$llama_dir" \
        -DGGML_CUDA=on \
        2>&1 | tee -a "$LOG_FILE"

    cmake --build "${llama_dir}/build" \
        --config Release \
        -j"$(nproc)" \
        2>&1 | tee -a "$LOG_FILE"

    info "Symlinking llama.cpp binaries to /usr/local/bin/..."
    # Link the main server and CLI binaries
    local binaries=("llama-server" "llama-cli" "llama-quantize" "llama-bench" "llama-perplexity")
    for bin in "${binaries[@]}"; do
        local src="${llama_dir}/build/bin/${bin}"
        if [[ -f "$src" ]]; then
            ln -sf "$src" "/usr/local/bin/${bin}"
            info "  Linked: ${bin}"
        else
            warn "  Binary not found (may have been renamed): ${bin}"
        fi
    done

    ok "llama.cpp built successfully."
}

# ---------------------------------------------------------------------------
# Stage 12: HuggingFace TGI
# ---------------------------------------------------------------------------
stage_12_tgi() {
    info "Installing HuggingFace Text Generation Inference (pip)..."
    # Install the launcher/client via pip. TGI is primarily a Rust server
    # so the pip package provides the Python client and a basic launcher.
    "${ML_VENV}/bin/pip" install text-generation 2>&1 | tee -a "$LOG_FILE" || true

    # Attempt the server package — this may fail on some systems as TGI
    # server is primarily distributed as a Docker image.
    if "${ML_VENV}/bin/pip" install text-generation-launcher 2>&1 | tee -a "$LOG_FILE"; then
        ok "TGI launcher installed via pip."
    else
        warn "TGI launcher pip install failed (expected on some platforms)."
    fi

    info ""
    info "NOTE: For production TGI, the Docker image is recommended:"
    info "  docker run --gpus all --shm-size 1g -p 8080:80 \\"
    info "    -v /data/models/huggingface:/data \\"
    info "    ghcr.io/huggingface/text-generation-inference:latest \\"
    info "    --model-id <MODEL_ID>"
    info ""
}

# ---------------------------------------------------------------------------
# Stage 13: Training Tools
# ---------------------------------------------------------------------------
stage_13_training_tools() {
    info "Installing ML training ecosystem in venv..."

    info "Installing Transformers, Datasets, Accelerate, PEFT..."
    "${ML_VENV}/bin/pip" install \
        transformers \
        datasets \
        accelerate \
        peft \
        2>&1 | tee -a "$LOG_FILE"

    info "Installing bitsandbytes (4/8-bit quantization)..."
    "${ML_VENV}/bin/pip" install bitsandbytes 2>&1 | tee -a "$LOG_FILE"

    info "Installing DeepSpeed..."
    "${ML_VENV}/bin/pip" install deepspeed 2>&1 | tee -a "$LOG_FILE"

    info "Installing Weights & Biases (wandb)..."
    "${ML_VENV}/bin/pip" install wandb 2>&1 | tee -a "$LOG_FILE"

    info "Installing safetensors, sentencepiece, tokenizers..."
    "${ML_VENV}/bin/pip" install \
        safetensors \
        sentencepiece \
        tokenizers \
        2>&1 | tee -a "$LOG_FILE"

    ok "Training tools installed."
}

# ---------------------------------------------------------------------------
# Stage 14: Model Storage Setup
# ---------------------------------------------------------------------------
stage_14_model_storage() {
    info "Creating data directory structure..."

    local dirs=(
        "${MODEL_DIR}/ollama"
        "${MODEL_DIR}/huggingface"
        "${MODEL_DIR}/gguf"
        "${TRAINING_DIR}"
        "${CHECKPOINT_DIR}"
        "${DATA_LOG_DIR}"
    )

    for d in "${dirs[@]}"; do
        mkdir -p "$d"
        info "  Created: ${d}"
    done

    info "Setting ownership of /data to ${TARGET_USER}..."
    chown -R "${TARGET_USER}:${TARGET_USER}" /data

    info "Configuring environment variables in /etc/environment..."
    # Append if not already present
    if ! grep -q "^HF_HOME=" /etc/environment 2>/dev/null; then
        echo "HF_HOME=/data/models/huggingface" >> /etc/environment
    fi
    if ! grep -q "^OLLAMA_MODELS=" /etc/environment 2>/dev/null; then
        echo "OLLAMA_MODELS=/data/models/ollama" >> /etc/environment
    fi

    ok "Model storage directories and environment configured."
}

# ---------------------------------------------------------------------------
# Stage 15: Monitoring Tools
# ---------------------------------------------------------------------------
stage_15_monitoring() {
    info "Installing monitoring tools (htop, iotop, nvtop)..."
    apt_install htop iotop

    # Try apt first for nvtop, fall back to snap
    if apt_install nvtop 2>/dev/null; then
        ok "nvtop installed via apt."
    else
        warn "nvtop apt install failed, trying snap..."
        snap install nvtop --classic 2>&1 | tee -a "$LOG_FILE" || \
            snap install --no-wait nvtop 2>&1 | tee -a "$LOG_FILE" || \
            warn "nvtop snap install also failed. Install manually later."
    fi
}

# ---------------------------------------------------------------------------
# Stage 16: Tailscale
# ---------------------------------------------------------------------------
stage_16_tailscale() {
    info "Installing Tailscale..."
    curl -fsSL https://tailscale.com/install.sh | sh 2>&1 | tee -a "$LOG_FILE"

    if command -v tailscale &>/dev/null; then
        ok "Tailscale installed."
    fi

    info ""
    info ">>> To connect to your tailnet, run:"
    info ">>>   sudo tailscale up"
    info ""
}

# ---------------------------------------------------------------------------
# Stage 17: Systemd Services
# ---------------------------------------------------------------------------
stage_17_systemd_services() {
    info "Creating vLLM systemd service..."

    # vLLM environment file (user edits this to set model)
    mkdir -p /etc/vllm
    cat > /etc/vllm/default.env <<'VLLMENV'
# vLLM Configuration
# Set the model to serve. Examples:
#   MODEL=meta-llama/Llama-2-7b-chat-hf
#   MODEL=mistralai/Mistral-7B-Instruct-v0.2
MODEL=
VLLM_HOST=0.0.0.0
VLLM_PORT=8000
VLLM_MAX_MODEL_LEN=4096
VLLM_GPU_MEMORY_UTILIZATION=0.90
# Tensor parallelism across both RTX 3090 GPUs (48GB total VRAM)
VLLM_TENSOR_PARALLEL_SIZE=2
VLLMENV

    cat > /etc/systemd/system/vllm.service <<'VLLMSVC'
[Unit]
Description=vLLM Inference Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=zero20nez
Group=zero20nez
EnvironmentFile=/etc/vllm/default.env
Environment="HF_HOME=/data/models/huggingface"
ExecStart=/data/venvs/ml/bin/python -m vllm.entrypoints.openai.api_server \
    --model ${MODEL} \
    --host ${VLLM_HOST} \
    --port ${VLLM_PORT} \
    --max-model-len ${VLLM_MAX_MODEL_LEN} \
    --gpu-memory-utilization ${VLLM_GPU_MEMORY_UTILIZATION} \
    --tensor-parallel-size ${VLLM_TENSOR_PARALLEL_SIZE}
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
VLLMSVC
    info "Created /etc/systemd/system/vllm.service (disabled by default)"

    info "Creating llama.cpp server systemd service..."

    # llama-server environment file
    mkdir -p /etc/llama-cpp
    cat > /etc/llama-cpp/default.env <<'LLAMAENV'
# llama.cpp Server Configuration
# Set the model path (GGUF file). Example:
#   MODEL_PATH=/data/models/gguf/mistral-7b-instruct-v0.2.Q4_K_M.gguf
MODEL_PATH=
LLAMA_HOST=0.0.0.0
LLAMA_PORT=8080
LLAMA_CTX_SIZE=4096
LLAMA_N_GPU_LAYERS=99
# Split model across both RTX 3090 GPUs (auto-split)
LLAMA_SPLIT_MODE=layer
LLAMAENV

    cat > /etc/systemd/system/llama-server.service <<'LLAMASVC'
[Unit]
Description=llama.cpp Inference Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=zero20nez
Group=zero20nez
EnvironmentFile=/etc/llama-cpp/default.env
ExecStart=/usr/local/bin/llama-server \
    --model ${MODEL_PATH} \
    --host ${LLAMA_HOST} \
    --port ${LLAMA_PORT} \
    --ctx-size ${LLAMA_CTX_SIZE} \
    --n-gpu-layers ${LLAMA_N_GPU_LAYERS} \
    --split-mode ${LLAMA_SPLIT_MODE}
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
LLAMASVC
    info "Created /etc/systemd/system/llama-server.service (disabled by default)"

    info "Enabling Ollama service..."
    systemctl enable ollama.service 2>&1 | tee -a "$LOG_FILE" || \
        warn "Could not enable ollama service (may not exist yet)."

    info "Reloading systemd daemon..."
    systemctl daemon-reload 2>&1 | tee -a "$LOG_FILE"

    ok "Systemd services configured."
    info "  vLLM:        sudo systemctl enable --now vllm.service"
    info "  llama.cpp:   sudo systemctl enable --now llama-server.service"
    info "  Ollama:      enabled (starts on boot)"
}

# ---------------------------------------------------------------------------
# Stage 18: Final Configuration
# ---------------------------------------------------------------------------
stage_18_final_config() {
    local bash_aliases="/home/${TARGET_USER}/.bash_aliases"

    info "Setting up bash aliases for ${TARGET_USER}..."
    # Create or append, avoid duplicates
    touch "$bash_aliases"

    local -A aliases=(
        ["ml"]="source /data/venvs/ml/bin/activate"
        ["gpu"]="nvidia-smi"
        ["gpumon"]="nvtop"
    )

    for alias_name in "${!aliases[@]}"; do
        local alias_cmd="${aliases[$alias_name]}"
        if ! grep -q "alias ${alias_name}=" "$bash_aliases" 2>/dev/null; then
            echo "alias ${alias_name}='${alias_cmd}'" >> "$bash_aliases"
            info "  Added alias: ${alias_name}"
        else
            info "  Alias already exists: ${alias_name}"
        fi
    done

    chown "${TARGET_USER}:${TARGET_USER}" "$bash_aliases"

    info "Ensuring ownership of /data..."
    chown -R "${TARGET_USER}:${TARGET_USER}" /data 2>/dev/null || true

    info "Ensuring ownership of /home/${TARGET_USER}..."
    chown -R "${TARGET_USER}:${TARGET_USER}" "/home/${TARGET_USER}" 2>/dev/null || true

    # -----------------------------------------------------------------------
    # Print final summary
    # -----------------------------------------------------------------------
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${GREEN}================================================================${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${GREEN}  ML/AI Stack Installation Complete (2x RTX 3090 — 48GB VRAM)${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${GREEN}================================================================${NC}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Installed Components:${NC}" | tee -a "$LOG_FILE"
    echo -e "  NVIDIA Driver:    headless-${NVIDIA_DRIVER_VER}" | tee -a "$LOG_FILE"
    echo -e "  CUDA Toolkit:     ${CUDA_VER//-/.}" | tee -a "$LOG_FILE"
    echo -e "  cuDNN:            libcudnn9-cuda-12" | tee -a "$LOG_FILE"
    echo -e "  Node.js:          $(node --version 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo -e "  Claude CLI:       $(claude --version 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo -e "  Python:           $(python3.11 --version 2>/dev/null || echo '3.11')" | tee -a "$LOG_FILE"
    echo -e "  PyTorch:          $(${ML_VENV}/bin/python -c 'import torch; print(torch.__version__)' 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo -e "  vLLM:             $(${ML_VENV}/bin/python -c 'import vllm; print(vllm.__version__)' 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo -e "  Ollama:           $(ollama --version 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo -e "  llama.cpp:        /opt/llama.cpp (CUDA build)" | tee -a "$LOG_FILE"
    echo -e "  Tailscale:        $(tailscale version 2>/dev/null || echo 'installed')" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}ML Virtual Environment:${NC}" | tee -a "$LOG_FILE"
    echo -e "  ${ML_VENV}" | tee -a "$LOG_FILE"
    echo -e "  Activate:  source ${ML_VENV}/bin/activate  (or use alias 'ml')" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Training Libraries:${NC}" | tee -a "$LOG_FILE"
    echo -e "  transformers, datasets, accelerate, peft" | tee -a "$LOG_FILE"
    echo -e "  bitsandbytes, deepspeed, wandb" | tee -a "$LOG_FILE"
    echo -e "  safetensors, sentencepiece, tokenizers" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Data Directories:${NC}" | tee -a "$LOG_FILE"
    echo -e "  Models (HF):      /data/models/huggingface" | tee -a "$LOG_FILE"
    echo -e "  Models (Ollama):   /data/models/ollama" | tee -a "$LOG_FILE"
    echo -e "  Models (GGUF):     /data/models/gguf" | tee -a "$LOG_FILE"
    echo -e "  Training data:     /data/training" | tee -a "$LOG_FILE"
    echo -e "  Checkpoints:       /data/checkpoints" | tee -a "$LOG_FILE"
    echo -e "  Logs:              /data/logs" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Services (systemd):${NC}" | tee -a "$LOG_FILE"
    echo -e "  ollama.service        enabled (auto-start)" | tee -a "$LOG_FILE"
    echo -e "  vllm.service          disabled (configure model first)" | tee -a "$LOG_FILE"
    echo -e "  llama-server.service  disabled (configure model first)" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Quick-Start:${NC}" | tee -a "$LOG_FILE"
    echo -e "  ml          -- activate ML venv" | tee -a "$LOG_FILE"
    echo -e "  gpu         -- show nvidia-smi" | tee -a "$LOG_FILE"
    echo -e "  gpumon      -- launch nvtop" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}Log file:${NC}  ${LOG_FILE}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}State file:${NC} ${STATE_FILE}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
}

# ---------------------------------------------------------------------------
# Stage 19: Reboot Check
# ---------------------------------------------------------------------------
stage_19_reboot_check() {
    # Check if the NVIDIA driver stage completed but nvidia-smi doesn't work
    # (meaning we haven't rebooted since driver install)
    local needs_reboot=0

    if stage_done 2; then
        if ! nvidia-smi &>/dev/null; then
            needs_reboot=1
        fi
    fi

    if (( needs_reboot )); then
        echo "" | tee -a "$LOG_FILE"
        echo -e "${BOLD}${YELLOW}================================================================${NC}" | tee -a "$LOG_FILE"
        echo -e "${BOLD}${YELLOW}  REBOOT REQUIRED${NC}" | tee -a "$LOG_FILE"
        echo -e "${BOLD}${YELLOW}================================================================${NC}" | tee -a "$LOG_FILE"
        echo "" | tee -a "$LOG_FILE"
        echo -e "The NVIDIA headless driver was installed but the system has not" | tee -a "$LOG_FILE"
        echo -e "been rebooted. A reboot is required to load the NVIDIA kernel" | tee -a "$LOG_FILE"
        echo -e "module and enable GPU access." | tee -a "$LOG_FILE"
        echo "" | tee -a "$LOG_FILE"
        echo -e "After reboot, verify with:  ${BOLD}nvidia-smi${NC}" | tee -a "$LOG_FILE"
        echo "" | tee -a "$LOG_FILE"

        if (( SKIP_REBOOT )); then
            info "Reboot skipped (--skip-reboot flag set)."
            info "Reboot manually when ready:  sudo reboot"
        else
            echo -e "${BOLD}To reboot now, run:${NC}" | tee -a "$LOG_FILE"
            echo -e "  sudo reboot" | tee -a "$LOG_FILE"
            echo "" | tee -a "$LOG_FILE"
        fi
    else
        if stage_done 2; then
            ok "NVIDIA driver is loaded. GPUs are accessible."
            nvidia-smi 2>&1 | tee -a "$LOG_FILE" || true
            local gpu_count
            gpu_count="$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | wc -l)"
            if (( gpu_count >= 2 )); then
                ok "Detected ${gpu_count} NVIDIA GPUs — dual 3090 confirmed."
            else
                warn "Expected 2 NVIDIA GPUs but detected ${gpu_count}. Check PCIe seating."
            fi
        fi
        ok "No reboot required."
    fi
}

# ---------------------------------------------------------------------------
# Main execution
# ---------------------------------------------------------------------------
main() {
    echo "" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${CYAN}================================================================${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${CYAN}  Headless Ubuntu ML/AI Stack Provisioner v${SCRIPT_VERSION}${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${CYAN}  $(date)${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}${CYAN}================================================================${NC}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"

    if (( START_STAGE > 1 )); then
        info "Resuming from stage ${START_STAGE}"
    fi

    # Export DEBIAN_FRONTEND for the entire run
    export DEBIAN_FRONTEND=noninteractive

    # Ensure CUDA is on PATH if already installed (for resumed runs)
    if [[ -d "${CUDA_PATH}/bin" ]]; then
        export PATH="${CUDA_PATH}/bin:$PATH"
        export LD_LIBRARY_PATH="${CUDA_PATH}/lib64:${LD_LIBRARY_PATH:-}"
    fi

    run_stage  1 "System Update"              stage_01_system_update
    run_stage  2 "NVIDIA Headless Driver"      stage_02_nvidia_driver
    run_stage  3 "CUDA 12.4 Toolkit"           stage_03_cuda_toolkit
    run_stage  4 "cuDNN"                       stage_04_cudnn
    run_stage  5 "Node.js 20"                  stage_05_nodejs
    run_stage  6 "Claude CLI"                  stage_06_claude_cli
    run_stage  7 "Python 3.11"                 stage_07_python
    run_stage  8 "PyTorch (CUDA 12.4)"         stage_08_pytorch
    run_stage  9 "vLLM"                        stage_09_vllm
    run_stage 10 "Ollama"                      stage_10_ollama
    run_stage 11 "llama.cpp (CUDA build)"      stage_11_llama_cpp
    run_stage 12 "HuggingFace TGI"             stage_12_tgi
    run_stage 13 "Training Tools"              stage_13_training_tools
    run_stage 14 "Model Storage Setup"         stage_14_model_storage
    run_stage 15 "Monitoring Tools"            stage_15_monitoring
    run_stage 16 "Tailscale"                   stage_16_tailscale
    run_stage 17 "Systemd Services"            stage_17_systemd_services
    run_stage 18 "Final Configuration"         stage_18_final_config
    run_stage 19 "Reboot Check"                stage_19_reboot_check

    echo "" | tee -a "$LOG_FILE"
    log "All stages complete. Total stages: ${TOTAL_STAGES}"
    echo "" | tee -a "$LOG_FILE"
}

# Run
main "$@"
