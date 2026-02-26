SUI  := $(HOME)/.local/bin/sui
APTOS := $(HOME)/.local/bin/aptos
PKG  := cage_nft

.PHONY: build test publish clean faucet balance envs address

# ── Build & Test ─────────────────────────────────────────
build:
	cd $(PKG) && $(SUI) move build

test:
	cd $(PKG) && $(SUI) move test

# ── Deploy ───────────────────────────────────────────────
publish:
	cd $(PKG) && $(SUI) client publish --gas-budget 100000000

publish-devnet:
	$(SUI) client switch --env devnet
	cd $(PKG) && $(SUI) client publish --gas-budget 100000000

publish-testnet:
	$(SUI) client switch --env testnet
	cd $(PKG) && $(SUI) client publish --gas-budget 100000000

# ── Wallet ───────────────────────────────────────────────
address:
	$(SUI) client active-address

balance:
	node scripts/balance.js

faucet:
	@echo "Open: https://faucet.sui.io/?address=$$($(SUI) client active-address)"

envs:
	$(SUI) client envs

switch-testnet:
	$(SUI) client switch --env testnet

switch-devnet:
	$(SUI) client switch --env devnet

# ── Mint ─────────────────────────────────────────────────
# Usage: make mint-nft PKG_ID=0x...
mint-nft:
	@test -n "$(PKG_ID)" || (echo "Usage: make mint-nft PKG_ID=0x..." && exit 1)
	node scripts/mint-nft.js $(PKG_ID)

# ── Aptos (secondary) ───────────────────────────────────
aptos-init:
	$(APTOS) init --network testnet

aptos-build:
	$(APTOS) move compile --package-dir $(PKG)

# ── Clean ────────────────────────────────────────────────
clean:
	rm -rf $(PKG)/build

# ── Toolchain info ───────────────────────────────────────
versions:
	@echo "Sui:   $$($(SUI) --version 2>/dev/null || echo 'not installed')"
	@echo "Aptos: $$($(APTOS) --version 2>/dev/null || echo 'not installed')"
	@echo "Rust:  $$(rustc --version 2>/dev/null || echo 'not installed')"
	@echo "Node:  $$(node --version 2>/dev/null || echo 'not installed')"
