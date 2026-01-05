#!/bin/bash

# PIN Client Build Script

set -e

echo "==================================="
echo "  PIN Client Build Script"
echo "==================================="

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed."
    echo "Install it from: https://rustup.rs/"
    exit 1
fi

# Check for Tauri CLI
if ! command -v cargo-tauri &> /dev/null; then
    echo "Installing Tauri CLI..."
    cargo install tauri-cli
fi

cd "$(dirname "$0")"

echo ""
echo "Building PIN Client..."
echo ""

# Development build
if [ "$1" == "dev" ]; then
    echo "Starting development server..."
    cd src-tauri
    cargo tauri dev
else
    # Production build
    echo "Building release..."
    cd src-tauri
    cargo tauri build
    
    echo ""
    echo "==================================="
    echo "  Build Complete!"
    echo "==================================="
    echo ""
    echo "Binaries located in:"
    echo "  - macOS: src-tauri/target/release/bundle/macos/"
    echo "  - Windows: src-tauri/target/release/bundle/msi/"
    echo "  - Linux: src-tauri/target/release/bundle/deb/"
fi
