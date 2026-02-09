# ===========================================================================
# Makefile — Master build system for headless-ubuntu-auto
#
# Orchestrates the full workflow: generate SSH keys, build the autoinstall ISO,
# flash to USB, discover the 3090 on the network, provision the ML stack,
# configure LUKS remote unlock, and verify the installation.
#
# Usage:
#   make help        Show all available targets
#   make keys        Generate SSH keypair for the 3090
#   make iso         Build custom Ubuntu autoinstall ISO
#   make flash       Write ISO to USB drive (requires sudo)
#   make find        Discover the 3090 on the local network
#   make provision   SSH into 3090 and install full ML stack
#   make dropbear    Configure LUKS SSH remote unlock
#   make verify      Run system health check on the 3090
#   make all         Full build: keys + ISO + flash
#   make clean       Remove build artifacts
# ===========================================================================

SHELL := /bin/bash
.DEFAULT_GOAL := help

# ---------------------------------------------------------------------------
# Paths and variables
# ---------------------------------------------------------------------------
PROJECT_DIR  := $(shell pwd)
SCRIPTS_DIR  := $(PROJECT_DIR)/scripts
KEYS_DIR     := $(PROJECT_DIR)/keys
CONFIG_DIR   := $(PROJECT_DIR)/config
BUILD_DIR    := $(PROJECT_DIR)/build
ISO_NAME     := ubuntu-24.04-gpu3090-autoinstall.iso
TARGET_IP_FILE := $(PROJECT_DIR)/.target-ip
TARGET_USER  := zero20nez
SSH_KEY      := $(KEYS_DIR)/3090-headless

# SSH options for connecting to the 3090
# -o StrictHostKeyChecking=no   — accept new host keys (first connection)
# -o UserKnownHostsFile=/dev/null — don't pollute known_hosts
# -o LogLevel=ERROR             — suppress warnings for cleaner output
SSH_OPTS     := -o StrictHostKeyChecking=no \
                -o UserKnownHostsFile=/dev/null \
                -o LogLevel=ERROR

# Constructed SSH and SCP commands
SSH_CMD      = ssh $(SSH_OPTS) -i $(SSH_KEY) $(TARGET_USER)@$(TARGET_IP)
SCP_CMD      = scp $(SSH_OPTS) -i $(SSH_KEY)

# ---------------------------------------------------------------------------
# Phony targets (these don't correspond to files)
# ---------------------------------------------------------------------------
.PHONY: keys iso flash network pxe pxe-stop find provision dropbear verify all clean help

# ---------------------------------------------------------------------------
# Helper: read the target IP from .target-ip, with a helpful error message
# ---------------------------------------------------------------------------
define read_target_ip
	$(eval TARGET_IP := $(shell cat $(TARGET_IP_FILE) 2>/dev/null))
	@if [ -z "$(TARGET_IP)" ]; then \
		echo ""; \
		echo "ERROR: Target IP not found."; \
		echo ""; \
		echo "  The file .target-ip does not exist or is empty."; \
		echo "  Run 'make find' first to discover the 3090 on the network."; \
		echo ""; \
		exit 1; \
	fi
	@echo "Target: $(TARGET_USER)@$(TARGET_IP)"
endef

# ===========================================================================
# Target: keys
# Generate an ed25519 SSH keypair for authenticating to the 3090.
# The keys are stored in keys/3090-headless and keys/3090-headless.pub.
# ===========================================================================
keys: ## Generate SSH keypair for the 3090
	@echo ""
	@echo "=== Generating SSH Keys ==="
	@bash $(SCRIPTS_DIR)/generate-keys.sh

# ===========================================================================
# Target: iso
# Build the custom Ubuntu autoinstall ISO using the autoinstall config
# and the generated SSH public key.
# ===========================================================================
iso: ## Build custom Ubuntu autoinstall ISO
	@echo ""
	@echo "=== Building Autoinstall ISO ==="
	@if [ ! -f "$(SSH_KEY).pub" ]; then \
		echo "WARNING: SSH public key not found at $(SSH_KEY).pub"; \
		echo "Run 'make keys' first to generate the keypair."; \
		echo ""; \
	fi
	@bash $(SCRIPTS_DIR)/build-iso.sh

