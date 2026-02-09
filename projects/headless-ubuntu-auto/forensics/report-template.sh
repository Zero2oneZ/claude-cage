#!/usr/bin/env bash
# =============================================================================
# report-template.sh — Human-Readable Forensic Report Generator
# =============================================================================
# Takes the JSON findings produced by scan-windows.sh and generates a
# comprehensive, human-readable report.txt summary.
#
# Usage:
#   ./report-template.sh REPORT_DIR [MOUNT_POINT] [SCAN_TIMESTAMP]
#
# Arguments:
#   REPORT_DIR      — Directory containing the JSON findings (required)
#   MOUNT_POINT     — Where the Windows C: drive was mounted (default: /mnt/windows)
#   SCAN_TIMESTAMP  — ISO 8601 timestamp of when the scan started (default: now)
#
# Expected input files in REPORT_DIR:
#   docker-findings.json
#   malware-findings.json
#   encryption-findings.json
#   file-hashes.txt
#   timeline.txt
#   installed-software.txt
#   network-config.txt
# =============================================================================
set -euo pipefail

# ---------------------------------------------------------------------------
# Arguments & Configuration
# ---------------------------------------------------------------------------
REPORT_DIR="${1:?Usage: report-template.sh REPORT_DIR [MOUNT_POINT] [SCAN_TIMESTAMP]}"
MOUNT_POINT="${2:-/mnt/windows}"
SCAN_TIMESTAMP="${3:-$(date -u '+%Y-%m-%dT%H:%M:%SZ')}"

# Input files
DOCKER_JSON="${REPORT_DIR}/docker-findings.json"
MALWARE_JSON="${REPORT_DIR}/malware-findings.json"
ENCRYPTION_JSON="${REPORT_DIR}/encryption-findings.json"
HASHES_FILE="${REPORT_DIR}/file-hashes.txt"
TIMELINE_FILE="${REPORT_DIR}/timeline.txt"
SOFTWARE_FILE="${REPORT_DIR}/installed-software.txt"
NETWORK_FILE="${REPORT_DIR}/network-config.txt"

# Output
REPORT_TXT="${REPORT_DIR}/report.txt"

# ---------------------------------------------------------------------------
# Helper: safe jq extraction (returns default if file missing or jq fails)
# ---------------------------------------------------------------------------
safe_jq() {
    local file="$1"
    local query="$2"
    local default="${3:-0}"
    if [[ -f "${file}" ]]; then
        jq -r "${query}" "${file}" 2>/dev/null || echo "${default}"
    else
        echo "${default}"
    fi
}

# ---------------------------------------------------------------------------
# Collect Machine Info
# ---------------------------------------------------------------------------
collect_machine_info() {
    local hostname_val="unknown"
    local os_version="unknown"
    local product_name="unknown"

    # Try to extract Windows hostname and version from SOFTWARE hive if available
    local software_hive="${MOUNT_POINT}/Windows/System32/config/SOFTWARE"
    if [[ -f "${software_hive}" ]] && command -v hivexregedit &>/dev/null; then
        product_name=$(hivexregedit --export "${software_hive}" \
            'Microsoft\Windows NT\CurrentVersion' 2>/dev/null \
            | grep '"ProductName"' | head -1 \
            | sed 's/.*="\(.*\)"/\1/' | tr -d '\r') || true
        os_version=$(hivexregedit --export "${software_hive}" \
            'Microsoft\Windows NT\CurrentVersion' 2>/dev/null \
            | grep '"CurrentBuild"' | head -1 \
            | sed 's/.*="\(.*\)"/\1/' | tr -d '\r') || true
    fi

    # Try to get hostname from SYSTEM hive
    local system_hive="${MOUNT_POINT}/Windows/System32/config/SYSTEM"
    if [[ -f "${system_hive}" ]] && command -v hivexregedit &>/dev/null; then
        hostname_val=$(hivexregedit --export "${system_hive}" \
            'ControlSet001\Control\ComputerName\ComputerName' 2>/dev/null \
            | grep '"ComputerName"' | head -1 \
            | sed 's/.*="\(.*\)"/\1/' | tr -d '\r') || true
    fi

    echo "Hostname       : ${hostname_val:-unknown}"
    echo "Windows Version: ${product_name:-unknown} (Build ${os_version:-unknown})"
    echo "Scanner Host   : $(hostname 2>/dev/null || echo 'unknown')"
    echo "Scanner Kernel : $(uname -r 2>/dev/null || echo 'unknown')"
}

