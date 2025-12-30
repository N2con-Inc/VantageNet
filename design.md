# VantageNet: Enhanced Network Discovery & Monitoring Platform

**Project Plan - Scanopy Fork**  
**Version:** 1.0  
**Date:** December 30, 2024  
**Status:** Planning

---

## Executive Summary

This document outlines the development plan for **VantageNet**, a fork of the open-source Scanopy project. VantageNet extends Scanopy's network discovery and visualization capabilities with enterprise MSP features including multi-tenant client management, certificate monitoring, VPN detection, change alerting, and AI-assisted service identification.

The project targets Debian 13 as the primary daemon platform with native systemd installation, moving away from Docker-only deployments to enable full network visibility including Layer 2 access.

---

## Architecture Overview

### Current Scanopy Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Scanopy Server                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Svelte UI │  │ Rust Backend│  │  PostgreSQL │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────┬───────────────────────────────────┘
                          │ API
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│   Daemon 1    │ │   Daemon 2    │ │   Daemon 3    │
│  (Docker)     │ │  (Docker)     │ │  (Docker)     │
└───────────────┘ └───────────────┘ └───────────────┘
```

### VantageNet Target Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         VantageNet Server                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Svelte UI│ │  Rust    │ │PostgreSQL│ │  Redis   │ │TimescaleDB│      │
│  │          │ │  Backend │ │          │ │ (cache)  │ │(metrics) │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
│                     │                                                     │
│  ┌──────────────────┴────────────────────────────────────────┐           │
│  │                    New Modules                             │           │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌─────────┐ │           │
│  │  │ Cert       │ │ VPN/Route  │ │ Change     │ │ AI      │ │           │
│  │  │ Monitor    │ │ Monitor    │ │ Detector   │ │ Processor│ │           │
│  │  └────────────┘ └────────────┘ └────────────┘ └─────────┘ │           │
│  └───────────────────────────────────────────────────────────┘           │
└─────────────────────────────┬────────────────────────────────────────────┘
                              │ API + WebSocket
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│   VantageNet      │   │   VantageNet      │   │   VantageNet      │
│   Daemon        │   │   Daemon        │   │   Daemon        │
│   (Native)      │   │   (Native)      │   │   (Native)      │
│                 │   │                 │   │                 │
│ ┌─────────────┐ │   │ ┌─────────────┐ │   │ ┌─────────────┐ │
│ │mDNS/Bonjour│ │   │ │SNMP Poller │ │   │ │Cert Scanner │ │
│ │LLDP/CDP    │ │   │ │DHCP Monitor│ │   │ │SSH Keys    │ │
│ │MTR/Latency │ │   │ │Route Watch │ │   │ │Screenshot  │ │
│ └─────────────┘ │   └─────────────┘ │   │ └─────────────┘ │
└─────────────────┘   └─────────────────┘   └─────────────────┘
       │                     │                     │
   Client A              Client B              Client C
   Site: HQ              Site: Branch          Site: Remote
```

---

## Data Model

### Hierarchy

```
Organization (MSP)
└── Client (Tenant - Isolated)
    └── Site / Location
        └── Network
            └── Subnet
                └── Host
                    ├── Interfaces
                    ├── Services
                    ├── Certificates
                    └── SSH Keys
```

### Key Entities

| Entity | Description |
|--------|-------------|
| **Client** | Top-level tenant, completely isolated from other clients |
| **Site** | Physical or logical location (e.g., "HQ", "Branch Office", "AWS US-East") |
| **Network** | Logical network boundary, can span subnets |
| **Subnet** | IP address range, belongs to one Network |
| **Host** | Discovered device, may have multiple interfaces |
| **Interface** | Network interface with IP/MAC, links Host to Subnet |
| **Service** | Running service on a port |
| **Certificate** | TLS certificate with chain, expiry, trust status |
| **VPN Link** | Connection between two Networks with route/latency data |

---

## Phase 1: Foundation (Weeks 1-4)

**Goal:** Establish the forked codebase, implement multi-tenant hierarchy, and create native Debian daemon installation.

### 1.1 Repository Setup

