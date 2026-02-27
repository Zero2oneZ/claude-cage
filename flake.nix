{
  description = "claude-cage — GentlyOS containerized sandbox + 28-crate Rust application layer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pin Rust toolchain — matches system (1.93) not Docker (1.75)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # System libs needed by Cargo crates
        buildInputs = with pkgs; [
          openssl
          libpcap
          pkg-config
        ];

        # Shared native build deps
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
          cargo
        ];

        # Python environment for PTC engine + embeddings
        pythonEnv = pkgs.python312.withPackages (ps: with ps; [
          ps.requests
        ]);

        # Python ML environment (heavy — only for embedding/training shells)
        pythonMlEnv = pkgs.python312.withPackages (ps: with ps; [
          ps.requests
          ps.torch
          ps.sentence-transformers
          ps.transformers
        ]);

      in {
        # ── Dev Shells ──────────────────────────────────────────────

        devShells = {

          # Full development environment — everything
          default = pkgs.mkShell {
            name = "cage-dev";
            inherit buildInputs;
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
              # Node
              nodejs_20
              nodePackages.npm

              # Python (lightweight — no ML deps)
              pythonEnv

              # Docker
              docker
              docker-compose

              # Sui / Move
              # sui-cli not in nixpkgs — installed via suiup outside Nix

              # Tools
              git
              jq
              ripgrep
              curl
              gnumake
              tini

              # Security
              apparmor-utils
              apparmor-profiles

              # MongoDB Atlas CLI (not in nixpkgs — use ~/bin/atlas)
            ]);

            shellHook = ''
              echo "══════════════════════════════════════════════"
              echo "  cage-dev — full GentlyOS development shell"
              echo "══════════════════════════════════════════════"
              echo "  rust   $(rustc --version | cut -d' ' -f2)"
              echo "  node   $(node --version)"
              echo "  python $(python3 --version | cut -d' ' -f2)"
              echo "  cargo  $(cargo --version | cut -d' ' -f2)"
              echo ""
              echo "  make build-web       cage-web dashboard"
              echo "  make build-gently    GentlyOS 28 crates"
              echo "  make run-cli         Docker sandbox"
              echo "  make docs            Circular doc system"
              echo "══════════════════════════════════════════════"

              export OPENSSL_DIR="${pkgs.openssl.dev}"
              export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libpcap}/lib/pkgconfig"
            '';
          };

          # FAFO security test shell — minimal, fast
          fafo = pkgs.mkShell {
            name = "cage-fafo";
            inherit buildInputs nativeBuildInputs;

            shellHook = ''
              echo "══════════════════════════════════════════════"
              echo "  cage-fafo — FAFO security test shell"
              echo "══════════════════════════════════════════════"
              echo ""
              echo "  cargo test -p gently-security --lib"
              echo "  cargo test -p gently-security -- fafo"
              echo "  cargo test -p gently-sandbox --lib"
              echo "══════════════════════════════════════════════"

              export OPENSSL_DIR="${pkgs.openssl.dev}"
              export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libpcap}/lib/pkgconfig"
            '';
          };

          # CODIE parser + PTC orchestration test shell
          codie = pkgs.mkShell {
            name = "cage-codie";
            nativeBuildInputs = nativeBuildInputs ++ [ pkgs.nodejs_20 pythonEnv ];
            inherit buildInputs;

            shellHook = ''
              echo "══════════════════════════════════════════════"
              echo "  cage-codie — CODIE + PTC test shell"
              echo "══════════════════════════════════════════════"
              echo ""
              echo "  cargo test -p gently-codie --lib"
              echo "  cargo test -p gently-ptc --lib"
              echo "  python3 -m ptc.engine route 'intent'"
              echo "  python3 -m ptc.docs status"
              echo "  make codie-parse FILE=codie-maps/install.codie"
              echo "══════════════════════════════════════════════"

              export OPENSSL_DIR="${pkgs.openssl.dev}"
              export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
            '';
          };

          # ML / embeddings shell — heavy deps (torch, transformers)
          ml = pkgs.mkShell {
            name = "cage-ml";
            nativeBuildInputs = nativeBuildInputs ++ [ pkgs.nodejs_20 pythonMlEnv ];
            inherit buildInputs;

            shellHook = ''
              echo "══════════════════════════════════════════════"
              echo "  cage-ml — ML + embeddings shell"
              echo "══════════════════════════════════════════════"
              echo ""
              echo "  python3 -m ptc.embeddings"
              echo "  python3 -m ptc.lora"
              echo "  python3 -m ptc.docs interconnect"
              echo "══════════════════════════════════════════════"
            '';
          };

          # Sui / Move development shell
          sui = pkgs.mkShell {
            name = "cage-sui";
            nativeBuildInputs = with pkgs; [
              nodejs_20
              nodePackages.npm
              git
              curl
            ];

            shellHook = ''
              echo "══════════════════════════════════════════════"
              echo "  cage-sui — Sui / Move development shell"
              echo "══════════════════════════════════════════════"
              echo ""
              echo "  sui move build"
              echo "  sui move test"
              echo "  sui client publish --gas-budget 100000000"
              echo ""
              echo "  NOTE: sui CLI installed via suiup (not Nix)"
              echo "  Run: curl -sSfL https://sui.io/install.sh | sh"
              echo "══════════════════════════════════════════════"

              # Add suiup-installed binaries to PATH
              export PATH="$HOME/.sui/bin:$PATH"
            '';
          };
        };

        # ── Packages (reproducible builds) ──────────────────────────

        packages = {

          # cage-web dashboard binary
          cage-web = pkgs.rustPlatform.buildRustPackage {
            pname = "cage-web";
            version = "1.0.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            inherit buildInputs nativeBuildInputs;

            # Only build cage-web, not gentlyos-core
            buildAndTestSubdir = "cage-web";

            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          };
        };

        # ── Checks (run in CI) ──────────────────────────────────────

        checks = {

          # FAFO security test suite
          fafo-tests = pkgs.runCommand "fafo-tests" {
            nativeBuildInputs = nativeBuildInputs ++ [ pkgs.openssl pkgs.libpcap ];
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libpcap}/lib/pkgconfig";
          } ''
            cd ${./gentlyos-core}
            cargo test -p gently-security --lib 2>&1
            touch $out
          '';

          # CODIE parser tests
          codie-tests = pkgs.runCommand "codie-tests" {
            nativeBuildInputs = nativeBuildInputs ++ [ pkgs.openssl pkgs.libpcap ];
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          } ''
            cd ${./gentlyos-core}
            cargo test -p gently-codie --lib 2>&1
            touch $out
          '';

          # cage-web compilation check
          cage-web-build = self.packages.${system}.cage-web;
        };
      }
    );
}
