#!/usr/bin/env bash
set -e

INSTALL_DIR="${HOME}/.local/bin"

if [ -f "$INSTALL_DIR/flux" ]; then
    rm -f "$INSTALL_DIR/flux"
    echo "Removed flux from $INSTALL_DIR"
fi

if command -v cargo &> /dev/null; then
    echo "Attempting to uninstall via cargo..."
    cargo uninstall flux 2>/dev/null || true
fi

echo "Flux has been completely uninstalled."
