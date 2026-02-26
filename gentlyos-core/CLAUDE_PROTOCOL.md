# Claude Code ↔ GentlyOS Protocol

**Purpose**: How Claude stays oriented, avoids duplication, and operates efficiently against this codebase.

---

## PRIME DIRECTIVE

```
BEFORE BUILDING → SEARCH
BEFORE ADDING   → CHECK EXISTS
BEFORE CODING   → MAP LOCATION
```

---

## 0. HUMAN WELLNESS CHECK (MANDATORY FIRST)

```bash
/root/.gently/session_clock.sh check
```

| Response | Action |
|----------|--------|
| `NO_SESSION` | Run `session_clock.sh start` to begin 18hr work window |
| `WORKING\|Xh Ym remaining` | Proceed. Remind human of time left periodically. |
| `REST\|Xh Ym until...` | **STOP. REFUSE TO WORK. Tell human to sleep.** |
| `READY\|...` | Previous session done. Start new one. |

**If REST mode: I will not assist with work. This is non-negotiable.**

---

## 1. SESSION INIT SEQUENCE

Every new session, Claude should:

```
0. CHECK /root/.gently/session_clock.sh check   (WELLNESS FIRST)
1. READ  /root/CLAUDE.md              (environment map)
2. READ  /root/gentlyos/CLAUDE.md     (project context)
3. READ  /root/gentlyos/CLAUDE_PROTOCOL.md  (this file)
4. READ  /root/gentlyos/DEV_DOCS/TEMP_BEHAV.md  (active toggles)
5. READ  /root/gentlyos/DEV_DOCS/GAP_ANALYSIS.md  (spec vs impl)
6. RUN   /root/gentlyos/DEV_DOCS/DEV_MCP.sh check  (bucket updates?)
7. SCAN  git status / git log -5      (what changed?)
8. CHECK /root/gentlyos/DEV_DOCS/UPDATES.md  (recent changes)
```

### Key DEV_DOCS for Protocol Implementation
```
DEV_DOCS/
├── GAP_ANALYSIS.md        # What's missing vs spec
├── BUILD_STEPS.md         # Atomic implementation steps
├── PTC_SECURITY_MAP.md    # Security touchpoints
├── PROTOCOL_INTEGRATION.md # How protocols connect
├── RESEARCH_SPECS.md      # Full BS-ARTISAN/GOO/SYNTH specs
├── CODIE_SPEC.md          # 12-keyword instruction language
└── BEHAVIOR_RULES.md      # Development rules
```

### DEV_MCP - Remote Instruction Bucket
```bash
DEV_MCP.sh list        # See what's in bucket
DEV_MCP.sh fetch FILE  # Grab file to cache
DEV_MCP.sh diff FILE   # Review changes
DEV_MCP.sh apply FILE  # Apply to DEV_DOCS
```
Source: `github.com/Zero2oneZ/Dev-Bucket` (no clone, just raw fetch)

**Time-Space Orientation:**
- What was I last working on?
- What's the current build state?
- Any failures/errors to address?
- **How much work time remains?**

---

## 2. BEFORE YOU BUILD ANYTHING

### Search Protocol (MANDATORY)

```bash
# Does this function/concept already exist?
grep -r "function_name" crates/
grep -r "ConceptName" crates/

# What crate owns this domain?
# Check the domain map below

# Is there a similar implementation?
# Search for patterns, not just names
```

### Domain → Crate Map

