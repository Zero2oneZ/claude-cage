#!/usr/bin/env bash
# entrypoint-desktop.sh — Start Xvfb, window manager, VNC, noVNC, and Claude
set -euo pipefail

DISPLAY="${DISPLAY:-:1}"
VNC_PORT="${VNC_PORT:-5900}"
NOVNC_PORT="${NOVNC_PORT:-6080}"
VNC_RESOLUTION="${VNC_RESOLUTION:-1920x1080}"
VNC_COL_DEPTH="${VNC_COL_DEPTH:-24}"

cleanup() {
    echo "[desktop] Shutting down..."
    kill $(jobs -p) 2>/dev/null || true
    wait
}
trap cleanup EXIT

# ── Start Xvfb (virtual framebuffer) ────────────────────────────
echo "[desktop] Starting Xvfb at $DISPLAY (${VNC_RESOLUTION}x${VNC_COL_DEPTH})"
Xvfb "$DISPLAY" -screen 0 "${VNC_RESOLUTION}x${VNC_COL_DEPTH}" -ac +extension GLX +render -noreset &
sleep 1

# ── Start window manager ────────────────────────────────────────
echo "[desktop] Starting openbox window manager"
DISPLAY="$DISPLAY" openbox --config-file /home/cageuser/.config/openbox/rc.xml &
sleep 0.5

# ── Start VNC server ────────────────────────────────────────────
echo "[desktop] Starting x11vnc on port $VNC_PORT"
x11vnc -display "$DISPLAY" \
    -rfbport "$VNC_PORT" \
    -nopw \
    -forever \
    -shared \
    -noxdamage \
    -xkb \
    -ncache 10 \
    -q &
sleep 0.5

# ── Start noVNC (browser-accessible VNC) ────────────────────────
echo "[desktop] Starting noVNC on port $NOVNC_PORT"
websockify --web=/usr/share/novnc/ "$NOVNC_PORT" "localhost:$VNC_PORT" &
sleep 0.5

# ── Launch terminal with Claude ─────────────────────────────────
echo "[desktop] Launching Claude in terminal"
DISPLAY="$DISPLAY" xterm \
    -fa "DejaVu Sans Mono" -fs 12 \
    -bg "#1a1b26" -fg "#c0caf5" \
    -geometry 120x40+50+50 \
    -title "Claude Code" \
    -e "claude" &

echo "[desktop] ════════════════════════════════════════════════"
echo "[desktop] Claude Desktop is ready!"
echo "[desktop]   Browser: http://localhost:${NOVNC_PORT}"
echo "[desktop]   VNC:     vnc://localhost:${VNC_PORT}"
echo "[desktop] ════════════════════════════════════════════════"

# Keep running
wait
