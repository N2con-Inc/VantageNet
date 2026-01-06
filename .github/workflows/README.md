# GitHub Actions CI Build

The `.github/workflows/build-deb.yml` workflow has been created to automate Debian package builds.

## Triggers

1. **On tag push** (recommended for releases)
   ```bash
   git tag v0.13.0
   git push origin v0.13.0
   ```
   This creates a GitHub release with attached .deb packages.

2. **On main branch push**
   Builds for testing, doesn't create release.

3. **Manual dispatch**
   Via GitHub Actions UI → "Run workflow"

## What It Does

| Step | Action |
|-------|--------|
| Checkout | Clones repository |
| Install Rust targets | Adds `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu` |
| Cache cargo | Speeds up builds |
| Build daemon | `cargo build --release --target <arch>` |
| Package .deb | Creates full Debian package structure |
| Upload artifacts | Makes .deb files available for download |
| Create release | Attaches .deb to GitHub release (tags only) |

## Matrix Build

Builds both architectures in parallel:
- **amd64** (x86_64) for Intel/AMD servers
- **arm64** (aarch64) for ARM servers like bowlister

## Artifacts

Each workflow run produces:
1. `vantagenet-daemon_<version>_amd64.deb`
2. `vantagenet-daemon_<version>_arm64.deb`
3. `SHA256SUMS` checksums file

## Downloading Packages

### From Workflow Run

1. Go to GitHub Actions tab
2. Select workflow run
3. Download artifacts at bottom

### From GitHub Release

```bash
wget https://github.com/N2con-Inc/VantageNet/releases/download/v0.13.0/vantagenet-daemon_0.13.0_arm64.deb
```

## Local Build Alternative

If you need to build locally on Linux:

```bash
# Install Rust targets
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu

# Run build script
./scripts/build-deb.sh arm64  # or amd64, or all

# Output: target/debian/vantagenet-daemon_0.13.0_arm64.deb
```

## Next Step: Deploy to bowlister

Once you have the .deb package (from CI or local build):

```bash
# Upload to bowlister
scp target/debian/vantagenet-daemon_0.13.0_arm64.deb legend@bowlister:/tmp/

# Install and configure
ssh legend@bowlister << 'EOF'
sudo dpkg -i /tmp/vantagenet-daemon_0.13.0_arm64.deb

# Configure
sudo nano /etc/vantagenet/daemon.toml
# Set: server_url = "http://your-server:60072"

# Start
sudo systemctl enable --now vantagenet-daemon

# Verify
sudo systemctl status vantagenet-daemon
sudo journalctl -u vantagenet-daemon -f
EOF
```

## Verification Checklist

After installation on bowlister:

- [ ] Service status: `systemctl status vantagenet-daemon` shows `active (running)`
- [ ] Capabilities: `systemctl show vantagenet-daemon | grep Capability` shows `CAP_NET_RAW CAP_NET_ADMIN`
- [ ] L2 access: Logs show "ARP scanning available" (not permission denied)
- [ ] Real MACs: Discovery shows physical MACs (00:xx:xx:xx:xx) NOT Docker bridge (02:42:...)
- [ ] Registration: Daemon connects to VantageNet server