# ===========================================================================
# Target: flash
# Write the autoinstall ISO to a USB drive.
# Requires sudo for direct disk access.
# ===========================================================================
flash: ## Write ISO to USB drive (requires sudo)
	@echo ""
	@echo "=== Flashing ISO to USB ==="
	@if [ ! -f "$(BUILD_DIR)/$(ISO_NAME)" ] && [ ! -f "$(ISO_NAME)" ]; then \
		echo "ERROR: ISO not found."; \
		echo "  Looked for: $(BUILD_DIR)/$(ISO_NAME)"; \
		echo "  Also tried: $(ISO_NAME)"; \
		echo "  Run 'make iso' first to build the ISO."; \
		exit 1; \
	fi
	@ISO_PATH="$$([ -f "$(BUILD_DIR)/$(ISO_NAME)" ] && echo "$(BUILD_DIR)/$(ISO_NAME)" || echo "$(ISO_NAME)")"; \
	sudo bash $(SCRIPTS_DIR)/flash-usb.sh "$${ISO_PATH}"

# ===========================================================================
# Target: network
# Configure your local ethernet interface for direct connection to the 3090.
# Sets up 10.0.0.1/30 on enp2s0, enables NAT via WiFi.
# ===========================================================================
network: ## Configure local ethernet for direct 3090 connection
	@echo ""
	@echo "=== Setting up Direct Ethernet Connection ==="
	@sudo bash $(SCRIPTS_DIR)/setup-local-network.sh

# ===========================================================================
# Target: pxe
# Start PXE boot server — 3090 boots Ubuntu over ethernet, no USB needed.
# Requires the 3090 BIOS to have Network/PXE boot enabled.
# ===========================================================================
pxe: ## Start PXE boot server (network install, no USB)
	@echo ""
	@echo "=== Starting PXE Boot Server ==="
	@sudo bash $(SCRIPTS_DIR)/pxe-server.sh start

pxe-stop: ## Stop PXE boot server
	@sudo bash $(SCRIPTS_DIR)/pxe-server.sh stop

# ===========================================================================
# Target: find
# Wait for the 3090 to come online and verify SSH connectivity.
# Direct connection: 10.0.0.2 (static IP from autoinstall)
# ===========================================================================
find: ## Wait for 3090 to boot and verify SSH
	@echo ""
	@echo "=== Finding GPU-3090 ==="
	@bash $(SCRIPTS_DIR)/find-machine.sh

# ===========================================================================
# Target: provision
# Copy the post-install provisioning script to the 3090 and execute it.
# Installs the full ML stack (NVIDIA drivers, CUDA, PyTorch, Ollama, etc).
# ===========================================================================
provision: ## SSH into 3090 and install full ML stack
	@echo ""
	@echo "=== Provisioning ML Stack on 3090 ==="
	$(call read_target_ip)
	@echo "Copying provisioning script to $(TARGET_USER)@$(TARGET_IP)..."
	@$(SCP_CMD) $(SCRIPTS_DIR)/post-install.sh $(TARGET_USER)@$(TARGET_IP):/tmp/post-install.sh
	@echo "Running provisioning script (this will take a while)..."
	@$(SSH_CMD) "chmod +x /tmp/post-install.sh && sudo /tmp/post-install.sh"
	@echo ""
	@echo "=== Provisioning Complete ==="
	@echo "Run 'make verify' to check the installation."

# ===========================================================================
# Target: dropbear
# Configure dropbear-initramfs on the 3090 for remote LUKS unlock.
# Copies the setup script and the generated SSH public key to the 3090.
# ===========================================================================
dropbear: ## Configure LUKS SSH remote unlock
	@echo ""
	@echo "=== Configuring Dropbear for LUKS Remote Unlock ==="
	$(call read_target_ip)
	@echo "Copying dropbear setup script to $(TARGET_USER)@$(TARGET_IP)..."
	@$(SCP_CMD) $(SCRIPTS_DIR)/setup-dropbear.sh $(TARGET_USER)@$(TARGET_IP):/tmp/setup-dropbear.sh
	@if [ -f "$(SSH_KEY).pub" ]; then \
		echo "Copying SSH public key for dropbear authorized_keys..."; \
		$(SCP_CMD) $(SSH_KEY).pub $(TARGET_USER)@$(TARGET_IP):/tmp/3090-headless.pub; \
		echo "Running dropbear setup with generated key..."; \
		$(SSH_CMD) "chmod +x /tmp/setup-dropbear.sh && sudo /tmp/setup-dropbear.sh /tmp/3090-headless.pub"; \
	else \
		echo "WARNING: No generated SSH key found. Using only the default authorized key."; \
		echo "Running dropbear setup..."; \
		$(SSH_CMD) "chmod +x /tmp/setup-dropbear.sh && sudo /tmp/setup-dropbear.sh"; \
	fi
	@echo ""
	@echo "=== Dropbear Configuration Complete ==="

