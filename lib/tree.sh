#!/usr/bin/env bash
# tree.sh — Universal node tree operations
# One pattern. Every scale. Same shape.
# A node has: inputs, outputs, children, parent, rules, escalation.
# That's a crate. A department. A session. A security layer. A lib module.
# This file provides tree operations for ANY project, including claude-cage itself.

TREE_SCHEMA="$CAGE_ROOT/universal-node.schema.json"

# ── tree_load: load a tree.json and return node count ──────────
# Usage: tree_load <tree_json_path>
tree_load() {
    local tree_path="${1:-$CAGE_ROOT/tree.json}"
    if [[ ! -f "$tree_path" ]]; then
        echo "0"
        return 1
    fi
    python3 -c "
import json, sys
tree = json.load(open('$tree_path'))
nodes = tree.get('nodes', [])
print(len(nodes))
" 2>/dev/null || echo "0"
}

# ── tree_show: render tree hierarchy to stdout ─────────────────
# Usage: tree_show [tree_json_path] [root_id]
tree_show() {
    local tree_path="${1:-$CAGE_ROOT/tree.json}"
    local root_id="${2:-}"
    if [[ ! -f "$tree_path" ]]; then
        echo "(no tree found at $tree_path)"
        return 1
    fi
    python3 -c "
import json
tree = json.load(open('$tree_path'))
nodes = {n['id']: n for n in tree.get('nodes', [])}
root = '${root_id}' or next((nid for nid, n in nodes.items() if n.get('parent') is None), None)
def show(nid, d=0):
    n = nodes.get(nid)
    if not n: return
    s = n.get('metadata',{}).get('sephira_mapping','')
    extra = f' ({s})' if s else ''
    rules = len(n.get('rules', []))
    esc = n.get('escalation',{}).get('threshold','?')
    print('  ' * d + '├── ' + n['name'] + ' [' + n['scale'] + ']' + extra + f'  rules={rules} esc@{esc}')
    for c in n.get('children', []):
        show(c, d + 1)
if root:
    show(root)
else:
    print('(no root node found)')
" 2>/dev/null
}

# ── tree_node: get a single node as JSON ───────────────────────
# Usage: tree_node <tree_json_path> <node_id>
tree_node() {
    local tree_path="$1"
    local node_id="$2"
    python3 -c "
import json
tree = json.load(open('$tree_path'))
for n in tree.get('nodes', []):
    if n['id'] == '$node_id':
        print(json.dumps(n, indent=2))
        break
else:
    print('{\"error\": \"node not found\"}')
" 2>/dev/null
}

# ── tree_blast_radius: find affected nodes for a set of targets ─
# Usage: tree_blast_radius <tree_json_path> <target1,target2,...>
tree_blast_radius() {
    local tree_path="$1"
    local targets="$2"
    python3 -c "
import json
tree = json.load(open('$tree_path'))
nodes = {n['id']: n for n in tree.get('nodes', [])}
targets = '${targets}'.split(',')

affected = set()
for nid, node in nodes.items():
    owned = node.get('metadata', {}).get('crates_owned', [])
    # Also match by name or id containing the target
    match = any(t in owned for t in targets) or 'ALL' in owned
    if not match:
        match = any(t in nid or t in node.get('name','').lower() for t in targets)
    if match:
        current = nid
        while current:
            affected.add(current)
            current = nodes.get(current, {}).get('parent')

parents = set()
for a in sorted(affected):
    n = nodes.get(a)
    if n:
        scale = n['scale']
        name = n['name']
        print(f'  {scale:12} {a:28} {name}')
        if scale == 'department':
            parents.add(a)

print(f'')
print(f'Departments affected: {len(parents)}')
risk = min(10, len(parents) * 2 + 1)
print(f'Risk level: {risk}/10')
cascade = 'Captain' if risk <= 3 else 'Director' if risk <= 6 else 'CTO' if risk <= 8 else 'Human Architect'
print(f'Approval: {cascade}')
" 2>/dev/null
}

# ── tree_route: route an intent through the tree ───────────────
# Usage: tree_route <tree_json_path> <intent_keywords>
tree_route() {
    local tree_path="$1"
    local intent="$2"
    python3 -c "
import json
tree = json.load(open('$tree_path'))
nodes = {n['id']: n for n in tree.get('nodes', [])}
intent = '${intent}'.lower().split()

# Find nodes whose names/crates/ids match intent keywords
matches = []
for nid, node in nodes.items():
    score = 0
    owned = node.get('metadata', {}).get('crates_owned', [])
    text = (node['name'] + ' ' + nid + ' ' + ' '.join(owned)).lower()
    for word in intent:
        if word in text:
            score += 1
    if score > 0:
        matches.append((score, nid, node))

matches.sort(key=lambda x: (-x[0], x[1]))
print('ROUTING DECISION')
print('─' * 50)
if matches:
    for score, nid, node in matches[:5]:
        print(f'  {node[\"name\"]:24} [{node[\"scale\"]}] match={score}')
    primary = matches[0]
    node = primary[2]
    esc = node.get('escalation', {})
    print(f'')
    print(f'Primary target: {node[\"name\"]} ({primary[1]})')
    print(f'Escalation path: {esc.get(\"target\", \"none\")} @ risk >= {esc.get(\"threshold\", \"?\")}')
    if esc.get('cascade'):
        print(f'Full cascade: {\" → \".join(esc[\"cascade\"])}')
else:
    print('  No matching nodes found for intent')
" 2>/dev/null
}

