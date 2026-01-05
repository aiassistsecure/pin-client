#!/bin/bash

# PIN Client Build Script

set -e

echo ""
echo "     █████╗ ██╗ █████╗ ███████╗    ██████╗ ██╗███╗   ██╗"
echo "    ██╔══██╗██║██╔══██╗██╔════╝    ██╔══██╗██║████╗  ██║"
echo "    ███████║██║███████║███████╗    ██████╔╝██║██╔██╗ ██║"
echo "    ██╔══██║██║██╔══██║╚════██║    ██╔═══╝ ██║██║╚██╗██║"
echo "    ██║  ██║██║██║  ██║███████║    ██║     ██║██║ ╚████║"
echo "    ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝╚══════╝    ╚═╝     ╚═╝╚═╝  ╚═══╝"
echo ""
echo "    ╔════════════════════════════════════════════════════════╗"
echo "    ║      P2P Inference Network - Operator Client           ║"
echo "    ║                                                        ║"
echo "    ║              https://AiAssist.net                      ║"
echo "    ╚════════════════════════════════════════════════════════╝"
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "    [ERROR] Rust is not installed."
    echo ""
    echo "    Install Rust:"
    echo "      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    exit 1
fi

# Check for Tauri CLI
if ! command -v cargo-tauri &> /dev/null; then
    echo "    [INFO] Installing Tauri CLI..."
    cargo install tauri-cli
fi

cd "$(dirname "$0")"

# Development build
if [ "$1" == "dev" ]; then
    echo "    [MODE] Development"
    echo ""
    echo "    Starting development server..."
    cd src-tauri
    cargo tauri dev
else
    # Production build
    echo "    [MODE] Production Release"
    echo ""
    echo "    Building optimized binary..."
    echo ""
    cd src-tauri
    cargo tauri build
    
    echo ""
    echo "    ╔════════════════════════════════════════════════════════╗"
    echo "    ║                   BUILD SUCCESSFUL                     ║"
    echo "    ╚════════════════════════════════════════════════════════╝"
    echo ""
    echo "    Output binaries:"
    echo ""
    echo "      Linux (deb):     target/release/bundle/deb/*.deb"
    echo "      Linux (AppImage): target/release/bundle/appimage/*.AppImage"
    echo "      macOS:           target/release/bundle/macos/PIN Client.app"
    echo "      Windows:         target/release/bundle/msi/*.msi"
    echo ""
    echo "    Get your credentials at https://AiAssist.net/pin"
    echo ""
fi
