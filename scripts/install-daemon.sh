#!/bin/bash
# VantageNet Daemon One-Line Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/N2con-Inc/VantageNet/main/scripts/install-daemon.sh | sudo bash

set -e

VERSION="0.13.0"
ARCH=$(uname -m)
GITHUB_URL="https://github.com/N2con-Inc/VantageNet/releases/download/v${VERSION}"

# Map architecture to package name
case "$ARCH" in
    x86_64|amd64)
        DEB_ARCH="amd64"
        ;;
    aarch64|arm64)
        DEB_ARCH="arm64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        echo "Supported: x86_64/amd64, aarch64/arm64"
        exit 1
        ;;
esac

DEB_PACKAGE="vantagenet-daemon_${VERSION}_${DEB_ARCH}.deb"
DEB_URL="${GITHUB_URL}/${DEB_PACKAGE}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root (use sudo)"
    exit 1
fi

echo "=========================================="
echo "  VantageNet Daemon v${VERSION}"
echo "=========================================="
echo ""
echo "Downloading package for ${DEB_ARCH}..."

# Download .deb package
if ! curl -fsSL -o "/tmp/${DEB_PACKAGE}" "$DEB_URL"; then
    echo "Error: Failed to download package"
    echo "Please check: $DEB_URL"
    exit 1
fi

echo "Installing package..."
dpkg -i "/tmp/${DEB_PACKAGE}"

# Cleanup
rm -f "/tmp/${DEB_PACKAGE}"

echo ""
echo "=========================================="
echo "  Installation Complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo ""
echo "1. Edit the configuration:"
echo "   sudo nano /etc/vantagenet/daemon.toml"
echo ""
echo "2. Set your server_url:"
echo "   server_url = \"http://your-server:60072\""
echo ""
echo "3. Start the daemon:"
echo "   sudo systemctl enable --now vantagenet-daemon"
echo ""
echo "4. Check status:"
echo "   sudo systemctl status vantagenet-daemon"
echo ""
echo "5. View logs:"
echo "   sudo journalctl -u vantagenet-daemon -f"
echo ""
echo "Documentation: https://github.com/N2con-Inc/VantageNet"
