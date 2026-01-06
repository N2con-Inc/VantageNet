# P0: Native Daemon Package - Deliverables

## Overview

This document describes the Debian packaging infrastructure created for VantageNet daemon.

## Why Native (Not Docker)?

Docker networking prevents proper Layer 2 visibility:
- Container sees Docker bridge MACs, not real device MACs
- ARP scanning cannot reach physical devices
- LLDP/CDP passive listening doesn't work
- DHCP monitoring misses host traffic

## Package Structure

```
packaging/debian/
├── DEBIAN/                         # Generated during build
│   ├── control                    # Package metadata
│   ├── conffiles                  # User-editable config files
│   ├── postinst                   # Post-installation script
│   └── prerm                      # Pre-removal script
├── vantagenet-daemon.service    # systemd service file
└── daemon.toml.example           # Default configuration template

scripts/
├── build-deb.sh               # Build & package script
└── install-daemon.sh            # One-line installation
```

## Files Created

### 1. Package Control Files

**control.amd64** - Package metadata for x86_64
- Depends: libc6, libssl3
- Recommends: postgresql-client
- Conflicts: scanopy-daemon

**control.arm64** - Package metadata for aarch64
- Same dependencies as amd64

**conffiles** - Marks `/etc/vantagenet/daemon.toml` as user-editable
- Config preserved across upgrades

**postinst** - Runs after installation
- Reloads systemd daemon
- Prints setup instructions
- Doesn't auto-start (user must configure first)

**prerm** - Runs before removal
- Stops daemon gracefully

### 2. Systemd Service

**vantagenet-daemon.service**
- Security hardening:
  - `AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN`
  - `CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN`
  - `NoNewPrivileges=yes`
  - `ProtectSystem=strict`
  - `ProtectHome=yes`
- Requires `network-online.target`
- Runs as root (needed for CAP_NET_RAW)

### 3. Configuration File

**daemon.toml.example** - Default configuration
All configurable options with comments:
- server_url
- mode (Push/Pull)
- port, bind_address
- log_level, heartbeat_interval
- concurrent_scans
- Docker proxy settings

### 4. Build Script

**scripts/build-deb.sh**
Usage: `./scripts/build-deb.sh [amd64|arm64|all]`

Features:
- Auto-installs Rust cross-compilation targets
- Builds optimized release binary (strip, LTO)
- Creates full .deb package structure
- Calculates installed size dynamically
- Generates SHA256 checksums

Output: `target/debian/vantagenet-daemon_${VERSION}_${ARCH}.deb`

### 5. Installation Script

**scripts/install-daemon.sh**
Usage: `curl -fsSL https://.../install-daemon.sh | sudo bash`

Features:
- Auto-detects architecture (x86_64/aarch64)
- Downloads correct .deb from GitHub releases
- Installs via dpkg
- Prints setup instructions

## Capabilities Required

| Capability | Purpose |
|-------------|----------|
| CAP_NET_RAW | Raw socket access for ARP, LLDP, CDP |
| CAP_NET_ADMIN | Modify routing tables, interface configuration |

## How to Build

### Prerequisites
```bash
# Install Rust cross-compilation targets
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu

# Ensure dpkg-deb is installed (Linux)
# On macOS, cannot build .deb packages - build on Linux host or use cross
```

### Build Commands
```bash
# Build specific architecture
./scripts/build-deb.sh arm64   # For bowlister
./scripts/build-deb.sh amd64   # For x86_64 servers

# Build both
./scripts/build-deb.sh all
```

### Expected Output
```
target/debian/
├── vantagenet-daemon_0.13.0_amd64.deb
├── vantagenet-daemon_0.13.0_arm64.deb
└── SHA256SUMS
```

## Installation on Target (bowlister)

### Option 1: One-Liner
```bash
curl -fsSL https://raw.githubusercontent.com/N2con-Inc/VantageNet/main/scripts/install-daemon.sh | sudo bash
```

### Option 2: Manual Install
```bash
# Upload .deb to bowlister
scp target/debian/vantagenet-daemon_0.13.0_arm64.deb legend@bowlister:/tmp/

# Install on bowlister
ssh legend@bowlister
sudo dpkg -i /tmp/vantagenet-daemon_0.13.0_arm64.deb
```

### Post-Install Steps
```bash
# 1. Configure
sudo nano /etc/vantagenet/daemon.toml
# Set: server_url = "http://your-server:60072"

# 2. Start
sudo systemctl enable --now vantagenet-daemon

# 3. Verify
sudo systemctl status vantagenet-daemon
sudo journalctl -u vantagenet-daemon -f
```

## Testing Checklist

- [ ] Cross-compile builds successfully (aarch64)
- [ ] .deb package installs without errors
- [ ] systemd service starts with CAP_NET_RAW/CAP_NET_ADMIN
- [ ] Daemon config file created at `/etc/vantagenet/daemon.toml`
- [ ] ARP scanning works (can open raw sockets)
- [ ] Daemon registers with VantageNet server
- [ ] Discovery finds hosts with **real MAC addresses** (not Docker bridge MACs)

## Known Limitations

1. **Cross-compilation on macOS**: Building .deb packages requires dpkg-deb which is Linux-only
   - Solution: Build on Linux host, or build .deb in CI/VM
   - The `build-deb.sh` script assumes Linux for packaging

2. **Rust toolchain**: Cross-compilation targets must be installed
   - `rustup target add aarch64-unknown-linux-gnu`
   - Dependencies must compile for target (OpenSSL, etc.)

## Next Steps

1. Build on Linux host or set up CI pipeline
2. Test package installation on fresh Debian 13 VM/container
3. Verify L2 capabilities work correctly
4. Create GitHub release v0.13.0 with packages attached
5. Update documentation with installation instructions

## Files Modified/Created

- **Created**: `packaging/debian/control.{amd64,arm64}`
- **Created**: `packaging/debian/conffiles`
- **Created**: `packaging/debian/postinst`
- **Created**: `packaging/debian/prerm`
- **Created**: `packaging/debian/vantagenet-daemon.service`
- **Created**: `packaging/debian/daemon.toml.example`
- **Created**: `scripts/build-deb.sh`
- **Created**: `scripts/install-daemon.sh`
- **Updated**: `WORKPLAN.md` (added nmap as P5 enhancement)
