#!/usr/bin/env bash
# AlmanX install script
set -e

BINARY="almanx"
INSTALL_DIR="${HOME}/.local/bin"

echo "Building AlmanX in release mode..."
cargo build --release

mkdir -p "$INSTALL_DIR"
cp "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"

echo "Installed to $INSTALL_DIR/$BINARY"
echo ""
echo "Add the following to your shell config:"
echo ""
echo "  # bash (~/.bashrc)"
echo '  eval "$(almanx init bash)"'
echo ""
echo "  # zsh (~/.zshrc)"
echo '  eval "$(almanx init zsh)"'
echo ""
echo "  # fish (~/.config/fish/config.fish)"
echo "  almanx init fish | source"
