---
description: Build cage container images — CLI, Desktop, or all
argument-hint: [target] (cli, desktop, all — defaults to all)
allowed-tools: [Bash]
---

# /build — Build Container Images

The user invoked `/build` with: $ARGUMENTS

## Routing

Parse `$ARGUMENTS`:

### `cli`
```bash
cd /home/zero20nez/Desktop/claude-cage && make build-cli
```

### `desktop`
```bash
cd /home/zero20nez/Desktop/claude-cage && make build-desktop
```

### `all` or empty
```bash
cd /home/zero20nez/Desktop/claude-cage && make build
```

### `clean`
```bash
cd /home/zero20nez/Desktop/claude-cage && make clean-images
```

### `rebuild`
```bash
cd /home/zero20nez/Desktop/claude-cage && make clean-images && make build
```

## After Build

1. Show the resulting images: `docker images | grep claude-cage`
2. Log to MongoDB:
   ```bash
   node /home/zero20nez/Desktop/claude-cage/mongodb/store.js log "docker" "build:$TARGET" '{"status":"ok"}'
   ```
3. Suggest next step: `claude-cage start --mode cli`
