"""docker_manager.py — Docker SDK wrapper for container lifecycle.

Uses the Docker SDK (python-docker) for all container operations.
Same naming conventions as lib/docker.sh: containers named cage-<name>,
labeled managed-by=claude-cage, volumes named cage-data-<name>.
"""

import os
import threading

import docker

from sandbox import build_container_kwargs, ensure_filtered_network, apply_network_filter
from config_loader import get as cfg_get

CAGE_LABEL = "managed-by=claude-cage"
LABEL_KEY = "managed-by"
LABEL_VAL = "claude-cage"

_client = None
_client_lock = threading.Lock()


def get_client():
    """Get or create a Docker client (thread-safe singleton)."""
    global _client
    with _client_lock:
        if _client is None:
            _client = docker.from_env()
        return _client


def check():
    """Check if Docker is reachable. Returns (ok, message)."""
    try:
        client = get_client()
        client.ping()
        return True, "Docker is running"
    except Exception as e:
        return False, str(e)


def build_image(mode, cage_root, cfg):
    """Build a Docker image. Returns (success, message)."""
    client = get_client()
    if mode == "cli":
        tag = cfg_get(cfg, "image_cli", "claude-cage-cli:latest")
        dockerfile = os.path.join(cage_root, "docker", "cli", "Dockerfile")
        context = os.path.join(cage_root, "docker", "cli")
    elif mode == "desktop":
        tag = cfg_get(cfg, "image_desktop", "claude-cage-desktop:latest")
        dockerfile = os.path.join(cage_root, "docker", "desktop", "Dockerfile")
        context = os.path.join(cage_root, "docker", "desktop")
    else:
        return False, f"Unknown mode: {mode}"

    try:
        client.images.build(
            path=context,
            dockerfile=dockerfile,
            tag=tag,
            rm=True,
        )
        return True, f"{mode} image built: {tag}"
    except Exception as e:
        return False, str(e)


def run_session(name, mode, api_key, network, cpus, memory, gpu,
                ephemeral, persist, mounts, ports, env_vars,
                cage_root, cfg):
    """Create and start a container. Returns (success, info_dict)."""
    client = get_client()
    container_name = f"cage-{name}"

    # Ensure filtered network exists
    if network == "filtered":
        ensure_filtered_network(client)

    # Pick image
    if mode == "cli":
        image = cfg_get(cfg, "image_cli", "claude-cage-cli:latest")
    else:
        image = cfg_get(cfg, "image_desktop", "claude-cage-desktop:latest")

    # Build security kwargs
    kwargs = build_container_kwargs(network, cpus, memory, gpu, cage_root, cfg)

    # Labels
    kwargs["labels"] = {
        LABEL_KEY: LABEL_VAL,
        "cage.mode": mode,
        "cage.session": name,
    }

    # Container name and hostname
    kwargs["name"] = container_name
    kwargs["hostname"] = container_name

    # Detach (always detach for web dashboard; CLI sessions too)
    kwargs["detach"] = True
    kwargs["tty"] = True
    kwargs["stdin_open"] = (mode == "cli")

    # Auto-remove
    if ephemeral:
        kwargs["auto_remove"] = True

    # Restart policy for desktop
    if mode == "desktop" and not ephemeral:
        kwargs["restart_policy"] = {"Name": "unless-stopped"}

    # Environment
    environment = {"ANTHROPIC_API_KEY": api_key}
    if mode == "desktop":
        environment["DISPLAY"] = ":1"
    for ev in (env_vars or []):
        if "=" in ev:
            k, v = ev.split("=", 1)
            environment[k] = v
    kwargs["environment"] = environment

    # Volumes
    volumes = {}
    if persist:
        vol_name = f"cage-data-{name}"
        volumes[vol_name] = {"bind": "/home/cageuser/.claude", "mode": "rw"}
    for m in (mounts or []):
        abs_path = os.path.abspath(m)
        mount_target = f"/workspace/{os.path.basename(abs_path)}"
        volumes[abs_path] = {"bind": mount_target, "mode": "rw"}
    if volumes:
        kwargs["volumes"] = volumes

    # Port mappings
    port_bindings = {}
    for p in (ports or []):
        if ":" in p:
            host_port, container_port = p.split(":", 1)
            port_bindings[container_port] = host_port
    if mode == "desktop":
        desktop_port = cfg_get(cfg, "desktop_port", "6080")
        port_bindings["6080/tcp"] = desktop_port
        port_bindings["5900/tcp"] = cfg_get(cfg, "vnc_port", "5900")
    if port_bindings:
        kwargs["ports"] = port_bindings

    # Working directory
    kwargs["working_dir"] = "/workspace"

    try:
        container = client.containers.run(image, **kwargs)

        # Post-launch network filtering
        if network == "filtered":
            allowed = cfg_get(cfg, "allowed_hosts", "api.anthropic.com")
            apply_network_filter(container_name, allowed)

        return True, {
            "name": name,
            "container": container_name,
            "mode": mode,
            "image": image,
            "id": container.short_id,
        }
    except Exception as e:
        return False, {"error": str(e)}


def stop_session(name):
    """Stop a running container. Returns (success, message)."""
    client = get_client()
    try:
        container = client.containers.get(f"cage-{name}")
        container.stop(timeout=10)
        return True, f"Stopped {name}"
    except docker.errors.NotFound:
        return False, f"Container cage-{name} not found"
    except Exception as e:
        return False, str(e)


def start_session(name):
    """Restart a stopped container. Returns (success, message)."""
    client = get_client()
    try:
        container = client.containers.get(f"cage-{name}")
        container.start()
        return True, f"Started {name}"
    except docker.errors.NotFound:
        return False, f"Container cage-{name} not found"
    except Exception as e:
        return False, str(e)


