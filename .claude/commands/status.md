---
description: Quick status overview — running sessions, MongoDB connectivity, images, system health
argument-hint: (no arguments needed)
allowed-tools: [Bash, Read]
---

# /status — System Status Overview

The user invoked `/status`.

Run ALL of these checks in parallel and present a unified status card:

## 1. Running Sessions
```bash
docker ps --filter "label=managed-by=claude-cage" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null || echo "No sessions running"
```

## 2. Container Images
```bash
docker images --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}" | grep -E "claude-cage|REPOSITORY" || echo "No images built"
```

## 3. MongoDB Atlas
```bash
node /home/zero20nez/Desktop/claude-cage/mongodb/store.js ping 2>/dev/null || echo "MongoDB not reachable"
```

## 4. Docker Network
```bash
docker network ls --format "{{.Name}}" | grep cage-filtered && echo "Filtered network: OK" || echo "Filtered network: NOT CREATED"
```

## 5. Resource Usage (if sessions running)
```bash
docker stats --no-stream --filter "label=managed-by=claude-cage" --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}" 2>/dev/null
```

## 6. Recent Events (from MongoDB)
```bash
node /home/zero20nez/Desktop/claude-cage/mongodb/store.js get events '{}' 5 2>/dev/null
```

## Presentation Format

```
CLAUDE-CAGE STATUS
══════════════════════════════════════
Sessions:    X running, Y stopped
Images:      CLI [built/missing], Desktop [built/missing]
MongoDB:     [connected/unreachable]
Network:     [cage-filtered exists/missing]
──────────────────────────────────────
[Recent activity table if available]
══════════════════════════════════════
```
