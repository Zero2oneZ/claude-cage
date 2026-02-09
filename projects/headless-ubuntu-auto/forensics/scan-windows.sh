#!/usr/bin/env bash
# =============================================================================
# scan-windows.sh — Comprehensive Windows Forensic Scanner
# =============================================================================
# Runs from a Ubuntu live USB environment to forensically scan a mounted
# Windows NTFS partition for Docker artifacts, malware indicators, encryption
# evidence, and registry analysis.
#
# Usage:
#   sudo ./scan-windows.sh [REPORT_DIR] [MOUNT_POINT]
#
# Arguments:
#   REPORT_DIR   — Directory to save findings (default: /tmp/forensic-report)
#   MOUNT_POINT  — Where the Windows C: drive is mounted (default: /mnt/windows)
#
# Requirements:
#   - Must run as root
#   - Ubuntu live USB environment (or any Debian-based live system)
#   - NTFS partition accessible
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration & Defaults
# ---------------------------------------------------------------------------
REPORT_DIR="${1:-/tmp/forensic-report}"
MOUNT_POINT="${2:-/mnt/windows}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATTERNS_DIR="${SCRIPT_DIR}/patterns"
LOG_FILE="${REPORT_DIR}/scan.log"
SCAN_TIMESTAMP="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
TIMELINE_DAYS=90

# Pattern files
DOCKER_PATTERNS="${PATTERNS_DIR}/docker-artifacts.txt"
MALWARE_PATTERNS="${PATTERNS_DIR}/malware-signatures.txt"
ENCRYPTION_PATTERNS="${PATTERNS_DIR}/encryption-indicators.txt"

# Output files
DOCKER_JSON="${REPORT_DIR}/docker-findings.json"
MALWARE_JSON="${REPORT_DIR}/malware-findings.json"
ENCRYPTION_JSON="${REPORT_DIR}/encryption-findings.json"
REPORT_TXT="${REPORT_DIR}/report.txt"
HASHES_FILE="${REPORT_DIR}/file-hashes.txt"
TIMELINE_FILE="${REPORT_DIR}/timeline.txt"
SOFTWARE_FILE="${REPORT_DIR}/installed-software.txt"
NETWORK_FILE="${REPORT_DIR}/network-config.txt"

# ---------------------------------------------------------------------------
# Color Output Helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

log() {
    local level="$1"
    shift
    local msg="$*"
    local ts
    ts="$(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo "[${ts}] [${level}] ${msg}" >> "${LOG_FILE}"
    case "${level}" in
        INFO)    echo -e "${GREEN}[*]${NC} ${msg}" ;;
        WARN)    echo -e "${YELLOW}[!]${NC} ${msg}" ;;
        ERROR)   echo -e "${RED}[X]${NC} ${msg}" ;;
        SECTION) echo -e "\n${BOLD}${BLUE}=== ${msg} ===${NC}" ;;
        PROGRESS) echo -e "${CYAN}  -> ${msg}${NC}" ;;
    esac
}

# ---------------------------------------------------------------------------
# Pre-flight Checks
# ---------------------------------------------------------------------------
preflight() {
    log SECTION "Pre-flight Checks"

    # Must be root
    if [[ $EUID -ne 0 ]]; then
        log ERROR "This script must be run as root (use sudo)."
        exit 1
    fi

    # Verify pattern files exist
    for pf in "${DOCKER_PATTERNS}" "${MALWARE_PATTERNS}" "${ENCRYPTION_PATTERNS}"; do
        if [[ ! -f "${pf}" ]]; then
            log ERROR "Pattern file not found: ${pf}"
            exit 1
        fi
    done

    # Create report directory
    mkdir -p "${REPORT_DIR}"
    : > "${LOG_FILE}"
    log INFO "Report directory: ${REPORT_DIR}"
    log INFO "Log file: ${LOG_FILE}"
    log INFO "Scan started at ${SCAN_TIMESTAMP}"

    # Install required tools if missing
    install_dependencies
}