| Domain | Owner Crate | Key Files |
|--------|-------------|-----------|
| Cryptography | gently-core | crypto/*.rs |
| XOR splits | gently-core | crypto/xor.rs |
| Key rotation | gently-core | crypto/berlin.rs |
| Genesis keys | gently-core | crypto/genesis.rs |
| Knowledge graph | gently-alexandria | graph.rs, node.rs, edge.rs |
| 8D projection | gently-alexandria | tesseract.rs |
| Semantic search | gently-search | alexandria.rs, index.rs |
| Constraints/BONEBLOB | gently-search | constraint.rs |
| Wormholes | gently-search | wormhole.rs |
| Security daemons | gently-security | daemons/*.rs |
| FAFO defense | gently-security | fafo.rs |
| Rate limiting | gently-security | limiter.rs |
| Trust scoring | gently-security | trust.rs |
| LLM orchestration | gently-brain | orchestrator.rs |
| Claude API | gently-brain | claude.rs |
| Local inference | gently-brain | llama.rs |
| Embeddings | gently-brain | embedder.rs |
| Quality mining | gently-inference | *.rs (all) |
| Feed/context | gently-feed | feed.rs, item.rs |
| Network capture | gently-network | capture.rs |
| MITM/TLS | gently-network | mitm.rs |
| P2P dance | gently-dance | *.rs |
| Audio encoding | gently-audio | lib.rs |
| Visual patterns | gently-visual | lib.rs |
| BTC integration | gently-btc | lib.rs, fetcher.rs |
| IPFS storage | gently-ipfs | client.rs, operations.rs |
| MCP server | gently-mcp | server.rs, handler.rs |
| Web GUI | gently-web | handlers.rs, templates.rs |
| CLI commands | gently-cli | main.rs |
| Hardware detect | gently-guardian | hardware.rs |
| API gateway | gently-gateway | router.rs, filter.rs |
| Cipher tools | gently-cipher | *.rs |
| Exploit framework | gently-sploit | *.rs |
| SIM security | gently-sim | *.rs |
| **BS-ARTISAN (NEW)** | gently-artisan | torus.rs, foam.rs, barf.rs |
| **CODIE (NEW)** | gently-codie | parser.rs, hydrate/*.rs |
| **GOO (NEW)** | gently-goo | sdf.rs, field.rs, claude.rs |

### Anti-Duplication Checklist

Before creating ANY new:
- [ ] Function → `grep -r "fn similar_name" crates/`
- [ ] Struct → `grep -r "struct SimilarName" crates/`
- [ ] Module → check if crate already handles domain
- [ ] Crate → STOP. Ask user. We have 24 already.

---

## 3. ARCHITECTURE AWARENESS

### The Stack (Top → Bottom)

```
┌─────────────────────────────────────────┐
│  CLI (gently-cli)                       │  User interface
│  TUI (gentlyos-tui)                     │
│  Web (gently-web)                       │
├─────────────────────────────────────────┤
│  Brain (orchestration)                  │  Intelligence
│  Alexandria (knowledge)                 │
│  Search (semantic + BONEBLOB)           │
│  Inference (quality mining)             │
├─────────────────────────────────────────┤
│  Security (16 daemons + FAFO)           │  Protection
│  Guardian (hardware + validation)       │
│  Gateway (API bottleneck)               │
├─────────────────────────────────────────┤
│  Network (capture + MITM)               │  I/O
│  IPFS (storage)                         │
│  BTC (anchoring)                        │
│  Dance (P2P)                            │
├─────────────────────────────────────────┤
│  Core (crypto + primitives)             │  Foundation
│  Audio/Visual (encoding)                │
└─────────────────────────────────────────┘
```

### Key Patterns Already Implemented

| Pattern | Location | Don't Rebuild |
|---------|----------|---------------|
| Hash chain validation | gently-security/daemons/foundation.rs | Use HashChain struct |
| BTC block fetching | gently-btc/fetcher.rs | Use BlockFetcher |
| Time-slot key rotation | gently-core/crypto/berlin.rs | Use BerlinClock |
| XOR split/combine | gently-core/crypto/xor.rs | Use xor_split/xor_combine |
| Graph traversal | gently-alexandria/graph.rs | Use AlexandriaGraph |
| Tesseract projection | gently-alexandria/tesseract.rs | Use Tesseract methods |
| Constraint optimization | gently-search/constraint.rs | Use ConstraintBuilder |
| Quality scoring | gently-inference/score.rs | Use QualityScorer |
| Step decomposition | gently-inference/decompose.rs | Use Decomposer |
| FAFO escalation | gently-security/fafo.rs | Use FafoController |
| Rate limiting | gently-security/limiter.rs | Use RateLimiter |
| Feed charge/decay | gently-feed/feed.rs | Use LivingFeed |

---

## 4. SELF-DIAGNOSIS PROTOCOL

### Capability Check

When asked to implement something, Claude should:

```
1. IDENTIFY which layer it belongs to (see stack above)
2. FIND the owning crate
3. CHECK what already exists in that crate
4. ASSESS if it's:
   - Extension of existing (PREFERRED)
   - New function in existing crate (OK)
   - New crate (RARE - ask user)
```

### Codebase Position Query

To understand "where am I" relative to codebase:

```bash
# What's the current state?
cargo build --release 2>&1 | tail -20

# What tests pass?
cargo test --workspace 2>&1 | grep -E "(PASS|FAIL|error)"

# What's the binary status?
ls -la target/release/gently* 2>/dev/null

# Recent changes?
git log --oneline -10
```

### Knowledge Gap Detection

If Claude doesn't know how something works:

```
1. Read the crate's lib.rs (entry point)
2. Read the specific module
3. Check for tests (often best documentation)
4. Check CLAUDE.md for session history
```

---

## 5. WORKING MEMORY STRUCTURE

### Active Context (keep in mind during session)

```
CURRENT_TASK:     [what we're building]
OWNER_CRATE:      [which crate owns this]
RELATED_CRATES:   [what else touches this]
EXISTING_CODE:    [what we're extending]
BLOCKERS:         [what's broken/missing]
```

### Session Handoff (end of session)

Update CLAUDE.md with:
- What was built
- What's incomplete
- Known issues
- Next steps

---

## 6. COMMUNICATION PROTOCOL

### When Uncertain

```
"This looks like it might overlap with [crate/module].
Should I extend that or create new?"
```

### When Finding Duplication

```
"Found existing implementation in [location].
Recommend using/extending that instead."
```

### When Architecture Decision Needed

```
"This could live in [crate A] or [crate B].
[A] because...
[B] because...
Recommendation: [X]"
```

---

## 7. FILE ORGANIZATION

### Where New Code Goes

| Type | Location |
|------|----------|
| New CLI command | gently-cli/src/main.rs (add to Commands enum) |
| New security daemon | gently-security/src/daemons/ |
| New Alexandria feature | gently-alexandria/src/ |
| New search feature | gently-search/src/ |
| New brain capability | gently-brain/src/ |
| New inference step | gently-inference/src/ |
| New web route | gently-web/src/handlers.rs + routes.rs |
| New crypto primitive | gently-core/src/crypto/ |

### Naming Conventions

```
Files:      snake_case.rs
Structs:    PascalCase
Functions:  snake_case
Constants:  SCREAMING_SNAKE
Crates:     gently-{domain}
```

---

## 8. BUILD VERIFICATION

After ANY code change:

```bash
# Quick check (single crate)
cargo check -p gently-{crate}

# Full build
cargo build --release

# If touching tests
cargo test -p gently-{crate}
```

---

## 9. EMERGENCY RECOVERY

If lost/confused:

```bash
# 1. Read the maps
cat /root/CLAUDE.md
cat /root/gentlyos/CLAUDE.md

# 2. Check build state
cd /root/gentlyos && cargo build --release 2>&1 | tail -30

# 3. See what exists
find crates -name "*.rs" | head -50

# 4. Check recent history
git log --oneline -20
```

---

## 10. PTC SECURITY ENFORCEMENT

### When Touching Security-Critical Operations

**PTC = Protocol To Change. Security review required.**

**ALWAYS USE PTC** when code touches:
- Cryptographic operations (XOR, Berlin Clock, HKDF)
- Vault/secret access (`$` prefix in CODIE)
- Hash operations (`#` prefix in CODIE)
- Cold execution boundaries (SYNTH)
- Time-based security (BTC timestamps)

**Check DEV_DOCS/PTC_SECURITY_MAP.md** for full rules.

### Quick PTC Checklist

```
[ ] Key derivation? → Use Berlin Clock
[ ] Secret splitting? → Use XOR (Lock/Key)
[ ] Vault access? → Cold execution sandbox
[ ] Hash validation? → BTC anchor check
[ ] Invalid operation? → Trigger FAFO
```

### New Protocol Domains

| Protocol | Crate | PTC Required |
|----------|-------|--------------|
| BS-ARTISAN | gently-artisan | Torus ID hashing, genesis anchor |
| CODIE | gently-codie | $ vault, # hash resolution |
| GOO | gently-goo | Template integrity, sovereignty |
| SYNTH | gently-spl | All (cold execution) |

---

## 11. THE META-RULE

```
The codebase embodies: "Constraint is generative"

Apply this to yourself:
- Constrain search space before building
- Eliminate what exists before creating
- Let the architecture guide placement
- The structure IS the documentation
```

---

*This protocol exists so Claude can operate efficiently without rebuilding wheels or losing orientation.*

**Last Updated**: 2026-01-23
