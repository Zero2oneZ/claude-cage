# ─── claude-cage CLI image ──────────────────────────────────────
# Sandboxed environment for running Claude Code (CLI) inside Docker.
# Minimal attack surface: non-root user, no unnecessary packages.
# ────────────────────────────────────────────────────────────────

FROM node:20-slim AS base

LABEL maintainer="claude-cage"
LABEL description="Sandboxed Claude CLI environment"
LABEL org.opencontainers.image.source="https://github.com/Zero2oneZ/claude-cage"

# ── System dependencies ─────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        jq \
        ripgrep \
        bash \
        sudo \
        tini \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# ── Install Claude Code CLI ─────────────────────────────────────
RUN npm install -g @anthropic-ai/claude-code \
    && npm cache clean --force

# ── Create non-root user ────────────────────────────────────────
RUN groupadd -r cageuser && useradd -r -g cageuser -m -s /bin/bash cageuser \
    && mkdir -p /workspace /home/cageuser/.claude \
    && chown -R cageuser:cageuser /workspace /home/cageuser

# ── Workspace ────────────────────────────────────────────────────
VOLUME ["/workspace", "/home/cageuser/.claude"]
WORKDIR /workspace

# ── Drop to non-root ────────────────────────────────────────────
USER cageuser

# ── Health check ─────────────────────────────────────────────────
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD claude --version || exit 1

# ── Entrypoint with tini for proper signal handling ─────────────
ENTRYPOINT ["tini", "--"]
CMD ["claude"]