# ===========================================================================
# Target: verify
# Run a comprehensive system health check on the 3090 and display the report.
# ===========================================================================
verify: ## Run system health check on the 3090
	@echo ""
	@echo "=== Running System Health Check on 3090 ==="
	$(call read_target_ip)
	@echo "Copying verification script to $(TARGET_USER)@$(TARGET_IP)..."
	@$(SCP_CMD) $(SCRIPTS_DIR)/verify-install.sh $(TARGET_USER)@$(TARGET_IP):/tmp/verify-install.sh
	@echo "Running health check..."
	@echo ""
	@$(SSH_CMD) "chmod +x /tmp/verify-install.sh && sudo /tmp/verify-install.sh"

# ===========================================================================
# Target: all
# Full build pipeline: generate keys, build the ISO, flash to USB.
# After flashing, you install Ubuntu on the 3090, then use 'make find'
# to discover it on the network.
# ===========================================================================
all: keys iso flash ## Full build: keys + ISO + flash
	@echo ""
	@echo "=== Full Build Complete ==="
	@echo ""
	@echo "Next steps (NO MONITOR NEEDED):"
	@echo ""
	@echo "  1. Run: make network"
	@echo "     (configures your ethernet port for direct connection)"
	@echo ""
	@echo "  2. Connect ethernet cable between this machine and the 3090"
	@echo ""
	@echo "  3. Insert USB into 3090, power it on"
	@echo "     (must boot from USB — may need BIOS change if it boots Windows)"
	@echo ""
	@echo "  4. Run: make find"
	@echo "     (waits for 3090 to finish installing and boot into Ubuntu)"
	@echo "     (3090 will be at 10.0.0.2)"
	@echo ""
	@echo "  5. Run: make provision"
	@echo "     (installs NVIDIA drivers, CUDA, PyTorch, vLLM, Ollama, etc.)"
	@echo ""
	@echo "  6. Run: make dropbear"
	@echo "     (configures LUKS remote unlock via SSH on port 2222)"
	@echo ""
	@echo "  7. Run: make verify"
	@echo "     (shows full system health report)"

# ===========================================================================
# Target: clean
# Remove build artifacts, custom ISOs, and temporary files.
# Does NOT remove keys (those are precious) or .target-ip (convenience).
# ===========================================================================
clean: ## Remove build artifacts
	@echo ""
	@echo "=== Cleaning Build Artifacts ==="
	@rm -rf $(BUILD_DIR)
	@rm -f $(PROJECT_DIR)/$(ISO_NAME)
	@rm -rf $(PROJECT_DIR)/custom-iso
	@echo "Removed: build/, custom-iso/, $(ISO_NAME)"
	@echo ""
	@echo "NOTE: keys/ and .target-ip were preserved."
	@echo "  To remove keys:      rm -rf $(KEYS_DIR)/3090-headless*"
	@echo "  To remove target IP: rm -f $(TARGET_IP_FILE)"

# ===========================================================================
# Target: help
# Self-documenting help — extracts ## comments from target definitions.
# ===========================================================================
help: ## Show this help
	@echo ""
	@echo "headless-ubuntu-auto — Automated Ubuntu + ML stack for GPU-3090"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Option A: PXE Boot (no USB needed, fully remote):"
	@echo "  make keys       # Generate SSH keys"
	@echo "  make pxe        # Start PXE server (serves Ubuntu over network)"
	@echo "  (connect ethernet, power on 3090 — boots from network)"
	@echo "  make find       # Wait for 3090 to boot, verify SSH"
	@echo "  make pxe-stop   # Stop PXE server"
	@echo "  make provision  # Install ML stack"
	@echo ""
	@echo "Option B: USB Boot (if PXE not available in BIOS):"
	@echo "  make keys       # Generate SSH keys"
	@echo "  make iso        # Build autoinstall ISO"
	@echo "  make flash      # Flash to USB"
	@echo "  make network    # Configure ethernet (10.0.0.1)"
	@echo "  (plug USB + ethernet, power on 3090)"
	@echo "  make find       # Wait for boot, verify SSH"
	@echo "  make provision  # Install ML stack"
	@echo ""
	@echo "After either option:"
	@echo "  make dropbear   # Configure remote LUKS unlock"
	@echo "  make verify     # Run health check"
	@echo ""
