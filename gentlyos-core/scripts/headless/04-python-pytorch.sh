#!/usr/bin/env bash
# ============================================================
# Stage 4: Python & PyTorch
# ============================================================
# Installs Python, creates a venv, installs PyTorch with
# auto-detected compute backend (CUDA/ROCm/CPU).
# ============================================================
set -euo pipefail

log() { echo "[stage-4] $*"; }

# --- Detect distro for package manager ---
detect_distro() {
    if [[ -f /etc/os-release ]]; then
        # shellcheck source=/dev/null
        source /etc/os-release
        echo "$ID"
    else
        echo "unknown"
    fi
}

DISTRO=$(detect_distro)

# --- Install Python ---
log "Installing Python ${PYTHON_VERSION}..."

case "$DISTRO" in
    ubuntu|debian|pop|linuxmint)
        # Add deadsnakes PPA for specific versions on Ubuntu
        if [[ "$DISTRO" == "ubuntu" ]]; then
            apt-get install -y software-properties-common
            add-apt-repository -y ppa:deadsnakes/ppa 2>/dev/null || true
            apt-get update -y
        fi
        apt-get install -y \
            "python${PYTHON_VERSION}" \
            "python${PYTHON_VERSION}-venv" \
            "python${PYTHON_VERSION}-dev" \
            python3-pip \
            2>/dev/null || apt-get install -y python3 python3-venv python3-dev python3-pip
        PYTHON_BIN="python${PYTHON_VERSION}"
        if ! command -v "$PYTHON_BIN" &>/dev/null; then
            PYTHON_BIN="python3"
        fi
        ;;
    fedora|rhel|centos|rocky|almalinux)
        dnf install -y \
            "python${PYTHON_VERSION//./}" \
            "python${PYTHON_VERSION//./}-devel" \
            "python${PYTHON_VERSION//./}-pip" \
            2>/dev/null || dnf install -y python3 python3-devel python3-pip
        PYTHON_BIN="python${PYTHON_VERSION}"
        if ! command -v "$PYTHON_BIN" &>/dev/null; then
            PYTHON_BIN="python3"
        fi
        ;;
    arch|manjaro)
        pacman -S --noconfirm --needed python python-pip python-virtualenv
        PYTHON_BIN="python3"
        ;;
    alpine)
        apk add python3 python3-dev py3-pip py3-virtualenv
        PYTHON_BIN="python3"
        ;;
    *)
        log "Attempting generic Python install..."
        PYTHON_BIN="python3"
        if ! command -v python3 &>/dev/null; then
            log "ERROR: Python 3 not found. Install manually and re-run --stage 4"
            exit 1
        fi
        ;;
esac

log "Python binary: $($PYTHON_BIN --version)"

# --- Create virtual environment ---
log "Creating Python venv at ${PYTORCH_VENV_PATH}..."
mkdir -p "$(dirname "$PYTORCH_VENV_PATH")"
$PYTHON_BIN -m venv "$PYTORCH_VENV_PATH"

# Activate venv
source "${PYTORCH_VENV_PATH}/bin/activate"

# Upgrade pip
pip install --upgrade pip setuptools wheel

# --- Detect compute backend ---
detect_compute() {
    if [[ "${PYTORCH_COMPUTE}" != "auto" ]]; then
        echo "$PYTORCH_COMPUTE"
        return
    fi

    # Check for NVIDIA GPU
    if command -v nvidia-smi &>/dev/null; then
        CUDA_VER=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -1)
        log "NVIDIA GPU detected (driver: $CUDA_VER)"

        # Check CUDA toolkit version
        if command -v nvcc &>/dev/null; then
            NVCC_VER=$(nvcc --version | grep -oP 'release \K[0-9]+\.[0-9]+')
            log "CUDA toolkit: $NVCC_VER"
            case "$NVCC_VER" in
                12.4*|12.5*|12.6*) echo "cu124" ;;
                12.1*|12.2*|12.3*) echo "cu121" ;;
                11.8*)             echo "cu118" ;;
                *)                 echo "cu124" ;;  # default to latest
            esac
        else
            # No nvcc but nvidia-smi exists, use latest CUDA
            echo "cu124"
        fi
        return
    fi

    # Check for AMD ROCm
    if command -v rocminfo &>/dev/null || [[ -d /opt/rocm ]]; then
        log "AMD ROCm detected"
        echo "rocm6.0"
        return
    fi

    log "No GPU detected, using CPU"
    echo "cpu"
}

COMPUTE=$(detect_compute)
log "Compute backend: $COMPUTE"

# --- Install PyTorch ---
log "Installing PyTorch (compute: $COMPUTE)..."

case "$COMPUTE" in
    cu124)
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu124
        ;;
    cu121)
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
        ;;
    cu118)
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118
        ;;
    rocm6.0|rocm)
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.0
        ;;
    cpu|*)
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
        ;;
esac

# --- Install additional ML packages ---
log "Installing additional ML packages..."
pip install \
    transformers \
    accelerate \
    safetensors \
    sentencepiece \
    protobuf \
    numpy \
    requests \
    fastapi \
    uvicorn

# --- Verify installation ---
log "Verifying PyTorch installation..."
python -c "
import torch
print(f'PyTorch version: {torch.__version__}')
print(f'CUDA available:  {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'CUDA device:     {torch.cuda.get_device_name(0)}')
    print(f'CUDA version:    {torch.version.cuda}')
print(f'CPU threads:     {torch.get_num_threads()}')
"

# --- Set ownership ---
chown -R "${GENTLY_USER}:${GENTLY_USER}" "$PYTORCH_VENV_PATH"

log "Python & PyTorch installed successfully."
log "  Venv: $PYTORCH_VENV_PATH"
log "  Activate: source ${PYTORCH_VENV_PATH}/bin/activate"
