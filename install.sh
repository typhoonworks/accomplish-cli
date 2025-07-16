#!/bin/bash

# Accomplish CLI Installation Script
# This script downloads and installs the latest version of the Accomplish CLI

set -e

# Configuration
REPO="typhoonworks/accomplish-cli"
BIN_NAME="acc"
INSTALL_DIR="/usr/local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

# Detect platform
detect_platform() {
    local platform
    case "$(uname -s)" in
        Linux*)  platform="x86_64-unknown-linux-gnu";;
        Darwin*) 
            case "$(uname -m)" in
                arm64) platform="aarch64-apple-darwin";;
                x86_64) platform="x86_64-apple-darwin";;
                *) error "Unsupported macOS architecture: $(uname -m)";;
            esac
            ;;
        CYGWIN*|MINGW*|MSYS*) platform="x86_64-pc-windows-msvc";;
        *) error "Unsupported platform: $(uname -s)";;
    esac
    echo "$platform"
}

# Get latest release version
get_latest_version() {
    curl -s "https://api.github.com/repos/$REPO/releases/latest" | \
        grep '"tag_name":' | \
        sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install
main() {
    info "Starting Accomplish CLI installation..."
    
    # Check if curl is available
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed. Please install curl and try again."
    fi
    
    # Detect platform
    local platform=$(detect_platform)
    info "Detected platform: $platform"
    
    # Get latest version
    local version=$(get_latest_version)
    if [ -z "$version" ]; then
        error "Failed to get latest version information"
    fi
    info "Latest version: $version"
    
    # Determine binary name and download URL
    local binary_name="$BIN_NAME"
    if [[ "$platform" == *"windows"* ]]; then
        binary_name="$BIN_NAME.exe"
    fi
    
    local download_url="https://github.com/$REPO/releases/download/$version/$BIN_NAME-$platform"
    if [[ "$platform" == *"windows"* ]]; then
        download_url="$download_url.exe"
    fi
    
    # Create temporary directory
    local temp_dir=$(mktemp -d)
    local temp_file="$temp_dir/$binary_name"
    
    # Download binary
    info "Downloading $download_url..."
    if ! curl -L -o "$temp_file" "$download_url"; then
        error "Failed to download binary"
    fi
    
    # Make binary executable
    chmod +x "$temp_file"
    
    # Install binary
    info "Installing to $INSTALL_DIR..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$temp_file" "$INSTALL_DIR/$BIN_NAME"
    else
        # Try with sudo
        if command -v sudo &> /dev/null; then
            sudo mv "$temp_file" "$INSTALL_DIR/$BIN_NAME"
        else
            error "Cannot write to $INSTALL_DIR and sudo is not available"
        fi
    fi
    
    # Cleanup
    rm -rf "$temp_dir"
    
    # Verify installation
    if command -v "$BIN_NAME" &> /dev/null; then
        info "Successfully installed Accomplish CLI!"
        info "Run '$BIN_NAME --help' to get started."
    else
        warn "Installation completed but '$BIN_NAME' is not in your PATH."
        warn "You may need to add $INSTALL_DIR to your PATH or restart your terminal."
    fi
}

# Run main function
main "$@"