# Deployment Guide: Tom Collaboration Engine

## What This Is

A Claude Code skill that teaches any Claude instance your thinking patterns,
methodology, best ideas, and collaboration protocols. When loaded, Claude
skips the warmup phase entirely — it knows how you work, where you're at,
and what mode to operate in.

## Installation (Claude Code CLI)

### Option 1: Project-Level Skill (Recommended)

Place in your project's `.claude/skills/` directory:

```bash
mkdir -p /path/to/gentlyos/.claude/skills/tom-collab
cp SKILL.md /path/to/gentlyos/.claude/skills/tom-collab/
cp -r references/ /path/to/gentlyos/.claude/skills/tom-collab/
```

Then reference in your project's `CLAUDE.md`:

```markdown
## Skills

Load `/path/to/.claude/skills/tom-collab/SKILL.md` at session start.
Apply the collaboration engine to all interactions.
```

### Option 2: Global Skill (All Projects)

Place in `~/.claude/skills/`:

```bash
mkdir -p ~/.claude/skills/tom-collab
cp SKILL.md ~/.claude/skills/tom-collab/
cp -r references/ ~/.claude/skills/tom-collab/
```

### Option 3: CLAUDE.md Direct Embed

For simpler setups, paste the core sections directly into your
project's CLAUDE.md file. The most critical sections are:

1. Section 1.3 (Ideation Cascade) — stage detection
2. Section 2 (BBBCP) — problem-solving methodology
3. Section 4 (How We Work) — collaboration modes
4. Section 5 (Staging Protocol) — session initialization

## File Structure

```
tom-collab/
├── SKILL.md                          # Core skill (main file Claude reads)
├── references/
│   └── idea-inventory.md             # Ideas, ratings, transmutation chains
└── DEPLOY.md                         # This file
```

## How It Works in Practice

### Session Start
Claude reads SKILL.md → calibrates to your cognitive patterns → detects
your current energy/mode from first message → responds appropriately.

### During Session
Claude continuously monitors for stage transitions (Spark → Torrent →
Crystal → Architect → Build → Drift) and adjusts response style.

### Key Triggers
- You say "rate this" → Honesty Gate activates (Section 4.1)
- You say "let's Bone Blob this" → BBBCP framework invoked (Section 2)
- You drift to tangent → MVP Guardian flags it (Section 1.3, Stage 6)
- You describe weird spatial insight → Pattern Translation mode (Section 4.1)
- You're coding fast → Build Sprint mode, tight and fast (Section 4.1)

## Updating

This is a LIVING document. As new patterns emerge, new ideas are rated,
or new methodology develops, update:
- SKILL.md Section 3 for new idea ratings
- references/idea-inventory.md for new discoveries
- SKILL.md Section 1.3 if new cascade stages emerge
- SKILL.md Section 2 if BBBCP methodology evolves

## Integration with Other Skills

This skill is designed to work alongside:
- `gui-planner` — for GentlyOS interface work
- Any future GentlyOS-specific skills
- The Prometheus Protocol (already in memory edits)
