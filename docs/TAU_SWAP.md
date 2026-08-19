# Tau Swap — cage×tau hybrid (2026-08-19)

Replaces the Claude Code CLI inside the sandbox with the **tau** terminal
(provider-neutral, append-only JSONL sessions). The sandbox, supervision, and
PTC orchestration are unchanged — they were never agent-specific.

## What changed

| file | change |
|---|---|
| `ptc/executor.py` | agent adapter: `CAGE_AGENT_BIN` picks tau/claude; `_argv_tau` maps `--print` → `tau --print … --output-format text`; `agent_env()` sets `TAU_ENTRYPOINT` |
| `docker/cli/Dockerfile.tau` | python-slim + `pip install tau-ai`, state volume at `~/.tau` |
| `docker-compose.yml` | new `cli-tau` service + `cli-tau-data` volume |

## The one policy decision (Tom)

The egress allowlist is Anthropic-only (`api.anthropic.com`). tau is
provider-neutral — ollama (local), kimi, HF. Opening the filter to those hosts
changes the sandbox's threat posture. Edit `config/default.yaml allowed_hosts`
or run the `dev` tier (unfiltered) until decided.

## What survives

8-layer sandbox, PTC engine, approval cascade, supervision, dashboards — all
agent-agnostic. The `.claude/` plugin layer (slash commands/hooks) does not run
under tau; it was already latent (no settings.json ever registered the hooks).
