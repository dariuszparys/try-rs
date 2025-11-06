#!/bin/sh
# Installation script for try - lightweight directory navigation tool
# Usage: curl -sSf https://raw.githubusercontent.com/dariuszparys/try-rs/main/install.sh | sh
# Or: curl -sSf https://raw.githubusercontent.com/dariuszparys/try-rs/main/install.sh | sh -s -- --no-shell-integration

set -e

# Colors for output
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# Configuration
REPO="dariuszparys/try-rs"
BINARY_NAME="try"
INSTALL_DIR="${HOME}/.local/bin"
SHELL_INTEGRATION=true

# Parse arguments
for arg in "$@"; do
    case $arg in
        --no-shell-integration)
            SHELL_INTEGRATION=false
            shift
            ;;
        --help)
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-shell-integration    Skip automatic shell integration setup"
            echo "  --help                    Show this help message"
            exit 0
            ;;
    esac
done

# Utility functions
info() {
    printf "${BLUE}==>${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}✓${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
    exit 1
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1" >&2
}

# Detect OS
detect_os() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$OS" in
        linux*)
            echo "linux"
            ;;
        darwin*)
            echo "macos"
            ;;
        mingw* | msys* | cygwin*)
            echo "windows"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac
}

# Detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64 | amd64)
            echo "x86_64"
            ;;
        aarch64 | arm64)
            echo "aarch64"
            ;;
        armv7* | armv8*)
            echo "armv7"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    # Try to get latest release from GitHub API
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them and try again."
    fi

    if [ -z "$VERSION" ]; then
        error "Failed to get latest version. Please check your internet connection or install manually."
    fi

    echo "$VERSION"
}

# Download file
download() {
    URL="$1"
    OUTPUT="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$URL" -o "$OUTPUT"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$OUTPUT" "$URL"
    else
        error "Neither curl nor wget found. Please install one of them and try again."
    fi
}

# Detect shell
detect_shell() {
    if [ -n "$BASH_VERSION" ]; then
        echo "bash"
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    elif [ -n "$FISH_VERSION" ]; then
        echo "fish"
    else
        # Fallback to checking SHELL environment variable
        case "$SHELL" in
            */bash)
                echo "bash"
                ;;
            */zsh)
                echo "zsh"
                ;;
            */fish)
                echo "fish"
                ;;
            *)
                echo "unknown"
                ;;
        esac
    fi
}

# Get shell RC file
get_shell_rc() {
    SHELL_TYPE="$1"
    case "$SHELL_TYPE" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                echo "$HOME/.bashrc"
            else
                echo "$HOME/.bash_profile"
            fi
            ;;
        zsh)
            echo "$HOME/.zshrc"
            ;;
        fish)
            echo "$HOME/.config/fish/config.fish"
            ;;
        *)
            echo ""
            ;;
    esac
}

# Add shell integration
add_shell_integration() {
    SHELL_TYPE=$(detect_shell)
    RC_FILE=$(get_shell_rc "$SHELL_TYPE")

    if [ -z "$RC_FILE" ]; then
        warn "Could not determine shell RC file. Please add the following to your shell configuration manually:"
        echo ""
        echo "  eval \"\$(try init)\""
        echo ""
        return
    fi

    # Check if already configured
    if [ -f "$RC_FILE" ] && grep -q "try init" "$RC_FILE"; then
        info "Shell integration already configured in $RC_FILE"
        return
    fi

    # Create config directory if needed (for fish)
    if [ "$SHELL_TYPE" = "fish" ]; then
        mkdir -p "$(dirname "$RC_FILE")"
    fi

    # Add integration
    info "Adding shell integration to $RC_FILE"

    if [ "$SHELL_TYPE" = "fish" ]; then
        echo "" >> "$RC_FILE"
        echo "# try - directory navigation tool" >> "$RC_FILE"
        echo "eval \"\$(try init | string collect)\"" >> "$RC_FILE"
    else
        echo "" >> "$RC_FILE"
        echo "# try - directory navigation tool" >> "$RC_FILE"
        echo "eval \"\$(try init)\"" >> "$RC_FILE"
    fi

    success "Shell integration added to $RC_FILE"
    info "Please restart your shell or run: source $RC_FILE"
}

# Main installation
main() {
    info "Installing try..."

    # Detect system
    OS=$(detect_os)
    ARCH=$(detect_arch)
    info "Detected: $OS ($ARCH)"

    # Get latest version
    info "Fetching latest version..."
    VERSION=$(get_latest_version)
    success "Latest version: $VERSION"

    # Construct download URL
    # Format: try-{version}-{os}-{arch}.tar.gz
    ARCHIVE_NAME="${BINARY_NAME}-${VERSION}-${OS}-${ARCH}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"

    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TMP_DIR"' EXIT

    # Download binary
    info "Downloading $ARCHIVE_NAME..."
    download "$DOWNLOAD_URL" "$TMP_DIR/$ARCHIVE_NAME" || error "Failed to download binary from $DOWNLOAD_URL"
    success "Downloaded successfully"

    # Extract archive
    info "Extracting archive..."
    tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$TMP_DIR" || error "Failed to extract archive"

    # Create install directory
    mkdir -p "$INSTALL_DIR" || error "Failed to create install directory $INSTALL_DIR"

    # The archive contains a directory, so we need to find the binary inside it
    EXTRACTED_DIR="$TMP_DIR/${ARCHIVE_NAME%.tar.gz}"

    # Install binary
    info "Installing to $INSTALL_DIR/$BINARY_NAME..."
    if [ -f "$EXTRACTED_DIR/$BINARY_NAME" ]; then
        mv "$EXTRACTED_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME" || error "Failed to move binary to $INSTALL_DIR"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
        success "Binary installed"
    else
        error "Binary not found in extracted archive. Expected: $EXTRACTED_DIR/$BINARY_NAME"
    fi

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            info "Add the following to your shell configuration:"
            echo ""
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            ;;
    esac

    # Add shell integration
    if [ "$SHELL_INTEGRATION" = true ]; then
        add_shell_integration
    fi

    echo ""
    success "Installation complete! 🎉"
    echo ""
    info "Quick start:"
    echo "  1. Restart your shell or run: source ~/.bashrc (or ~/.zshrc)"
    echo "  2. Run: try"
    echo "  3. Type to search/create directories"
    echo ""
    info "For more information, visit: https://github.com/${REPO}"
}

main "$@"
