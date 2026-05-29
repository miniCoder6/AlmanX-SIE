#!/usr/bin/env bash
set -e

echo "Building Flux..."
cargo build --release -p flux-tui

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"
cp target/release/flux "$INSTALL_DIR/flux"
echo "Installed flux to $INSTALL_DIR/flux"

echo ""
echo "Add shell integration — choose your shell:"
echo ""
echo "  Bash:  Add to ~/.bashrc:"
echo '    eval "$(flux init bash)"'
echo ""
echo "  Zsh:   Add to ~/.zshrc:"
echo '    eval "$(flux init zsh)"'
echo ""
echo "  Fish:  Add to ~/.config/fish/config.fish:"
echo '    flux init fish | source'
echo ""
echo "Done! Restart your shell or source your rc file."
