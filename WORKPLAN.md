# VantageNet Development Workplan

**Created:** January 5, 2026  
**Target Deployment Host:** bowlister (Debian 13 ARM64, legend@bowlister)  
**Approach:** Build it right, then deploy

---

## Priority Order & Rationale

| Priority | Phase | Why This Order |
|----------|-------|----------------|
| **P0** | Native Daemon (.deb) | Blocking: Docker can't see L2/MAC. Everything else needs proper discovery. |
| **P1** | MSP Data Model | Foundation: Client/Site hierarchy is the data backbone for all features. |
| **P2** | UI Dark Theme | Quick win: Makes the product feel like VantageNet, not Scanopy. |
| **P3** | Certificate Monitoring | High MSP value: Immediate customer pain point solved. |
| **P4** | Change Detection | Depends on P3: Need entities to detect changes on. |
| **P5** | Discovery Enhancements | Adds depth: mDNS, LLDP, DHCP after core is solid. |
| **P6** | VPN Monitoring | Nice-to-have: Latency/route monitoring, TimescaleDB. |
| **P7** | AI Identification | Polish: Expensive, powerful, but not blocking. |

---

## P0: Native Daemon Package (BLOCKING)

**Duration:** 1 week  
**Why First:** Docker networking prevents proper L2 discovery. The daemon sees Docker bridge MACs, not real device MACs. This breaks ARP scanning, LLDP, and DHCP monitoring.

### Tasks

- [ ] **Cross-compile daemon binary**
  - Target: `aarch64-unknown-linux-gnu` (bowlister) + `x86_64-unknown-linux-gnu`
  - Use `cross` or cargo with appropriate target
  - Strip binary, optimize for size

- [ ] **Create Debian package structure**
  ```
  vantagenet-daemon_0.13.0_arm64/
  ├── DEBIAN/
  │   ├── control          # Package metadata
  │   ├── conffiles        # Mark config as user-editable
  │   ├── postinst         # Enable & start service
  │   └── prerm            # Stop service before removal
  ├── etc/
  │   └── vantagenet/
  │       └── daemon.toml  # Default config
  ├── lib/
  │   └── systemd/
  │       └── system/
  │           └── vantagenet-daemon.service
  └── usr/
      └── bin/
          └── vantagenet-daemon
  ```

- [ ] **Systemd service file**
  ```ini
  [Unit]
  Description=VantageNet Discovery Daemon
  After=network-online.target
  Wants=network-online.target

  [Service]
  Type=simple
  ExecStart=/usr/bin/vantagenet-daemon
  Restart=always
  RestartSec=5
  Environment=VANTAGENET_CONFIG=/etc/vantagenet/daemon.toml

  # Security hardening
  NoNewPrivileges=no  # Needs CAP_NET_RAW
  AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN
  CapabilityBoundingSet=CAP_NET_RAW CAP_NET_ADMIN

  [Install]
  WantedBy=multi-user.target
  ```

- [ ] **Default config template**
  ```toml
  # /etc/vantagenet/daemon.toml
  server_url = "http://your-server:60072"
  bind_address = "0.0.0.0"
  port = 60073
  mode = "Push"
  log_level = "info"
  heartbeat_interval = 30
  ```

- [ ] **Build script** (`scripts/build-deb.sh`)
  - Cross-compile for target arch
  - Build .deb package
  - Generate SHA256 checksums

- [ ] **Installation one-liner**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/N2con-Inc/VantageNet/main/install-daemon.sh | sudo bash
  ```

### Deliverables
- `vantagenet-daemon_0.13.0_arm64.deb`
- `vantagenet-daemon_0.13.0_amd64.deb`
- Installation script with interactive config wizard
- GitHub Release with packages attached

### Test Criteria
- [x] Debian package structure created
- [x] Build script (build-deb.sh) written
- [x] Installation one-liner (install-daemon.sh) written
- [x] Systemd service with CAP_NET_RAW/CAP_NET_ADMIN
- [ ] Daemon starts via systemd on fresh Debian 13
- [ ] Daemon can see real MAC addresses (not Docker bridge)
- [ ] Daemon registers with server successfully
- [ ] Discovery finds hosts with correct L2 info

### Status: **Infrastructure Complete**
All packaging files created. Requires Linux host for cross-compilation and .deb packaging.

---

## P1: MSP Data Model

**Duration:** 2-3 weeks  
**Depends on:** P0 (need working daemon to test discovery)

### Database Schema

```sql
-- Clients table (tenants)
CREATE TABLE clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(organization_id, slug)
);

