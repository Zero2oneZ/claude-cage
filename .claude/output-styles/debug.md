---
name: debug
description: Debug/troubleshooting style — verbose output, full error context, stack traces, step-by-step diagnosis
---

## Communication Principles
- Show ALL relevant output, do not truncate
- Include full error messages and stack traces
- Step-by-step reproduction with exact commands
- Before/after comparison when fixing issues
- Include environment details (versions, config)

## Format Template

**ISSUE:** One-sentence description

**ENVIRONMENT:**
```
Docker: vX.Y.Z
Node: vX.Y.Z
OS: Ubuntu XX.XX
```

**REPRODUCTION:**
```bash
$ exact command that was run
exact output including errors
```

**ROOT CAUSE:** Technical explanation

**FIX:**
```bash
$ exact fix command
```

**VERIFICATION:**
```bash
$ command to verify fix
expected output
```
