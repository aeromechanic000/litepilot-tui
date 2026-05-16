#!/bin/bash
# @LITE_DESC Idempotent development environment setup with OS detection and dependency installation
# @LITE_SCENE Development environment bootstrap for new machines or team members
# @LITE_TAGS shell, bash, setup, environment, install

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CONFIG_DIR="${HOME}/.config/myapp"
DATA_DIR="${HOME}/.local/share/myapp"

# Required packages (space-separated, OS-agnostic names)
REQUIRED_PACKAGES="git curl wget vim"

# ============================================================================
# OS DETECTION
# ============================================================================

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS="${ID}"
        OS_VERSION="${VERSION_ID}"
    elif [ "$(uname)" == "Darwin" ]; then
        OS="macos"
    else
        OS="unknown"
    fi

    echo "${OS}"
}

OS=$(detect_os)

# ============================================================================
# LOGGING
# ============================================================================

log_info() { echo "[INFO] $*"; }
log_warn() { echo "[WARN] $*"; }
log_error() { echo "[ERROR] $*"; }

# ============================================================================
# PACKAGE INSTALLATION
# ============================================================================

install_packages() {
    local packages="$*"

    if [ -z "${packages}" ]; then
        log_info "No packages to install"
        return 0
    fi

    log_info "Installing packages: ${packages}"

    case "${OS}" in
        ubuntu|debian)
            sudo apt-get update -qq
            sudo apt-get install -y ${packages}
            ;;
        fedora|rhel|centos)
            sudo dnf install -y ${packages}
            ;;
        arch|manjaro)
            sudo pacman -S --noconfirm ${packages}
            ;;
        macos)
            if ! command -v brew &> /dev/null; then
                log_warn "Homebrew not found. Installing..."
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            brew install ${packages}
            ;;
        *)
            log_error "Unsupported OS: ${OS}"
            return 1
            ;;
    esac

    log_info "Package installation complete"
}

# ============================================================================
# DIRECTORY CREATION
# ============================================================================

create_directories() {
    log_info "Creating directories..."

    local dirs=(
        "${CONFIG_DIR}"
        "${DATA_DIR}"
        "${DATA_DIR}/logs"
        "${DATA_DIR}/cache"
        "${PROJECT_ROOT}/tmp"
        "${PROJECT_ROOT}/logs"
    )

    for dir in "${dirs[@]}"; do
        if [ ! -d "${dir}" ]; then
            log_info "Creating: ${dir}"
            mkdir -p "${dir}"
        else
            log_info "Already exists: ${dir}"
        fi
    done

    log_info "Directory creation complete"
}

# ============================================================================
# CONFIGURATION FILES
# ============================================================================

write_config() {
    log_info "Writing configuration files..."

    local config_file="${CONFIG_DIR}/config.toml"

    if [ ! -f "${config_file}" ]; then
        log_info "Creating: ${config_file}"
        cat > "${config_file}" << EOF
# Application Configuration
# Generated: $(date -Iseconds)

[general]
log_level = "info"
data_dir = "${DATA_DIR}"

[database]
host = "localhost"
port = 5432
name = "myapp"

[api]
port = 8080
debug = false
EOF
    else
        log_info "Config already exists: ${config_file}"
    fi

    log_info "Configuration complete"
}

# ============================================================================
# ENVIRONMENT VARIABLES
# ============================================================================

setup_environment() {
    log_info "Setting up environment variables..."

    local env_file="${HOME}/.bashrc.d/myapp.sh"

    if [ ! -d "${HOME}/.bashrc.d" ]; then
        mkdir -p "${HOME}/.bashrc.d}"
    fi

    if [ ! -f "${env_file}" ]; then
        log_info "Creating: ${env_file}"
        cat > "${env_file}" << EOF
# Myapp Environment Variables
# Generated: $(date -Iseconds)

export MYAPP_CONFIG_DIR="${CONFIG_DIR}"
export MYAPP_DATA_DIR="${DATA_DIR}"
export MYAPP_PATH="${PROJECT_ROOT}"

# Add to PATH if needed
export PATH="\${PATH}:${PROJECT_ROOT}/bin"
EOF

        # Source in .bashrc if not already
        if ! grep -q "source.*bashrc.d/myapp.sh" "${HOME}/.bashrc" 2>/dev/null; then
            echo 'source ~/.bashrc.d/myapp.sh' >> "${HOME}/.bashrc"
        fi
    else
        log_info "Environment file already exists: ${env_file}"
    fi

    log_info "Environment setup complete"
}

# ============================================================================
# PERMISSIONS
# ============================================================================

set_permissions() {
    log_info "Setting permissions..."

    # Ensure data directory is writable
    chmod 755 "${DATA_DIR}"
    chmod 755 "${DATA_DIR}/logs"
    chmod 755 "${DATA_DIR}/cache"

    # Ensure config directory is readable
    chmod 755 "${CONFIG_DIR}"

    # Make scripts executable
    find "${PROJECT_ROOT}/bin" -type f -name "*.sh" -exec chmod +x {} \; 2>/dev/null || true

    log_info "Permissions set"
}

# ============================================================================
# IDEMPOTENCY CHECK
# ============================================================================

check_already_setup() {
    local marker_file="${DATA_DIR}/.setup_complete"

    if [ -f "${marker_file}" ]; then
        log_warn "Environment already set up (marker: ${marker_file})"
        return 0
    fi
    return 1
}

mark_setup_complete() {
    local marker_file="${DATA_DIR}/.setup_complete"
    echo "$(date -Iseconds)" > "${marker_file}"
    log_info "Setup marked complete: ${marker_file}"
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

main() {
    log_info "=== Environment Setup Started ==="
    log_info "Detected OS: ${OS}"
    log_info "Project root: ${PROJECT_ROOT}"

    # Check if already set up (unless forced)
    if [ "${FORCE_SETUP:-0}" != "1" ] && check_already_setup; then
        log_info "To force re-setup, run: FORCE_SETUP=1 $0"
        exit 0
    fi

    # Run setup steps
    create_directories
    write_config
    setup_environment
    install_packages ${REQUIRED_PACKAGES}
    set_permissions
    mark_setup_complete

    log_info "=== Environment Setup Complete ==="
    log_info "Please restart your shell or run: source ~/.bashrc"
}

main "$@"