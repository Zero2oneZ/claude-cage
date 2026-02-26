#!/usr/bin/env bash
# sandbox.sh — Sandbox policy enforcement and security flag construction

sandbox_build_flags() {
    local network="$1"
    local cpus="$2"
    local memory="$3"
    local gpu="$4"
    local -a flags=()

    # ── Resource limits ──────────────────────────────────────────
    flags+=(--cpus "$cpus")
    flags+=(--memory "$memory")
    flags+=(--pids-limit 512)
    flags+=(--ulimit nofile=1024:2048)
    flags+=(--ulimit nproc=256:512)

    # ── Filesystem hardening ─────────────────────────────────────
    if [[ "$(config_get read_only_root true)" == "true" ]]; then
        flags+=(--read-only)
        # Writable tmpfs for runtime needs
        flags+=(--tmpfs /tmp:rw,noexec,nosuid,size=512m)
        flags+=(--tmpfs /run:rw,noexec,nosuid,size=64m)
    fi

    # ── Capability dropping ──────────────────────────────────────
    flags+=(--cap-drop ALL)
    # Minimal capabilities needed
    flags+=(--cap-add CHOWN)
    flags+=(--cap-add DAC_OVERRIDE)
    flags+=(--cap-add SETGID)
    flags+=(--cap-add SETUID)

    # ── Security profiles ────────────────────────────────────────
    local seccomp_profile
    seccomp_profile="$(config_get seccomp_profile default)"
    if [[ "$seccomp_profile" == "default" ]]; then
        local seccomp_file="$CAGE_ROOT/security/seccomp-default.json"
        if [[ -f "$seccomp_file" ]]; then
            flags+=(--security-opt "seccomp=$seccomp_file")
        fi
    elif [[ "$seccomp_profile" != "unconfined" ]]; then
        flags+=(--security-opt "seccomp=$seccomp_profile")
    fi

    # AppArmor (if profile is loaded)
    local apparmor_profile="$CAGE_ROOT/security/apparmor-profile"
    if [[ -f "$apparmor_profile" ]] && command -v apparmor_parser &>/dev/null; then
        flags+=(--security-opt "apparmor=claude-cage")
    fi

    # ── No new privileges ────────────────────────────────────────
    flags+=(--security-opt no-new-privileges)

    # ── Network policy ───────────────────────────────────────────
    case "$network" in
        none)
            flags+=(--network none)
            ;;
        host)
            flags+=(--network host)
            ;;
        filtered)
            # Use bridge network; iptables rules applied post-start
            flags+=(--network cage-filtered)
            flags+=(--dns "$(config_get dns 1.1.1.1)")
            ;;
    esac

    # ── GPU passthrough ──────────────────────────────────────────
    if [[ "$gpu" == "true" ]]; then
        flags+=(--gpus all)
    fi

    # ── User namespace ───────────────────────────────────────────
    flags+=(--userns host)

    echo "${flags[@]}"
}

# Create the filtered network (called once during setup)
sandbox_create_network() {
    if ! docker network inspect cage-filtered &>/dev/null; then
        echo "==> Creating filtered network: cage-filtered"
        docker network create \
            --driver bridge \
            --opt com.docker.network.bridge.enable_icc=false \
            --opt com.docker.network.bridge.enable_ip_masquerade=true \
            --subnet 172.28.0.0/16 \
            cage-filtered
    fi
}

# Returns allowed hosts for a given tier (additive — higher tiers include lower)
sandbox_tier_hosts() {
    local tier="${1:-free}"
    local hosts="api.anthropic.com,cdn.anthropic.com"

    case "$tier" in
        basic)
            hosts="$hosts,github.com,pypi.org,files.pythonhosted.org"
            ;;
        pro)
            hosts="$hosts,github.com,pypi.org,files.pythonhosted.org"
            hosts="$hosts,registry-1.docker.io,auth.docker.io,production.cloudflare.docker.com"
            hosts="$hosts,registry.npmjs.org"
            ;;
        dev|founder|admin)
            # Full outbound — return empty to skip filtering
            return 0
            ;;
    esac

    # Add IPFS bootstrap hosts when GENTLY_TIER is set
    if [[ -n "${GENTLY_TIER:-}" ]]; then
        hosts="$hosts,dweb.link,ipfs.io,gateway.pinata.cloud"
    fi

    echo "$hosts"
}

