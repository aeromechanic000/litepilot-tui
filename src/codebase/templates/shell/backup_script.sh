#!/bin/bash
# @LITE_DESC Backup script with tar compression, rotation, logging, and error handling
# @LITE_SCENE System administration and automated backup operations
# @LITE_TAGS shell, bash, backup, script, automation

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

BACKUP_SRC="${BACKUP_SRC:-/data}"
BACKUP_DEST="${BACKUP_DEST:-/backups}"
BACKUP_NAME="${BACKUP_NAME:-data_backup}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${BACKUP_DEST}/backup_${TIMESTAMP}.log"
MAX_BACKUPS="${MAX_BACKUPS:-7}"

# Exclude patterns (space-separated)
EXCLUDE_PATTERNS="${EXCLUDE_PATTERNS:-*.tmp *.log node_modules .git}"

# ============================================================================
# LOGGING FUNCTIONS
# ============================================================================

log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] [${level}] ${message}" | tee -a "${LOG_FILE}"
}

log_info() { log "INFO" "$@"; }
log_warn() { log "WARN" "$@"; }
log_error() { log "ERROR" "$@"; }

# ============================================================================
# ERROR HANDLING
# ============================================================================

cleanup() {
    local exit_code=$?
    if [ ${exit_code} -ne 0 ]; then
        log_error "Backup failed with exit code ${exit_code}"
    fi
    exit ${exit_code}
}

trap cleanup EXIT INT TERM

# ============================================================================
# VALIDATION
# ============================================================================

validate_paths() {
    log_info "Validating paths..."

    if [ ! -d "${BACKUP_SRC}" ]; then
        log_error "Source directory does not exist: ${BACKUP_SRC}"
        return 1
    fi

    if [ ! -d "${BACKUP_DEST}" ]; then
        log_info "Creating destination directory: ${BACKUP_DEST}"
        mkdir -p "${BACKUP_DEST}"
    fi

    if [ ! -w "${BACKUP_DEST}" ]; then
        log_error "Destination directory is not writable: ${BACKUP_DEST}"
        return 1
    fi

    log_info "Path validation complete"
    return 0
}

# ============================================================================
# BACKUP FUNCTIONS
# ============================================================================

create_exclusion_args() {
    local exclude_args=""
    for pattern in ${EXCLUDE_PATTERNS}; do
        exclude_args="${exclude_args} --exclude=${pattern}"
    done
    echo "${exclude_args}"
}

create_backup() {
    log_info "Starting backup: ${BACKUP_SRC} -> ${BACKUP_DEST}"

    local backup_file="${BACKUP_DEST}/${BACKUP_NAME}_${TIMESTAMP}.tar.gz"
    local exclude_args=$(create_exclusion_args)

    if tar -czf "${backup_file}" ${exclude_args} -C "${BACKUP_SRC}" . 2>> "${LOG_FILE}"; then
        local size=$(du -h "${backup_file}" | cut -f1)
        log_info "Backup created successfully: ${backup_file} (${size})"
        echo "${backup_file}"
        return 0
    else
        log_error "Backup creation failed"
        return 1
    fi
}

verify_backup() {
    local backup_file="$1"

    log_info "Verifying backup: ${backup_file}"

    if tar -tzf "${backup_file}" > /dev/null 2>> "${LOG_FILE}"; then
        log_info "Backup verification successful"
        return 0
    else
        log_error "Backup verification failed"
        return 1
    fi
}

# ============================================================================
# ROTATION FUNCTIONS
# ============================================================================

rotate_backups() {
    log_info "Rotating backups (keeping last ${MAX_BACKUPS})..."

    local backups=($(ls -t "${BACKUP_DEST}"/${BACKUP_NAME}_*.tar.gz 2>/dev/null))
    local total=${#backups[@]}

    if [ ${total} -gt ${MAX_BACKUPS} ]; then
        local to_remove=$((total - MAX_BACKUPS))
        log_info "Removing ${to_remove} old backup(s)"

        for ((i=MAX_BACKUPS; i<total; i++)); do
            local old_backup="${backups[$i]}"
            log_info "Removing old backup: ${old_backup}"
            rm -f "${old_backup}"
        done
    else
        log_info "No backups to rotate (total: ${total}, max: ${MAX_BACKUPS})"
    fi
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

main() {
    log_info "=== Backup Script Started ==="
    log_info "Configuration:"
    log_info "  Source: ${BACKUP_SRC}"
    log_info "  Destination: ${BACKUP_DEST}"
    log_info "  Max backups: ${MAX_BACKUPS}"

    validate_paths || exit 1

    backup_file=$(create_backup) || exit 1
    verify_backup "${backup_file}" || exit 1
    rotate_backups

    log_info "=== Backup Script Completed Successfully ==="
}

main "$@"