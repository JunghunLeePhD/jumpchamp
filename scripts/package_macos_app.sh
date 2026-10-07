#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# JumpChamp macOS Application Bundle & Icon Packager
# Builds JumpChamp.app with embedded icons and ad-hoc code-signing
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${WORKSPACE_DIR}"

# Ensure ~/.cargo/bin is included in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Unset TERM to prevent cargo-bundle terminal styling issues in some subshells
unset TERM 2>/dev/null || true

# Check if cargo is installed
if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo is not found in PATH. Please install Rust from https://rustup.rs" >&2
    exit 1
fi

# Check and install cargo-bundle if missing
if ! command -v cargo-bundle >/dev/null 2>&1; then
    echo " cargo-bundle is not installed. Installing via 'cargo install cargo-bundle'..."
    cargo install cargo-bundle
fi

echo " Packaging JumpChamp.app with native icon..."
cargo bundle --release --format osx --bin jumpchamp_gui

APP_BUNDLE="target/release/bundle/osx/JumpChamp.app"

if [ -d "${APP_BUNDLE}" ]; then
    echo " Applying ad-hoc code signature for macOS (Apple Silicon & Intel)..."
    if command -v codesign >/dev/null 2>&1; then
        codesign --force --deep --sign - "${APP_BUNDLE}"
        echo " Signed successfully."
    else
        echo " Note: 'codesign' command not found in environment (skipped local signing)."
    fi
    echo " Packaging complete!"
    echo " App bundle location: ${WORKSPACE_DIR}/${APP_BUNDLE}"

    # Reveal in Finder if running on macOS with GUI
    if command -v open >/dev/null 2>&1 && [ "$(uname -s)" = "Darwin" ]; then
        open "$(dirname "${APP_BUNDLE}")"
    fi
else
    echo "Error: Bundle directory not found at ${APP_BUNDLE}" >&2
    exit 1
fi
