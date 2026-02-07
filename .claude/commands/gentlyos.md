---
description: "GentlyOS tree orchestration — route tasks, query nodes, calculate blast radius"
argument-hint: "<subcommand> [args]"
allowed-tools:
  - Bash
  - Read
  - Grep
  - Glob
  - Task
---

# /gentlyos — Tree Orchestration

One pattern. Every scale. Same shape.

## Subcommands

Route based on `$ARGUMENTS`:

### `route <intent>`
Parse the intent, find affected nodes in `gentlyos/tree.json`, calculate blast radius,
apply node rules, and produce a routing decision.

```bash
# Read the tree and find affected nodes
cat gentlyos/tree.json | python3 -c "
import json, sys
tree = json.load(sys.stdin)
nodes = {n['id']: n for n in tree['nodes']}
# Analyze intent and find affected departments/captains
for nid, node in nodes.items():
    print(f'{nid}: {node[\"name\"]} (scale={node[\"scale\"]}, parent={node[\"parent\"]})')
"
```

Then produce a decision using the CTO output format:
```
DECISION: [APPROVE|REJECT|ESCALATE|DEFER]
BLAST_RADIUS: [affected nodes]
RISK_LEVEL: [1-10]
RATIONALE: [why]
PHASE: [which coordination phase to start at]
```

### `node <id>`
Show full details for a specific node in the tree.

```bash
cat gentlyos/tree.json | python3 -c "
import json, sys
tree = json.load(sys.stdin)
target = '$NODE_ID'
for node in tree['nodes']:
    if node['id'] == target:
        print(json.dumps(node, indent=2))
        break
"
```

### `blast-radius <crate>`
Calculate which departments and captains are affected by changes to a crate.

```bash
cat gentlyos/tree.json | python3 -c "
import json, sys
tree = json.load(sys.stdin)
nodes = {n['id']: n for n in tree['nodes']}
target = sys.argv[1] if len(sys.argv) > 1 else ''
affected = []
for nid, node in nodes.items():
    owned = node.get('metadata', {}).get('crates_owned', [])
    if target in owned or 'ALL' in owned:
        affected.append({'id': nid, 'name': node['name'], 'scale': node['scale']})
        # Walk up
        p = node.get('parent')
        while p:
            pn = nodes.get(p)
            if pn:
                affected.append({'id': p, 'name': pn['name'], 'scale': pn['scale']})
                p = pn.get('parent')
            else:
                break
seen = set()
for a in affected:
    if a['id'] not in seen:
        seen.add(a['id'])
        print(f'  {a[\"scale\"]:12} {a[\"id\"]:24} {a[\"name\"]}')
" "$CRATE_NAME"
```

### `tree`
Show the full tree hierarchy.

```bash
cat gentlyos/tree.json | python3 -c "
import json, sys
tree = json.load(sys.stdin)
nodes = {n['id']: n for n in tree['nodes']}
def show(nid, depth=0):
    node = nodes.get(nid)
    if not node: return
    indent = '  ' * depth
    rules = len(node.get('rules', []))
    esc = node.get('escalation', {}).get('threshold', '?')
    print(f'{indent}├── {node[\"name\"]} [{node[\"scale\"]}] (rules={rules}, escalate@{esc})')
    for child in node.get('children', []):
        show(child, depth + 1)
show('root:human')
"
```

### `seed`
Seed all GentlyOS documents, tree, and schema into MongoDB.

```bash
node gentlyos/seed.js
```

### `sephirot`
Show the Tree of Life → Department mapping.

```bash
cat gentlyos/tree.json | python3 -c "
import json, sys
tree = json.load(sys.stdin)
mapping = tree.get('sephirot_mapping', {})
for k, v in mapping.items():
    if k.startswith('_'): continue
    print(f'  {k:12} → {v}')
"
```

### `approve <node_id> <risk_level>`
Simulate an approval gate cascade for a given risk level.

Read the node's escalation cascade and determine who approves:
- Risk 1-3: Captain approves
- Risk 4-6: Director approves after Captain
- Risk 7-8: CTO approves after Director + Captain
- Risk 9-10: Human Architect final call

Log the approval decision to MongoDB.

## MongoDB Logging

Every routing decision is logged:
```bash
node mongodb/store.js log "gentlyos:route" "$INTENT" '{"nodes":[],"risk":N,"decision":"..."}'
```
