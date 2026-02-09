#!/usr/bin/env bash
set -euo pipefail
#
# verify-install.sh — Comprehensive system health report for the GPU-3090.
#
# This script is designed to run ON the 3090 machine (via SSH from the Makefile).
# It checks all major subsystems and generates a formatted report, both to the
# terminal and to /tmp/gpu-3090-report.txt.
#

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
REPORT_FILE="/tmp/gpu-3090-report.txt"
TIMESTAMP="$(date '+%Y-%m-%d %H:%M:%S')"
HOSTNAME_VAL="$(hostname 2>/dev/null || echo 'unknown')"

# Width for the report
W=55

# ---------------------------------------------------------------------------
# Helpers — formatted output that goes to both terminal and report file
# ---------------------------------------------------------------------------

# Initialize the report file
init_report() {
    : > "${REPORT_FILE}"
}

# Print to both stdout and the report file
out() {
    printf '%s\n' "$*"
    printf '%s\n' "$*" >> "${REPORT_FILE}"
}

# Print a major section header
section() {
    out ""
    out "--- ${1} $(printf '%0.s-' $(seq 1 $((W - ${#1} - 5))))"
}

# Print a key-value pair with dot-leaders
kv() {
    local key="$1"
    local value="$2"
    local dots
    local key_len=${#key}
    local dot_count=$((20 - key_len))
    if (( dot_count < 2 )); then dot_count=2; fi
    dots="$(printf '%0.s.' $(seq 1 "${dot_count}"))"
    out "  ${key} ${dots} ${value}"
}

# Print a sub-section label
sub() {
    out ""
    out "  $1:"
}

# Print indented content (for multi-line command output)
indented() {
    while IFS= read -r line; do
        out "    ${line}"
    done <<< "$1"
}

# Safely run a command and return output or a fallback message
safe() {
    local output
    if output="$(eval "$1" 2>/dev/null)"; then
        echo "${output}"
    else
        echo "${2:-not available}"
    fi
}

# Check if a command exists
has_cmd() {
    command -v "$1" &>/dev/null
}

# Get systemd service status (one-liner)
service_status() {
    local svc="$1"
    if systemctl is-active --quiet "${svc}" 2>/dev/null; then
        echo "active (running)"
    elif systemctl is-enabled --quiet "${svc}" 2>/dev/null; then
        echo "enabled (not running)"
    elif systemctl list-unit-files "${svc}.service" &>/dev/null; then
        echo "inactive"
    else
        echo "not installed"
    fi
}

# ---------------------------------------------------------------------------
# Report Sections
# ---------------------------------------------------------------------------

report_header() {
    local bar
    bar="$(printf '%0.s=' $(seq 1 "${W}"))"
    out "${bar}"
    out "  GPU-3090 SYSTEM REPORT (2x RTX 3090)"
    out "  Generated: ${TIMESTAMP}"
    out "${bar}"
}

report_system() {
    section "SYSTEM"

    kv "Hostname" "${HOSTNAME_VAL}"

    local ubuntu_ver
    ubuntu_ver="$(safe 'lsb_release -ds' 'unknown')"
    kv "Ubuntu" "${ubuntu_ver}"

    local kernel
    kernel="$(uname -r)"
    kv "Kernel" "${kernel}"

    local arch
    arch="$(uname -m)"
    kv "Architecture" "${arch}"

    local uptime_val
    uptime_val="$(safe 'uptime -p' 'unknown')"
    kv "Uptime" "${uptime_val}"

    local load
    load="$(safe "awk '{print \$1, \$2, \$3}' /proc/loadavg" 'unknown')"
    kv "Load Average" "${load}"

    local mem_total mem_avail
    mem_total="$(safe "free -h | awk '/^Mem:/{print \$2}'" 'unknown')"
    mem_avail="$(safe "free -h | awk '/^Mem:/{print \$7}'" 'unknown')"
    kv "Memory" "${mem_total} total, ${mem_avail} available"
}

report_gpu() {
    section "GPU"

    if has_cmd nvidia-smi; then
        # Count GPUs
        local gpu_count
        gpu_count="$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | wc -l)"
        kv "NVIDIA GPUs" "${gpu_count}"

        # Per-GPU details
        local idx=0
        while IFS=',' read -r name mem_total; do
            name="$(echo "$name" | xargs)"
            mem_total="$(echo "$mem_total" | xargs)"
            kv "GPU ${idx}" "${name} (${mem_total})"
            (( idx++ )) || true
        done < <(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null)

        # Total VRAM
        local total_mb=0
        while IFS= read -r mem; do
            mem="$(echo "$mem" | grep -oP '[0-9]+')"
            (( total_mb += mem )) || true
        done < <(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>/dev/null)
        kv "Total VRAM" "$((total_mb / 1024)) GB"

        local driver_ver
        driver_ver="$(safe 'nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1' 'unknown')"
        kv "Driver Version" "${driver_ver}"

        local compute_cap
        compute_cap="$(safe 'nvidia-smi --query-gpu=compute_cap --format=csv,noheader | head -1' 'unknown')"
        kv "Compute Cap" "${compute_cap}"

        # Per-GPU temps and utilization
        sub "GPU Status (per-GPU)"
        indented "$(nvidia-smi --format=csv,noheader --query-gpu=index,name,memory.used,memory.total,utilization.gpu,temperature.gpu 2>/dev/null || echo 'unable to query')"

        # NVLink / P2P check
        local p2p_status
        p2p_status="$(nvidia-smi topo -m 2>/dev/null | head -5 || echo 'topology query unavailable')"
        if [[ -n "${p2p_status}" ]]; then
            sub "GPU Topology (NVLink/P2P)"
            indented "${p2p_status}"
        fi

        # CUDA toolkit version (from nvcc if available)
        if has_cmd nvcc; then
            local nvcc_ver
            nvcc_ver="$(safe 'nvcc --version | grep "release" | awk "{print \$NF}"' 'unknown')"
            kv "CUDA Toolkit" "${nvcc_ver}"
        else
            kv "CUDA Toolkit" "nvcc not in PATH"
        fi

        # Intel iGPU detection
        if lspci 2>/dev/null | grep -qi "intel.*vga\|intel.*display\|intel.*graphics"; then
            local intel_gpu
            intel_gpu="$(lspci 2>/dev/null | grep -i 'intel.*vga\|intel.*display\|intel.*graphics' | head -1 | sed 's/.*: //')"
            kv "Intel iGPU" "${intel_gpu} (not used for ML)"
        fi
    else
        kv "nvidia-smi" "NOT FOUND"
        out "  WARNING: NVIDIA drivers do not appear to be installed."
    fi
}

report_storage() {
    section "STORAGE"

    sub "Block devices (lsblk)"
    indented "$(lsblk -o NAME,SIZE,TYPE,FSTYPE,MOUNTPOINT 2>/dev/null || echo 'lsblk not available')"

    sub "LUKS status"
    if has_cmd cryptsetup; then
        local luks_devs
        luks_devs="$(lsblk -o NAME,FSTYPE -n | grep crypto_LUKS | awk '{print $1}' || true)"
        if [[ -n "${luks_devs}" ]]; then
            while IFS= read -r dev; do
                local status
                status="$(safe "cryptsetup status /dev/mapper/${dev}" 'unknown')"
                kv "LUKS device" "/dev/${dev}"
                indented "${status}"
            done <<< "${luks_devs}"
        else
            # Try to find dm- devices that are LUKS
            local dm_devs
            dm_devs="$(dmsetup ls --target crypt 2>/dev/null | awk '{print $1}' || true)"
            if [[ -n "${dm_devs}" ]]; then
                while IFS= read -r dev; do
                    kv "LUKS mapped" "/dev/mapper/${dev}"
                done <<< "${dm_devs}"
            else
                out "    No active LUKS devices detected."
            fi
        fi
    else
        out "    cryptsetup not installed."
    fi

    sub "Disk usage (df -h)"
    indented "$(df -h --output=source,size,used,avail,pcent,target -x tmpfs -x devtmpfs -x squashfs 2>/dev/null || df -h 2>/dev/null || echo 'df not available')"
}

report_network() {
    section "NETWORK"

    sub "IP addresses"
    indented "$(ip -4 addr show | grep 'inet ' | awk '{print $NF ": " $2}' 2>/dev/null || echo 'unable to query')"

    local gw
    gw="$(safe "ip route | grep default | awk '{print \$3}'" 'unknown')"
    kv "Gateway" "${gw}"

    sub "DNS servers"
    indented "$(safe "resolvectl dns 2>/dev/null || grep nameserver /etc/resolv.conf 2>/dev/null" 'unable to determine')"

    sub "Tailscale"
    if has_cmd tailscale; then
        local ts_status
        ts_status="$(safe 'tailscale status --json 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"Status: {d.get(\"BackendState\",\"unknown\")}\")" 2>/dev/null' '')"
        if [[ -n "${ts_status}" ]]; then
            indented "${ts_status}"
        fi
        local ts_ip
        ts_ip="$(safe 'tailscale ip -4' 'not connected')"
        kv "Tailscale IP" "${ts_ip}"
        local ts_hostname
        ts_hostname="$(safe 'tailscale status --self --json 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get(\"Self\",{}).get(\"HostName\",\"unknown\"))" 2>/dev/null' 'unknown')"
        kv "Tailscale Host" "${ts_hostname}"
    else
        kv "Tailscale" "not installed"
    fi
}

report_ssh() {
    section "SSH"

    kv "sshd" "$(service_status sshd)"
    kv "dropbear" "$(service_status dropbear)"
    kv "dropbear-initramfs" "$(service_status dropbear-initramfs)"

    sub "SSH host key fingerprints"
    for keyfile in /etc/ssh/ssh_host_*_key.pub; do
        if [[ -f "${keyfile}" ]]; then
            indented "$(ssh-keygen -lf "${keyfile}" 2>/dev/null || echo "unable to read ${keyfile}")"
        fi
    done

    sub "Authorized keys count"
    local user_auth="${HOME}/.ssh/authorized_keys"
    if [[ -f "${user_auth}" ]]; then
        local count
        count="$(grep -c '^ssh-' "${user_auth}" 2>/dev/null || echo 0)"
        kv "User keys" "${count} key(s) in ${user_auth}"
    else
        kv "User keys" "no authorized_keys file"
    fi

    local dropbear_auth="/etc/dropbear/initramfs/authorized_keys"
    if [[ -f "${dropbear_auth}" ]]; then
        local count
        count="$(grep -c 'ssh-' "${dropbear_auth}" 2>/dev/null || echo 0)"
        kv "Dropbear keys" "${count} key(s) in ${dropbear_auth}"
    else
        kv "Dropbear keys" "not configured"
    fi
}

report_software() {
    section "SOFTWARE VERSIONS"

    # Python
    if has_cmd python3; then
        kv "Python" "$(safe 'python3 --version 2>&1 | awk "{print \$2}"' 'unknown')"
    else
        kv "Python" "not installed"
    fi

    # PyTorch + CUDA check
    if has_cmd python3; then
        local torch_ver
        torch_ver="$(safe 'python3 -c "import torch; print(torch.__version__)" 2>/dev/null' 'not installed')"
        kv "PyTorch" "${torch_ver}"

        if [[ "${torch_ver}" != "not installed" ]]; then
            local cuda_available
            cuda_available="$(safe 'python3 -c "import torch; print(torch.cuda.is_available())" 2>/dev/null' 'unknown')"
            kv "PyTorch CUDA" "${cuda_available}"

            if [[ "${cuda_available}" == "True" ]]; then
                local cuda_device
                cuda_device="$(safe 'python3 -c "import torch; print(torch.cuda.get_device_name(0))" 2>/dev/null' 'unknown')"
                kv "CUDA Device" "${cuda_device}"
            fi
        fi
    fi

    # Node.js
    if has_cmd node; then
        kv "Node.js" "$(safe 'node --version' 'unknown')"
    else
        kv "Node.js" "not installed"
    fi

    # npm
    if has_cmd npm; then
        kv "npm" "$(safe 'npm --version' 'unknown')"
    else
        kv "npm" "not installed"
    fi

    # Claude CLI
    if has_cmd claude; then
        kv "Claude CLI" "$(safe 'claude --version 2>&1 | head -1' 'unknown')"
    else
        kv "Claude CLI" "not installed"
    fi

    # Ollama
    if has_cmd ollama; then
        kv "Ollama" "$(safe 'ollama --version 2>&1 | head -1' 'unknown')"
    else
        kv "Ollama" "not installed"
    fi

    # vLLM
    if has_cmd vllm; then
        kv "vLLM" "$(safe 'vllm --version 2>&1 | head -1' 'unknown')"
    elif python3 -c "import vllm" 2>/dev/null; then
        kv "vLLM" "$(safe 'python3 -c "import vllm; print(vllm.__version__)"' 'installed (version unknown)')"
    else
        kv "vLLM" "not installed"
    fi

    # llama.cpp
    if has_cmd llama-server || has_cmd llama-cli; then
        local llama_bin
        llama_bin="$(command -v llama-server 2>/dev/null || command -v llama-cli 2>/dev/null)"
        kv "llama.cpp" "installed (${llama_bin})"
    elif [[ -d "/opt/llama.cpp" || -d "${HOME}/llama.cpp" ]]; then
        kv "llama.cpp" "installed (source build)"
    else
        kv "llama.cpp" "not installed"
    fi
}

report_services() {
    section "SERVICES"

    local services=(
        "ollama"
        "vllm"
        "dropbear"
        "sshd"
        "tailscaled"
        "fail2ban"
        "ufw"
        "docker"
    )

    for svc in "${services[@]}"; do
        kv "${svc}" "$(service_status "${svc}")"
    done
}

report_model_storage() {
    section "MODEL STORAGE"

    if [[ -d "/data" ]]; then
        sub "/data usage breakdown"
        indented "$(du -sh /data/*/ 2>/dev/null || echo 'empty or not accessible')"
        echo ""
        local total
        total="$(safe 'du -sh /data 2>/dev/null | awk "{print \$1}"' 'unknown')"
        kv "Total /data" "${total}"
    else
        out "    /data directory does not exist."
    fi

    # Also check common model locations
    for dir in "${HOME}/.ollama/models" "${HOME}/.cache/huggingface"; do
        if [[ -d "${dir}" ]]; then
            local size
            size="$(safe "du -sh '${dir}' 2>/dev/null | awk '{print \$1}'" 'unknown')"
            kv "$(basename "$(dirname "${dir}")")/$(basename "${dir}")" "${size}"
        fi
    done
}

report_security() {
    section "SECURITY"

    # UFW firewall
    if has_cmd ufw; then
        local ufw_status
        ufw_status="$(safe 'ufw status | head -1' 'unknown')"
        kv "UFW Firewall" "${ufw_status}"

        sub "UFW rules"
        indented "$(safe 'ufw status numbered 2>/dev/null' 'unable to query')"
    else
        kv "UFW Firewall" "not installed"
    fi

    # fail2ban
    if has_cmd fail2ban-client; then
        kv "fail2ban" "$(service_status fail2ban)"
        sub "fail2ban jails"
        indented "$(safe 'fail2ban-client status 2>/dev/null' 'unable to query')"
    else
        kv "fail2ban" "not installed"
    fi

    # SSH root login
    local root_login
    root_login="$(safe "grep -E '^PermitRootLogin' /etc/ssh/sshd_config 2>/dev/null | awk '{print \$2}'" 'unknown')"
    kv "SSH Root Login" "${root_login:-not explicitly set}"

    # Password auth
    local pass_auth
    pass_auth="$(safe "grep -E '^PasswordAuthentication' /etc/ssh/sshd_config 2>/dev/null | awk '{print \$2}'" 'unknown')"
    kv "SSH Password Auth" "${pass_auth:-not explicitly set}"
}

report_footer() {
    local bar
    bar="$(printf '%0.s=' $(seq 1 "${W}"))"
    out ""
    out "${bar}"
    out "  Report saved to: ${REPORT_FILE}"
    out "  Run 'scp ${HOSTNAME_VAL}:${REPORT_FILE} .' to download."
    out "${bar}"
    out ""
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    init_report

    report_header
    report_system
    report_gpu
    report_storage
    report_network
    report_ssh
    report_software
    report_services
    report_model_storage
    report_security
    report_footer
}

main "$@"
