#!/usr/bin/env bash
set -euo pipefail
#
# generate-keys.sh — Generate an ed25519 SSH keypair for the 3090 headless machine.
#
# Creates:
#   keys/3090-headless       (private key, mode 600)
#   keys/3090-headless.pub   (public key, mode 644)
#
# Also displays the user's existing personal key for reference.
#

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
KEYS_DIR="${PROJECT_DIR}/keys"
KEY_NAME="3090-headless"
KEY_PATH="${KEYS_DIR}/${KEY_NAME}"
KEY_COMMENT="gpu-3090-headless-access"

# The user's existing personal SSH public key
USER_PUBKEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJM8FDlkumIPCkCLGA8Y+10800s35YSWw25Ml748FZVR tomlee3ddesign@gmail.com"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
info()  { printf '\033[1;34m[INFO]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[WARN]\033[0m  %s\n' "$*"; }
ok()    { printf '\033[1;32m[ OK ]\033[0m  %s\n' "$*"; }
err()   { printf '\033[1;31m[ERR ]\033[0m  %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
    info "SSH Key Generator for GPU-3090 Headless Machine"
    echo ""

    # Ensure the keys directory exists
    mkdir -p "${KEYS_DIR}"

    # -----------------------------------------------------------------------
    # Check for existing keys — warn and ask before overwriting
    # -----------------------------------------------------------------------
    if [[ -f "${KEY_PATH}" || -f "${KEY_PATH}.pub" ]]; then
        warn "SSH keypair already exists:"
        [[ -f "${KEY_PATH}" ]]     && warn "  Private: ${KEY_PATH}"
        [[ -f "${KEY_PATH}.pub" ]] && warn "  Public:  ${KEY_PATH}.pub"
        echo ""

        # If running non-interactively, refuse to overwrite
        if [[ ! -t 0 ]]; then
            err "Running non-interactively — refusing to overwrite existing keys."
            err "Delete the existing keys manually if you want to regenerate."
            exit 1
        fi

        read -r -p "Overwrite existing keys? [y/N] " answer
        case "${answer}" in
            [Yy]|[Yy][Ee][Ss])
                warn "Overwriting existing keys..."
                rm -f "${KEY_PATH}" "${KEY_PATH}.pub"
                ;;
            *)
                info "Keeping existing keys. Exiting."
                echo ""
                info "Existing public key:"
                cat "${KEY_PATH}.pub"
                exit 0
                ;;
        esac
    fi

    # -----------------------------------------------------------------------
    # Generate the keypair
    # -----------------------------------------------------------------------
    info "Generating ed25519 SSH keypair..."
    ssh-keygen -t ed25519 -C "${KEY_COMMENT}" -f "${KEY_PATH}" -N "" -q

    # Set proper permissions
    chmod 600 "${KEY_PATH}"
    chmod 644 "${KEY_PATH}.pub"

    ok "Keypair generated successfully."
    echo ""
    echo "  Private key: ${KEY_PATH}  (mode 600)"
    echo "  Public key:  ${KEY_PATH}.pub  (mode 644)"
    echo ""

    # -----------------------------------------------------------------------
    # Display the generated public key
    # -----------------------------------------------------------------------
    info "Generated public key (add this to the 3090 authorized_keys):"
    echo ""
    printf '  %s\n' "$(cat "${KEY_PATH}.pub")"
    echo ""

    # -----------------------------------------------------------------------
    # Show the user's existing personal key for reference
    # -----------------------------------------------------------------------
    info "Your existing personal SSH key (also authorized on the 3090):"
    echo ""
    printf '  %s\n' "${USER_PUBKEY}"
    echo ""

    # -----------------------------------------------------------------------
    # Verify the key fingerprints
    # -----------------------------------------------------------------------
    info "Key fingerprints:"
    echo "  Generated: $(ssh-keygen -lf "${KEY_PATH}.pub")"

    # Try to show the fingerprint of the user's personal key if it exists
    if [[ -f "${HOME}/.ssh/id_ed25519.pub" ]]; then
        echo "  Personal:  $(ssh-keygen -lf "${HOME}/.ssh/id_ed25519.pub")"
    fi
    echo ""

    ok "Done. Use 'make iso' to build the autoinstall ISO with this key baked in."
}

main "$@"
