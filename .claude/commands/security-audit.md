---
description: Run a security audit on a cage container — verify all 8 defense-in-depth layers
argument-hint: [session-name] (defaults to most recent running session)
allowed-tools: [Bash, Read, Grep, Task]
---

# /security-audit — Container Security Verification

The user invoked `/security-audit` with: $ARGUMENTS

Delegate this to the **security-auditor** subagent for a thorough 8-layer security audit.

## Steps

1. If `$ARGUMENTS` is empty, find the most recent running cage container:
   ```bash
   docker ps --filter "label=managed-by=claude-cage" --format "{{.Names}}" | head -1 | sed 's/^cage-//'
   ```

2. Use the Task tool to delegate to the `security-auditor` subagent:
   ```
   Run a complete 8-layer security audit on cage session: <name>
   Check: read-only rootfs, capabilities, seccomp, AppArmor, resource limits, network filtering, no-new-privileges, ICC disabled.
   Present results as a formatted audit report with PASS/FAIL for each layer.
   ```

3. After the audit completes, present the results and log to MongoDB:
   ```bash
   node /home/zero20nez/Desktop/claude-cage/mongodb/store.js log "security" "audit:<name>" '{"triggered_by":"slash_command"}'
   ```