install_dependencies() {
    log PROGRESS "Checking required tools..."
    local packages_needed=()

    command -v ntfs-3g    &>/dev/null || packages_needed+=(ntfs-3g)
    command -v hivexregedit &>/dev/null || packages_needed+=(libhivex-bin)
    command -v jq         &>/dev/null || packages_needed+=(jq)
    command -v file       &>/dev/null || packages_needed+=(file)
    command -v sha256sum  &>/dev/null || true  # coreutils, almost always present

    if [[ ${#packages_needed[@]} -gt 0 ]]; then
        log INFO "Installing missing packages: ${packages_needed[*]}"
        apt-get update -qq >> "${LOG_FILE}" 2>&1 || true
        apt-get install -y -qq "${packages_needed[@]}" >> "${LOG_FILE}" 2>&1 || {
            log WARN "Some packages could not be installed. Continuing with available tools."
        }
    else
        log INFO "All required tools are available."
    fi
}

# ---------------------------------------------------------------------------
# NTFS Partition Detection & Mounting
# ---------------------------------------------------------------------------
detect_and_mount() {
    log SECTION "NTFS Partition Detection & Mounting"

    # If already mounted at the mount point, use it
    if mountpoint -q "${MOUNT_POINT}" 2>/dev/null; then
        log INFO "Partition already mounted at ${MOUNT_POINT}"
        # Verify it looks like a Windows C: drive
        if [[ -d "${MOUNT_POINT}/Windows" ]]; then
            log INFO "Confirmed: Windows directory found at ${MOUNT_POINT}/Windows"
            return 0
        else
            log WARN "Mount point exists but no Windows/ directory found. Proceeding anyway."
            return 0
        fi
    fi

    # Auto-detect NTFS partitions
    log PROGRESS "Scanning for NTFS partitions..."
    local ntfs_parts=()
    local best_part=""
    local best_size=0

    while IFS= read -r line; do
        local dev size
        dev="$(echo "${line}" | awk '{print $1}')"
        size="$(echo "${line}" | awk '{print $2}')"
        ntfs_parts+=("${dev}:${size}")
        log PROGRESS "  Found NTFS partition: ${dev} (${size} bytes)"
    done < <(lsblk -rbnp -o NAME,SIZE,FSTYPE 2>/dev/null | awk '$3 == "ntfs" {print $1, $2}')

    if [[ ${#ntfs_parts[@]} -eq 0 ]]; then
        log ERROR "No NTFS partitions found. Please mount the Windows C: drive manually at ${MOUNT_POINT}"
        exit 1
    fi

    # Pick the largest NTFS partition, or the one containing Windows/
    for entry in "${ntfs_parts[@]}"; do
        local dev="${entry%%:*}"
        local size="${entry##*:}"

        # Try a quick temporary mount to check for Windows/
        local tmp_mnt
        tmp_mnt="$(mktemp -d)"
        if mount -t ntfs3 -o ro,noexec,nosuid,nodev "${dev}" "${tmp_mnt}" 2>/dev/null || \
           mount -t ntfs-3g -o ro,noexec,nosuid,nodev "${dev}" "${tmp_mnt}" 2>/dev/null; then
            if [[ -d "${tmp_mnt}/Windows" ]]; then
                log INFO "Found Windows directory on ${dev} — selecting as C: drive"
                umount "${tmp_mnt}" 2>/dev/null || true
                rmdir "${tmp_mnt}" 2>/dev/null || true
                best_part="${dev}"
                break
            fi
            umount "${tmp_mnt}" 2>/dev/null || true
        fi
        rmdir "${tmp_mnt}" 2>/dev/null || true

        # Fallback: use the largest partition
        if [[ "${size}" -gt "${best_size}" ]]; then
            best_size="${size}"
            best_part="${dev}"
        fi
    done

    if [[ -z "${best_part}" ]]; then
        log ERROR "Could not determine the Windows C: partition."
        exit 1
    fi

    # Mount the selected partition read-only
    log INFO "Mounting ${best_part} at ${MOUNT_POINT} (read-only)"
    mkdir -p "${MOUNT_POINT}"

    if mount -t ntfs3 -o ro,noexec,nosuid,nodev "${best_part}" "${MOUNT_POINT}" 2>>"${LOG_FILE}"; then
        log INFO "Mounted using ntfs3 kernel driver."
    elif mount -t ntfs-3g -o ro,noexec,nosuid,nodev "${best_part}" "${MOUNT_POINT}" 2>>"${LOG_FILE}"; then
        log INFO "Mounted using ntfs-3g FUSE driver."
    else
        log ERROR "Failed to mount ${best_part}. Check log for details."
        exit 1
    fi

    log INFO "Windows C: drive mounted at ${MOUNT_POINT}"
}

# ---------------------------------------------------------------------------
# Pattern File Reader
# ---------------------------------------------------------------------------
# Reads a pattern file, strips comments and blank lines, converts Windows
# paths to Linux paths relative to MOUNT_POINT.
read_patterns() {
    local pattern_file="$1"
    local patterns=()

    while IFS= read -r line; do
        # Skip comments and blank lines
        [[ -z "${line}" || "${line}" =~ ^[[:space:]]*# ]] && continue
        # Skip content-scan and special directives
        [[ "${line}" =~ ^(CONTENT_SCAN|ADS_SCAN|ENTROPY_SCAN|BDE_SCAN): ]] && continue

        # Convert Windows path to Linux path under mount point
        # Replace backslash with forward slash
        local linux_path="${line//\\//}"
        # Remove the drive letter prefix (C:)
        linux_path="${linux_path#C:}"
        linux_path="${linux_path#c:}"
        # Prepend mount point
        linux_path="${MOUNT_POINT}${linux_path}"

        patterns+=("${linux_path}")
    done < "${pattern_file}"

    printf '%s\n' "${patterns[@]}"
}

# Read content-scan directives from a pattern file
read_content_scan_directives() {
    local pattern_file="$1"
    while IFS= read -r line; do
        [[ -z "${line}" || "${line}" =~ ^[[:space:]]*# ]] && continue
        if [[ "${line}" =~ ^CONTENT_SCAN: ]]; then
            echo "${line#CONTENT_SCAN:}"
        fi
    done < "${pattern_file}"
}

# ---------------------------------------------------------------------------
# Scanner: Docker Artifacts
# ---------------------------------------------------------------------------
scan_docker_artifacts() {
    log SECTION "Docker Artifact Scan"
    local findings=()
    local count=0

    while IFS= read -r pattern; do
        log PROGRESS "Searching: ${pattern}"
        # Use find with glob expansion; handle wildcards
        # Convert glob pattern for find
        local search_dir
        local search_name

        if [[ "${pattern}" == *"**"* ]]; then
            # Recursive search: split at **
            search_dir="${pattern%%\*\**}"
            search_name="${pattern##*\*\*/}"
            # Expand the base directory wildcard if present
            local expanded_dirs=()
            if compgen -G "${search_dir}" > /dev/null 2>&1; then
                while IFS= read -r d; do
                    expanded_dirs+=("${d}")
                done < <(compgen -G "${search_dir}")
            fi
            for edir in "${expanded_dirs[@]}"; do
                [[ -d "${edir}" ]] || continue
                while IFS= read -r f; do
                    findings+=("{\"path\":\"${f}\",\"type\":\"docker-artifact\",\"size\":$(stat -c%s "${f}" 2>/dev/null || echo 0),\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                done < <(find "${edir}" -maxdepth 10 -name "${search_name}" -type f 2>/dev/null || true)
            done
        else
            # Direct glob expansion
            while IFS= read -r f; do
                if [[ -e "${f}" ]]; then
                    local ftype="file"
                    [[ -d "${f}" ]] && ftype="directory"
                    local fsize=0
                    [[ -f "${f}" ]] && fsize=$(stat -c%s "${f}" 2>/dev/null || echo 0)
                    findings+=("{\"path\":\"${f}\",\"type\":\"${ftype}\",\"size\":${fsize},\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                fi
            done < <(compgen -G "${pattern}" 2>/dev/null || true)
        fi
    done < <(read_patterns "${DOCKER_PATTERNS}")

    # Write JSON output
    {
        echo "{"
        echo "  \"scan_type\": \"docker-artifacts\","
        echo "  \"scan_timestamp\": \"${SCAN_TIMESTAMP}\","
        echo "  \"total_findings\": ${count},"
        echo "  \"mount_point\": \"${MOUNT_POINT}\","
        echo "  \"findings\": ["
        local first=true
        for f in "${findings[@]}"; do
            if [[ "${first}" == "true" ]]; then
                echo "    ${f}"
                first=false
            else
                echo "    ,${f}"
            fi
        done
        echo "  ]"
        echo "}"
    } > "${DOCKER_JSON}"

    log INFO "Docker artifact scan complete: ${count} findings"
    echo "${count}"
}

# ---------------------------------------------------------------------------
# Scanner: Malware Signatures
# ---------------------------------------------------------------------------
scan_malware() {
    log SECTION "Malware Signature Scan"
    local findings=()
    local count=0

    # Path-based scanning
    while IFS= read -r pattern; do
        log PROGRESS "Searching: ${pattern}"

        if [[ "${pattern}" == *"**"* ]]; then
            local search_dir="${pattern%%\*\**}"
            local search_name="${pattern##*\*\*/}"
            local expanded_dirs=()
            if compgen -G "${search_dir}" > /dev/null 2>&1; then
                while IFS= read -r d; do
                    expanded_dirs+=("${d}")
                done < <(compgen -G "${search_dir}")
            fi
            for edir in "${expanded_dirs[@]}"; do
                [[ -d "${edir}" ]] || continue
                while IFS= read -r f; do
                    local severity="medium"
                    # Classify severity
                    [[ "${f}" == *"svchost.exe" ]] && severity="critical"
                    [[ "${f}" == *".pdf.exe" || "${f}" == *".doc.exe" || "${f}" == *".jpg.exe" ]] && severity="high"
                    [[ "${f}" == *"Temp/"* && "${f}" == *".exe" ]] && severity="high"
                    [[ "${f}" == *"Startup/"* ]] && severity="high"
                    local fsize=0
                    [[ -f "${f}" ]] && fsize=$(stat -c%s "${f}" 2>/dev/null || echo 0)
                    findings+=("{\"path\":\"${f}\",\"severity\":\"${severity}\",\"category\":\"suspicious-file\",\"size\":${fsize},\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                done < <(find "${edir}" -maxdepth 10 -name "${search_name}" -type f 2>/dev/null || true)
            done
        else
            while IFS= read -r f; do
                if [[ -e "${f}" ]]; then
                    local severity="medium"
                    [[ "${f}" == *"svchost.exe" && "${f}" != *"System32/svchost.exe" ]] && severity="critical"
                    [[ "${f}" == *"Startup/"* ]] && severity="high"
                    [[ "${f}" == *"Tasks/"* ]] && severity="medium"
                    [[ "${f}" == *"NTUSER.DAT" ]] && severity="info"
                    [[ "${f}" == *"Recycle.Bin"* && ( "${f}" == *".exe" || "${f}" == *".dll" ) ]] && severity="high"
                    local fsize=0
                    [[ -f "${f}" ]] && fsize=$(stat -c%s "${f}" 2>/dev/null || echo 0)
                    findings+=("{\"path\":\"${f}\",\"severity\":\"${severity}\",\"category\":\"pattern-match\",\"size\":${fsize},\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                fi
            done < <(compgen -G "${pattern}" 2>/dev/null || true)
        fi
    done < <(read_patterns "${MALWARE_PATTERNS}")

    # Content-based scanning for encoded PowerShell and suspicious script content
    log PROGRESS "Running content-based scans for suspicious patterns..."
    local content_count=0
    while IFS= read -r directive; do
        # directive format: glob_pattern:search_term
        local glob_part="${directive%%:*}"
        local search_term="${directive#*:}"
        local search_base="${MOUNT_POINT}/Users"

        if [[ -d "${search_base}" ]]; then
            while IFS= read -r match_file; do
                findings+=("{\"path\":\"${match_file}\",\"severity\":\"high\",\"category\":\"content-match\",\"indicator\":\"${search_term}\",\"size\":$(stat -c%s "${match_file}" 2>/dev/null || echo 0),\"mtime\":\"$(stat -c%Y "${match_file}" 2>/dev/null || echo 0)\"}")
                ((count++)) || true
                ((content_count++)) || true
            done < <(find "${search_base}" -maxdepth 8 -name "${glob_part}" -type f -exec grep -l "${search_term}" {} \; 2>/dev/null || true)
        fi
    done < <(read_content_scan_directives "${MALWARE_PATTERNS}")
    log PROGRESS "Content-based scan found ${content_count} matches."

    # Write JSON output
    {
        echo "{"
        echo "  \"scan_type\": \"malware-signatures\","
        echo "  \"scan_timestamp\": \"${SCAN_TIMESTAMP}\","
        echo "  \"total_findings\": ${count},"
        echo "  \"mount_point\": \"${MOUNT_POINT}\","
        echo "  \"severity_summary\": {"
        local crit=0 high=0 med=0 low=0 info=0
        for f in "${findings[@]}"; do
            case "${f}" in
                *'"critical"'*) ((crit++)) || true ;;
                *'"high"'*)     ((high++)) || true ;;
                *'"medium"'*)   ((med++)) || true ;;
                *'"low"'*)      ((low++)) || true ;;
                *'"info"'*)     ((info++)) || true ;;
            esac
        done
        echo "    \"critical\": ${crit},"
        echo "    \"high\": ${high},"
        echo "    \"medium\": ${med},"
        echo "    \"low\": ${low},"
        echo "    \"info\": ${info}"
        echo "  },"
        echo "  \"findings\": ["
        local first=true
        for f in "${findings[@]}"; do
            if [[ "${first}" == "true" ]]; then
                echo "    ${f}"
                first=false
            else
                echo "    ,${f}"
            fi
        done
        echo "  ]"
        echo "}"
    } > "${MALWARE_JSON}"

    log INFO "Malware scan complete: ${count} findings (critical=${crit}, high=${high}, medium=${med}, low=${low}, info=${info})"
    echo "${count}"
}

# ---------------------------------------------------------------------------
# Scanner: Encryption Indicators
# ---------------------------------------------------------------------------
scan_encryption() {
    log SECTION "Encryption Indicator Scan"
    local findings=()
    local count=0

    # Scan for ransomware file extensions
    log PROGRESS "Scanning for ransomware file extensions..."
    while IFS= read -r line; do
        [[ -z "${line}" || "${line}" =~ ^[[:space:]]*# ]] && continue
        [[ "${line}" =~ ^(CONTENT_SCAN|ADS_SCAN|ENTROPY_SCAN|BDE_SCAN): ]] && continue

        # Handle extension patterns (*.ext) — search broadly
        if [[ "${line}" == \*.* && "${line}" != *\\* && "${line}" != */* ]]; then
            local ext_pattern="${line}"
            log PROGRESS "  Extension: ${ext_pattern}"
            # Search user directories for ransomware extensions
            local user_dirs=("${MOUNT_POINT}/Users")
            for udir in "${user_dirs[@]}"; do
                [[ -d "${udir}" ]] || continue
                while IFS= read -r f; do
                    local cat="ransomware-extension"
                    [[ "${f}" == *"README"* || "${f}" == *"DECRYPT"* || "${f}" == *"RECOVER"* || "${f}" == *"HELP"* || "${f}" == *"RANSOM"* ]] && cat="ransom-note"
                    findings+=("{\"path\":\"${f}\",\"category\":\"${cat}\",\"indicator\":\"${ext_pattern}\",\"size\":$(stat -c%s "${f}" 2>/dev/null || echo 0),\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                done < <(find "${udir}" -maxdepth 6 -name "${ext_pattern}" -type f 2>/dev/null | head -500 || true)
            done
            continue
        fi

        # Handle ransom note filename patterns (contain path-like characters)
        if [[ "${line}" == *"*"* && "${line}" != *\\* && "${line}" != */* ]]; then
            local note_pattern="${line}"
            log PROGRESS "  Ransom note: ${note_pattern}"
            local user_dirs=("${MOUNT_POINT}/Users")
            for udir in "${user_dirs[@]}"; do
                [[ -d "${udir}" ]] || continue
                while IFS= read -r f; do
                    findings+=("{\"path\":\"${f}\",\"category\":\"ransom-note\",\"indicator\":\"${note_pattern}\",\"size\":$(stat -c%s "${f}" 2>/dev/null || echo 0),\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                    ((count++)) || true
                done < <(find "${udir}" -maxdepth 6 -name "${note_pattern}" -type f 2>/dev/null | head -100 || true)
            done
            continue
        fi

        # Handle full path patterns (Windows paths)
        local linux_path="${line//\\//}"
        linux_path="${linux_path#C:}"
        linux_path="${linux_path#c:}"
        linux_path="${MOUNT_POINT}${linux_path}"

        while IFS= read -r f; do
            if [[ -e "${f}" ]]; then
                local cat="encryption-artifact"
                [[ "${f}" == *"BitLocker"* ]] && cat="bitlocker"
                [[ "${f}" == *"Crypto"* || "${f}" == *"Protect"* ]] && cat="efs"
                [[ "${f}" == *"Certificate"* || "${f}" == *".pfx" || "${f}" == *".p12" || "${f}" == *".pem" || "${f}" == *".key" ]] && cat="certificate"
                [[ "${f}" == *"VeraCrypt"* || "${f}" == *".hc" || "${f}" == *".tc" ]] && cat="veracrypt"
                [[ "${f}" == *"gnupg"* || "${f}" == *".gpg" || "${f}" == *".pgp" || "${f}" == *".asc" ]] && cat="gpg"
                local fsize=0
                [[ -f "${f}" ]] && fsize=$(stat -c%s "${f}" 2>/dev/null || echo 0)
                findings+=("{\"path\":\"${f}\",\"category\":\"${cat}\",\"indicator\":\"${line}\",\"size\":${fsize},\"mtime\":\"$(stat -c%Y "${f}" 2>/dev/null || echo 0)\"}")
                ((count++)) || true
            fi
        done < <(compgen -G "${linux_path}" 2>/dev/null || true)
    done < "${ENCRYPTION_PATTERNS}"

    # Write JSON output
    {
        echo "{"
        echo "  \"scan_type\": \"encryption-indicators\","
        echo "  \"scan_timestamp\": \"${SCAN_TIMESTAMP}\","
        echo "  \"total_findings\": ${count},"
        echo "  \"mount_point\": \"${MOUNT_POINT}\","
        echo "  \"findings\": ["
        local first=true
        for f in "${findings[@]}"; do
            if [[ "${first}" == "true" ]]; then
                echo "    ${f}"
                first=false
            else
                echo "    ,${f}"
            fi
        done
        echo "  ]"
        echo "}"
    } > "${ENCRYPTION_JSON}"

    log INFO "Encryption indicator scan complete: ${count} findings"
    echo "${count}"
}

# ---------------------------------------------------------------------------
# Registry Analysis
# ---------------------------------------------------------------------------
analyze_registry() {
    log SECTION "Windows Registry Analysis"

    local reg_dir="${MOUNT_POINT}/Windows/System32/config"
    local software_hive="${reg_dir}/SOFTWARE"
    local system_hive="${reg_dir}/SYSTEM"
    local sam_hive="${reg_dir}/SAM"

    # -------------------------------------------
    # Installed Software from Registry
    # -------------------------------------------
    log PROGRESS "Extracting installed software list..."
    {
        echo "============================================================"
        echo "Installed Software — Extracted from Windows Registry"
        echo "Scan timestamp: ${SCAN_TIMESTAMP}"
        echo "============================================================"
        echo ""
    } > "${SOFTWARE_FILE}"

    if [[ -f "${software_hive}" ]]; then
        # Extract Uninstall keys (64-bit)
        echo "--- 64-bit Software (Uninstall) ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${software_hive}" \
            'Microsoft\Windows\CurrentVersion\Uninstall' 2>/dev/null \
            | grep -E '("DisplayName"|"DisplayVersion"|"Publisher"|"InstallDate")' \
            | sed 's/^[[:space:]]*//' >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract 64-bit uninstall keys)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"

        # Extract Uninstall keys (32-bit on 64-bit)
        echo "--- 32-bit Software (WoW6432Node\\Uninstall) ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${software_hive}" \
            'WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall' 2>/dev/null \
            | grep -E '("DisplayName"|"DisplayVersion"|"Publisher"|"InstallDate")' \
            | sed 's/^[[:space:]]*//' >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract 32-bit uninstall keys)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"

        # Extract Run keys (persistence)
        log PROGRESS "Extracting Run keys (persistence)..."
        echo "--- Run Keys (Machine-level persistence) ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${software_hive}" \
            'Microsoft\Windows\CurrentVersion\Run' 2>>"${LOG_FILE}" \
            >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract Run keys)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"

        echo "--- RunOnce Keys ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${software_hive}" \
            'Microsoft\Windows\CurrentVersion\RunOnce' 2>>"${LOG_FILE}" \
            >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract RunOnce keys)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"

        # Extract registered services from SOFTWARE hive
        echo "--- Registered Services (SOFTWARE hive) ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${software_hive}" \
            'Microsoft\Windows\CurrentVersion\RunServices' 2>>"${LOG_FILE}" \
            >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(No RunServices key found — normal for modern Windows)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"
    else
        echo "(SOFTWARE hive not found at ${software_hive})" >> "${SOFTWARE_FILE}"
        log WARN "SOFTWARE registry hive not found."
    fi

    # -------------------------------------------
    # Service Configurations from SYSTEM hive
    # -------------------------------------------
    if [[ -f "${system_hive}" ]]; then
        log PROGRESS "Extracting service configurations..."
        echo "--- Services (SYSTEM hive — ControlSet001) ---" >> "${SOFTWARE_FILE}"
        hivexregedit --export "${system_hive}" \
            'ControlSet001\Services' 2>>"${LOG_FILE}" \
            | grep -E '^\[|"DisplayName"|"ImagePath"|"Start"|"Type"' \
            | head -2000 >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract service entries)" >> "${SOFTWARE_FILE}"
            }
        echo "" >> "${SOFTWARE_FILE}"
    else
        log WARN "SYSTEM registry hive not found."
    fi

    # -------------------------------------------
    # Network Configuration from SYSTEM hive
    # -------------------------------------------
    log PROGRESS "Extracting network configuration..."
    {
        echo "============================================================"
        echo "Network Configuration — Extracted from Windows Registry"
        echo "Scan timestamp: ${SCAN_TIMESTAMP}"
        echo "============================================================"
        echo ""
    } > "${NETWORK_FILE}"

    if [[ -f "${system_hive}" ]]; then
        echo "--- TCP/IP Parameters ---" >> "${NETWORK_FILE}"
        hivexregedit --export "${system_hive}" \
            'ControlSet001\Services\Tcpip\Parameters' 2>>"${LOG_FILE}" \
            >> "${NETWORK_FILE}" 2>/dev/null || {
                echo "(Could not extract TCP/IP parameters)" >> "${NETWORK_FILE}"
            }
        echo "" >> "${NETWORK_FILE}"

        echo "--- Network Interface Configurations ---" >> "${NETWORK_FILE}"
        hivexregedit --export "${system_hive}" \
            'ControlSet001\Services\Tcpip\Parameters\Interfaces' 2>>"${LOG_FILE}" \
            | head -1000 >> "${NETWORK_FILE}" 2>/dev/null || {
                echo "(Could not extract interface configurations)" >> "${NETWORK_FILE}"
            }
        echo "" >> "${NETWORK_FILE}"

        echo "--- DNS Client Settings ---" >> "${NETWORK_FILE}"
        hivexregedit --export "${system_hive}" \
            'ControlSet001\Services\Dnscache\Parameters' 2>>"${LOG_FILE}" \
            >> "${NETWORK_FILE}" 2>/dev/null || {
                echo "(Could not extract DNS settings)" >> "${NETWORK_FILE}"
            }
        echo "" >> "${NETWORK_FILE}"

        echo "--- Network Profiles ---" >> "${NETWORK_FILE}"
        if [[ -f "${software_hive}" ]]; then
            hivexregedit --export "${software_hive}" \
                'Microsoft\Windows NT\CurrentVersion\NetworkList\Profiles' 2>>"${LOG_FILE}" \
                | head -500 >> "${NETWORK_FILE}" 2>/dev/null || {
                    echo "(Could not extract network profiles)" >> "${NETWORK_FILE}"
                }
        fi
        echo "" >> "${NETWORK_FILE}"

        echo "--- Firewall Rules (summary) ---" >> "${NETWORK_FILE}"
        if [[ -f "${system_hive}" ]]; then
            hivexregedit --export "${system_hive}" \
                'ControlSet001\Services\SharedAccess\Parameters\FirewallPolicy' 2>>"${LOG_FILE}" \
                | head -200 >> "${NETWORK_FILE}" 2>/dev/null || {
                    echo "(Could not extract firewall policy)" >> "${NETWORK_FILE}"
                }
        fi
        echo "" >> "${NETWORK_FILE}"
    else
        echo "(SYSTEM hive not found — cannot extract network configuration)" >> "${NETWORK_FILE}"
    fi

    # -------------------------------------------
    # SAM — User Accounts
    # -------------------------------------------
    log PROGRESS "Extracting user accounts from SAM..."
    echo "" >> "${SOFTWARE_FILE}"
    echo "--- User Accounts (SAM hive) ---" >> "${SOFTWARE_FILE}"
    if [[ -f "${sam_hive}" ]]; then
        hivexregedit --export "${sam_hive}" \
            'SAM\Domains\Account\Users\Names' 2>>"${LOG_FILE}" \
            | grep '^\[' | sed 's/.*\\Names\\\(.*\)\]/\1/' \
            >> "${SOFTWARE_FILE}" 2>/dev/null || {
                echo "(Could not extract user names from SAM)" >> "${SOFTWARE_FILE}"
            }
    else
        echo "(SAM hive not found)" >> "${SOFTWARE_FILE}"
        log WARN "SAM registry hive not found."
    fi

    log INFO "Registry analysis complete."
}

# ---------------------------------------------------------------------------
# File Modification Timeline
# ---------------------------------------------------------------------------
generate_timeline() {
    log SECTION "File Modification Timeline (last ${TIMELINE_DAYS} days)"

    {
        echo "============================================================"
        echo "File Modification Timeline — Last ${TIMELINE_DAYS} Days"
        echo "Scan timestamp: ${SCAN_TIMESTAMP}"
        echo "Mount point: ${MOUNT_POINT}"
        echo "============================================================"
        echo ""
        echo "Format: YYYY-MM-DD HH:MM:SS | SIZE | PATH"
        echo "------------------------------------------------------------"
    } > "${TIMELINE_FILE}"

    # Find files modified in the last N days, sorted by modification time
    find "${MOUNT_POINT}" -maxdepth 8 -type f \
        -mtime "-${TIMELINE_DAYS}" \
        -printf '%TY-%Tm-%Td %TH:%TM:%TS | %s | %p\n' 2>/dev/null \
        | sort -r \
        | head -10000 \
        >> "${TIMELINE_FILE}" 2>/dev/null || {
            echo "(Timeline generation encountered errors — partial results may be present)" >> "${TIMELINE_FILE}"
        }

    local timeline_count
    timeline_count=$(wc -l < "${TIMELINE_FILE}" 2>/dev/null || echo 0)
    log INFO "Timeline generated: ${timeline_count} entries"
}

# ---------------------------------------------------------------------------
# SHA256 Hashing of Suspicious Files
# ---------------------------------------------------------------------------
hash_suspicious_files() {
    log SECTION "SHA256 Hashing of Suspicious Files"

    {
        echo "============================================================"
        echo "SHA256 Hashes of Suspicious Files"
        echo "Scan timestamp: ${SCAN_TIMESTAMP}"
        echo "============================================================"
        echo ""
    } > "${HASHES_FILE}"

    local hash_count=0

    # Collect suspicious file paths from all three JSON findings
    local suspicious_files=()
    for json_file in "${DOCKER_JSON}" "${MALWARE_JSON}" "${ENCRYPTION_JSON}"; do
        if [[ -f "${json_file}" ]]; then
            while IFS= read -r fpath; do
                [[ -n "${fpath}" && -f "${fpath}" ]] && suspicious_files+=("${fpath}")
            done < <(jq -r '.findings[]?.path // empty' "${json_file}" 2>/dev/null || true)
        fi
    done

    # Deduplicate
    local unique_files=()
    if [[ ${#suspicious_files[@]} -gt 0 ]]; then
        while IFS= read -r uf; do
            unique_files+=("${uf}")
        done < <(printf '%s\n' "${suspicious_files[@]}" | sort -u)
    fi

    log PROGRESS "Hashing ${#unique_files[@]} unique suspicious files..."
    for fpath in "${unique_files[@]}"; do
        if [[ -f "${fpath}" && -r "${fpath}" ]]; then
            local hash
            hash=$(sha256sum "${fpath}" 2>/dev/null | awk '{print $1}') || true
            if [[ -n "${hash}" ]]; then
                echo "${hash}  ${fpath}" >> "${HASHES_FILE}"
                ((hash_count++)) || true
            fi
        fi
    done

    log INFO "Hashed ${hash_count} files."
}

# ---------------------------------------------------------------------------
# Main Execution Flow
# ---------------------------------------------------------------------------
main() {
    echo -e "${BOLD}${CYAN}"
    echo "============================================================"
    echo "   Windows Forensic Scanner v1.0"
    echo "   $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo "============================================================"
    echo -e "${NC}"

    # Pre-flight checks and dependency installation
    preflight

    # Detect and mount the Windows partition
    detect_and_mount

    # ---------------------
    # Run scanners in parallel where possible
    # ---------------------
    log SECTION "Starting Parallel Scanners"

    local docker_count_file malware_count_file encryption_count_file
    docker_count_file=$(mktemp)
    malware_count_file=$(mktemp)
    encryption_count_file=$(mktemp)

    # Launch Docker artifact scan in background
    (scan_docker_artifacts > "${docker_count_file}" 2>>"${LOG_FILE}") &
    local docker_pid=$!

    # Launch malware scan in background
    (scan_malware > "${malware_count_file}" 2>>"${LOG_FILE}") &
    local malware_pid=$!

    # Launch encryption scan in background
    (scan_encryption > "${encryption_count_file}" 2>>"${LOG_FILE}") &
    local encryption_pid=$!

    # Wait for all scanners to complete
    log PROGRESS "Waiting for parallel scanners to complete..."
    wait "${docker_pid}" || log WARN "Docker artifact scan encountered errors"
    wait "${malware_pid}" || log WARN "Malware scan encountered errors"
    wait "${encryption_pid}" || log WARN "Encryption scan encountered errors"
    log INFO "All parallel scanners complete."

    # Registry analysis (must be sequential — shared output files)
    analyze_registry

    # Generate timeline
    generate_timeline

    # Hash suspicious files (depends on scan results)
    hash_suspicious_files

    # ---------------------
    # Generate the human-readable report
    # ---------------------
    log SECTION "Generating Report"
    if [[ -x "${SCRIPT_DIR}/report-template.sh" ]]; then
        bash "${SCRIPT_DIR}/report-template.sh" \
            "${REPORT_DIR}" \
            "${MOUNT_POINT}" \
            "${SCAN_TIMESTAMP}" || {
            log WARN "Report generation script failed. Writing fallback summary."
            generate_fallback_report
        }
    else
        log WARN "report-template.sh not found or not executable. Writing fallback summary."
        generate_fallback_report
    fi

    # Clean up temp files
    rm -f "${docker_count_file}" "${malware_count_file}" "${encryption_count_file}" 2>/dev/null || true

    # ---------------------
    # Final Summary
    # ---------------------
    echo ""
    log SECTION "Scan Complete"
    log INFO "Report directory: ${REPORT_DIR}"
    echo ""
    echo -e "${BOLD}Generated files:${NC}"
    ls -lh "${REPORT_DIR}"/ 2>/dev/null || true
    echo ""
    log INFO "Review ${REPORT_TXT} for the human-readable summary."
}

# ---------------------------------------------------------------------------
# Fallback Report Generator (if report-template.sh is missing)
# ---------------------------------------------------------------------------
generate_fallback_report() {
    local docker_count malware_count encryption_count
    docker_count=$(jq '.total_findings // 0' "${DOCKER_JSON}" 2>/dev/null || echo 0)
    malware_count=$(jq '.total_findings // 0' "${MALWARE_JSON}" 2>/dev/null || echo 0)
    encryption_count=$(jq '.total_findings // 0' "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)

    {
        echo "============================================================"
        echo "FORENSIC SCAN REPORT (Fallback Summary)"
        echo "============================================================"
        echo "Scan timestamp : ${SCAN_TIMESTAMP}"
        echo "Mount point    : ${MOUNT_POINT}"
        echo "Report dir     : ${REPORT_DIR}"
        echo ""
        echo "Docker artifacts found  : ${docker_count}"
        echo "Malware indicators found: ${malware_count}"
        echo "Encryption indicators   : ${encryption_count}"
        echo ""
        echo "See individual JSON files for detailed findings."
        echo "See file-hashes.txt for SHA256 hashes of suspicious files."
        echo "See timeline.txt for recent file modifications."
        echo "See installed-software.txt for registry-extracted software."
        echo "See network-config.txt for network configuration."
        echo "============================================================"
    } > "${REPORT_TXT}"
}

# ---------------------------------------------------------------------------
# Entry Point
# ---------------------------------------------------------------------------
main "$@"