# ── tree_init: create a new tree.json for a project ────────────
# Usage: tree_init <project_dir> <project_name>
tree_init() {
    local project_dir="$1"
    local project_name="$2"

    if [[ -f "$project_dir/tree.json" ]]; then
        echo "tree.json already exists in $project_dir"
        return 1
    fi

    # Copy the template
    if [[ -f "$CAGE_ROOT/templates/project/tree.json" ]]; then
        cp "$CAGE_ROOT/templates/project/tree.json" "$project_dir/tree.json"
    else
        # Create minimal tree
        python3 -c "
import json
tree = {
    '_meta': {
        'title': '${project_name} Tree',
        'description': 'One pattern. Every scale. Same shape.',
        'schema': '../universal-node.schema.json',
        'version': '1.0.0'
    },
    'nodes': [
        {
            'id': 'root:architect',
            'name': 'Architect',
            'scale': 'executive',
            'parent': None,
            'children': [],
            'inputs': [{'name': 'intent', 'type': 'task', 'from': None}],
            'outputs': [{'name': 'direction', 'type': 'decision', 'to': None}],
            'rules': [{'name': 'mvp_guard', 'condition': 'scope_creep detected', 'action': 'block'}],
            'escalation': {'target': None, 'threshold': 10, 'cascade': []},
            'metadata': {'project': '${project_name}'}
        }
    ],
    'coordination': {
        'phases': ['INTAKE', 'TRIAGE', 'PLAN', 'REVIEW', 'EXECUTE', 'VERIFY', 'INTEGRATE', 'SHIP'],
        'approval_cascade': {
            'low_1_3': 'captain approves',
            'medium_4_6': 'director approves',
            'high_7_8': 'CTO approves',
            'critical_9_10': 'human final call'
        }
    }
}
print(json.dumps(tree, indent=2))
" > "$project_dir/tree.json"
    fi

    # Copy schema
    cp "$CAGE_ROOT/universal-node.schema.json" "$project_dir/"

    echo "Initialized tree for $project_name in $project_dir"
    echo "  tree.json — your project tree (add nodes as you build)"
    echo "  universal-node.schema.json — the one schema"
}

# ── tree_add_node: add a node to an existing tree ──────────────
# Usage: tree_add_node <tree_json_path> <node_json>
tree_add_node() {
    local tree_path="$1"
    local node_json="$2"
    python3 -c "
import json
tree = json.load(open('$tree_path'))
node = json.loads('$node_json')
# Add to nodes array
tree['nodes'].append(node)
# Add to parent's children
parent_id = node.get('parent')
if parent_id:
    for n in tree['nodes']:
        if n['id'] == parent_id:
            if node['id'] not in n.get('children', []):
                n.setdefault('children', []).append(node['id'])
            break
json.dump(tree, open('$tree_path', 'w'), indent=2)
print(f'Added {node[\"id\"]} ({node[\"name\"]}) to tree')
" 2>/dev/null
}

# ── tree_seed: seed a tree into MongoDB ────────────────────────
# Usage: tree_seed <tree_json_path> [project_name]
tree_seed() {
    local tree_path="$1"
    local project="${2:-unknown}"

    if ! $MONGO_READY; then
        echo "MongoDB not available"
        return 1
    fi

    local count
    count=$(python3 -c "
import json
tree = json.load(open('$tree_path'))
nodes = tree.get('nodes', [])
print(len(nodes))
")

    # Seed the full tree as one artifact
    local tree_content
    tree_content=$(cat "$tree_path")
    mongo_put "artifacts" "{\"name\":\"${project}-tree\",\"type\":\"tree\",\"project\":\"$project\",\"node_count\":$count,\"_ts\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}"

    # Seed individual nodes
    python3 -c "
import json, subprocess, os
tree = json.load(open('$tree_path'))
store = os.path.join('$CAGE_ROOT', 'mongodb', 'store.js')
for node in tree.get('nodes', []):
    node['project'] = '$project'
    node['_ts'] = '$(date -u +%Y-%m-%dT%H:%M:%SZ)'
    cmd = ['node', store, 'put', 'nodes', json.dumps(node)]
    subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
print(f'Seeded {len(tree[\"nodes\"])} nodes for $project')
" 2>/dev/null

    mongo_log "tree:seed" "$project" "{\"nodes\":$count}"
}
