# Security Architecture

claude-cage implements 8 layers of defense-in-depth security for running Claude CLI and Claude Desktop in isolated Docker containers. No single layer is sufficient alone; the combination provides robust containment against container escape, privilege escalation, resource exhaustion, network exfiltration, inter-container lateral movement, and persistent filesystem compromise.

---

## Table of Contents

1. [Layer 1: Read-Only Root Filesystem](#layer-1-read-only-root-filesystem)
2. [Layer 2: Capability Dropping](#layer-2-capability-dropping)
3. [Layer 3: Seccomp Profile](#layer-3-seccomp-profile)
4. [Layer 4: AppArmor Profile](#layer-4-apparmor-profile)
5. [Layer 5: Resource Limits](#layer-5-resource-limits)
6. [Layer 6: Network Filtering](#layer-6-network-filtering)
7. [Layer 7: No-New-Privileges](#layer-7-no-new-privileges)
8. [Layer 8: Bridge Network Isolation](#layer-8-bridge-network-isolation)
9. [Verification](#verification)
10. [Implementation Locations](#implementation-locations)
11. [Threat Model](#threat-model)

---

## Layer 1: Read-Only Root Filesystem

The container root filesystem is mounted read-only via the `--read-only` flag. This prevents any process inside the container from persistently modifying the container's filesystem, including planting backdoors, modifying binaries, or altering configuration files.

Two tmpfs mounts are provided for runtime needs:

| Mount   | Size  | Options                  | Purpose                              |
|---------|-------|--------------------------|--------------------------------------|
| `/tmp`  | 512 MB | `rw,nosuid,nodev,noexec` | Temporary files, scratch space       |
| `/run`  | 64 MB  | `rw,nosuid,nodev,noexec` | Runtime state (PID files, sockets)   |

Both tmpfs mounts enforce `nosuid` (setuid binaries ignored), `nodev` (device files blocked), and `noexec` (execution of binaries from tmpfs blocked). Data in tmpfs is volatile and lost when the container stops.

**Configuration:**

```yaml
# docker-compose.yml (x-common anchor)
read_only: true
tmpfs:
  - /tmp:rw,noexec,nosuid,size=512m
  - /run:rw,noexec,nosuid,size=64m
```

```bash
# launch script (build_security_flags)
flags+=(--read-only)
flags+=(--tmpfs /tmp:rw,noexec,nosuid,size=512m)
flags+=(--tmpfs /run:rw,noexec,nosuid,size=64m)
```

```bash
# lib/sandbox.sh (sandbox_build_flags)
if [[ "$(config_get read_only_root true)" == "true" ]]; then
    flags+=(--read-only)
    flags+=(--tmpfs /tmp:rw,noexec,nosuid,size=512m)
    flags+=(--tmpfs /run:rw,noexec,nosuid,size=64m)
fi
```

---

## Layer 2: Capability Dropping

Linux capabilities subdivide the traditional root/non-root privilege model into discrete units. claude-cage drops ALL capabilities, then re-adds only the four required for the container to function.

**Policy:** `--cap-drop ALL` followed by selective `--cap-add`.

| Capability     | What It Grants                                       | Why It Is Needed                                                              |
|----------------|------------------------------------------------------|-------------------------------------------------------------------------------|
| `CHOWN`        | Change file owner and group                          | Required for npm/Node.js to manage file ownership in the cageuser home directory and workspace |
| `DAC_OVERRIDE` | Bypass file read/write/execute permission checks     | Required for the non-root cageuser to access files owned by other UIDs in mounted volumes      |
| `SETGID`       | Set the process group ID and supplementary group list | Required by the container entrypoint (tini) to set the correct group for cageuser              |
| `SETUID`       | Set the process user ID                              | Required by the container entrypoint (tini) to drop from root to the cageuser UID              |

All other capabilities -- including `NET_RAW` (raw sockets), `SYS_ADMIN` (mount, namespace manipulation), `SYS_PTRACE` (process debugging), `NET_ADMIN` (network configuration), `SYS_MODULE` (kernel module loading), and dozens more -- are permanently dropped.

**Configuration:**

```yaml
# docker-compose.yml (x-common anchor)
cap_drop:
  - ALL
cap_add:
  - CHOWN
  - DAC_OVERRIDE
  - SETGID
  - SETUID
```

---

## Layer 3: Seccomp Profile

### What Is Seccomp

Seccomp (secure computing mode) is a Linux kernel facility that restricts which system calls a process can make. By default, Docker applies a broad seccomp profile, but claude-cage uses a custom restrictive profile that explicitly allows only the syscalls needed for Node.js and Claude CLI operation.

### Profile Details

- **File:** `security/seccomp-default.json`
- **Default action:** `SCMP_ACT_ERRNO` with errno `1` (EPERM) -- any syscall not explicitly allowed is denied
- **Allowed syscalls:** approximately 147 distinct syscalls

### Architecture Support

| Architecture       | Sub-architectures       |
|--------------------|-------------------------|
| `SCMP_ARCH_X86_64` | `SCMP_ARCH_X86`, `SCMP_ARCH_X32` |
| `SCMP_ARCH_AARCH64`| `SCMP_ARCH_ARM`         |

### Allowed Syscall Categories

| Category              | Examples                                                                      | Count |
|-----------------------|-------------------------------------------------------------------------------|-------|
| File operations       | `open`, `openat`, `read`, `write`, `close`, `stat`, `fstat`, `lstat`, `access`, `chmod`, `chown`, `link`, `unlink`, `rename`, `mkdir`, `rmdir`, `symlink`, `readlink`, `truncate`, `fallocate` | ~35 |
| Process management    | `clone`, `clone3`, `fork`, `vfork`, `execve`, `execveat`, `exit`, `exit_group`, `wait4`, `waitid`, `waitpid`, `kill`, `tgkill`, `tkill`, `prctl` | ~15 |
| Memory management     | `mmap`, `mprotect`, `munmap`, `mremap`, `brk`, `madvise`, `mlock`, `mlock2`, `munlock`, `mincore`, `membarrier`, `memfd_create`, `remap_file_pages` | ~13 |
| Networking            | `socket`, `socketpair`, `bind`, `listen`, `accept`, `accept4`, `connect`, `send`, `sendto`, `sendmsg`, `sendmmsg`, `recv`, `recvfrom`, `recvmsg`, `recvmmsg`, `shutdown`, `getsockname`, `getpeername`, `getsockopt`, `setsockopt` | ~22 |
| Signals               | `rt_sigaction`, `rt_sigprocmask`, `rt_sigreturn`, `rt_sigsuspend`, `rt_sigpending`, `rt_sigqueueinfo`, `rt_sigtimedwait`, `sigaltstack`, `signalfd`, `signalfd4` | ~10 |
| I/O multiplexing      | `epoll_create`, `epoll_create1`, `epoll_ctl`, `epoll_wait`, `epoll_pwait`, `epoll_pwait2`, `poll`, `ppoll`, `select`, `pselect6` | ~10 |
| Timers and clocks     | `clock_gettime`, `clock_getres`, `clock_nanosleep`, `nanosleep`, `gettimeofday`, `timer_create`, `timer_delete`, `timer_gettime`, `timer_settime`, `timerfd_create`, `timerfd_gettime`, `timerfd_settime` | ~15 |
| IPC                   | `semctl`, `semget`, `semop`, `semtimedop`, `shmat`, `shmctl`, `shmdt`, `shmget`, `pipe`, `pipe2`, `eventfd`, `eventfd2` | ~12 |
| Identity and info     | `getuid`, `geteuid`, `getgid`, `getegid`, `getpid`, `getppid`, `gettid`, `uname`, `sysinfo`, `getrlimit`, `prlimit64`, `getrusage` | ~15 |

### AF_VSOCK Block

A second rule specifically blocks `AF_VSOCK` (address family 40) sockets even though `socket` is generally allowed. AF_VSOCK enables communication between a virtual machine and its hypervisor host. Blocking it prevents potential VM escape vectors:

```json
{
  "names": ["socket"],
  "action": "SCMP_ACT_ALLOW",
  "args": [
    {
      "index": 0,
      "value": 40,
      "op": "SCMP_CMP_NE"
    }
  ],
  "comment": "Deny AF_VSOCK (40) sockets"
}
```

### Why a Custom Profile Matters

Docker's default seccomp profile allows over 300 syscalls. The claude-cage profile cuts this roughly in half, blocking dangerous syscalls such as `mount`, `umount`, `reboot`, `swapon`, `swapoff`, `kexec_load`, `init_module`, `finit_module`, `delete_module`, `acct`, `settimeofday`, `adjtimex`, `pivot_root`, `chroot`, and `ptrace`. Each blocked syscall closes an attack surface.

**Applied via:**

```bash
--security-opt seccomp=security/seccomp-default.json
```

---

## Layer 4: AppArmor Profile

### What Is AppArmor

AppArmor is a Linux Security Module (LSM) that provides Mandatory Access Control (MAC). Unlike discretionary access control (file permissions), MAC policies are enforced by the kernel and cannot be overridden by the process, even if running as root.

### Profile Details

- **File:** `security/apparmor-profile`
- **Profile name:** `claude-cage`
- **Flags:** `attach_disconnected`, `mediate_deleted`
- **Base abstractions:** `base`, `nameservice`, `openssl`

### Denied Operations

| Category                | Rules                                                               | Purpose                                    |
|-------------------------|---------------------------------------------------------------------|--------------------------------------------|
| Mount operations        | `deny mount`, `deny umount`, `deny pivot_root`                     | Prevents filesystem namespace manipulation |
| Kernel modules          | `deny /lib/modules/** w`, `deny @{PROC}/sys/kernel/modules_disabled w` | Prevents loading kernel modules    |
| Process tracing         | `deny ptrace (trace)`, `deny ptrace (read)`                        | Prevents debugging other processes         |
| Kernel interfaces       | `deny @{PROC}/sys/** w`, `deny @{PROC}/sysrq-trigger rw`, `deny @{PROC}/kcore r`, `deny /sys/firmware/** r`, `deny /sys/kernel/security/** rw` | Blocks write access to kernel tunables |
| Raw network access      | `deny network raw`, `deny network packet`                          | Prevents raw socket creation (packet sniffing, spoofing) |

### Allowed Operations

| Category           | Rules                                                                                  |
|--------------------|----------------------------------------------------------------------------------------|
| Network (TCP/UDP)  | `inet stream`, `inet dgram`, `inet6 stream`, `inet6 dgram`, `unix stream`, `unix dgram` |
| Workspace files    | `/workspace/**` read/write, `/home/cageuser/**` read/write                             |
| Temporary files    | `/tmp/**` read/write, `/run/**` read/write                                             |
| System libraries   | `/usr/**`, `/lib/**`, `/etc/**`, `/opt/**` read-only                                   |
| Executables        | `/bin/bash`, `/bin/sh`, `/usr/bin/git`, `/usr/bin/curl`, `/usr/bin/jq`, `/usr/bin/rg`, `/usr/local/bin/node`, `/usr/local/bin/claude` -- read, inherit, execute |
| Proc filesystem    | Limited read access: `status`, `fd`, `maps`, `stat`, `version`, `meminfo`, `cpuinfo`, `uptime`, `loadavg` |
| Device nodes       | `/dev/null`, `/dev/zero`, `/dev/urandom`, `/dev/random`, `/dev/tty`, `/dev/pts/**`, `/dev/shm/**` |

### Loading the Profile

```bash
# Via Makefile
make load-apparmor

# Manually
sudo apparmor_parser -r -W /path/to/claude-cage/security/apparmor-profile
```

**Applied via:**

```bash
--security-opt apparmor=claude-cage
```

Note: The launch script and `lib/sandbox.sh` check whether the AppArmor profile is loaded before applying it. If the profile is not loaded, the container runs without AppArmor confinement (the other 7 layers remain active).

---

## Layer 5: Resource Limits

Resource limits prevent a compromised container from exhausting host resources (CPU, memory, process table, file descriptors) as a denial-of-service attack vector.

### Standard Mode (CLI and Desktop)

| Resource         | Limit          | Docker Flag                     |
|------------------|----------------|---------------------------------|
| CPU cores        | 2              | `--cpus 2`                      |
| Memory           | 4 GB           | `--memory 4g`                   |
| Process IDs      | 512            | `--pids-limit 512`              |
| Open files (soft)| 1,024          | `--ulimit nofile=1024:2048`     |
| Open files (hard)| 2,048          | `--ulimit nofile=1024:2048`     |
| Processes (soft) | 256            | `--ulimit nproc=256:512`        |
| Processes (hard) | 512            | `--ulimit nproc=256:512`        |

### Isolated Mode (No Network)

| Resource         | Limit          | Docker Flag                     |
|------------------|----------------|---------------------------------|
| CPU cores        | 1              | `--cpus 1`                      |
| Memory           | 2 GB           | `--memory 2g`                   |
| Process IDs      | 256            | `--pids-limit 256`              |
| Open files (soft)| 1,024          | `--ulimit nofile=1024:2048`     |
| Open files (hard)| 2,048          | `--ulimit nofile=1024:2048`     |
| Processes (soft) | 256            | `--ulimit nproc=256:512`        |
| Processes (hard) | 512            | `--ulimit nproc=256:512`        |

The isolated mode receives tighter CPU and memory limits because it has no network access and is intended for offline analysis tasks only.

---

## Layer 6: Network Filtering

### Filtered Mode (Default)

Containers on the `cage-filtered` bridge network have outbound traffic restricted to a whitelist of allowed hosts. The filtering is applied post-launch via iptables rules injected by `sandbox_apply_network_filter()` in `lib/sandbox.sh`.

**Process:**

1. Container launches on the `cage-filtered` bridge network (`172.28.0.0/16`)
2. DNS is set to `1.1.1.1` (Cloudflare)
3. `sandbox_apply_network_filter()` is called with the container name
4. The container's IP address is obtained via `docker inspect`
5. Each host in `allowed_hosts` is resolved to IP addresses via `getent hosts`
6. iptables `DOCKER-USER` chain rules are inserted:
   - `ACCEPT` rules for each resolved IP of each allowed host
   - `ACCEPT` rules for DNS traffic (UDP and TCP port 53)
   - A final `DROP` rule for all other traffic from the container

**Default allowed hosts:**

| Host                    | Purpose                          |
|-------------------------|----------------------------------|
| `api.anthropic.com`     | Claude API endpoint              |
| `cdn.anthropic.com`     | Claude CDN (model downloads)     |

**iptables rules applied (per container):**

```bash
# Allow traffic to resolved IPs of allowed hosts
iptables -I DOCKER-USER -s <container_ip> -d <allowed_ip> -j ACCEPT

# Allow DNS resolution
iptables -I DOCKER-USER -s <container_ip> -p udp --dport 53 -j ACCEPT
iptables -I DOCKER-USER -s <container_ip> -p tcp --dport 53 -j ACCEPT

# Drop everything else
iptables -A DOCKER-USER -s <container_ip> -j DROP
```

### Isolated Mode (No Network)

The `cli-isolated` service uses `network_mode: none`, which provides zero network connectivity. No network interfaces are created other than the loopback adapter. No iptables rules are needed because no network stack exists.

```yaml
# docker-compose.yml
cli-isolated:
  network_mode: none
```

```bash
# launch script
flags+=(--network none)
```

---

## Layer 7: No-New-Privileges

The `no-new-privileges` security option prevents processes inside the container from gaining additional privileges through any mechanism, including:

- **Setuid binaries:** Executables with the setuid bit set will not gain the file owner's privileges when executed
- **Capability inheritance:** Child processes cannot inherit capabilities not held by the parent
- **Setuid/setgid transitions:** The `execve()` syscall will not honor setuid/setgid bits

This is a kernel-level enforcement that cannot be bypassed from within the container.

**Applied via:**

```bash
--security-opt no-new-privileges
```

```yaml
# docker-compose.yml (x-common anchor)
security_opt:
  - no-new-privileges
```

---

## Layer 8: Bridge Network Isolation

The `cage-filtered` bridge network is configured with inter-container communication (ICC) disabled. This means containers on the same bridge network cannot communicate with each other, even if they know each other's IP addresses.

**Network configuration:**

| Parameter                                           | Value           | Purpose                                        |
|-----------------------------------------------------|-----------------|-------------------------------------------------|
| `driver`                                            | `bridge`        | Standard Docker bridge networking               |
| `subnet`                                            | `172.28.0.0/16` | Dedicated subnet for cage containers            |
| `com.docker.network.bridge.enable_icc`              | `false`         | Disables inter-container communication          |
| `com.docker.network.bridge.enable_ip_masquerade`    | `true`          | Enables outbound NAT for filtered internet access |

```yaml
# docker-compose.yml
networks:
  cage-net:
    name: cage-filtered
    driver: bridge
    driver_opts:
      com.docker.network.bridge.enable_icc: "false"
      com.docker.network.bridge.enable_ip_masquerade: "true"
    ipam:
      config:
        - subnet: 172.28.0.0/16
```

```bash
# launch script (ensure_network) and lib/sandbox.sh (sandbox_create_network)
docker network create \
    --driver bridge \
    --opt com.docker.network.bridge.enable_icc=false \
    --opt com.docker.network.bridge.enable_ip_masquerade=true \
    --subnet 172.28.0.0/16 \
    cage-filtered
```

Each session is isolated from every other session. Even if two CLI containers are running simultaneously, they cannot see or communicate with each other.

---

## Verification

`make verify-sandbox` runs the `sandbox_verify()` function from `lib/sandbox.sh` against a running container. It inspects the container via `docker inspect` and reports pass/fail for each check.

### Checks Performed

| Check                      | Inspection Method                                      | Pass Criteria         |
|----------------------------|--------------------------------------------------------|-----------------------|
| Read-only root filesystem  | `docker inspect -f '{{.HostConfig.ReadonlyRootfs}}'`  | `true`                |
| Capabilities dropped       | `docker inspect -f '{{.HostConfig.CapDrop}}'`         | Contains `ALL`        |
| No-new-privileges          | `docker inspect -f '{{.HostConfig.SecurityOpt}}'`     | Contains `no-new-privileges` |
| Memory limit               | `docker inspect -f '{{.HostConfig.Memory}}'`          | Non-zero value        |

### Example Output

```
==> Verifying sandbox for: cage-swift-fox-a1b2
  [PASS] Read-only root filesystem
  [PASS] All capabilities dropped
  [PASS] no-new-privileges set
  [PASS] Memory limit set: 4294967296 bytes
  Sandbox verification: ALL CHECKS PASSED
```

---

## Implementation Locations

Security is configured in three independent locations that must be kept in sync. Changes to security policy require updating all three.

| Location                     | Function/Section              | Layers Implemented                                               |
|------------------------------|-------------------------------|------------------------------------------------------------------|
| `docker-compose.yml`         | `x-common` anchor             | Layers 1-5, 7 (read-only, caps, seccomp, ulimits, no-new-privs) |
| `docker-compose.yml`         | `networks.cage-net`           | Layer 8 (bridge isolation with ICC disabled)                     |
| `docker-compose.yml`         | `cli-isolated` service        | Layer 6 (network_mode: none)                                     |
| `launch`                     | `build_security_flags()`      | Layers 1-5, 7 (all docker run security flags)                    |
| `launch`                     | `ensure_network()`            | Layer 8 (creates cage-filtered bridge)                           |
| `lib/sandbox.sh`             | `sandbox_build_flags()`       | Layers 1-5, 7 (all docker run security flags)                    |
| `lib/sandbox.sh`             | `sandbox_create_network()`    | Layer 8 (creates cage-filtered bridge)                           |
| `lib/sandbox.sh`             | `sandbox_apply_network_filter()` | Layer 6 (post-launch iptables rules)                          |
| `lib/sandbox.sh`             | `sandbox_verify()`            | Verification of layers 1, 2, 5, 7                               |
| `security/seccomp-default.json` | Entire file                | Layer 3 (syscall allowlist)                                      |
| `security/apparmor-profile`  | Entire file                   | Layer 4 (MAC confinement)                                        |

---

## Threat Model

The 8 layers collectively defend against the following threat categories:

| Threat                              | Mitigating Layers                    | Description                                                                                 |
|--------------------------------------|--------------------------------------|---------------------------------------------------------------------------------------------|
| Container escape                     | 2, 3, 4, 7                          | Dropped capabilities, restricted syscalls, MAC confinement, and no-new-privileges prevent breakout to the host |
| Privilege escalation                 | 2, 3, 4, 7                          | No setuid escalation, no capability inheritance, kernel operations blocked by seccomp and AppArmor           |
| Resource exhaustion (DoS)            | 5                                    | CPU, memory, PID, file descriptor, and process limits prevent host resource starvation                       |
| Network exfiltration                 | 6, 8                                | Outbound traffic restricted to Anthropic API only; isolated mode has zero network connectivity               |
| Inter-container lateral movement     | 8                                    | ICC disabled on bridge network; containers cannot reach each other                                           |
| Persistent filesystem compromise     | 1                                    | Read-only root prevents persistent modification; tmpfs is volatile and noexec                                |
| Raw socket attacks (sniffing/spoofing) | 2, 3, 4                           | NET_RAW capability dropped, raw/packet network denied by AppArmor, AF_VSOCK blocked by seccomp              |
| Kernel manipulation                  | 3, 4                                | Module loading denied by AppArmor, mount/reboot/kexec blocked by seccomp                                    |
| Process debugging/injection          | 4, 7                                | ptrace denied by AppArmor, no-new-privileges prevents capability gain                                       |
| VM escape via VSOCK                  | 3                                    | AF_VSOCK (address family 40) explicitly blocked in seccomp profile                                          |

### What This Does NOT Protect Against

- **Kernel vulnerabilities:** A kernel exploit could bypass all userspace protections. Keep the host kernel updated.
- **Docker daemon compromise:** If the Docker daemon itself is compromised, container isolation is meaningless.
- **Mounted volume exposure:** The workspace directory is mounted read-write. Sensitive data in the workspace is accessible to the container.
- **API key exposure:** The `ANTHROPIC_API_KEY` is passed as an environment variable and is visible inside the container.
- **Supply chain attacks:** If the base images (`node:20-slim`, `ubuntu:24.04`) are compromised, the container contents are compromised. Pin image digests for production use.
