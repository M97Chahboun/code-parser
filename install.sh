#!/usr/bin/env bash
#
# code-parser installation script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/your-org/code-parser/main/install.sh | bash
#
# Options (via environment variables or flags):
#   VERSION     - Specific version to install (default: latest)
#   PREFIX      - Installation directory (default: ~/.local)
#   WITH_CMDS   - Install Claude Code commands too (default: false)
#
# Examples:
#   curl -fsSL .../install.sh | bash
#   curl -fsSL .../install.sh | bash -s -- --version v0.1.0
#   curl -fsSL .../install.sh | bash -s -- --with-commands
#   curl -fsSL .../install.sh | bash -s -- --prefix /usr/local
#

set -euo pipefail

# --- Configuration ---
REPO="your-org/code-parser"  # TODO: Update with actual GitHub repo
BINARY_NAME="code-parser"
COMMANDS_DIR="code-parser-commands"

# --- Defaults ---
VERSION="${VERSION:-}"
PREFIX="${PREFIX:-$HOME/.local}"
WITH_CMDS="${WITH_CMDS:-false}"

# --- Colors for output ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# --- Helper functions ---
log_info() {
    echo -e "${BLUE}▶${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1" >&2
}

# --- Parse command line arguments ---
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --with-commands)
            WITH_CMDS="true"
            shift
            ;;
        --help)
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --version VERSION   Install specific version (default: latest)"
            echo "  --prefix PATH       Install to custom directory (default: ~/.local)"
            echo "  --with-commands     Also install Claude Code slash commands"
            echo "  --help              Show this help message"
            echo ""
            echo "Examples:"
            echo "  curl -fsSL .../install.sh | bash"
            echo "  curl -fsSL .../install.sh | bash -s -- --version v0.1.0"
            echo "  curl -fsSL .../install.sh | bash -s -- --with-commands"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# --- Platform detection ---
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        *)
            log_error "Unsupported OS: $(uname -s)"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64)  arch="x86_64" ;;
        arm64)   arch="aarch64" ;;
        aarch64) arch="aarch64" ;;
        *)
            log_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# --- Check for required tools ---
check_requirements() {
    local missing=()

    if ! command -v curl &> /dev/null; then
        missing+=("curl")
    fi

    if ! command -v mkdir &> /dev/null; then
        missing+=("mkdir")
    fi

    if ! command -v cp &> /dev/null; then
        missing+=("cp")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        log_info "Please install the missing tools and try again"
        exit 1
    fi
}

# --- Get latest version from GitHub ---
get_latest_version() {
    if command -v curl &> /dev/null; then
        local latest
        latest=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | \
                 grep -o '"tag_name": "[^"]*"' | head -1 | cut -d'"' -f4)
        if [[ -n "$latest" ]]; then
            echo "$latest"
            return 0
        fi
    fi
    return 1
}

# --- Download binary from GitHub releases ---
download_binary() {
    local version="$1"
    local platform="$2"
    local temp_dir="$3"

    local filename="${BINARY_NAME}-${version}-${platform}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${filename}"

    log_info "Downloading ${filename}..."

    if ! curl -fsSL "$url" -o "${temp_dir}/${filename}"; then
        log_error "Failed to download from $url"
        log_info "The binary may not be available for this platform/version"
        return 1
    fi

    # Extract
    if ! tar -xzf "${temp_dir}/${filename}" -C "$temp_dir" 2>/dev/null; then
        log_error "Failed to extract archive"
        return 1
    fi

    # Find the binary (may be in a subdirectory)
    if [[ -f "${temp_dir}/${BINARY_NAME}" ]]; then
        mv "${temp_dir}/${BINARY_NAME}" "${temp_dir}/${BINARY_NAME}.bin"
    elif [[ -f "${temp_dir}/target/release/${BINARY_NAME}" ]]; then
        mv "${temp_dir}/target/release/${BINARY_NAME}" "${temp_dir}/${BINARY_NAME}.bin"
    else
        # Try to find any executable named code-parser
        find "$temp_dir" -name "$BINARY_NAME" -type f -executable -exec mv {} "${temp_dir}/${BINARY_NAME}.bin" \; 2>/dev/null || true
        if [[ ! -f "${temp_dir}/${BINARY_NAME}.bin" ]]; then
            log_error "Binary not found in archive"
            return 1
        fi
    fi

    return 0
}

# --- Build from source ---
build_from_source() {
    local temp_dir="$1"

    log_info "Building from source..."

    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo (Rust) is required to build from source"
        log_info "Install Rust from: https://rustup.rs/"
        return 1
    fi

    # Clone repo
    log_info "Cloning repository..."
    local clone_dir="${temp_dir}/source"

    if command -v git &> /dev/null; then
        if ! git clone --depth 1 "https://github.com/${REPO}.git" "$clone_dir" 2>/dev/null; then
            log_error "Failed to clone repository"
            return 1
        fi

        # Checkout specific version if requested
        if [[ -n "$VERSION" ]]; then
            (cd "$clone_dir" && git checkout "$VERSION" 2>/dev/null) || true
        fi
    else
        # Fallback: assume we're already in the repo directory (for local installs)
        if [[ -f "Cargo.toml" ]]; then
            cp -r . "$clone_dir"
        else
            log_error "Not in a code-parser directory and git not available"
            return 1
        fi
    fi

    # Build release
    log_info "Building release binary..."
    if ! (cd "$clone_dir" && cargo build --release 2>&1); then
        log_error "Build failed"
        return 1
    fi

    # Copy binary
    if [[ -f "${clone_dir}/target/release/${BINARY_NAME}" ]]; then
        cp "${clone_dir}/target/release/${BINARY_NAME}" "${temp_dir}/${BINARY_NAME}.bin"
        return 0
    else
        log_error "Binary not found after build"
        return 1
    fi
}