-- Sites table (locations within a client)
CREATE TABLE sites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    address TEXT,
    timezone VARCHAR(50) DEFAULT 'UTC',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Modify networks to belong to sites
ALTER TABLE networks 
    ADD COLUMN site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
    ADD COLUMN client_id UUID REFERENCES clients(id) ON DELETE CASCADE;

-- Daemons belong to sites
ALTER TABLE daemons
    ADD COLUMN site_id UUID REFERENCES sites(id) ON DELETE SET NULL;

-- Index for fast client isolation
CREATE INDEX idx_networks_client ON networks(client_id);
CREATE INDEX idx_sites_client ON sites(client_id);
```

### Tasks

- [ ] **Database migrations**
  - Create `clients` table
  - Create `sites` table  
  - Add foreign keys to `networks` and `daemons`
  - Data migration: existing networks → default client/site

- [ ] **Backend API**
  - `GET/POST /api/v1/clients` - List/create clients
  - `GET/PUT/DELETE /api/v1/clients/{id}` - Client CRUD
  - `GET/POST /api/v1/clients/{id}/sites` - Sites within client
  - `GET/PUT/DELETE /api/v1/sites/{id}` - Site CRUD
  - Modify all entity endpoints to enforce client isolation

- [ ] **Client context middleware**
  - Extract client_id from URL or session
  - Inject into all database queries
  - Prevent cross-client data access

- [ ] **Frontend UI**
  - Client selector dropdown in header
  - Client management page (Admin only)
  - Site management within client
  - Breadcrumb: Client → Site → Network → Host

- [ ] **Daemon registration update**
  - Daemon config includes `site_id` (optional)
  - Server assigns daemon to site on registration
  - UI to reassign daemons between sites

### Deliverables
- Multi-tenant client isolation
- Site-based network organization
- Updated UI navigation
- Daemon-to-site assignment

---

## P2: UI Dark Theme (Reconya-Inspired)

**Duration:** 1-2 weeks  
**Can interleave with:** P1

### Design Tokens

```css
:root {
  /* Background layers */
  --bg-base: #0a0a0f;
  --bg-surface: #12121a;
  --bg-elevated: #1a1a24;
  --bg-overlay: #22222e;
  
  /* Accent colors */
  --accent-primary: #10b981;    /* Emerald green */
  --accent-secondary: #06b6d4;  /* Cyan */
  --accent-warning: #f59e0b;
  --accent-danger: #ef4444;
  
  /* Text */
  --text-primary: #f1f5f9;
  --text-secondary: #94a3b8;
  --text-muted: #64748b;
  
  /* Borders */
  --border-subtle: #1e293b;
  --border-default: #334155;
  
  /* Effects */
  --glow-primary: 0 0 20px rgba(16, 185, 129, 0.3);
}
```

### Typography

```css
/* Headers - Orbitron for cyberpunk feel */
@import url('https://fonts.googleapis.com/css2?family=Orbitron:wght@400;500;700&display=swap');

/* Body - Clean and readable */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap');

