---
description: "Route an intent through the tree — blast radius, risk level, approval cascade"
argument-hint: "<intent>"
allowed-tools:
  - Bash
  - Read
---

# /route — Pre-Work Routing Gate

Route `$ARGUMENTS` through the claude-cage tree before starting work.

## Steps

1. **Run PTC dry-run** to decompose the intent and find affected nodes:

```bash
cd $CLAUDE_PROJECT_DIR && CAGE_ROOT="$CLAUDE_PROJECT_DIR" PYTHONPATH="$CLAUDE_PROJECT_DIR" python3 -m ptc.engine --tree tree.json --intent "$ARGUMENTS"
```

2. **Calculate blast radius** — extract the affected crates from the PTC output and run:

```bash
cd $CLAUDE_PROJECT_DIR && source lib/tree.sh && tree_blast_radius tree.json "$(echo '$ARGUMENTS' | tr ' ' ',')"
```

3. **Log the routing decision** to MongoDB:

```bash
node $CLAUDE_PROJECT_DIR/mongodb/store.js log "coordination:phase" "INTAKE:route" "{\"intent\":\"$ARGUMENTS\"}"
node $CLAUDE_PROJECT_DIR/mongodb/store.js log "coordination:phase" "TRIAGE:routed" "{\"intent\":\"$ARGUMENTS\"}"
```

4. **Output the decision** using the CTO format:

```
ROUTING DECISION
════════════════════════════════════════════════
Intent:       $ARGUMENTS
Affected:     [list affected departments/captains]
Risk Level:   N/10
Approval:     Captain|Director|CTO|Human
Phase:        PLAN → Ready to execute
```

If risk >= 7, warn that this is a high-risk change requiring executive approval.
If risk <= 3, auto-approve and proceed to PLAN phase.
