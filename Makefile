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

web: ## Launch web dashboard (http://localhost:5000)
	@chmod +x bin/claude-cage
	@bin/claude-cage web

# ── MongoDB ─────────────────────────────────────────────────────

.PHONY: mongo-install mongo-ping mongo-status
mongo-install: ## Install MongoDB store dependencies
	cd mongodb && npm install

mongo-ping: ## Test MongoDB Atlas connectivity
	node mongodb/store.js ping

mongo-status: ## Show event counts in MongoDB
	@node mongodb/store.js count events 2>/dev/null || echo "MongoDB not reachable"
	@node mongodb/store.js count artifacts 2>/dev/null || echo ""

mongo-seed: ## Seed all artifacts + project metadata into MongoDB
	node mongodb/seed-artifacts.js

mongo-search: ## Search MongoDB artifacts (usage: make mongo-search Q="query text")
	node mongodb/store.js search artifacts "$(Q)" 10

mongo-stats: ## Show full MongoDB statistics
	@node mongodb/store.js stats 2>/dev/null || echo "MongoDB not reachable"

mongo-events: ## Show recent events (usage: make mongo-events N=20)
	@node mongodb/store.js get events '{}' $(or $(N),10)

# ── Observability ────────────────────────────────────────────

.PHONY: observe health metrics
observe: ## Show observability dashboard for all running sessions
	@chmod +x bin/claude-cage
	@CAGE_ROOT="$(CAGE_ROOT)" bin/claude-cage observe