def stop_all():
    """Stop all claude-cage containers. Returns count stopped."""
    client = get_client()
    containers = client.containers.list(filters={"label": CAGE_LABEL})
    count = 0
    for c in containers:
        try:
            c.stop(timeout=10)
            count += 1
        except Exception:
            pass
    return count


def destroy_session(name, force=False):
    """Remove container + volume. Returns (success, message)."""
    client = get_client()
    container_name = f"cage-{name}"

    try:
        container = client.containers.get(container_name)
        container.remove(force=force)
    except docker.errors.NotFound:
        pass
    except Exception as e:
        return False, str(e)

    # Remove volume
    try:
        vol = client.volumes.get(f"cage-data-{name}")
        vol.remove()
    except docker.errors.NotFound:
        pass
    except Exception:
        pass

    return True, f"Destroyed {name}"


def inspect_session(name):
    """Return detailed container info as dict."""
    client = get_client()
    try:
        container = client.containers.get(f"cage-{name}")
        attrs = container.attrs
        host_cfg = attrs.get("HostConfig", {})
        state = attrs.get("State", {})
        net = attrs.get("NetworkSettings", {})

        # Security checks
        ro = host_cfg.get("ReadonlyRootfs", False)
        caps = host_cfg.get("CapDrop") or []
        sec_opts = host_cfg.get("SecurityOpt") or []

        security = {
            "read_only_root": ro,
            "caps_dropped": "ALL" in caps,
            "no_new_privileges": any("no-new-privileges" in s for s in sec_opts),
            "seccomp_active": any("seccomp" in s for s in sec_opts),
        }

        # Memory in GB
        mem_bytes = host_cfg.get("Memory", 0)
        mem_gb = round(mem_bytes / (1024 ** 3), 1) if mem_bytes else 0

        # CPUs
        nano_cpus = host_cfg.get("NanoCpus", 0)
        cpu_count = round(nano_cpus / 1e9, 1) if nano_cpus else 0

        # Ports
        port_bindings = host_cfg.get("PortBindings") or {}

        return {
            "name": name,
            "container": f"cage-{name}",
            "status": state.get("Status", "unknown"),
            "running": state.get("Running", False),
            "image": attrs.get("Config", {}).get("Image", ""),
            "started_at": state.get("StartedAt", ""),
            "memory_gb": mem_gb,
            "cpus": cpu_count,
            "security": security,
            "ports": port_bindings,
            "ip": _get_container_ip(net),
        }
    except docker.errors.NotFound:
        return None
    except Exception as e:
        return {"error": str(e)}


def get_logs(name, tail=200):
    """Return container logs as string."""
    client = get_client()
    try:
        container = client.containers.get(f"cage-{name}")
        logs = container.logs(tail=tail, timestamps=False)
        if isinstance(logs, bytes):
            return logs.decode("utf-8", errors="replace")
        return str(logs)
    except docker.errors.NotFound:
        return ""
    except Exception as e:
        return f"Error: {e}"


def get_stats(name):
    """Return a single snapshot of container stats."""
    client = get_client()
    try:
        container = client.containers.get(f"cage-{name}")
        stats = container.stats(stream=False)

        # CPU percentage
        cpu_delta = (
            stats["cpu_stats"]["cpu_usage"]["total_usage"]
            - stats["precpu_stats"]["cpu_usage"]["total_usage"]
        )
        system_delta = (
            stats["cpu_stats"]["system_cpu_usage"]
            - stats["precpu_stats"]["system_cpu_usage"]
        )
        num_cpus = stats["cpu_stats"]["online_cpus"]
        cpu_pct = (cpu_delta / system_delta * num_cpus * 100) if system_delta > 0 else 0

        # Memory
        mem_usage = stats["memory_stats"].get("usage", 0)
        mem_limit = stats["memory_stats"].get("limit", 0)
        mem_pct = (mem_usage / mem_limit * 100) if mem_limit > 0 else 0

        return {
            "cpu_percent": round(cpu_pct, 1),
            "memory_usage": mem_usage,
            "memory_limit": mem_limit,
            "memory_percent": round(mem_pct, 1),
        }
    except docker.errors.NotFound:
        return None
    except Exception:
        return None


def list_containers():
    """List all claude-cage containers (running + stopped)."""
    client = get_client()
    result = []
    try:
        containers = client.containers.list(
            all=True, filters={"label": CAGE_LABEL}
        )
        for c in containers:
            labels = c.labels or {}
            result.append({
                "name": labels.get("cage.session", c.name),
                "container": c.name,
                "mode": labels.get("cage.mode", "?"),
                "status": c.status,
                "image": c.image.tags[0] if c.image.tags else "",
            })
    except Exception:
        pass
    return result


def list_images():
    """List claude-cage Docker images."""
    client = get_client()
    result = []
    try:
        for tag_prefix in ("claude-cage-cli", "claude-cage-desktop"):
            images = client.images.list(name=tag_prefix)
            for img in images:
                for tag in img.tags:
                    result.append({
                        "tag": tag,
                        "id": img.short_id,
                        "size_mb": round(img.attrs["Size"] / (1024 * 1024)),
                        "created": img.attrs.get("Created", ""),
                    })
    except Exception:
        pass
    return result


def _get_container_ip(network_settings):
    """Extract container IP from network settings."""
    networks = network_settings.get("Networks", {})
    for net_name, net_info in networks.items():
        ip = net_info.get("IPAddress")
        if ip:
            return ip
    return ""
