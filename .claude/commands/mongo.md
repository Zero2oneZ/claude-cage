---
description: Query MongoDB Atlas store — events, artifacts, projects, raw queries
argument-hint: <subcommand> [args...] (e.g., events, artifacts, status, query <collection> <filter>, search <text>)
allowed-tools: [Bash, Read]
---

# /mongo — MongoDB Store Query Interface

The user invoked `/mongo` with: $ARGUMENTS

You are querying the claude-cage MongoDB Atlas store.

## Environment

Store CLI: `node /home/zero20nez/Desktop/claude-cage/mongodb/store.js`

## Subcommand Routing

### `status` or `stats`
Show collection counts and connectivity:
```bash
node mongodb/store.js ping
node mongodb/store.js count events
node mongodb/store.js count artifacts
node mongodb/store.js count projects
```

### `events` [type] [limit]
Query events collection:
```bash
# All recent events
node mongodb/store.js get events '{}' 20
# Filtered by type
node mongodb/store.js get events '{"type":"session"}' 20
node mongodb/store.js get events '{"type":"command"}' 20
node mongodb/store.js get events '{"type":"docker"}' 20
node mongodb/store.js get events '{"type":"security"}' 10
```

### `artifacts` [project] [limit]
Query artifacts:
```bash
node mongodb/store.js get artifacts '{}' 10
node mongodb/store.js get artifacts '{"project":"claude-cage"}' 20
node mongodb/store.js get artifacts '{"type":"skill"}' 10
```

### `projects`
List all projects:
```bash
node mongodb/store.js get projects '{}' 50
```

### `query` or `get` <collection> [filter_json] [limit]
Raw query pass-through:
```bash
node mongodb/store.js get "$COLLECTION" '$FILTER' $LIMIT
```

### `search` <text> [collection] [limit]
Vector search (requires vector index):
```bash
node mongodb/store.js search artifacts "$TEXT" $LIMIT
```

### `aggregate` <collection> <pipeline_json>
Raw aggregation pipeline:
```bash
node mongodb/store.js aggregate "$COLLECTION" '$PIPELINE'
```

### `put` <collection> <json>
Insert a document:
```bash
node mongodb/store.js put "$COLLECTION" '$JSON'
```

### `log` <type> <key> [value]
Log a structured event:
```bash
node mongodb/store.js log "$TYPE" "$KEY" '$VALUE'
```

## Presentation

- Format JSON output as readable tables when possible
- Use `| python3 -m json.tool` or `| jq .` for pretty-printing if available
- Highlight key fields: _ts, type, key, project
- Show document counts alongside results