# --- Install Claude Code commands ---
install_commands() {
    local dest="$1"
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    log_info "Installing Claude Code commands to $dest..."

    mkdir -p "$dest"

    # Check if we're in a local repo or need to download commands
    if [[ -d "${script_dir}/${COMMANDS_DIR}" ]]; then
        # Local installation
        for f in "${script_dir}/${COMMANDS_DIR}"/*.md; do
            if [[ -f "$f" ]]; then
                local name
                name="$(basename "$f")"
                cp "$f" "$dest/$name"
                log_success "Installed $name"
            fi
        done
    else
        # Download from repo
        local commands_url="https://raw.githubusercontent.com/${REPO}/main/${COMMANDS_DIR}"
        local files=("index.md" "parse-find.md" "parse-read.md" "parse-edit.md" "parse-audit.md" "parse-stats.md")

        for file in "${files[@]}"; do
            if curl -fsSL "${commands_url}/${file}" -o "$dest/$file" 2>/dev/null; then
                log_success "Installed $file"
            else
                log_warn "Failed to download $file"
            fi
        done
    fi

    echo ""
    echo -e "${GREEN}Done.${NC} Claude Code commands installed to:"
    echo "  $dest"
    echo ""
    echo "Available commands:"
    echo "  /index          [path]                  — index a file or directory"
    echo "  /parse-find     <ClassName> [path]      — locate a class or method"
    echo "  /parse-read     <file> <start> <end>    — read specific lines"
    echo "  /parse-edit     <file> <Class.method>   — surgical edit via index"
    echo "  /parse-audit    [path]                  — full project structure report"
    echo "  /parse-stats    [path]                  — token saving summary"
}

# --- Main installation ---
main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║   code-parser Installation Script      ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
    echo ""

    # Check requirements
    check_requirements

    # Detect platform
    local platform
    platform=$(detect_platform)
    log_info "Detected platform: $platform"

    # Determine version
    if [[ -z "$VERSION" ]]; then
        log_info "Fetching latest version..."
        VERSION=$(get_latest_version) || true
        if [[ -z "$VERSION" ]]; then
            log_warn "Could not determine latest version, using 'latest'"
            VERSION="latest"
        fi
    fi
    log_info "Installing version: $VERSION"

    # Setup installation directory
    local bin_dir="${PREFIX}/bin"
    log_info "Installation prefix: $PREFIX"
    log_info "Binary will be installed to: $bin_dir"

    mkdir -p "$bin_dir"

    # Create temp directory
    local temp_dir
    temp_dir=$(mktemp -d)
    trap 'rm -rf "$temp_dir"' EXIT

    # Try to download binary first, fall back to source build
    if ! download_binary "$VERSION" "$platform" "$temp_dir"; then
        log_warn "Binary download failed, attempting to build from source..."
        if ! build_from_source "$temp_dir"; then
            log_error "Installation failed"
            echo ""
            echo "Manual installation options:"
            echo "  1. Build from source: git clone && cd code-parser && cargo build --release"
            echo "  2. Download from: https://github.com/${REPO}/releases"
            exit 1
        fi
    fi

    # Install binary
    log_info "Installing binary..."
    mv "${temp_dir}/${BINARY_NAME}.bin" "${bin_dir}/${BINARY_NAME}"
    chmod +x "${bin_dir}/${BINARY_NAME}"

    # Verify installation
    if ! command -v "${BINARY_NAME}" &> /dev/null; then
        # Try direct execution
        if "${bin_dir}/${BINARY_NAME}" --version &> /dev/null; then
            log_success "Binary installed successfully"
        else
            log_error "Binary verification failed"
            exit 1
        fi
    else
        log_success "Binary installed successfully"
    fi

    # Print installation summary
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   Installation Complete!               ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BINARY_NAME} ${VERSION} has been installed to:${NC}"
    echo "  ${bin_dir}/${BINARY_NAME}"
    echo ""

    # Check if bin directory is in PATH
    if [[ ":$PATH:" != *":${bin_dir}:"* ]]; then
        echo -e "${YELLOW}⚠  Warning:${NC} ${bin_dir} is not in your PATH"
        echo ""
        echo "Add it to your PATH by adding one of these lines to your shell config:"
        echo ""
        if [[ "$SHELL" == *"zsh"* ]]; then
            echo "  echo 'export PATH=\"${bin_dir}:\$PATH\"' >> ~/.zshrc"
        elif [[ "$SHELL" == *"bash"* ]]; then
            echo "  echo 'export PATH=\"${bin_dir}:\$PATH\"' >> ~/.bashrc"
        else
            echo "  export PATH=\"${bin_dir}:\$PATH\""
        fi
        echo ""
        echo "Then restart your shell or run: source ~/.zshrc (or ~/.bashrc)"
        echo ""
    fi

    # Install Claude Code commands if requested
    if [[ "$WITH_CMDS" == "true" ]]; then
        echo ""
        install_commands "$HOME/.claude/commands"
    fi

    # Quick start guide
    echo -e "${BLUE}Quick Start:${NC}"
    echo ""
    echo "  # Parse a single file"
    echo "  ${BINARY_NAME} src/main.dart --format pretty"
    echo ""
    echo "  # Index entire project"
    echo "  ${BINARY_NAME} ./my_project | jq '.'"
    echo ""
    echo "  # Get help"
    echo "  ${BINARY_NAME} --help"
    echo ""

    # Show version if available
    if command -v "${BINARY_NAME}" &> /dev/null; then
        echo -e "${BLUE}Version:${NC}"
        "${BINARY_NAME}" --version 2>/dev/null || true
    fi

    echo ""
    log_success "Installation complete!"
}

# Run main function
main "$@"
