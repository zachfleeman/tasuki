#!/usr/bin/env bash

set -euo pipefail

REPO="zachfleeman/tasuki"
INSTALL_DIR="${TASUKI_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf "\033[0;34m%s\033[0m\n" "$1"; }
success() { printf "\033[0;32m%s\033[0m\n" "$1"; }
error() { printf "\033[0;31m%s\033[0m\n" "$1" >&2; exit 1; }

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    linux) OS="linux" ;;
    *) error "Unsupported OS: $OS — tasuki requires Linux (Waybar/Wayland)" ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

TARGET="${OS}-${ARCH}"
info "Detected platform: ${TARGET}"

# Get latest release tag
info "Fetching latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$LATEST" ]; then
    error "Failed to fetch latest release. Check https://github.com/${REPO}/releases"
fi
info "Latest version: ${LATEST}"

# Download binary
ASSET="tasuki-${TARGET}"
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}.tar.gz"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading ${URL}..."
if ! curl -fsSL "$URL" -o "${TMPDIR}/tasuki.tar.gz"; then
    error "Download failed. Binary may not exist for ${TARGET}.\nTry: cargo install --git https://github.com/${REPO}"
fi

# Extract and install
tar -xzf "${TMPDIR}/tasuki.tar.gz" -C "$TMPDIR"
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/tasuki" "${INSTALL_DIR}/tasuki"
chmod +x "${INSTALL_DIR}/tasuki"

success "Installed tasuki ${LATEST} to ${INSTALL_DIR}/tasuki"

# Download example config if no config exists
CONFIG_DIR="$HOME/.config/tasuki"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    mkdir -p "$CONFIG_DIR"
    curl -fsSL "https://github.com/${REPO}/raw/main/config.example.toml" \
        -o "${CONFIG_DIR}/config.toml" 2>/dev/null && \
        info "Created config at ${CONFIG_DIR}/config.toml"
fi

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^${INSTALL_DIR}$"; then
    echo ""
    info "Add ${INSTALL_DIR} to your PATH:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

# Detect terminal for Waybar on-click
if [ -n "${TERMINAL:-}" ]; then
    TERM_CMD="$TERMINAL"
elif command -v xdg-terminal-exec >/dev/null 2>&1; then
    TERM_CMD="xdg-terminal-exec"
else
    TERM_CMD=""
fi

ON_CLICK="${TERM_CMD:-<your-terminal>} -e tasuki tui"

# Waybar setup hint
echo ""
info "To add tasuki to Waybar, add this to your Waybar config:"
echo ""
echo '  "custom/tasks": {'
echo '      "exec": "tasuki",'
echo '      "return-type": "json",'
echo '      "format": "{}",'
echo "      \"on-click\": \"${ON_CLICK}\","
echo '      "interval": 30,'
echo '      "tooltip": true'
echo '  }'
echo ""
info "Then add \"custom/tasks\" to your bar modules list."