# ---------------------------------------------------------------------------
# Report: Header
# ---------------------------------------------------------------------------
write_header() {
    cat <<HEADER
##############################################################################
#                                                                            #
#                   WINDOWS FORENSIC SCAN REPORT                             #
#                                                                            #
##############################################################################

Scan Timestamp : ${SCAN_TIMESTAMP}
Report Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')
Mount Point    : ${MOUNT_POINT}
Report Directory: ${REPORT_DIR}

--- Machine Information ---
$(collect_machine_info)

##############################################################################
HEADER
}

# ---------------------------------------------------------------------------
# Report: Docker Findings Summary
# ---------------------------------------------------------------------------
write_docker_summary() {
    local total
    total=$(safe_jq "${DOCKER_JSON}" '.total_findings' '0')

    cat <<DOCKER

==============================================================================
 SECTION 1: DOCKER ARTIFACT FINDINGS
==============================================================================

Total Docker artifacts found: ${total}

DOCKER

    if [[ "${total}" -gt 0 && -f "${DOCKER_JSON}" ]]; then
        # Count by type
        local dirs files
        dirs=$(jq '[.findings[] | select(.type == "directory")] | length' "${DOCKER_JSON}" 2>/dev/null || echo 0)
        files=$(jq '[.findings[] | select(.type == "file")] | length' "${DOCKER_JSON}" 2>/dev/null || echo 0)

        echo "  Directories : ${dirs}"
        echo "  Files       : ${files}"
        echo ""
        echo "  Notable findings:"
        echo "  -----------------"

        # List unique top-level paths (first 3 directory components)
        jq -r '.findings[].path' "${DOCKER_JSON}" 2>/dev/null \
            | awk -F'/' '{print "/" $2 "/" $3 "/" $4 "/" $5}' \
            | sort -u | head -20 | while IFS= read -r p; do
                echo "    - ${p}"
            done

        echo ""

        # Docker config files found
        local config_count
        config_count=$(jq '[.findings[] | select(.path | test("config\\.v2\\.json|config\\.json|daemon\\.json"))] | length' \
            "${DOCKER_JSON}" 2>/dev/null || echo 0)
        echo "  Docker config files found: ${config_count}"

        # Docker Compose files found
        local compose_count
        compose_count=$(jq '[.findings[] | select(.path | test("docker-compose\\.(yml|yaml)|compose\\.(yml|yaml)"))] | length' \
            "${DOCKER_JSON}" 2>/dev/null || echo 0)
        echo "  Docker Compose files found: ${compose_count}"

        # Dockerfiles found
        local dockerfile_count
        dockerfile_count=$(jq '[.findings[] | select(.path | test("Dockerfile"))] | length' \
            "${DOCKER_JSON}" 2>/dev/null || echo 0)
        echo "  Dockerfiles found: ${dockerfile_count}"

        # WSL2 virtual disk images
        local vhdx_count
        vhdx_count=$(jq '[.findings[] | select(.path | test("ext4\\.vhdx"))] | length' \
            "${DOCKER_JSON}" 2>/dev/null || echo 0)
        echo "  WSL2 disk images (ext4.vhdx): ${vhdx_count}"
    else
        echo "  No Docker artifacts were detected on this system."
    fi
}

