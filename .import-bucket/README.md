# .import-bucket/

Staging area for external imports. Drop things here — they won't be committed.

## What goes here

- **Data exports** — Claude conversations, API dumps, database exports
- **3rd party repos** — cloned repos for reference, vendoring, or extraction
- **Downloads** — zip archives, tarballs, binary assets
- **Research material** — papers, docs, datasets being evaluated

## Rules

1. **Transient** — contents are not tracked by git (only this README is)
2. **Project-purpose-centric** — everything here should relate to claude-cage or GentlyOS work
3. **Process, then clean** — extract what you need into the proper project location, then delete the source
4. **No secrets** — same rules as everywhere else: no keys, tokens, or credentials

## Workflow

```
# Drop a file
cp ~/Downloads/some-export.zip .import-bucket/

# Extract and work with it
cd .import-bucket && unzip some-export.zip

# When done, clean up
rm -rf .import-bucket/some-export*
```