# Returns allowed ports for a given tier
sandbox_tier_ports() {
    local tier="${1:-free}"
    # All tiers need HTTPS (443) and DNS (53)
    local ports="443,53"

    case "$tier" in
        basic)
            ports="$ports,22"  # SSH for git
            ;;
        pro)
            ports="$ports,22,5000"  # SSH + registry
            ;;
        dev|founder|admin)
            # Full outbound
            echo ""
            return 0
            ;;
    esac

    echo "$ports"
}

# Apply firewall rules for filtered mode (restrict to allowed hosts only)
sandbox_apply_network_filter() {
    local container_name="$1"
    local tier="${2:-free}"

    # Dev+ tiers get full outbound — skip filtering
    if [[ "$tier" == "dev" || "$tier" == "founder" || "$tier" == "admin" ]]; then
        echo "==> Tier '$tier': full outbound access (no filtering)"
        return 0
    fi

    local allowed_hosts
    # Use tier-specific hosts, falling back to config
    allowed_hosts="$(sandbox_tier_hosts "$tier")"
    if [[ -z "$allowed_hosts" ]]; then
        allowed_hosts="$(config_get allowed_hosts "api.anthropic.com")"
    fi

    if ! command -v iptables &>/dev/null; then
        echo "Warning: iptables not available — network filtering not applied." >&2
        return
    fi

    # Get container IP
    local container_ip
    container_ip=$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$container_name" 2>/dev/null)
    if [[ -z "$container_ip" ]]; then
        return
    fi

    echo "==> Applying network filter for $container_name ($container_ip)"
    echo "    Allowed hosts: $allowed_hosts"

    # Resolve allowed hosts and add rules
    IFS=',' read -ra hosts <<< "$allowed_hosts"
    for host in "${hosts[@]}"; do
        host="$(echo "$host" | xargs)"  # trim whitespace
        local ips
        ips=$(getent hosts "$host" 2>/dev/null | awk '{print $1}' || true)
        for ip in $ips; do
            iptables -I DOCKER-USER -s "$container_ip" -d "$ip" -j ACCEPT 2>/dev/null || true
        done
    done

    # Allow DNS
    iptables -I DOCKER-USER -s "$container_ip" -p udp --dport 53 -j ACCEPT 2>/dev/null || true
    iptables -I DOCKER-USER -s "$container_ip" -p tcp --dport 53 -j ACCEPT 2>/dev/null || true

    # Drop everything else from this container
    iptables -A DOCKER-USER -s "$container_ip" -j DROP 2>/dev/null || true
}

# Verify sandbox is properly applied
sandbox_verify() {
    local container_name="$1"
    local -a checks=()
    local all_pass=true

    echo "==> Verifying sandbox for: $container_name"

    # Check read-only root
    local ro
    ro=$(docker inspect -f '{{.HostConfig.ReadonlyRootfs}}' "$container_name" 2>/dev/null)
    if [[ "$ro" == "true" ]]; then
        checks+=("  [PASS] Read-only root filesystem")
    else
        checks+=("  [WARN] Root filesystem is writable")
    fi

    # Check capabilities
    local caps
    caps=$(docker inspect -f '{{.HostConfig.CapDrop}}' "$container_name" 2>/dev/null)
    if [[ "$caps" == *"ALL"* ]]; then
        checks+=("  [PASS] All capabilities dropped")
    else
        checks+=("  [WARN] Not all capabilities dropped")
        all_pass=false
    fi

    # Check no-new-privileges
    local sec_opts
    sec_opts=$(docker inspect -f '{{.HostConfig.SecurityOpt}}' "$container_name" 2>/dev/null)
    if [[ "$sec_opts" == *"no-new-privileges"* ]]; then
        checks+=("  [PASS] no-new-privileges set")
    else
        checks+=("  [WARN] no-new-privileges not set")
        all_pass=false
    fi

    # Check resource limits
    local mem
    mem=$(docker inspect -f '{{.HostConfig.Memory}}' "$container_name" 2>/dev/null)
    if [[ "$mem" != "0" && -n "$mem" ]]; then
        checks+=("  [PASS] Memory limit set: $mem bytes")
    else
        checks+=("  [WARN] No memory limit")
        all_pass=false
    fi

    printf '%s\n' "${checks[@]}"

    if $all_pass; then
        echo "  Sandbox verification: ALL CHECKS PASSED"
    else
        echo "  Sandbox verification: SOME CHECKS FAILED — review above"
    fi
}
