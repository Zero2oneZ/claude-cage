# ─── claude-cage Desktop image ──────────────────────────────────
# Sandboxed environment running Claude Desktop via Xvfb + noVNC.
# Accessible through a browser at http://localhost:6080
# ────────────────────────────────────────────────────────────────

FROM ubuntu:24.04 AS base

LABEL maintainer="claude-cage"
LABEL description="Sandboxed Claude Desktop environment with browser access"
LABEL org.opencontainers.image.source="https://github.com/Zero2oneZ/claude-cage"

ENV DEBIAN_FRONTEND=noninteractive
ENV DISPLAY=:1
ENV VNC_PORT=5900
ENV NOVNC_PORT=6080
ENV VNC_RESOLUTION=1920x1080
ENV VNC_COL_DEPTH=24

# ── System dependencies ─────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        wget \
        git \
        jq \
        ripgrep \
        bash \
        sudo \
        tini \
        # X11 & VNC
        xvfb \
        x11vnc \
        # noVNC for browser access
        novnc \
        websockify \
        # Lightweight window manager
        openbox \
        # Terminal emulator
        xterm \
        # Fonts
        fonts-dejavu-core \
        fonts-liberation \
        # Misc X utilities
        xdg-utils \
        dbus-x11 \
        # Node.js for Claude Code
        nodejs \
        npm \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# ── Install Claude Code CLI ─────────────────────────────────────
RUN npm install -g @anthropic-ai/claude-code \
    && npm cache clean --force

# ── Create non-root user ────────────────────────────────────────
RUN groupadd -r cageuser && useradd -r -g cageuser -m -s /bin/bash cageuser \
    && mkdir -p /workspace /home/cageuser/.claude /home/cageuser/.vnc \
    && chown -R cageuser:cageuser /workspace /home/cageuser

# ── VNC / noVNC configuration ───────────────────────────────────
COPY entrypoint-desktop.sh /usr/local/bin/entrypoint-desktop.sh
RUN chmod +x /usr/local/bin/entrypoint-desktop.sh

# ── Openbox config ──────────────────────────────────────────────
RUN mkdir -p /home/cageuser/.config/openbox
COPY openbox-rc.xml /home/cageuser/.config/openbox/rc.xml
RUN chown -R cageuser:cageuser /home/cageuser/.config

# ── Workspace ────────────────────────────────────────────────────
VOLUME ["/workspace", "/home/cageuser/.claude"]
WORKDIR /workspace

# ── Expose ports ─────────────────────────────────────────────────
EXPOSE 5900 6080

# ── Drop to non-root ────────────────────────────────────────────
USER cageuser

# ── Health check ─────────────────────────────────────────────────
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -sf http://localhost:6080/ || exit 1

# ── Entrypoint ──────────────────────────────────────────────────
ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/entrypoint-desktop.sh"]