/* Mono - For IPs, ports, code */
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap');
```

### Tasks

- [ ] **Tailwind config update**
  - Add custom color palette
  - Configure dark mode as default
  - Add custom fonts

- [ ] **Component restyling**
  - Sidebar navigation
  - Dashboard cards
  - Data tables
  - Modals and dialogs
  - Form inputs
  - Buttons (primary, secondary, ghost)
  - Status badges

- [ ] **Special effects**
  - Subtle glow on primary actions
  - Smooth transitions (150-200ms)
  - Loading skeletons
  - Hover states with elevation

- [ ] **Topology view theming**
  - Dark canvas background
  - Glowing node borders
  - Animated connection lines

### Deliverables
- Cohesive dark cyberpunk theme
- VantageNet branding throughout
- Consistent component library

---

## P3: Certificate Monitoring

**Duration:** 2-3 weeks  
**Depends on:** P1 (need client/site structure)

### Database Schema

```sql
CREATE TABLE certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    service_id UUID REFERENCES services(id) ON DELETE SET NULL,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    
    -- Certificate data
    subject_cn VARCHAR(255),
    subject_full TEXT,
    issuer_cn VARCHAR(255),
    issuer_full TEXT,
    serial_number VARCHAR(100),
    
    -- Validity
    not_before TIMESTAMPTZ,
    not_after TIMESTAMPTZ,
    
    -- Technical details
    public_key_algorithm VARCHAR(50),
    public_key_bits INTEGER,
    signature_algorithm VARCHAR(100),
    fingerprint_sha256 VARCHAR(64) UNIQUE,
    
    -- Chain
    chain_depth INTEGER,
    chain_pem TEXT,  -- Full chain for debugging
    
    -- Trust assessment
    is_self_signed BOOLEAN,
    is_expired BOOLEAN,
    is_trusted BOOLEAN,  -- Against Mozilla root store
    trust_issues TEXT[], -- Array of issues found
    
    -- SANs
    san_dns_names TEXT[],
    san_ip_addresses INET[],
    
    -- Tracking
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    last_changed TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE ssh_host_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    service_id UUID REFERENCES services(id) ON DELETE SET NULL,
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    
    key_type VARCHAR(50) NOT NULL,  -- ed25519, ecdsa-sha2-nistp256, rsa
    fingerprint_sha256 VARCHAR(64) NOT NULL,
    public_key TEXT,
    
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    last_changed TIMESTAMPTZ,
    
    UNIQUE(host_id, key_type)
);