# ---------------------------------------------------------------------------
# Report: Malware Findings Summary
# ---------------------------------------------------------------------------
write_malware_summary() {
    local total
    total=$(safe_jq "${MALWARE_JSON}" '.total_findings' '0')

    cat <<MALWARE

==============================================================================
 SECTION 2: MALWARE INDICATOR FINDINGS
==============================================================================

Total malware indicators found: ${total}

MALWARE

    if [[ "${total}" -gt 0 && -f "${MALWARE_JSON}" ]]; then
        # Severity breakdown
        local crit high med low info
        crit=$(safe_jq "${MALWARE_JSON}" '.severity_summary.critical' '0')
        high=$(safe_jq "${MALWARE_JSON}" '.severity_summary.high' '0')
        med=$(safe_jq "${MALWARE_JSON}" '.severity_summary.medium' '0')
        low=$(safe_jq "${MALWARE_JSON}" '.severity_summary.low' '0')
        info=$(safe_jq "${MALWARE_JSON}" '.severity_summary.info' '0')

        echo "  Severity Breakdown:"
        echo "  -------------------"
        echo "    CRITICAL : ${crit}"
        echo "    HIGH     : ${high}"
        echo "    MEDIUM   : ${med}"
        echo "    LOW      : ${low}"
        echo "    INFO     : ${info}"
        echo ""

        # Risk Assessment
        if [[ "${crit}" -gt 0 ]]; then
            echo "  >>> RISK ASSESSMENT: CRITICAL <<<"
            echo "  Critical-severity indicators were found. This system may be"
            echo "  actively compromised. Immediate investigation is recommended."
            echo ""
            echo "  Critical findings:"
            jq -r '.findings[] | select(.severity == "critical") | "    [CRITICAL] \(.path)"' \
                "${MALWARE_JSON}" 2>/dev/null | head -20
        elif [[ "${high}" -gt 0 ]]; then
            echo "  >>> RISK ASSESSMENT: HIGH <<<"
            echo "  High-severity indicators were found. Further analysis is"
            echo "  strongly recommended."
        elif [[ "${med}" -gt 0 ]]; then
            echo "  >>> RISK ASSESSMENT: MODERATE <<<"
            echo "  Medium-severity indicators were found. Review recommended."
        else
            echo "  >>> RISK ASSESSMENT: LOW <<<"
            echo "  Only low-severity or informational indicators found."
        fi

        echo ""
        echo "  Categories detected:"
        echo "  --------------------"
        jq -r '[.findings[].category] | group_by(.) | map({category: .[0], count: length}) | .[] | "    \(.category): \(.count)"' \
            "${MALWARE_JSON}" 2>/dev/null || echo "    (could not parse categories)"

        # Content-match findings (encoded PowerShell, etc.)
        local content_matches
        content_matches=$(jq '[.findings[] | select(.category == "content-match")] | length' \
            "${MALWARE_JSON}" 2>/dev/null || echo 0)
        if [[ "${content_matches}" -gt 0 ]]; then
            echo ""
            echo "  Suspicious content matches: ${content_matches}"
            jq -r '.findings[] | select(.category == "content-match") | "    [\(.severity)] \(.path) — indicator: \(.indicator)"' \
                "${MALWARE_JSON}" 2>/dev/null | head -10
        fi
    else
        echo "  No malware indicators were detected on this system."
    fi
}

