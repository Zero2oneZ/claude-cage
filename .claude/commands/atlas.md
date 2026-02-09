---
description: Manage MongoDB Atlas — clusters, IP whitelist, db users, projects, metrics
argument-hint: <subcommand> [args...] (e.g., whitelist-add, clusters, dbusers, ping, login)
allowed-tools: [Bash, Read, Write, Grep]
---

# /atlas — MongoDB Atlas CLI Wrapper

The user invoked `/atlas` with: $ARGUMENTS

You are managing MongoDB Atlas infrastructure for the claude-cage project using the `atlas` CLI at `~/bin/atlas`.

## Environment Setup

Before running any atlas command, always export PATH:
```
export PATH="$HOME/bin:$PATH"
```

The MongoDB store configuration is at: `/home/zero20nez/Desktop/claude-cage/mongodb/.env`

## Subcommand Routing

Parse `$ARGUMENTS` and route to the appropriate operation:

### `login` or `auth`
Run `atlas auth login` — this opens a browser for OAuth. Guide the user through it.
After login, run `atlas auth whoami` to confirm.

### `whoami` or `status`
Run `atlas auth whoami` and `atlas config list` to show current auth and project context.

### `whitelist-add` or `ip-add` or `allow-ip`
Add the current machine's public IP to the Atlas access list:
1. Get current IP: `curl -s https://api.ipify.org`
2. Run: `atlas accessLists create <ip>/32 --comment "claude-cage $(date +%Y-%m-%d)" --output json`
3. Confirm with: `atlas accessLists list --output json`
4. Log to MongoDB: After success, note this enables the MongoDB store connection.

### `whitelist-list` or `ip-list`
Run: `atlas accessLists list --output json`
Present as a clean table.

### `clusters` or `cluster-list`
Run: `atlas clusters list --output json`
Present cluster name, state, tier, region, MongoDB version.

### `cluster-info` or `describe`
If arg provided: `atlas clusters describe <arg> --output json`
Otherwise describe the first/default cluster.

### `cluster-pause`
Run: `atlas clusters pause <cluster-name>`

### `cluster-resume`
Run: `atlas clusters start <cluster-name>`

### `dbusers` or `users`
Run: `atlas dbusers list --output json`
Present username, roles, auth type.

### `dbuser-create`
Parse args for username and password. Run:
`atlas dbusers create atlasAdmin --username <user> --password <pass> --output json`

### `projects`
Run: `atlas projects list --output json`

### `metrics`
Run: `atlas metrics processes list --output json` to see available processes.
Then `atlas metrics processes <processId> --granularity PT1H --period P1D` for recent metrics.

### `logs`
Run: `atlas logs download <hostname> mongodb.gz --output .` to get recent logs.

### `ping` or `test`
Test full connectivity:
1. `atlas auth whoami` — check auth
2. `atlas accessLists list --output json` — check IP whitelist
3. `node /home/zero20nez/Desktop/claude-cage/mongodb/store.js ping` — test MongoDB driver connection
Present a clear pass/fail status for each.

### `setup`
Run the full setup wizard:
1. `atlas auth login`
2. `atlas config set project_id <id>` (after listing projects)
3. Get and whitelist current IP
4. Test connectivity with `node mongodb/store.js ping`
5. Log success to MongoDB

### Any other args
Pass through directly to atlas: `atlas $ARGUMENTS`
Show the output and explain what happened.

## After Every Operation

After every successful atlas command:
1. Show the result clearly (tables for lists, JSON for details)
2. Log the operation to MongoDB if the store is available:
   ```bash
   node /home/zero20nez/Desktop/claude-cage/mongodb/store.js log "atlas" "<subcommand>" '{"args":"$ARGUMENTS","status":"ok"}'
   ```
3. If the operation was `whitelist-add`, remind the user to test with `make mongo-ping`

## Error Handling

- If `atlas` returns "not logged in": Guide user to run `/atlas login`
- If "project not set": Run `atlas projects list` and help select one
- If network errors: Check if they need to whitelist their IP first
- Always show the raw error AND a suggested fix