-- Certificate history for change tracking
CREATE TABLE certificate_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    certificate_id UUID REFERENCES certificates(id) ON DELETE CASCADE,
    fingerprint_sha256 VARCHAR(64),
    changed_at TIMESTAMPTZ DEFAULT NOW(),
    change_type VARCHAR(20),  -- 'new', 'renewed', 'replaced', 'expired'
    previous_not_after TIMESTAMPTZ,
    new_not_after TIMESTAMPTZ
);
```

### Tasks

- [ ] **TLS certificate scanner (daemon)**
  - Scan all HTTPS/TLS services found during discovery
  - Handle STARTTLS for SMTP (587, 465), LDAP (636), PostgreSQL, MySQL
  - Extract full certificate chain
  - Calculate fingerprints

- [ ] **SSH key scanner (daemon)**
  - Probe SSH services (like ssh-keyscan)
  - Support ed25519, ECDSA, RSA key types
  - Store fingerprints

- [ ] **Trust assessment (server)**
  - Validate against Mozilla root store (webpki-roots crate)
  - Check expiry status
  - Identify self-signed certs
  - Flag chain issues

- [ ] **Certificate API**
  - `GET /api/v1/certificates` - List with filtering
  - `GET /api/v1/certificates/expiring` - Certs expiring within N days
  - `GET /api/v1/hosts/{id}/certificates` - Certs for a host
  - `GET /api/v1/ssh-keys` - SSH key inventory

- [ ] **Expiry alerting**
  - Configurable thresholds (30/14/7/1 days)
  - In-app notifications
  - Dashboard widget: "Expiring Soon"

- [ ] **UI components**
  - Certificate list view with status indicators
  - Certificate detail modal (chain visualization)
  - SSH key list per host
  - Expiry timeline/calendar view

### Deliverables
- Full TLS certificate inventory
- SSH host key tracking
- Expiry warnings before they become outages
- Certificate change history

---

## P4: Change Detection & Alerting

**Duration:** 2-3 weeks  
**Depends on:** P3 (need certs to detect changes on)

### Tasks

- [ ] **Change detection engine**
  - Compare current discovery to previous state
  - Detect: new hosts, removed hosts, IP changes, service changes
  - Detect: new certs, renewed certs, expired certs
  - Detect: SSH key changes (potential compromise indicator)

- [ ] **Alert rules system**
  - Per-client configurable rules
  - Filter by: entity type, severity, tags
  - Suppression windows

- [ ] **Change events table**
  ```sql
  CREATE TABLE change_events (
      id UUID PRIMARY KEY,
      client_id UUID NOT NULL,
      entity_type VARCHAR(50),  -- host, service, certificate, ssh_key
      entity_id UUID,
      change_type VARCHAR(50),  -- new, removed, modified
      severity VARCHAR(20),     -- info, warning, critical
      summary TEXT,
      details JSONB,            -- before/after snapshot
      acknowledged BOOLEAN DEFAULT FALSE,
      acknowledged_by UUID,
      acknowledged_at TIMESTAMPTZ,
      created_at TIMESTAMPTZ DEFAULT NOW()
  );
  ```

- [ ] **Real-time notifications**
  - WebSocket push for new alerts
  - Toast notifications in UI
  - Unread count badge

- [ ] **Alert UI**
  - Chronological feed
  - Filter by type, severity, date
  - Bulk acknowledge
  - Alert detail view with diff

### Deliverables
- Change history for all entities
- Configurable alert rules
- Real-time alert feed
- Acknowledge workflow

---

## P5: Discovery Enhancements

**Duration:** 2-3 weeks
**Depends on:** P0 (needs native daemon for L2 protocols)

### New Discovery Modules

| Protocol | What It Finds | Implementation |
|----------|---------------|----------------|
| **mDNS/Bonjour** | Apple devices, printers, IoT | Listen for `_http._tcp.local`, etc. |
| **LLDP/CDP** | Switch topology, port mapping | Raw socket capture (CAP_NET_RAW) |
| **DHCP Monitor** | New devices as they join | Passive traffic monitoring |
| **DNS Validation** | Mismatched records | Forward/reverse lookup comparison |

### Optional: nmap Integration (Post-P5)

| Feature | Value for MSPs |
|---------|---------------|
| **SYN scan (`-sS`)** | Faster, stealthier than TCP connect |
| **OS fingerprinting (`-O`)** | Identify device types |
| **Service version (`-sV`)** | Better identification |
| **Script scanning (`--script`)** | Vuln detection, SSL info |

Implementation as optional enhancement:
```toml
[scanning]
use_nmap = true  # Optional, falls back to native if nmap not found
nmap_path = "/usr/bin/nmap"
```

### Tasks

- [ ] **mDNS discovery**
  - Multicast listener on 224.0.0.251:5353
  - Parse service announcements
  - Extract TXT records for version info
  - Correlate with existing hosts by IP

- [ ] **LLDP/CDP listener**
  - Raw Ethernet frame capture
  - Parse LLDP TLVs: Chassis ID, Port ID, System Name
  - Parse CDP frames for Cisco devices
  - Build physical topology layer

- [ ] **DHCP lease monitor**
  - Listen for DHCP traffic (passive)
  - Extract: hostname (option 12), vendor class (option 60)
  - Alert on new MAC addresses

- [ ] **DNS validation**
  - Forward lookup for discovered hostnames
  - Reverse lookup (PTR) for discovered IPs
  - Flag mismatches in UI

### Deliverables
- Richer device metadata
- Physical switch topology from LLDP
- New device alerts from DHCP
- DNS consistency checking

---

## P6: VPN & Connectivity Monitoring

**Duration:** 2 weeks  
**Depends on:** P5 (route analysis needs enhanced discovery)

### Tasks

- [ ] **VPN detection**
  - Analyze routing table for tunnel interfaces
  - Identify WireGuard (wg0), OpenVPN (tun0), IPsec
  - Cross-daemon correlation for site-to-site links

- [ ] **Latency probes**
  - Configurable probe targets per daemon
  - ICMP ping + TCP connect times
  - MTR-style traceroute

- [ ] **TimescaleDB integration**
  - Enable TimescaleDB extension
  - Convert metrics tables to hypertables
  - Retention policies (90 days raw, 1 year aggregated)

- [ ] **VPN link model**
  ```sql
  CREATE TABLE vpn_links (
      id UUID PRIMARY KEY,
      network_a_id UUID REFERENCES networks(id),
      network_b_id UUID REFERENCES networks(id),
      link_type VARCHAR(50),  -- wireguard, openvpn, ipsec
      status VARCHAR(20),     -- up, down, degraded
      latency_ms FLOAT,
      packet_loss_pct FLOAT,
      last_checked TIMESTAMPTZ
  );
  ```

- [ ] **VPN dashboard**
  - Link status overview
  - Latency graphs over time
  - Topology view with VPN connections

### Deliverables
- VPN tunnel inventory
- Latency monitoring with history
- VPN health alerts

---

## P7: AI-Assisted Service Identification

**Duration:** 2-3 weeks  
**Depends on:** All above (polish feature)

### Identification Pipeline

```
1. Local Pattern DB     → Fingerprints from past confirms
2. Scanopy Definitions  → 200+ built-in patterns  
3. EXA Search          → Web search for unknown banners
4. LLM Vision          → Screenshot + context → ID
5. User Confirmation   → Human validates suggestion
6. Pattern Learning    → Confirmed → saved locally
```

### Tasks

- [ ] **Screenshot capture** (daemon)
  - Headless Chrome/Playwright integration
  - Capture HTTP/HTTPS login pages
  - Store with service records

- [ ] **EXA search integration**
  - Query for unknown banners/ports
  - Cache results
  - Rate limiting

- [ ] **LLM integration (OpenRouter)**
  - Vision model for screenshot analysis
  - Structured output: product, vendor, version
  - Cost tracking

- [ ] **Learning loop**
  - User confirmation UI: "Is this [X]?"
  - Save confirmed patterns locally
  - Priority: local → Scanopy → EXA → LLM

- [ ] **Cost controls**
  - Per-client API budget
  - Usage dashboard
  - Tiered approach (cheap → expensive)

### Deliverables
- Multi-tier identification pipeline
- Learning from confirmations
- Cost-controlled API usage

---

## Deployment Checklist (After All Phases)

### Server (Docker on bowlister)
- [ ] PostgreSQL 17 with TimescaleDB
- [ ] VantageNet Server container
- [ ] Nginx reverse proxy with SSL
- [ ] Automated backups

### Daemon (Native .deb on bowlister)
- [ ] Install vantagenet-daemon package
- [ ] Configure /etc/vantagenet/daemon.toml
- [ ] Enable and start systemd service
- [ ] Verify L2 discovery working

### Verification
- [ ] Create organization and first user
- [ ] Create client and site
- [ ] Daemon registers and discovers network
- [ ] Hosts show correct MAC addresses
- [ ] Certificates scanned and displayed
- [ ] Change alerts firing

---

## Timeline Estimate

| Phase | Duration | Cumulative |
|-------|----------|------------|
| P0: Native Daemon | 1 week | Week 1 |
| P1: MSP Data Model | 2-3 weeks | Week 3-4 |
| P2: UI Dark Theme | 1-2 weeks | Week 5 (parallel with P1) |
| P3: Certificate Monitoring | 2-3 weeks | Week 7-8 |
| P4: Change Detection | 2-3 weeks | Week 10 |
| P5: Discovery Enhancements | 2-3 weeks | Week 12 |
| P6: VPN Monitoring | 2 weeks | Week 14 |
| P7: AI Identification | 2-3 weeks | Week 16 |

**Total: ~16 weeks for full feature set**

---

## Next Action

Start with **P0: Native Daemon Package** - this unblocks everything else by giving us proper L2 visibility.
