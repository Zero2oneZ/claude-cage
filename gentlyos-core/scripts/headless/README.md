# GentlyOS Headless Setup

Automated setup for running GentlyOS on a headless PC (no monitor). Installs everything
you need and configures the machine so you can SSH in or use the APIs from a 2nd PC
over ethernet.

## What It Installs

| Component | Details |
|-----------|---------|
| **System** | Build tools, SSH server, firewall, Rust toolchain |
| **Ollama** | Local LLM server + Qwen3-14B (configurable) |
| **Python** | Python 3.11, PyTorch (auto-detects CUDA/ROCm/CPU) |
| **GentlyOS** | CLI + Web interface built from source |
| **Networking** | Static IP for direct ethernet, nginx API gateway |

## Quick Start

### 1. Edit the config

```bash
cd scripts/headless
nano config.env
```

Key settings to change:
- `GENTLY_PASSWORD` - your login password
- `STATIC_IP` - the IP the headless machine will use (default: `192.168.1.100`)
- `OLLAMA_MODEL` - the model to pull (default: `qwen3:14b`)
- `PYTORCH_COMPUTE` - `auto` detects GPU, or force `cpu`/`cu124`/`rocm6.0`

### 2. Run on the headless PC

Plug in a monitor temporarily (or use a USB serial console) for this one step:

```bash
sudo bash scripts/headless/bootstrap.sh
```

Or if something fails partway through, resume from a specific stage:

```bash
sudo bash bootstrap.sh --stage 5   # resume from Ollama install
```

### 3. Connect from your 2nd PC

Connect an ethernet cable between the two machines. On your 2nd PC, set a static IP
in the same subnet:

```
IP:      192.168.1.2
Netmask: 255.255.255.0
Gateway: 192.168.1.1
```

Then:

```bash
# SSH in
ssh gently@192.168.1.100

# Test the API
curl http://192.168.1.100:9090/status

# Chat with the model
curl http://192.168.1.100:9090/api/chat -d '{
  "model": "qwen3:14b",
  "messages": [{"role": "user", "content": "Write a hello world in Rust"}]
}'

# Open GentlyOS Web UI in browser
open http://192.168.1.100:9090/
```

## Setup Stages

| # | Script | What It Does |
|---|--------|-------------|
| 1 | `01-system-base.sh` | OS detection, package install, Rust toolchain |
| 2 | `02-user-ssh.sh` | Create user account, SSH server config |
| 3 | `03-network.sh` | Static IP on ethernet, firewall rules |
| 4 | `04-python-pytorch.sh` | Python venv, PyTorch (CUDA/ROCm/CPU) |
| 5 | `05-ollama.sh` | Ollama install, model download |
| 6 | `06-gentlyos.sh` | Build GentlyOS from source, install |
| 7 | `07-services.sh` | systemd services, nginx API gateway |

Each stage is idempotent. If one fails, fix the issue and re-run with `--stage N`.
Completed stages are tracked in `/var/lib/gentlyos-setup/` and auto-skipped.

## Ports

| Port | Service | Access |
|------|---------|--------|
| 22 | SSH | `ssh gently@192.168.1.100` |
| 8080 | GentlyOS Web | `http://192.168.1.100:8080` |
| 9090 | API Gateway | `http://192.168.1.100:9090` (unified) |
| 11434 | Ollama | `http://192.168.1.100:11434` |

## API Gateway Routes (port 9090)

| Path | Routes To |
|------|-----------|
| `/` | GentlyOS Web UI |
| `/api/*` | Ollama API |
| `/ollama/*` | Ollama API |
| `/gently/*` | GentlyOS Web |
| `/status` | Health check JSON |

## Using With Tools on Your 2nd PC

### VS Code + Continue Extension
Set the Ollama endpoint to `http://192.168.1.100:11434`

### Open WebUI
```bash
docker run -d -p 3000:8080 \
  -e OLLAMA_BASE_URL=http://192.168.1.100:11434 \
  ghcr.io/open-webui/open-webui:main
```

### Python (requests)
```python
import requests
r = requests.post("http://192.168.1.100:9090/api/generate", json={
    "model": "qwen3:14b",
    "prompt": "Explain zero-knowledge proofs"
})
print(r.json()["response"])
```

## Supported Distros

- Ubuntu / Debian / Pop!_OS / Linux Mint
- Fedora / RHEL / CentOS / Rocky / Alma
- Arch / Manjaro
- Alpine Linux
- openSUSE

## Files

```
scripts/headless/
├── bootstrap.sh           # Master script (run this)
├── config.env             # All configuration in one place
├── 01-system-base.sh      # Stage 1: System packages + Rust
├── 02-user-ssh.sh         # Stage 2: User + SSH
├── 03-network.sh          # Stage 3: Static IP + firewall
├── 04-python-pytorch.sh   # Stage 4: Python + PyTorch
├── 05-ollama.sh           # Stage 5: Ollama + models
├── 06-gentlyos.sh         # Stage 6: Build GentlyOS
├── 07-services.sh         # Stage 7: Services + gateway
└── README.md              # This file
```

## Logs

All logs go to `/var/log/gentlyos-setup/`:
- `bootstrap.log` - Main orchestration log
- `stage-N.log` - Per-stage logs

## Troubleshooting

**Can't SSH in**: Check that your 2nd PC has an IP in the same subnet (192.168.1.x).
Try `ping 192.168.1.100` first.

**Model pull failed**: SSH in and run `ollama pull qwen3:14b` manually. Needs internet.

**PyTorch wrong backend**: Edit `config.env`, set `PYTORCH_COMPUTE=cpu` (or `cu124`),
then `sudo bash bootstrap.sh --stage 4`.

**Resume after failure**: Each stage creates a stamp file. Delete it to force re-run:
`sudo rm /var/lib/gentlyos-setup/stage-N.done && sudo bash bootstrap.sh --stage N`