# ---------------------------------------------------------------------------
# Report: Encryption Indicators Summary
# ---------------------------------------------------------------------------
write_encryption_summary() {
    local total
    total=$(safe_jq "${ENCRYPTION_JSON}" '.total_findings' '0')

    cat <<ENCRYPTION

==============================================================================
 SECTION 3: ENCRYPTION / RANSOMWARE INDICATORS
==============================================================================

Total encryption indicators found: ${total}

ENCRYPTION

    if [[ "${total}" -gt 0 && -f "${ENCRYPTION_JSON}" ]]; then
        # Count by category
        echo "  Category Breakdown:"
        echo "  -------------------"
        jq -r '[.findings[].category] | group_by(.) | map({category: .[0], count: length}) | .[] | "    \(.category): \(.count)"' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo "    (could not parse categories)"

        # Ransomware extension findings
        local ransom_ext_count
        ransom_ext_count=$(jq '[.findings[] | select(.category == "ransomware-extension")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${ransom_ext_count}" -gt 0 ]]; then
            echo ""
            echo "  !!! RANSOMWARE ACTIVITY DETECTED !!!"
            echo "  ${ransom_ext_count} files with ransomware extensions found."
            echo ""
            echo "  Sample files:"
            jq -r '.findings[] | select(.category == "ransomware-extension") | "    \(.path) [\(.indicator)]"' \
                "${ENCRYPTION_JSON}" 2>/dev/null | head -15
        fi

        # Ransom notes
        local ransom_note_count
        ransom_note_count=$(jq '[.findings[] | select(.category == "ransom-note")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${ransom_note_count}" -gt 0 ]]; then
            echo ""
            echo "  RANSOM NOTES FOUND: ${ransom_note_count}"
            jq -r '.findings[] | select(.category == "ransom-note") | "    \(.path)"' \
                "${ENCRYPTION_JSON}" 2>/dev/null | head -10
        fi

        # BitLocker
        local bitlocker_count
        bitlocker_count=$(jq '[.findings[] | select(.category == "bitlocker")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${bitlocker_count}" -gt 0 ]]; then
            echo ""
            echo "  BitLocker artifacts found: ${bitlocker_count}"
        fi

        # EFS
        local efs_count
        efs_count=$(jq '[.findings[] | select(.category == "efs")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${efs_count}" -gt 0 ]]; then
            echo ""
            echo "  EFS (Encrypting File System) artifacts found: ${efs_count}"
        fi

        # Certificates
        local cert_count
        cert_count=$(jq '[.findings[] | select(.category == "certificate")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${cert_count}" -gt 0 ]]; then
            echo ""
            echo "  Certificate files found: ${cert_count}"
            jq -r '.findings[] | select(.category == "certificate") | "    \(.path)"' \
                "${ENCRYPTION_JSON}" 2>/dev/null | head -10
        fi
    else
        echo "  No encryption or ransomware indicators were detected."
    fi
}

# ---------------------------------------------------------------------------
# Report: Top 20 Most Suspicious Files
# ---------------------------------------------------------------------------
write_suspicious_files() {
    cat <<'SUSP'

==============================================================================
 SECTION 4: TOP 20 MOST SUSPICIOUS FILES
==============================================================================

Files ranked by severity (critical > high > medium > low > info) and recency.

SUSP

    if [[ -f "${MALWARE_JSON}" ]]; then
        # Extract findings, sort by severity (custom order) then by mtime (newest first)
        jq -r '
            .findings
            | map(. + {
                severity_rank: (
                    if .severity == "critical" then 0
                    elif .severity == "high" then 1
                    elif .severity == "medium" then 2
                    elif .severity == "low" then 3
                    else 4
                    end
                )
              })
            | sort_by(.severity_rank, -.mtime)
            | .[0:20]
            | to_entries[]
            | "  \(.key + 1 | tostring | if length < 2 then " " + . else . end). [\(.value.severity | ascii_upcase)] \(.value.path)\n      Category: \(.value.category) | Size: \(.value.size) bytes | Modified: \(.value.mtime)"
        ' "${MALWARE_JSON}" 2>/dev/null || echo "  (Could not parse malware findings for ranking)"
    else
        echo "  (No malware findings file available)"
    fi

    # Also include any encryption findings that look like ransomware
    if [[ -f "${ENCRYPTION_JSON}" ]]; then
        local ransom_count
        ransom_count=$(jq '[.findings[] | select(.category == "ransomware-extension" or .category == "ransom-note")] | length' \
            "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)
        if [[ "${ransom_count}" -gt 0 ]]; then
            echo ""
            echo "  Additionally, ${ransom_count} ransomware-related files were found"
            echo "  (see Section 3 for details)."
        fi
    fi
}

# ---------------------------------------------------------------------------
# Report: Registry Analysis Summary
# ---------------------------------------------------------------------------
write_registry_summary() {
    cat <<'REG'

==============================================================================
 SECTION 5: REGISTRY ANALYSIS SUMMARY
==============================================================================

REG

    # Installed software count
    if [[ -f "${SOFTWARE_FILE}" ]]; then
        local sw_count
        sw_count=$(grep -c '"DisplayName"' "${SOFTWARE_FILE}" 2>/dev/null || echo 0)
        echo "  Installed software entries found: ${sw_count}"

        # List notable software
        echo ""
        echo "  Notable installed software:"
        echo "  ---------------------------"
        grep '"DisplayName"' "${SOFTWARE_FILE}" 2>/dev/null \
            | sed 's/.*="\(.*\)"/\1/' | tr -d '\r' \
            | sort -u | head -30 | while IFS= read -r name; do
                echo "    - ${name}"
            done

        # Run keys (persistence entries)
        echo ""
        echo "  Persistence entries (Run/RunOnce keys):"
        echo "  ----------------------------------------"
        local run_section=false
        while IFS= read -r line; do
            if [[ "${line}" == *"Run Keys"* || "${line}" == *"RunOnce Keys"* ]]; then
                run_section=true
                echo "    ${line}"
                continue
            fi
            if [[ "${run_section}" == "true" ]]; then
                if [[ "${line}" == "---"* || -z "${line}" ]]; then
                    run_section=false
                    continue
                fi
                echo "    ${line}"
            fi
        done < "${SOFTWARE_FILE}" 2>/dev/null || echo "    (Could not parse Run keys)"

        # User accounts
        echo ""
        echo "  User Accounts:"
        echo "  ---------------"
        local in_users=false
        while IFS= read -r line; do
            if [[ "${line}" == *"User Accounts"* ]]; then
                in_users=true
                continue
            fi
            if [[ "${in_users}" == "true" ]]; then
                if [[ "${line}" == "---"* || "${line}" == "==="* ]]; then
                    break
                fi
                [[ -n "${line}" ]] && echo "    - ${line}"
            fi
        done < "${SOFTWARE_FILE}" 2>/dev/null || echo "    (Could not parse user accounts)"

        # Service count
        echo ""
        local svc_count
        svc_count=$(grep -c '^\[' "${SOFTWARE_FILE}" 2>/dev/null || echo 0)
        echo "  Registry keys/services enumerated: approximately ${svc_count}"
    else
        echo "  (installed-software.txt not found)"
    fi

    # Network configuration summary
    echo ""
    echo "  Network Configuration:"
    echo "  -----------------------"
    if [[ -f "${NETWORK_FILE}" ]]; then
        local net_lines
        net_lines=$(wc -l < "${NETWORK_FILE}" 2>/dev/null || echo 0)
        echo "  Network configuration extracted: ${net_lines} lines"
        echo "  (See network-config.txt for full details)"
    else
        echo "  (network-config.txt not found)"
    fi
}

# ---------------------------------------------------------------------------
# Report: Timeline Summary
# ---------------------------------------------------------------------------
write_timeline_summary() {
    cat <<'TIME'

==============================================================================
 SECTION 6: FILE MODIFICATION TIMELINE SUMMARY
==============================================================================

TIME

    if [[ -f "${TIMELINE_FILE}" ]]; then
        local total_entries
        total_entries=$(wc -l < "${TIMELINE_FILE}" 2>/dev/null || echo 0)
        echo "  Total entries in timeline: ${total_entries}"
        echo "  (See timeline.txt for full listing)"
        echo ""
        echo "  Most recent 10 modifications:"
        echo "  ------------------------------"
        # Skip header lines (first 6), show next 10
        tail -n +7 "${TIMELINE_FILE}" 2>/dev/null | head -10 | while IFS= read -r line; do
            echo "    ${line}"
        done
    else
        echo "  (timeline.txt not found)"
    fi
}

# ---------------------------------------------------------------------------
# Report: File Hashes Summary
# ---------------------------------------------------------------------------
write_hashes_summary() {
    cat <<'HASH'

==============================================================================
 SECTION 7: FILE INTEGRITY HASHES
==============================================================================

HASH

    if [[ -f "${HASHES_FILE}" ]]; then
        local hash_count
        # Subtract header lines (4 lines)
        hash_count=$(tail -n +5 "${HASHES_FILE}" 2>/dev/null | grep -c '^[a-f0-9]' 2>/dev/null || echo 0)
        echo "  Files hashed: ${hash_count}"
        echo "  Algorithm: SHA-256"
        echo "  (See file-hashes.txt for the complete list)"
        echo ""

        if [[ "${hash_count}" -gt 0 ]]; then
            echo "  Sample hashes (first 10):"
            echo "  --------------------------"
            tail -n +5 "${HASHES_FILE}" 2>/dev/null | grep '^[a-f0-9]' | head -10 | while IFS= read -r line; do
                echo "    ${line}"
            done
        fi
    else
        echo "  (file-hashes.txt not found)"
    fi
}

# ---------------------------------------------------------------------------
# Report: Recommendations
# ---------------------------------------------------------------------------
write_recommendations() {
    local total_malware total_encryption total_docker
    total_malware=$(safe_jq "${MALWARE_JSON}" '.total_findings' '0')
    total_encryption=$(safe_jq "${ENCRYPTION_JSON}" '.total_findings' '0')
    total_docker=$(safe_jq "${DOCKER_JSON}" '.total_findings' '0')
    local crit high
    crit=$(safe_jq "${MALWARE_JSON}" '.severity_summary.critical' '0')
    high=$(safe_jq "${MALWARE_JSON}" '.severity_summary.high' '0')

    local ransom_count
    ransom_count=$(jq '[.findings[] | select(.category == "ransomware-extension" or .category == "ransom-note")] | length' \
        "${ENCRYPTION_JSON}" 2>/dev/null || echo 0)

    cat <<'REC'

==============================================================================
 SECTION 8: RECOMMENDATIONS
==============================================================================

REC

    echo "  Based on the forensic scan results, the following actions are recommended:"
    echo ""

    local rec_num=1

    # Critical malware
    if [[ "${crit}" -gt 0 ]]; then
        echo "  ${rec_num}. [URGENT] CRITICAL MALWARE INDICATORS FOUND"
        echo "     - Isolate this machine from the network immediately."
        echo "     - Do NOT boot from this drive until analysis is complete."
        echo "     - Submit critical files to a malware sandbox for analysis."
        echo "     - Consider engaging an incident response team."
        echo ""
        ((rec_num++))
    fi

    # Ransomware
    if [[ "${ransom_count}" -gt 0 ]]; then
        echo "  ${rec_num}. [URGENT] RANSOMWARE ACTIVITY DETECTED"
        echo "     - Do NOT pay any ransom demands."
        echo "     - Preserve this disk image as evidence."
        echo "     - Check for available decryptors at nomoreransom.org."
        echo "     - Restore from clean backups if available."
        echo "     - Report to law enforcement (FBI IC3, local CERT)."
        echo ""
        ((rec_num++))
    fi

    # High-severity malware
    if [[ "${high}" -gt 0 ]]; then
        echo "  ${rec_num}. [HIGH] Review high-severity malware indicators"
        echo "     - Examine files listed in the malware findings."
        echo "     - Cross-reference SHA256 hashes with VirusTotal."
        echo "     - Check for persistence mechanisms in Run keys."
        echo ""
        ((rec_num++))
    fi

    # Docker artifacts
    if [[ "${total_docker}" -gt 0 ]]; then
        echo "  ${rec_num}. Review Docker artifacts"
        echo "     - Docker was used on this system (${total_docker} artifacts found)."
        echo "     - Examine container configs, docker-compose files, and images."
        echo "     - Check for exposed secrets in Docker configurations."
        echo "     - Review mounted volumes for sensitive data."
        echo ""
        ((rec_num++))
    fi

    # Encryption artifacts
    if [[ "${total_encryption}" -gt 0 && "${ransom_count}" -eq 0 ]]; then
        echo "  ${rec_num}. Review encryption artifacts"
        echo "     - Encryption-related files found (${total_encryption} indicators)."
        echo "     - Check if BitLocker/EFS is expected on this system."
        echo "     - Preserve any recovery keys found."
        echo ""
        ((rec_num++))
    fi

    # General recommendations
    echo "  ${rec_num}. General forensic preservation"
    echo "     - Create a forensic disk image (dd or dc3dd) before any"
    echo "       further analysis if not already done."
    echo "     - Maintain chain of custody documentation."
    echo "     - Preserve all report files for the investigation record."
    echo ""
    ((rec_num++))

    echo "  ${rec_num}. File hash verification"
    echo "     - Cross-reference hashes in file-hashes.txt with known"
    echo "       malware databases (VirusTotal, MalwareBazaar, NIST NSRL)."
    echo ""
    ((rec_num++))

    echo "  ${rec_num}. Timeline analysis"
    echo "     - Review timeline.txt to identify suspicious activity windows."
    echo "     - Correlate file modifications with known incident timeline."
    echo ""
}

# ---------------------------------------------------------------------------
# Report: Footer
# ---------------------------------------------------------------------------
write_footer() {
    cat <<FOOTER

==============================================================================
 END OF REPORT
==============================================================================

Report generated by: Windows Forensic Scanner v1.0
Scan timestamp    : ${SCAN_TIMESTAMP}
Report timestamp  : $(date -u '+%Y-%m-%dT%H:%M:%SZ')
Report directory  : ${REPORT_DIR}

Files in this report:
  - report.txt             : This summary report
  - docker-findings.json   : Detailed Docker artifact findings (machine-readable)
  - malware-findings.json  : Detailed malware indicator findings (machine-readable)
  - encryption-findings.json : Detailed encryption/ransomware findings (machine-readable)
  - file-hashes.txt        : SHA256 hashes of all suspicious files
  - timeline.txt           : File modification timeline (last 90 days)
  - installed-software.txt : Software and services from Windows Registry
  - network-config.txt     : Network configuration from Windows Registry
  - scan.log               : Full scanner log with timestamps

DISCLAIMER: This report is generated by an automated forensic scanner.
Findings should be validated by a qualified digital forensics examiner.
This tool performs read-only analysis and does not modify the target disk.
FOOTER
}

# ---------------------------------------------------------------------------
# Main: Assemble the Report
# ---------------------------------------------------------------------------
main() {
    {
        write_header
        write_docker_summary
        write_malware_summary
        write_encryption_summary
        write_suspicious_files
        write_registry_summary
        write_timeline_summary
        write_hashes_summary
        write_recommendations
        write_footer
    } > "${REPORT_TXT}"

    echo "Report written to: ${REPORT_TXT}"
}

main "$@"
