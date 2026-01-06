#!/bin/bash
set -e

# Build Debian packages for VantageNet daemon
# Supports: amd64 (x86_64) and arm64 (aarch64)
#
# Usage: ./scripts/build-deb.sh [amd64|arm64|all]
#
# Requirements:
#   - Rust with cross-compilation targets installed
#   - dpkg-deb (for packaging)

VERSION="0.13.0"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGING_DIR="$PROJECT_ROOT/packaging/debian"
OUTPUT_DIR="$PROJECT_ROOT/target/debian"
BACKEND_DIR="$PROJECT_ROOT/backend"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Parse arguments
ARCH="${1:-all}"

validate_arch() {
    case "$1" in
        amd64|arm64|all)
            return 0
            ;;
        *)
            log_warn "Unknown architecture: $1"
            echo "Usage: $0 [amd64|arm64|all]"
            exit 1
            ;;
    esac
}

check_rust_target() {
    local target=$1
    if rustup target list --installed | grep -q "$target"; then
        return 0
    else
        log_warn "Rust target $target not installed"
        log_info "Installing target: rustup target add $target"
        rustup target add "$target"
    fi
}

build_binary() {
    local arch=$1
    local target=$2

    log_info "Building $arch binary ($target)..."

    check_rust_target "$target"

    cd "$BACKEND_DIR"

    # Build optimized release binary
    cargo build \
        --release \
        --bin daemon \
        --target "$target"

    log_success "Built $arch binary"
}

create_package() {
    local arch=$1
    local target=$2

    log_info "Creating $arch Debian package..."

    local pkg_dir="$OUTPUT_DIR/vantagenet-daemon_${VERSION}_${arch}"
    local pkg_build_dir="$pkg_dir/build"

    # Clean up previous build
    rm -rf "$pkg_dir"
    mkdir -p "$pkg_build_dir"

    # Copy binary
    mkdir -p "$pkg_build_dir/usr/bin"
    cp "$BACKEND_DIR/target/$target/release/daemon" \
       "$pkg_build_dir/usr/bin/vantagenet-daemon"

    # Copy systemd service
    mkdir -p "$pkg_build_dir/lib/systemd/system"
    cp "$PACKAGING_DIR/vantagenet-daemon.service" \
       "$pkg_build_dir/lib/systemd/system/"

    # Copy config
    mkdir -p "$pkg_build_dir/etc/vantagenet"
    cp "$PACKAGING_DIR/daemon.toml.example" \
       "$pkg_build_dir/etc/vantagenet/daemon.toml"

    # Copy DEBIAN control files
    mkdir -p "$pkg_dir/DEBIAN"
    cp "$PACKAGING_DIR/control.$arch" "$pkg_dir/DEBIAN/control"
    cp "$PACKAGING_DIR/conffiles" "$pkg_dir/DEBIAN/"
    cp "$PACKAGING_DIR/postinst" "$pkg_dir/DEBIAN/"
    cp "$PACKAGING_DIR/prerm" "$pkg_dir/DEBIAN/"

    # Make scripts executable
    chmod 755 "$pkg_dir/DEBIAN/postinst"
    chmod 755 "$pkg_dir/DEBIAN/prerm"

    # Calculate installed size (in KB)
    local du_size=$(du -sk "$pkg_build_dir" | cut -f1)
    sed -i "s/^Installed-Size:.*/Installed-Size: $du_size/" "$pkg_dir/DEBIAN/control"

    # Build .deb package
    log_info "Packaging $arch..."
    dpkg-deb --build "$pkg_dir" "$OUTPUT_DIR/"

    log_success "Created: $OUTPUT_DIR/vantagenet-daemon_${VERSION}_${arch}.deb"

    # Cleanup
    rm -rf "$pkg_dir"
}

generate_checksums() {
    log_info "Generating SHA256 checksums..."

    cd "$OUTPUT_DIR"
    sha256sum vantagenet-daemon_*.deb > SHA256SUMS

    log_success "Checksums in: $OUTPUT_DIR/SHA256SUMS"
    cat SHA256SUMS
}

main() {
    validate_arch "$ARCH"

    log_info "Starting VantageNet daemon package build v${VERSION}"
    log_info "Project root: $PROJECT_ROOT"

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Build requested architectures
    case "$ARCH" in
        amd64)
            build_binary amd64 x86_64-unknown-linux-gnu
            create_package amd64 x86_64-unknown-linux-gnu
            ;;
        arm64)
            build_binary arm64 aarch64-unknown-linux-gnu
            create_package arm64 aarch64-unknown-linux-gnu
            ;;
        all)
            build_binary amd64 x86_64-unknown-linux-gnu
            build_binary arm64 aarch64-unknown-linux-gnu

            create_package amd64 x86_64-unknown-linux-gnu
            create_package arm64 aarch64-unknown-linux-gnu
            ;;
    esac

    generate_checksums

    log_success "Build complete! Packages in $OUTPUT_DIR"
}

main "$@"
