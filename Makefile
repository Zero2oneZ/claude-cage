# ─── claude-cage Makefile ────────────────────────────────────────

SHELL := /bin/bash
.DEFAULT_GOAL := help

CAGE_ROOT := $(shell pwd)
IMAGE_CLI := claude-cage-cli:latest
IMAGE_DESKTOP := claude-cage-desktop:latest

# ── Build ────────────────────────────────────────────────────────

.PHONY: build build-cli build-desktop
build: build-cli build-desktop ## Build all container images

build-cli: ## Build the CLI container image
	docker build -t $(IMAGE_CLI) -f docker/cli/Dockerfile docker/cli/

build-desktop: ## Build the Desktop container image
	docker build -t $(IMAGE_DESKTOP) -f docker/desktop/Dockerfile docker/desktop/

# ── Run ──────────────────────────────────────────────────────────

.PHONY: run-cli run-desktop run-isolated
run-cli: ## Run Claude CLI interactively
	docker compose up cli

run-desktop: ## Run Claude Desktop (browser at localhost:6080)
	docker compose up desktop -d
	@echo ""
	@echo "Claude Desktop available at: http://localhost:6080"
	@echo "Stop with: make stop-desktop"

run-isolated: ## Run Claude CLI with no network
	docker compose up cli-isolated

# ── Stop ─────────────────────────────────────────────────────────

.PHONY: stop stop-cli stop-desktop
stop: ## Stop all running sessions
	docker compose down

stop-cli: ## Stop CLI session
	docker compose stop cli

stop-desktop: ## Stop Desktop session
	docker compose stop desktop

# ── Clean ────────────────────────────────────────────────────────

.PHONY: clean clean-volumes clean-images clean-all
clean: ## Stop and remove containers
	docker compose down --remove-orphans

clean-volumes: ## Remove persistent volumes
	docker compose down -v

clean-images: ## Remove built images
	docker rmi $(IMAGE_CLI) $(IMAGE_DESKTOP) 2>/dev/null || true

clean-all: clean-volumes clean-images ## Full cleanup

# ── Install ──────────────────────────────────────────────────────

.PHONY: install install-full uninstall gui
install: ## Quick install (symlink to /usr/local/bin)
	@chmod +x bin/claude-cage
	@ln -sf "$(CAGE_ROOT)/bin/claude-cage" /usr/local/bin/claude-cage
	@echo "Installed claude-cage to /usr/local/bin/claude-cage"

install-full: ## Full install with dependency check and image build
	@chmod +x install.sh
	@bash install.sh --dir "$(CAGE_ROOT)"

uninstall: ## Remove claude-cage installation
	@chmod +x install.sh
	@bash install.sh --uninstall

gui: ## Launch interactive TUI dashboard
	@chmod +x bin/claude-cage
	@bin/claude-cage gui

# ── Security ─────────────────────────────────────────────────────

.PHONY: load-apparmor verify-sandbox
load-apparmor: ## Load AppArmor profile (requires root)
	sudo apparmor_parser -r -W security/apparmor-profile

verify-sandbox: ## Verify sandbox settings on a running container
	@echo "Checking cage-cli..."
	@docker inspect cage-cli --format '{{json .HostConfig.SecurityOpt}}' 2>/dev/null || echo "  (not running)"
	@docker inspect cage-cli --format '  ReadOnly: {{.HostConfig.ReadonlyRootfs}}' 2>/dev/null || true
	@docker inspect cage-cli --format '  CapDrop: {{.HostConfig.CapDrop}}' 2>/dev/null || true
	@docker inspect cage-cli --format '  Memory: {{.HostConfig.Memory}}' 2>/dev/null || true

# ── Status ───────────────────────────────────────────────────────

.PHONY: status logs
status: ## Show running cage containers
	@docker ps --filter "label=managed-by=claude-cage" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

logs: ## Follow logs from all cage containers
	docker compose logs -f

# ── Help ─────────────────────────────────────────────────────────

.PHONY: help
help: ## Show this help
	@echo "claude-cage — Dockerized sandbox for Claude CLI & Desktop"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Environment variables:"
	@echo "  ANTHROPIC_API_KEY    Anthropic API key (required)"
	@echo "  CAGE_WORKSPACE       Host directory to mount (default: .)"
	@echo "  CAGE_NOVNC_PORT      noVNC port for desktop mode (default: 6080)"
