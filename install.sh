#!/usr/bin/env bash
# ─── AlmanX Install Script ────────────────────────────────────────────────────
set -euo pipefail

BINARY="almanx"
INSTALL_DIR="${HOME}/.local/bin"

echo ""
echo "┌─────────────────────────────────────┐"
echo "│  AlmanX — Shell Intelligence Engine │"
echo "│  Installation Script                │"
echo "└─────────────────────────────────────┘"
echo ""

# Check for Rust
if ! command -v cargo &>/dev/null; then
    echo "ERROR: Rust not found. Install from https://rustup.rs"
    exit 1
fi

echo "→ Building in release mode (this takes ~30s on first build)..."
cargo build --release

mkdir -p "$INSTALL_DIR"
cp "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"

echo ""
echo "✓ Installed to: $INSTALL_DIR/$BINARY"
echo ""

# Warn if not in PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "⚠  Add to your shell config:"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo "── Shell Integration ─────────────────────────────────────────────────"
echo ""
echo "  Add ONE of the following to your shell config, then reload:"
echo ""
echo "  # Bash (~/.bashrc)"
echo '  eval "$(almanx init bash)"'
echo ""
echo "  # Zsh (~/.zshrc)"
echo '  eval "$(almanx init zsh)"'
echo ""
echo "  # Fish (~/.config/fish/config.fish)"
echo "  almanx init fish | source"
echo ""
echo "── Quick Start After Setup ───────────────────────────────────────────"
echo ""
echo "  almanx              — open interactive TUI"
echo "  almanx suggest      — get alias suggestions"
echo "  almanx stats        — productivity analytics"
echo "  almanx search <q>   — fuzzy search history"
echo "  almanx workflows    — view mined workflows"
echo ""
echo "  Seed test data:  bash integration_tests/seed_data.sh"
echo "  Run tests:       bash integration_tests/test_hooks.sh"
echo ""