- [ ] Fork Scanopy repository
- [ ] Rename to VantageNet throughout codebase
- [ ] Set up CI/CD pipelines (GitHub Actions)
- [ ] Create development environment documentation
- [ ] Establish branching strategy (main, develop, feature/*)

### 1.2 Multi-Tenant Data Model

**Database Schema Changes:**

```sql
-- New tables
CREATE TABLE clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    settings JSONB DEFAULT '{}',
    retention_days INTEGER DEFAULT 365,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE sites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID REFERENCES clients(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    address TEXT,
    timezone VARCHAR(50),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Modify existing networks table
ALTER TABLE networks ADD COLUMN site_id UUID REFERENCES sites(id);
ALTER TABLE networks ADD COLUMN client_id UUID REFERENCES clients(id);

-- Add client isolation to all queries via RLS or application layer
```

**API Changes:**
- All existing endpoints prefixed with `/api/v2/clients/{client_id}/...`
- Add client CRUD endpoints
- Add site CRUD endpoints
- Modify daemon registration to include client assignment

### 1.3 Native Debian Daemon

**Package Structure:**

```
vantagenet-daemon/
├── DEBIAN/
│   ├── control
│   ├── postinst
│   ├── prerm
│   └── conffiles
├── etc/
│   └── vantagenet/
│       └── daemon.toml
├── lib/
│   └── systemd/
│       └── system/
│           └── vantagenet-daemon.service
└── usr/
    └── bin/
        └── vantagenet-daemon
```

**Installation Script:**

```bash
#!/bin/bash
# install.sh - Bootstrap installer for VantageNet daemon

set -euo pipefail

REPO_URL="https://github.com/yourorg/vantagenet"
VERSION="${1:-latest}"

# Add apt repository
curl -fsSL https://packages.vantagenet.io/gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/vantagenet.gpg
echo "deb [signed-by=/usr/share/keyrings/vantagenet.gpg] https://packages.vantagenet.io/debian stable main" | \
    sudo tee /etc/apt/sources.list.d/vantagenet.list

# Install
sudo apt update
sudo apt install -y vantagenet-daemon

# Interactive configuration
sudo vantagenet-daemon configure
```

**Systemd Service:**

```ini
[Unit]
Description=VantageNet Network Discovery Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=vantagenet
Group=vantagenet
ExecStart=/usr/bin/vantagenet-daemon
Restart=always
RestartSec=10
Environment=VANTAGENET_CONFIG=/etc/vantagenet/daemon.toml

# Security hardening
CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN

[Install]
WantedBy=multi-user.target
```

### 1.4 Auto-Update Mechanism

**Server-Initiated Updates:**

```
Server                          Daemon
   │                               │
   │─── Check version ────────────▶│
   │◀── Current: 1.2.3 ────────────│
   │                               │
   │─── Update available: 1.2.4 ──▶│
   │    Download from GitHub       │
   │                               │
   │◀── Downloading... ────────────│
   │◀── Restarting... ─────────────│
   │◀── Online: 1.2.4 ─────────────│
```

**Implementation:**
- Server tracks daemon versions
- Server sends update notification via WebSocket
- Daemon downloads new binary from GitHub releases
- Daemon verifies checksum/signature
- Daemon restarts itself via systemd

### Phase 1 Deliverables

| Deliverable | Description |
|-------------|-------------|
| Forked repo | Clean fork with VantageNet branding |
| Schema migration | Client/Site hierarchy in PostgreSQL |
| API v2 | Client-scoped endpoints |
| .deb package | Installable Debian package |
| Auto-updater | GitHub-based daemon updates |
| Documentation | Installation and configuration guide |

---

## Phase 2: Discovery Enhancements (Weeks 5-8)

**Goal:** Expand discovery capabilities beyond port scanning to include service discovery protocols and network device polling.

### 2.1 mDNS/DNS-SD (Bonjour)

**Implementation:**

```rust
// New discovery module
pub struct MdnsDiscovery {
    socket: UdpSocket,
    discovered: HashMap<String, MdnsService>,
}

impl MdnsDiscovery {
    pub async fn discover(&mut self) -> Vec<DiscoveredService> {
        // Send mDNS query for common service types
        let queries = vec![
            "_http._tcp.local",
            "_https._tcp.local",
            "_ssh._tcp.local",
            "_printer._tcp.local",
            "_airplay._tcp.local",
            "_googlecast._tcp.local",
            "_homekit._tcp.local",
            "_smb._tcp.local",
            "_ipp._tcp.local",
        ];
        
        for query in queries {
            self.send_query(query).await;
        }
        
        self.collect_responses().await
    }
}
```

**Data Captured:**
- Service type and name
- Hostname and IP
- Port
- TXT records (often contain version info)

### 2.2 SNMP Polling

**Configuration:**

```toml
[snmp]
enabled = true
communities = ["public", "private"]  # v1/v2c
v3_users = [
    { username = "monitor", auth = "SHA", priv = "AES" }
]
poll_interval = 300  # seconds

# OIDs to poll
[[snmp.oid_groups]]
name = "system"
oids = [
    "1.3.6.1.2.1.1.1.0",   # sysDescr
    "1.3.6.1.2.1.1.3.0",   # sysUptime
    "1.3.6.1.2.1.1.5.0",   # sysName
    "1.3.6.1.2.1.1.6.0",   # sysLocation
]

[[snmp.oid_groups]]
name = "interfaces"
table = "1.3.6.1.2.1.2.2"  # ifTable
```

**Data Captured:**
- Device identification (vendor, model, firmware)
- Interface table (ports, speeds, status)
- ARP tables (MAC-IP mappings)
- Routing tables
- VLAN assignments

### 2.3 LLDP/CDP Parsing

**Capture Method:**

```rust
// Requires CAP_NET_RAW
pub struct LldpListener {
    socket: RawSocket,
}

impl LldpListener {
    pub async fn capture_neighbors(&self) -> Vec<LldpNeighbor> {
        // Listen for LLDP frames (ethertype 0x88CC)
        // Listen for CDP frames (SNAP, OUI 0x00000C)
        
        // Parse TLVs for:
        // - Chassis ID
        // - Port ID
        // - System Name
        // - System Description
        // - Management Address
        // - Port VLAN ID
    }
}
```

**Value:** Physical topology mapping without switch credentials.

### 2.4 DHCP Lease Monitoring

**Approaches:**

1. **Passive monitoring** (preferred): Listen for DHCP traffic on local segment
2. **DHCP server integration**: Query ISC DHCP, Kea, Windows DHCP via API
3. **ARP table polling**: Detect new MAC addresses

**Data Captured:**
- MAC to IP assignments
- Lease duration
- Hostname (from DHCP option 12)
- Vendor class (from option 60)
- New device detection

### 2.5 DNS Zone Awareness

**Features:**
- Forward lookup validation (hostname → IP matches discovered IP?)
- Reverse DNS validation (PTR records exist and match?)
- DNS response time monitoring
- Split-horizon detection

### Phase 2 Deliverables

| Deliverable | Description |
|-------------|-------------|
| mDNS discovery | Bonjour/Avahi service discovery |
| SNMP module | v1/v2c/v3 polling with configurable OIDs |
| LLDP/CDP parser | Physical topology from L2 protocols |
| DHCP monitor | Lease tracking and new device detection |
| DNS validator | Forward/reverse consistency checking |

---

## Phase 3: Certificate & Key Monitoring (Weeks 9-12)

**Goal:** Comprehensive TLS certificate and SSH key tracking across all protocols.

### 3.1 TLS Certificate Scanner

**Supported Protocols:**

| Protocol | Port(s) | Method |
|----------|---------|--------|
| HTTPS | 443, 8443, etc. | Direct TLS |
| IMAPS | 993 | Direct TLS |
| SMTPS | 465 | Direct TLS |
| SMTP+STARTTLS | 25, 587 | STARTTLS upgrade |
| LDAPS | 636 | Direct TLS |
| LDAP+STARTTLS | 389 | STARTTLS upgrade |
| PostgreSQL | 5432 | STARTTLS upgrade |
| MySQL | 3306 | STARTTLS upgrade |
| MongoDB | 27017 | Direct TLS |
| Redis | 6379 | Direct TLS (if enabled) |
| RDP | 3389 | TLS wrapper |

**Certificate Data Model:**

```sql
CREATE TABLE certificates (
    id UUID PRIMARY KEY,
    host_id UUID REFERENCES hosts(id),
    service_id UUID REFERENCES services(id),
    
    -- Certificate details
    serial_number TEXT NOT NULL,
    subject_cn TEXT,
    subject_full TEXT,
    issuer_cn TEXT,
    issuer_full TEXT,
    
    -- SANs
    san_dns TEXT[],
    san_ip INET[],
    san_email TEXT[],
    
    -- Validity
    not_before TIMESTAMPTZ,
    not_after TIMESTAMPTZ,
    
    -- Chain
    chain_depth INTEGER,
    chain_pem TEXT[],  -- Full chain as array of PEMs
    
    -- Trust assessment
    is_self_signed BOOLEAN,
    is_expired BOOLEAN,
    is_publicly_trusted BOOLEAN,
    trust_issues TEXT[],  -- e.g., ["expired", "wrong_host", "weak_signature"]
    
    -- Fingerprints
    fingerprint_sha256 TEXT,
    fingerprint_sha1 TEXT,
    
    -- Metadata
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    last_changed TIMESTAMPTZ
);

CREATE INDEX idx_certs_expiry ON certificates(not_after);
CREATE INDEX idx_certs_host ON certificates(host_id);
```

**Trust Assessment Logic:**

```rust
pub fn assess_trust(cert: &X509, chain: &[X509], hostname: &str) -> TrustAssessment {
    let mut issues = Vec::new();
    
    // Check expiry
    if cert.not_after() < Utc::now() {
        issues.push("expired");
    } else if cert.not_after() < Utc::now() + Duration::days(30) {
        issues.push("expiring_soon");
    }
    
    // Check hostname match
    if !cert.verify_hostname(hostname) {
        issues.push("hostname_mismatch");
    }
    
    // Check self-signed
    if cert.issuer() == cert.subject() {
        issues.push("self_signed");
    }
    
    // Check signature algorithm
    if cert.signature_algorithm().is_weak() {
        issues.push("weak_signature");
    }
    
    // Check against Mozilla root store
    let is_trusted = verify_chain_against_roots(chain);
    
    TrustAssessment {
        is_publicly_trusted: is_trusted && issues.is_empty(),
        issues,
    }
}
```

### 3.2 SSH Key Fingerprint Tracking

**Data Model:**

```sql
CREATE TABLE ssh_host_keys (
    id UUID PRIMARY KEY,
    host_id UUID REFERENCES hosts(id),
    service_id UUID REFERENCES services(id),
    
    key_type TEXT NOT NULL,  -- rsa, ecdsa, ed25519
    key_bits INTEGER,
    fingerprint_sha256 TEXT NOT NULL,
    fingerprint_md5 TEXT,
    public_key TEXT,
    
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    last_changed TIMESTAMPTZ
);
```

**Detection:**

```rust
pub async fn probe_ssh_keys(host: &str, port: u16) -> Vec<SshHostKey> {
    // Use SSH protocol to fetch host keys without authentication
    // Similar to `ssh-keyscan`
    
    let algorithms = ["ssh-ed25519", "ecdsa-sha2-nistp256", "ssh-rsa"];
    let mut keys = Vec::new();
    
    for algo in algorithms {
        if let Ok(key) = fetch_host_key(host, port, algo).await {
            keys.push(key);
        }
    }
    
    keys
}
```

### 3.3 Certificate Change Detection

**Events Tracked:**
- New certificate (first seen)
- Certificate renewed (same subject, new serial)
- Certificate changed (different subject or issuer)
- Certificate expired
- Certificate expiring soon (30, 14, 7, 1 day warnings)
- Trust status changed

### Phase 3 Deliverables

| Deliverable | Description |
|-------------|-------------|
| TLS scanner | Multi-protocol certificate extraction |
| Chain validator | Full chain storage and trust assessment |
| SSH key tracker | Host key fingerprint monitoring |
| Expiry alerts | Configurable expiration warnings |
| Change detection | Certificate/key change events |

---

## Phase 4: VPN & Connectivity Monitoring (Weeks 13-16)

**Goal:** Detect VPN connections between sites, monitor connectivity health, and track route changes.

### 4.1 VPN Detection

**Detection Methods:**

1. **Route analysis**: Look for routes to private IP ranges via non-default gateways
2. **Interface inspection**: Identify tunnel interfaces (tun0, wg0, etc.)
3. **Latency profiling**: Higher latency + encryption = likely VPN
4. **Cross-daemon correlation**: Daemon A sees route to 10.20.0.0/24, Daemon B is on 10.20.0.0/24

**Route Monitoring:**

```rust
pub struct RouteMonitor {
    routes: HashMap<IpNetwork, RouteEntry>,
}

impl RouteMonitor {
    pub async fn get_routes(&self) -> Vec<RouteEntry> {
        // Parse /proc/net/route (Linux)
        // or use netlink for real-time updates
    }
    
    pub fn detect_vpn_routes(&self) -> Vec<VpnCandidate> {
        self.routes.values()
            .filter(|r| {
                r.destination.is_private() &&
                r.gateway != self.default_gateway &&
                r.interface.starts_with("tun") || 
                r.interface.starts_with("wg") ||
                r.interface.starts_with("tap")
            })
            .collect()
    }
}
```

### 4.2 MTR/Latency Probes

**Periodic Probing:**

```toml
[connectivity]
# Probe targets for this daemon
probe_targets = [
    { host = "10.20.0.1", name = "Branch Office Gateway", interval = 60 },
    { host = "8.8.8.8", name = "Google DNS", interval = 300 },
]

# MTR settings
mtr_count = 10
mtr_interval = 1.0
```

**Data Captured:**

```sql
CREATE TABLE connectivity_probes (
    id UUID PRIMARY KEY,
    daemon_id UUID REFERENCES daemons(id),
    target_host TEXT NOT NULL,
    target_name TEXT,
    
    -- Results
    probe_time TIMESTAMPTZ NOT NULL,
    latency_min_ms FLOAT,
    latency_avg_ms FLOAT,
    latency_max_ms FLOAT,
    latency_stddev_ms FLOAT,
    packet_loss_pct FLOAT,
    hop_count INTEGER,
    
    -- Full MTR data
    hops JSONB  -- Array of {hop, host, latency, loss}
);

-- For time-series queries
SELECT time_bucket('5 minutes', probe_time) AS bucket,
       avg(latency_avg_ms) AS latency,
       avg(packet_loss_pct) AS loss
FROM connectivity_probes
WHERE target_host = '10.20.0.1'
GROUP BY bucket
ORDER BY bucket;
```

### 4.3 VPN Link Visualization

**Data Model:**

```sql
CREATE TABLE vpn_links (
    id UUID PRIMARY KEY,
    
    -- Endpoints
    network_a_id UUID REFERENCES networks(id),
    network_b_id UUID REFERENCES networks(id),
    
    -- Detection source
    detected_by_daemon_id UUID REFERENCES daemons(id),
    detection_method TEXT,  -- 'route', 'interface', 'correlation'
    
    -- Link details
    tunnel_type TEXT,  -- 'wireguard', 'openvpn', 'ipsec', 'unknown'
    local_endpoint TEXT,
    remote_endpoint TEXT,
    
    -- Health
    status TEXT DEFAULT 'unknown',  -- 'up', 'down', 'degraded', 'unknown'
    last_probe_time TIMESTAMPTZ,
    current_latency_ms FLOAT,
    current_loss_pct FLOAT,
    
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW()
);
```

### 4.4 Alert Conditions

| Condition | Threshold | Severity |
|-----------|-----------|----------|
| Latency spike | >2x baseline | Warning |
| Latency sustained high | >100ms for 5min | Warning |
| Packet loss | >1% | Warning |
| Packet loss severe | >10% | Critical |
| Route changed | Any change | Info |
| VPN down | No response | Critical |
| New hop detected | MTR path changed | Info |

### Phase 4 Deliverables

| Deliverable | Description |
|-------------|-------------|
| Route monitor | Real-time route table watching |
| VPN detector | Tunnel interface and route analysis |
| MTR prober | Periodic latency measurement |
| Link status | VPN health dashboard |
| Latency alerts | Spike and degradation detection |

---

## Phase 5: Change Detection & Alerting (Weeks 17-20)

**Goal:** Monitor for changes across all tracked entities and provide in-app alerting.

### 5.1 Change Detection Engine

**Entities Monitored:**

| Entity | Changes Detected |
|--------|------------------|
| Host | New, removed, IP changed, MAC changed, hostname changed |
| Service | New, removed, port changed, version changed |
| Certificate | New, renewed, changed, expired, expiring |
| SSH Key | New, changed |
| Route | New, removed, gateway changed, metric changed |
| VPN Link | New, down, latency degraded |
| DNS | PTR mismatch, resolution failure |
| DHCP | New lease, lease expired, MAC conflict |

**Change Event Schema:**

```sql
CREATE TABLE change_events (
    id UUID PRIMARY KEY,
    client_id UUID REFERENCES clients(id),
    site_id UUID REFERENCES sites(id),
    network_id UUID REFERENCES networks(id),
    
    -- What changed
    entity_type TEXT NOT NULL,  -- 'host', 'service', 'certificate', etc.
    entity_id UUID NOT NULL,
    change_type TEXT NOT NULL,  -- 'created', 'modified', 'deleted'
    
    -- Change details
    field_name TEXT,           -- Which field changed (null for create/delete)
    old_value JSONB,
    new_value JSONB,
    
    -- Context
    detected_by_daemon_id UUID REFERENCES daemons(id),
    detected_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Alert status
    is_alertable BOOLEAN DEFAULT false,
    alert_severity TEXT,       -- 'info', 'warning', 'critical'
    acknowledged BOOLEAN DEFAULT false,
    acknowledged_by UUID,
    acknowledged_at TIMESTAMPTZ
);

CREATE INDEX idx_changes_client ON change_events(client_id, detected_at DESC);
CREATE INDEX idx_changes_unacked ON change_events(client_id) WHERE NOT acknowledged;
```

### 5.2 Alert Configuration

**Per-Client Alert Rules:**

```sql
CREATE TABLE alert_rules (
    id UUID PRIMARY KEY,
    client_id UUID REFERENCES clients(id),
    
    name TEXT NOT NULL,
    enabled BOOLEAN DEFAULT true,
    
    -- Matching
    entity_type TEXT,          -- null = all types
    entity_filter JSONB,       -- e.g., {"tags": ["production"]}
    change_types TEXT[],       -- e.g., ['modified', 'deleted']
    field_names TEXT[],        -- e.g., ['ip_address', 'mac_address']
    
    -- Alert settings
    severity TEXT DEFAULT 'warning',
    suppress_duration INTERVAL,  -- Don't re-alert for this long
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Example Rules:**

```json
{
  "name": "Production host changes",
  "entity_type": "host",
  "entity_filter": {"tags": ["production"]},
  "change_types": ["modified", "deleted"],
  "severity": "critical"
}

{
  "name": "Certificate expiring",
  "entity_type": "certificate",
  "change_types": ["modified"],
  "field_names": ["days_until_expiry"],
  "entity_filter": {"days_until_expiry": {"$lt": 30}},
  "severity": "warning"
}
```

### 5.3 In-App Alert UI

**Alert Dashboard Components:**
- Alert feed (chronological)
- Filter by client/site/network/entity type/severity
- Acknowledge individual or bulk
- Alert details with before/after comparison
- Quick action links (view entity, view history)

**Notification Badge:**
- Unacknowledged count in nav
- Real-time updates via WebSocket

### 5.4 Diff Notifications

**Daily Digest (optional):**

```
VantageNet Daily Summary - Client: Acme Corp
Date: 2024-12-30

CHANGES DETECTED:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🆕 New Hosts (3)
  • 10.1.1.45 - "printer-lobby" (Site: HQ)
  • 10.1.1.46 - "unknown" (Site: HQ)
  • 10.2.0.100 - "dev-server-3" (Site: Branch)

⚠️ Certificates Expiring Soon (2)
  • mail.acme.com - expires in 14 days
  • vpn.acme.com - expires in 7 days

🔄 Service Changes (1)
  • 10.1.1.10 - nginx version 1.24.0 → 1.26.0

📊 Connectivity
  • VPN to Branch: avg latency 45ms (normal)
  • No route changes detected
```

### Phase 5 Deliverables

| Deliverable | Description |
|-------------|-------------|
| Change engine | Diff detection for all entity types |
| Event store | Change history with retention |
| Alert rules | Configurable per-client alerting |
| Alert UI | Dashboard, feed, acknowledge flow |
| Digest builder | Summary generation for future email/webhook |

---

## Phase 6: AI-Assisted Identification (Weeks 21-24)

**Goal:** Use LLM to identify unknown services via screenshots and service fingerprinting.

### 6.1 Screenshot Capture

**Implementation:**

```rust
// Capture webpage screenshots using headless Chrome
pub struct ScreenshotCapture {
    browser: Browser,
}

impl ScreenshotCapture {
    pub async fn capture(&self, url: &str) -> Result<Vec<u8>, Error> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;
        tab.wait_until_navigated()?;
        
        // Wait for page load
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // Capture viewport
        let screenshot = tab.capture_screenshot(
            CaptureScreenshotFormat::Png,
            None,
            None,
            true
        )?;
        
        Ok(screenshot)
    }
}
```

**Dependencies:**
- `chromiumoxide` or `headless_chrome` crate
- Chromium/Chrome installed on daemon

### 6.2 OpenRouter Integration

**Configuration:**

```toml
[ai]
enabled = true
provider = "openrouter"
api_key_env = "VANTAGENET_OPENROUTER_KEY"  # Read from env

# Recommended models (vision-capable, cost-effective)
models = [
    "anthropic/claude-sonnet-4-20250514",
    "google/gemini-flash-1.5",
    "openai/gpt-4o-mini",
]

# Cost controls
max_requests_per_hour = 100
max_cost_per_day = 5.00  # USD
```

**Service Identification Prompt:**

```rust
const IDENTIFICATION_PROMPT: &str = r#"
You are analyzing a network service. Based on the provided information, identify:
1. What application/service this is (product name, vendor)
2. Version if detectable
3. Purpose/function
4. Notable configuration observations
5. Potential security concerns

Respond in JSON format:
{
  "identified": true/false,
  "product": "Product Name",
  "vendor": "Vendor Name",
  "version": "x.y.z or null",
  "category": "web_server|database|monitoring|...",
  "purpose": "Brief description",
  "observations": ["observation 1", "observation 2"],
  "security_notes": ["note 1", "note 2"]
}
"#;
```

### 6.3 Manual Processing Flow

**UI Flow:**

1. User views unknown service
2. Clicks "Identify with AI" button
3. System gathers context:
   - Port/protocol
   - Banner/headers captured during scan
   - Screenshot (if HTTP/HTTPS)
   - mDNS TXT records (if available)
4. Sends to OpenRouter
5. Displays result with confidence
6. User can accept, edit, or reject

**Cost Display:**

```
┌─────────────────────────────────────────────┐
│ AI Identification                           │
├─────────────────────────────────────────────┤
│ This will send data to OpenRouter.          │
│                                             │
│ Estimated cost: ~$0.003                     │
│ Today's usage: $0.45 / $5.00 limit          │
│                                             │
│ Data to be sent:                            │
│ • Service banner (142 bytes)                │
│ • Screenshot (47 KB)                        │
│ • Port/protocol info                        │
│                                             │
│ [Cancel]                    [Identify]      │
└─────────────────────────────────────────────┘
```

### 6.4 AI-Generated Insights

**Beyond identification:**
- Suggest missing DNS records
- Recommend certificate improvements
- Identify potential misconfigurations
- Generate documentation snippets

### Phase 6 Deliverables

| Deliverable | Description |
|-------------|-------------|
| Screenshot capture | Headless Chrome integration |
| OpenRouter client | API integration with cost tracking |
| Identification UI | Manual trigger with preview |
| Result storage | AI identifications stored with confidence |
| Cost controls | Per-day limits, usage tracking |

---

## Phase 7: Smart Consolidation & Host Movement (Weeks 25-28)

**Goal:** Intelligently merge data when hosts are discovered by multiple daemons, and track host movement between networks.

### 7.1 Consolidation Logic

**Scenarios:**

| Scenario | Resolution |
|----------|------------|
| Host has daemon + seen by remote daemon | Local daemon is authoritative |
| Same MAC seen on multiple subnets | Single host, multiple interfaces |
| Same IP seen by multiple daemons | Likely different hosts, keep separate |
| Hostname matches across sites | Prompt user, may be same or different |

**Authority Hierarchy:**

```rust
pub fn determine_authority(sources: &[DiscoverySource]) -> DiscoverySource {
    // Priority order:
    // 1. Local daemon on the host itself
    // 2. Daemon on same L2 segment (has MAC address)
    // 3. Daemon with most detailed data
    // 4. Most recently updated
    
    sources.iter()
        .max_by_key(|s| {
            let mut score = 0;
            if s.is_local { score += 1000; }
            if s.has_mac { score += 100; }
            score += s.service_count as i32;
            score
        })
        .unwrap()
}
```

**Merge Strategy:**

```rust
pub struct MergedHost {
    // Primary data from authoritative source
    primary_source: DiscoverySource,
    
    // Merged data
    interfaces: Vec<Interface>,      // Union of all seen interfaces
    services: Vec<Service>,          // Union, prefer local details
    certificates: Vec<Certificate>,  // From all sources
    
    // Tracking
    other_sources: Vec<DiscoverySource>,
    last_merge: DateTime<Utc>,
}
```

### 7.2 Host Movement Tracking

**Movement Events:**

```sql
CREATE TABLE host_movements (
    id UUID PRIMARY KEY,
    host_id UUID REFERENCES hosts(id),
    
    -- Movement details
    from_network_id UUID REFERENCES networks(id),
    from_subnet_id UUID REFERENCES subnets(id),
    from_ip INET,
    
    to_network_id UUID REFERENCES networks(id),
    to_subnet_id UUID REFERENCES subnets(id),
    to_ip INET,
    
    -- Metadata
    detected_at TIMESTAMPTZ DEFAULT NOW(),
    detected_by_daemon_id UUID REFERENCES daemons(id)
);
```

**Detection Logic:**

```rust
pub async fn detect_movement(&self, host: &Host, new_discovery: &Discovery) -> Option<Movement> {
    // Get current known location
    let current = self.get_host_location(host.id).await?;
    
    // Compare with new discovery
    if new_discovery.network_id != current.network_id ||
       new_discovery.subnet_id != current.subnet_id {
        
        // Verify it's actually the same host (MAC match)
        if new_discovery.mac == current.mac {
            return Some(Movement {
                host_id: host.id,
                from_network_id: current.network_id,
                from_subnet_id: current.subnet_id,
                from_ip: current.ip,
                to_network_id: new_discovery.network_id,
                to_subnet_id: new_discovery.subnet_id,
                to_ip: new_discovery.ip,
            });
        }
    }
    
    None
}
```

**UI Representation:**

```
┌─────────────────────────────────────────────────────────────┐
│ Host: dev-laptop-01                                         │
├─────────────────────────────────────────────────────────────┤
│ Current Location:                                           │
│   Network: HQ-Wireless                                      │
│   Subnet: 10.1.10.0/24                                      │
│   IP: 10.1.10.45                                            │
│                                                             │
│ Movement History:                                           │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 2024-12-30 09:15  HQ-Wired (10.1.1.102)                │ │
│ │       ↓                                                 │ │
│ │ 2024-12-30 10:30  HQ-Wireless (10.1.10.45) ← Current   │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 7.3 Manual Consolidation UI

For cases automated logic can't resolve:

- "These appear to be the same host" suggestions
- Side-by-side comparison
- Merge button with preview
- "Keep separate" option

### Phase 7 Deliverables

| Deliverable | Description |
|-------------|-------------|
| Authority resolver | Determine which daemon data wins |
| Merge engine | Combine data from multiple sources |
| Movement detector | Track hosts across networks |
| Movement history | Historical location timeline |
| Manual merge UI | User-driven consolidation |

---

## Phase 8: Advanced Features (Weeks 29-32)

**Goal:** Implement remaining enhancements for comprehensive network monitoring.

### 8.1 Webhook/API for External Tools

**REST API:**

```yaml
openapi: 3.0.0
paths:
  /api/v2/clients/{clientId}/hosts:
    get:
      summary: List all hosts for a client
      parameters:
        - name: network_id
          in: query
        - name: tags
          in: query
          
  /api/v2/clients/{clientId}/hosts/{hostId}:
    get:
      summary: Get host details
      
  /api/v2/clients/{clientId}/search:
    get:
      summary: Search across all entities
      parameters:
        - name: q
          in: query
          description: Search query (IP, hostname, service, MAC)
          
  /api/v2/clients/{clientId}/changes:
    get:
      summary: Get recent changes
      parameters:
        - name: since
          in: query
          schema:
            type: string
            format: date-time
```

**Webhook Outbound (future):**

```sql
CREATE TABLE webhook_endpoints (
    id UUID PRIMARY KEY,
    client_id UUID REFERENCES clients(id),
    
    url TEXT NOT NULL,
    secret TEXT,  -- For HMAC signing
    
    -- Filters
    event_types TEXT[],
    severity_min TEXT,
    
    enabled BOOLEAN DEFAULT true
);
```

### 8.2 Asset Lifecycle Tagging

**Built-in Tags:**

| Tag | Color | Meaning |
|-----|-------|---------|
| `production` | Red | Production system, alert on changes |
| `development` | Blue | Development/test system |
| `staging` | Yellow | Pre-production |
| `decommissioned` | Gray | Scheduled for removal |
| `new` | Green | Recently discovered, needs review |
| `unmanaged` | Orange | Not under management |

**Custom Tags:**
- User-defined with custom colors
- Tag inheritance (subnet → hosts)
- Bulk tagging

### 8.3 Scheduled Reports

**Report Types:**

1. **Network Summary**: Host/service counts, change summary
2. **Certificate Report**: Expiring certs, trust issues
3. **Compliance Report**: All hosts, services, open ports
4. **Change Report**: All changes in period

**Scheduling:**

```sql
CREATE TABLE scheduled_reports (
    id UUID PRIMARY KEY,
    client_id UUID REFERENCES clients(id),
    
    report_type TEXT NOT NULL,
    schedule TEXT NOT NULL,  -- cron expression
    
    -- Delivery (future)
    delivery_method TEXT DEFAULT 'download',  -- 'download', 'email', 'webhook'
    delivery_config JSONB,
    
    -- Settings
    format TEXT DEFAULT 'pdf',  -- 'pdf', 'csv', 'json'
    include_networks UUID[],    -- null = all
    
    last_run TIMESTAMPTZ,
    next_run TIMESTAMPTZ
);
```

### 8.4 Vulnerability Awareness (Opt-in)

**Integration with Nmap/Nuclei:**

```toml
[vulnerability_scanning]
enabled = false  # Opt-in only

# Nmap NSE scripts
nmap_scripts = ["vulners", "vulscan"]

# Nuclei templates (curated safe set)
nuclei_templates = [
    "ssl",
    "default-logins",
    "exposed-panels",
    "misconfigurations",
]

# Exclusions
exclude_hosts = ["10.1.1.1"]  # Don't scan these
exclude_ports = [22]           # Don't scan SSH
```

**Safety Controls:**
- Never enabled by default
- Requires explicit opt-in per client
- Rate limiting
- Audit logging of all scans

### Phase 8 Deliverables

| Deliverable | Description |
|-------------|-------------|
| REST API | Comprehensive query API |
| Webhook framework | Future notification delivery |
| Lifecycle tags | Asset status tracking |
| Report generator | PDF/CSV scheduled reports |
| Vuln integration | Opt-in Nmap/Nuclei |

---

## Technical Considerations

### Database

**PostgreSQL Extensions:**
- `uuid-ossp` - UUID generation
- `pg_trgm` - Fuzzy text search
- TimescaleDB (optional) - Time-series for latency metrics

**Retention:**
- Configurable per-client (default 365 days)
- Automatic cleanup job
- Change events compressed after 90 days

### Performance

**Scaling Targets:**
- 100 clients
- 1,000 networks
- 100,000 hosts
- 1,000,000 services

**Indexing Strategy:**
- B-tree on all foreign keys
- Partial indexes for active/unacknowledged items
- GIN indexes for JSONB columns

### Security

**Daemon-Server Communication:**
- TLS 1.3 required
- API key authentication
- Client certificate option for high-security

**Data Isolation:**
- Row-level security in PostgreSQL
- Client ID checked on every query
- No cross-client data leakage possible

---

## Timeline Summary

| Phase | Weeks | Focus |
|-------|-------|-------|
| **1: Foundation** | 1-4 | Fork, hierarchy, Debian daemon |
| **2: Discovery** | 5-8 | mDNS, SNMP, LLDP, DHCP, DNS |
| **3: Certificates** | 9-12 | TLS/SSH monitoring |
| **4: VPN** | 13-16 | VPN detection, latency monitoring |
| **5: Alerting** | 17-20 | Change detection, in-app alerts |
| **6: AI** | 21-24 | OpenRouter integration |
| **7: Consolidation** | 25-28 | Host merging, movement tracking |
| **8: Advanced** | 29-32 | API, reports, vuln scanning |

**Total:** ~32 weeks (8 months)

---

## Appendix A: Daemon Dependencies (Debian 13)

```bash
# Required packages
apt install -y \
    libssl-dev \
    libpcap-dev \
    chromium \
    nmap \
    mtr-tiny \
    snmp \
    net-tools \
    iproute2

# Optional for vulnerability scanning
apt install -y nuclei
```

---

## Appendix B: Configuration Reference

### Daemon Configuration (`/etc/vantagenet/daemon.toml`)

```toml
[server]
url = "https://vantagenet.example.com"
api_key = "nsd_xxxxxxxxxxxxx"

[discovery]
scan_interval = 3600  # seconds
host_timeout = 5      # seconds per host
concurrency = 50

[discovery.subnets]
# Explicit subnets to scan
include = ["10.1.0.0/16", "192.168.0.0/16"]
exclude = ["10.1.255.0/24"]

[mdns]
enabled = true
listen_duration = 30  # seconds

[snmp]
enabled = false
communities = ["public"]
timeout = 5

[lldp]
enabled = true
interface = "eth0"

[certificates]
enabled = true
ports = [443, 8443, 993, 995, 636, 3389]
check_interval = 86400  # daily

[ssh_keys]
enabled = true
ports = [22, 2222]
check_interval = 86400

[connectivity]
enabled = true
[[connectivity.targets]]
host = "8.8.8.8"
name = "Internet (Google)"
interval = 60

[ai]
enabled = false
# Requires VANTAGENET_OPENROUTER_KEY env var

[updates]
auto_update = true
channel = "stable"  # or "beta"
```

---

## Appendix C: Suggested OpenRouter Models

| Model | Cost (per 1K tokens) | Vision | Recommended For |
|-------|---------------------|--------|-----------------|
| `google/gemini-flash-1.5` | $0.0001 | ✓ | Default, cost-effective |
| `anthropic/claude-sonnet-4-20250514` | $0.003 | ✓ | Complex identification |
| `openai/gpt-4o-mini` | $0.00015 | ✓ | Good balance |

---

## Next Steps

1. **Review this plan** - Let me know what to adjust
2. **Prioritize phases** - Which capabilities are most urgent?
3. **Name confirmed** - VantageNet
4. **Set up repository** - Fork and rebrand
5. **Begin Phase 1** - Foundation work

---

*Document generated by Claude - December 30, 2024*
