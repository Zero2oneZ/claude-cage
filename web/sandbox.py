"""sandbox.py — Security flag construction for Docker containers.

Port of lib/sandbox.sh. Translates sandbox_build_flags() into Docker SDK kwargs.
"""

import os
import subprocess

import docker as docker_sdk


def build_container_kwargs(network, cpus, memory, gpu, cage_root, cfg):
    """Build Docker SDK kwargs dict equivalent to sandbox_build_flags().

    Returns a dict suitable for docker.containers.run(**kwargs).
    """
    kwargs = {}

    # Resource limits
    kwargs["nano_cpus"] = int(float(cpus) * 1e9)
    kwargs["mem_limit"] = memory
    kwargs["pids_limit"] = 512
    kwargs["ulimits"] = [
        docker_sdk.types.Ulimit(name="nofile", soft=1024, hard=2048),
        docker_sdk.types.Ulimit(name="nproc", soft=256, hard=512),
    ]

    # Filesystem hardening
    if cfg.get("read_only_root", "true") == "true":
        kwargs["read_only"] = True
        kwargs["tmpfs"] = {
            "/tmp": "rw,noexec,nosuid,size=512m",
            "/run": "rw,noexec,nosuid,size=64m",
        }

    # Capability dropping
    kwargs["cap_drop"] = ["ALL"]
    kwargs["cap_add"] = ["CHOWN", "DAC_OVERRIDE", "SETGID", "SETUID"]

    # Security options
    security_opt = ["no-new-privileges"]

    seccomp_profile = cfg.get("seccomp_profile", "default")
    if seccomp_profile == "default":
        seccomp_file = os.path.join(cage_root, "security", "seccomp-default.json")
        if os.path.isfile(seccomp_file):
            security_opt.append(f"seccomp={seccomp_file}")
    elif seccomp_profile != "unconfined":
        security_opt.append(f"seccomp={seccomp_profile}")

    # AppArmor
    apparmor_file = os.path.join(cage_root, "security", "apparmor-profile")
    if os.path.isfile(apparmor_file):
        try:
            subprocess.run(
                ["apparmor_parser", "--version"],
                capture_output=True, timeout=3
            )
            security_opt.append("apparmor=claude-cage")
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass

    kwargs["security_opt"] = security_opt

    # Network
    if network == "none":
        kwargs["network_mode"] = "none"
    elif network == "host":
        kwargs["network_mode"] = "host"
    elif network == "filtered":
        kwargs["network"] = "cage-filtered"
        kwargs["dns"] = [cfg.get("dns", "1.1.1.1")]

    # GPU
    if gpu:
        kwargs["device_requests"] = [
            docker_sdk.types.DeviceRequest(count=-1, capabilities=[["gpu"]])
        ]

    # User namespace
    kwargs["userns_mode"] = "host"

    return kwargs


def ensure_filtered_network(client):
    """Create the cage-filtered bridge network if it doesn't exist."""
    try:
        client.networks.get("cage-filtered")
    except docker_sdk.errors.NotFound:
        client.networks.create(
            "cage-filtered",
            driver="bridge",
            options={
                "com.docker.network.bridge.enable_icc": "false",
                "com.docker.network.bridge.enable_ip_masquerade": "true",
            },
            ipam=docker_sdk.types.IPAMConfig(
                pool_configs=[
                    docker_sdk.types.IPAMPool(subnet="172.28.0.0/16")
                ]
            ),
        )


def apply_network_filter(container_name, allowed_hosts):
    """Apply iptables rules to restrict outbound to allowed hosts.

    Shells out to iptables, same as sandbox_apply_network_filter() in bash.
    Returns dict with status info.
    """
    result = {"applied": False, "message": ""}

    # Check iptables
    try:
        subprocess.run(["iptables", "--version"], capture_output=True, timeout=3)
    except (FileNotFoundError, subprocess.TimeoutExpired):
        result["message"] = "iptables not available"
        return result

    # Get container IP
    try:
        r = subprocess.run(
            ["docker", "inspect", "-f",
             "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
             container_name],
            capture_output=True, text=True, timeout=5
        )
        container_ip = r.stdout.strip()
        if not container_ip:
            result["message"] = "Could not determine container IP"
            return result
    except (subprocess.TimeoutExpired, FileNotFoundError):
        result["message"] = "Docker inspect failed"
        return result

    # Resolve and allow each host
    hosts = [h.strip() for h in allowed_hosts.split(",") if h.strip()]
    for host in hosts:
        try:
            r = subprocess.run(
                ["getent", "hosts", host],
                capture_output=True, text=True, timeout=5
            )
            for line in r.stdout.strip().splitlines():
                ip = line.split()[0]
                subprocess.run(
                    ["iptables", "-I", "DOCKER-USER", "-s", container_ip,
                     "-d", ip, "-j", "ACCEPT"],
                    capture_output=True, timeout=5
                )
        except (subprocess.TimeoutExpired, FileNotFoundError, IndexError):
            pass

    # Allow DNS
    for proto in ("udp", "tcp"):
        subprocess.run(
            ["iptables", "-I", "DOCKER-USER", "-s", container_ip,
             "-p", proto, "--dport", "53", "-j", "ACCEPT"],
            capture_output=True, timeout=5
        )

    # Drop everything else
    subprocess.run(
        ["iptables", "-A", "DOCKER-USER", "-s", container_ip, "-j", "DROP"],
        capture_output=True, timeout=5
    )

    result["applied"] = True
    result["message"] = f"Filtered {container_ip} -> {', '.join(hosts)}"
    return result
