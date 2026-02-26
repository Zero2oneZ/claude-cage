# GentlyOS — Multi-stage build for claude-cage integration
# Build context: claude-cage root (not docker/gentlyos/)
#
# Usage:
#   docker build -t cage-gently:latest -f docker/gentlyos/Dockerfile .

# ── Build stage ──────────────────────────────────────────────────
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpcap-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for layer caching
COPY gentlyos-core/Cargo.toml gentlyos-core/Cargo.lock ./

# Copy crate manifests (creates empty src/lib.rs for each to cache deps)
COPY gentlyos-core/crates ./crates
COPY gentlyos-core/gently-cli ./gently-cli
COPY gentlyos-core/gentlyos-tui ./gentlyos-tui

# Build release binaries: gently-cli and gently-web
RUN cargo build --release -p gently-cli \
    && cargo build --release -p gently-web 2>/dev/null || true

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpcap0.8 \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Create gently user (non-root)
RUN useradd -m -s /bin/bash gently

# Copy binaries from builder
COPY --from=builder /app/target/release/gently /usr/local/bin/gently
# gently-web may not build (optional)
COPY --from=builder /app/target/release/gently-web /usr/local/bin/gently-web 2>/dev/null || true

# Data directories
RUN mkdir -p /data/blobs /data/genesis /data/knowledge /data/ipfs \
    && chown -R gently:gently /data

# Set user
USER gently
WORKDIR /home/gently

# Volumes for persistent data
VOLUME ["/data/blobs", "/data/genesis", "/data/knowledge", "/data/ipfs"]

# Environment
ENV GENTLY_DATA_DIR=/data
ENV GENTLY_BLOB_DIR=/data/blobs
ENV GENTLY_LOG_LEVEL=info
ENV RUST_LOG=info
ENV SHELL=/bin/bash

# Ports:
#   3000 — MCP server
#   8080 — Health/metrics
EXPOSE 3000 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8080/health || gently status || exit 1

ENTRYPOINT ["gently"]
CMD ["--help"]
