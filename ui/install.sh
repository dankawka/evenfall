#!/usr/bin/env bash
# Build and install the evenfall desktop app for the current user.
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release

install -Dm755 target/release/evenfall-ui "$HOME/.local/bin/evenfall-ui"
install -Dm644 assets/evenfall.png "$HOME/.local/share/icons/hicolor/256x256/apps/evenfall.png"
install -Dm644 evenfall-ui.desktop "$HOME/.local/share/applications/evenfall-ui.desktop"

command -v update-desktop-database >/dev/null &&
    update-desktop-database "$HOME/.local/share/applications" || true
if [ -f "$HOME/.local/share/icons/hicolor/index.theme" ] &&
    command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true
fi

echo "installed $HOME/.local/bin/evenfall-ui"