health: ## Check health of running sessions
	@docker ps --filter "label=managed-by=claude-cage" --format "{{.Names}}" | while read c; do \
		name=$${c#cage-}; \
		status=$$(docker inspect -f '{{.State.Status}}' "$$c" 2>/dev/null); \
		mem=$$(docker stats --no-stream --format "{{.MemPerc}}" "$$c" 2>/dev/null); \
		echo "  $$name: status=$$status mem=$$mem"; \
	done || echo "No running sessions"

# ── Memory ───────────────────────────────────────────────────

.PHONY: memory memory-list memory-clean
memory-list: ## List saved session memories
	@ls -la $(HOME)/.local/share/claude-cage/memory/*.json 2>/dev/null || echo "No saved memories"

memory-clean: ## Clean old session memories (30+ days)
	@find $(HOME)/.local/share/claude-cage/memory -name "*.json" -mtime +30 -delete 2>/dev/null; echo "Cleaned"

# ── Tree ──────────────────────────────────────────────────────

.PHONY: tree cage-tree init
tree: ## Show claude-cage's own architecture tree
	@CAGE_ROOT="$(CAGE_ROOT)" bin/claude-cage tree show tree.json

cage-tree: tree ## Alias for 'make tree'

ptc: ## Run PTC dry-run (usage: make ptc INTENT="add GPU monitoring")
	@chmod +x bin/claude-cage
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage ptc run "$(INTENT)" --verbose

ptc-live: ## Run PTC live (usage: make ptc-live INTENT="verify sandbox")
	@chmod +x bin/claude-cage
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage ptc run "$(INTENT)" --live --verbose

ptc-leaves: ## Show all PTC leaf workers
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage ptc leaves

# ── Training ─────────────────────────────────────────────────

.PHONY: train-extract train-pipeline train-stack train-preview
train-extract: ## Extract training data from PTC traces
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage train extract

train-pipeline: ## Generate full LoRA pipeline from tree
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage train pipeline

train-stack: ## Show LoRA stacking order
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage train stack

train-preview: ## Preview training data format from latest trace
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage train preview

init: ## Initialize new project with tree (usage: make init DIR=./myproject)
	@chmod +x bin/claude-cage
	@CAGE_ROOT="$(CAGE_ROOT)" bin/claude-cage init "$(DIR)" $(if $(NAME),--name $(NAME))

# ── Architect Mode ────────────────────────────────────────────

.PHONY: design design-list design-build design-verify
design: ## Create a design blueprint (usage: make design INTENT="add feature")
	@chmod +x bin/claude-cage
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage design create "$(INTENT)"

design-list: ## List all blueprints
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage design list

design-build: ## Build from blueprint (usage: make design-build ID=blueprint:xyz)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage design build "$(ID)"

design-verify: ## Verify blueprint implementation (usage: make design-verify ID=blueprint:xyz)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage design verify "$(ID)"

# ── IPFS ─────────────────────────────────────────────────────

.PHONY: ipfs-status ipfs-migrate
ipfs-status: ## Check IPFS daemon connectivity
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage ipfs status

ipfs-migrate: ## Migrate existing artifacts to IPFS
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage ipfs migrate

# ── Vector Search ────────────────────────────────────────────

.PHONY: vsearch embed-all vector-setup
vsearch: ## Semantic search (usage: make vsearch Q="query text")
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" bin/claude-cage vsearch "$(Q)"

embed-all: ## Generate embeddings for all artifacts
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.embeddings embed-all

vector-setup: ## Create MongoDB Atlas vector search indexes
	node mongodb/vector-setup.js

# ── Documentation Circle ─────────────────────────────────────

.PHONY: docs docs-generate docs-status docs-check docs-interconnect docs-search docs-graph docs-refresh
docs: docs-status ## Show documentation coverage and staleness

docs-generate: ## Generate docs for all tree nodes
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs generate-all

docs-status: ## Show documentation coverage stats
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs status

docs-check: ## Check all docs for staleness
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs check-stale

docs-interconnect: ## Build the full bidirectional graph (the circle)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs interconnect

docs-search: ## Semantic search docs (usage: make docs-search Q="query")
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs search "$(Q)"

docs-graph: ## Output interconnection graph as JSON
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs graph

docs-refresh: ## Refresh all stale docs
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.docs refresh

# ── Porkbun (Domains) ────────────────────────────────────────

.PHONY: porkbun-ping porkbun-domains porkbun-check porkbun-dns porkbun-pricing
porkbun-ping: ## Test Porkbun API connectivity
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.porkbun ping

porkbun-domains: ## List account domains
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.porkbun domains

porkbun-check: ## Check domain availability (usage: make porkbun-check DOMAIN=example.com)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.porkbun check "$(DOMAIN)"

porkbun-dns: ## Show DNS records (usage: make porkbun-dns DOMAIN=example.com)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.porkbun dns "$(DOMAIN)"

porkbun-pricing: ## Show TLD pricing (usage: make porkbun-pricing TLD=com)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.porkbun pricing $(TLD)

# ── Noun Project (Icons) ────────────────────────────────────

.PHONY: icons-search icons-download icons-batch
icons-search: ## Search icons (usage: make icons-search Q="rocket")
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.nounproject search "$(Q)"

icons-download: ## Download icon (usage: make icons-download ID=12345 PATH=./icon.svg)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.nounproject download "$(ID)" "$(PATH)"

icons-batch: ## Batch download icons (usage: make icons-batch Q="web" DIR=./icons)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.nounproject batch "$(Q)" "$(DIR)"

# ── Federation (Git Sovereignty) ─────────────────────────────

.PHONY: fork-init fork-status fork-pull fork-push fork-verify
fork-init: ## Initialize fork (usage: make fork-init URL=git@... DIR=./project)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.federation fork "$(URL)" "$(DIR)"

fork-status: ## Show federation sync status (usage: make fork-status DIR=.)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.federation status "$(or $(DIR),.)"

fork-pull: ## Sync from upstream (usage: make fork-pull DIR=.)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.federation pull "$(or $(DIR),.)"

fork-push: ## Push to upstream as PR (usage: make fork-push DIR=.)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.federation push "$(or $(DIR),.)"

fork-verify: ## Verify tree trust hashes (usage: make fork-verify DIR=.)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.federation verify "$(or $(DIR),.)"

# ── Hugging Face ─────────────────────────────────────────────

.PHONY: hf-status hf-download hf-search hf-cache
hf-status: ## HF Hub token + cache status
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.huggingface status

hf-download: ## Download model (usage: make hf-download REPO=meta-llama/Llama-2-7b)
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.huggingface download "$(REPO)"

hf-search: ## Search HF models (usage: make hf-search Q="text generation")
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.huggingface search "$(Q)"

hf-cache: ## Show HF cache status
	@CAGE_ROOT="$(CAGE_ROOT)" PYTHONPATH="$(CAGE_ROOT)" python3 -m ptc.huggingface cache

# ── Lifecycle ────────────────────────────────────────────────

.PHONY: seed-all projects gc reap
seed-all: ## Discover and seed all projects into MongoDB
	node mongodb/seed-all.js

projects: ## List all discovered projects with type and path
	@chmod +x bin/claude-cage
	@bin/claude-cage projects

gc: ## Garbage collect dead containers and orphan volumes
	@chmod +x bin/claude-cage
	@bin/claude-cage gc

reap: ## Stop idle and memory-heavy containers
	@chmod +x bin/claude-cage
	@bin/claude-cage reap

# ── GentlyOS ─────────────────────────────────────────────────
.PHONY: gentlyos-seed gentlyos-tree
gentlyos-seed: ## Seed GentlyOS docs, tree, and nodes into MongoDB
	node gentlyos/seed.js

gentlyos-tree: ## Show the GentlyOS recursive tree hierarchy
	@CAGE_ROOT="$(CAGE_ROOT)" bin/claude-cage tree show gentlyos/tree.json

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
	@echo "  ANTHROPIC_API_KEY    Anthropic API key (optional for Max users)"
	@echo "  CAGE_WORKSPACE       Host directory to mount (default: .)"
	@echo "  CAGE_NOVNC_PORT      noVNC port for desktop mode (default: 6080)"
	@echo ""
	@echo "Slash commands (in Claude Code):"
	@echo "  /atlas <cmd>         MongoDB Atlas management"
	@echo "  /session <cmd>       Session lifecycle management"
	@echo "  /mongo <cmd>         Query MongoDB store"
	@echo "  /build [target]      Build container images"
	@echo "  /status              System status overview"
	@echo "  /security-audit      Run security audit"
	@echo "  /gentlyos <cmd>      Tree orchestration (route, node, blast-radius, tree, seed)"
