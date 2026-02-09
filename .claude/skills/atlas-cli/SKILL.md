---
name: atlas-cli
description: >
  This skill should be used when the user asks about "MongoDB Atlas", "atlas CLI",
  "whitelist IP", "cluster management", "database users", "access list",
  "mongo connection", "MongoDB connectivity", "atlas setup", "atlas login",
  "mongo not connecting", "connection timeout", "IP whitelist", "atlas accessLists",
  or discusses MongoDB Atlas infrastructure, cluster operations, network access,
  or database administration. Also activates when troubleshooting MongoDB connection
  issues in the claude-cage project.
version: 1.0.0
---

# Atlas CLI Skill — MongoDB Atlas Management

## Overview

This skill provides knowledge for managing MongoDB Atlas infrastructure via the
`atlas` CLI tool, integrated with the claude-cage project's MongoDB fire-and-forget
store layer.

## Tool Location

- **Binary:** `~/bin/atlas` (v1.35.0)
- **Always export:** `export PATH="$HOME/bin:$PATH"` before running atlas commands
- **MongoDB store:** `/home/zero20nez/Desktop/claude-cage/mongodb/store.js`
- **Store config:** `/home/zero20nez/Desktop/claude-cage/mongodb/.env`

## Authentication

Atlas CLI uses OAuth browser-based login or API keys.

```bash
# Login (opens browser)
atlas auth login

# Check current auth
atlas auth whoami

# Logout
atlas auth logout

# Use API key instead (non-interactive)
atlas config set public_api_key <key>
atlas config set private_api_key <key>
atlas config set org_id <org>
atlas config set project_id <project>
```

## Most Common Operations

### IP Whitelist (Network Access)

This is the #1 issue when MongoDB connections fail. Atlas blocks all IPs by default.

```bash
# Get current public IP
curl -s https://api.ipify.org

# Add current IP to whitelist
atlas accessLists create $(curl -s https://api.ipify.org)/32 \
  --comment "claude-cage $(date +%Y-%m-%d)" --output json

# List all whitelisted IPs
atlas accessLists list --output json

# Delete an entry
atlas accessLists delete <cidr-block> --force
```

**Troubleshooting connection timeouts:**
1. First check: Is the IP whitelisted? `atlas accessLists list`
2. Get current IP: `curl -s https://api.ipify.org`
3. If not listed: `atlas accessLists create <ip>/32`
4. Allow 30-60 seconds for propagation
5. Test: `node mongodb/store.js ping`

### Cluster Management

```bash
# List clusters
atlas clusters list --output json

# Describe cluster
atlas clusters describe <name> --output json

# Pause cluster (saves cost on free/shared tier)
atlas clusters pause <name>

# Resume cluster
atlas clusters start <name>

# Connection string
atlas clusters connectionStrings describe <name>
```

### Database Users

```bash
# List users
atlas dbusers list --output json

# Create user with atlasAdmin role
atlas dbusers create atlasAdmin \
  --username <user> --password <pass> --output json

# Create read-only user
atlas dbusers create readAnyDatabase \
  --username <user> --password <pass> --output json

# Delete user
atlas dbusers delete <username> --force
```

### Projects

```bash
# List projects
atlas projects list --output json

# Create project
atlas projects create <name> --output json

# Set default project
atlas config set project_id <id>
```

## Integration with claude-cage MongoDB Store

The Atlas CLI manages the infrastructure. The MongoDB store (`mongodb/store.js`)
is the application layer that reads/writes documents.

**Connection flow:**
```
atlas CLI → manages Atlas cluster (infra)
    ↓
mongodb/.env → stores connection URI
    ↓
mongodb/store.js → connects via mongodb driver (app)
    ↓
lib/mongodb.sh → bash wrappers (fire-and-forget)
    ↓
lib/cli.sh, session.sh, docker.sh → log events automatically
```

**When connection fails (common patterns):**

| Error | Cause | Fix |
|-------|-------|-----|
| `EBADNAME` | Bad connection string format | Check MONGODB_URI in .env |
| `Server selection timed out` | IP not whitelisted | `/atlas whitelist-add` |
| `Authentication failed` | Wrong user/pass | Check .env credentials, `atlas dbusers list` |
| `Cluster paused` | Free tier auto-paused | `atlas clusters start <name>` |
| `ENOTFOUND` | DNS resolution failed | Check cluster hostname |

## Output Format

Always use `--output json` for programmatic access. For user-facing display,
parse the JSON and present as formatted tables.

## Logging Atlas Operations

After every atlas operation, log to MongoDB (if store is available):
```bash
node mongodb/store.js log "atlas" "<operation>" '{"args":"...","result":"ok"}'
```

This creates an audit trail of all infrastructure changes.

## Slash Command

Users can invoke `/atlas <subcommand>` for quick operations:
- `/atlas login` — authenticate
- `/atlas whitelist-add` — add current IP
- `/atlas clusters` — list clusters
- `/atlas ping` — full connectivity test
- `/atlas setup` — guided first-time setup

## Quick Reference

```
atlas auth login              # Authenticate
atlas auth whoami             # Check auth
atlas accessLists create      # Whitelist IP
atlas accessLists list        # List IPs
atlas clusters list           # List clusters
atlas clusters describe <n>   # Cluster details
atlas clusters pause <n>      # Pause cluster
atlas clusters start <n>      # Resume cluster
atlas dbusers list            # List DB users
atlas dbusers create          # Create DB user
atlas projects list           # List projects
atlas config set              # Set config value
atlas metrics processes       # Performance data
atlas logs download           # Get logs
```
