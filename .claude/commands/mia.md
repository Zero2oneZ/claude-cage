---
description: Encrypted secrets manager — encrypt, pin, spawn, list, status
argument-hint: <subcommand> [args...] (e.g., encrypt, pin, spawn twilio, list, status)
allowed-tools: [Bash, Read]
---

# /mia — SOPS + age Encrypted Secrets Manager

The user invoked `/mia` with: $ARGUMENTS

You are managing encrypted secrets via mia (SOPS + age + Pinata IPFS).

## Environment Setup

```
export CAGE_ROOT="/home/zero20nez/Desktop/claude-cage"
```

## Subcommand Routing

Route based on the first argument:

| Subcommand | Action |
|------------|--------|
| `init` | `python3 projects/mia/mia.py init` |
| `encrypt` | `python3 projects/mia/mia.py encrypt` |
| `decrypt` | `python3 projects/mia/mia.py decrypt [--project P]` |
| `pin` | `python3 projects/mia/mia.py pin [--project P]` |
| `spawn <project>` | `python3 projects/mia/mia.py spawn <project> [--keys K1,K2] [--os linux]` |
| `list` | `python3 projects/mia/mia.py list [--project P]` |
| `pull <cid>` | `python3 projects/mia/mia.py pull <cid> [-o file]` |
| `status` | `python3 projects/mia/mia.py status` |
| (empty) | `python3 projects/mia/mia.py status` |

## Security Rules

1. **NEVER** display raw decrypted secrets unless the user explicitly asks for `decrypt`
2. **NEVER** write decrypted values to files — decrypt outputs to stdout only
3. Raw `.env` files and age private keys must never be committed or uploaded
4. Only SOPS-encrypted ciphertext (`.enc.yaml`) is safe for IPFS pinning

## Workflow

Typical first-time flow:
```
mia init       → age keypair + .sops.yaml
mia encrypt    → .env → vault.global.enc.yaml
mia pin        → encrypted vault → Pinata → CID → MongoDB
mia spawn X    → project vault → encrypted subset
mia pin -p X   → project vault → Pinata → CID → MongoDB (parent chain)
```

## Prerequisites

- `age` — `sudo apt install age`
- `sops` — download from https://github.com/getsops/sops/releases
- Pinata JWT in `.secrets/.env.gently.global` (PINATA_JWT key)
- MongoDB Atlas reachable (`make mongo-ping`)
