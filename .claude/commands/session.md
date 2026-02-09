---
description: Manage cage sessions — start, stop, list, inspect, destroy
argument-hint: <subcommand> [args...] (e.g., start --mode cli, stop --all, list, inspect <name>)
allowed-tools: [Bash, Read]
---

# /session — Cage Session Management

The user invoked `/session` with: $ARGUMENTS

You are managing claude-cage container sessions.

## Environment Setup

```
export CAGE_ROOT="/home/zero20nez/Desktop/claude-cage"
```

## Subcommand Routing

Parse `$ARGUMENTS` and route:

### `start` or `launch`
Run: `$CAGE_ROOT/bin/claude-cage start $REMAINING_ARGS`
Default: `--mode cli --network filtered`
Show the session name and how to attach/stop.

### `stop`
If `--all`: `$CAGE_ROOT/bin/claude-cage stop --all`
Otherwise: `$CAGE_ROOT/bin/claude-cage stop <name>`

### `list` or `ls`
Run: `docker ps --filter "label=managed-by=claude-cage" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}"`
Also show stopped containers: `docker ps -a --filter "label=managed-by=claude-cage" --format "table {{.Names}}\t{{.Status}}"`

### `inspect` or `info`
Run: `docker inspect cage-<name> --format` for security settings, resource usage, network mode.
Present as a clean status card.

### `destroy` or `rm`
Run: `$CAGE_ROOT/bin/claude-cage destroy <name>`
Warn before destroying if container is running.

### `attach` or `shell`
Run: `$CAGE_ROOT/bin/claude-cage shell <name>`

### `logs`
Run: `docker logs cage-<name> --tail 50`

### `stats`
Run: `docker stats --no-stream --filter "label=managed-by=claude-cage"`

### No args or `help`
Show available subcommands with examples.

## After Every Operation

Log to MongoDB:
```bash
node /home/zero20nez/Desktop/claude-cage/mongodb/store.js log "command" "session:$SUBCOMMAND" '{"args":"$ARGUMENTS","status":"ok"}'
```
