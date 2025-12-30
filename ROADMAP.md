# VantageNet Development Roadmap

**Base:** Fork of [Scanopy](https://github.com/scanopy/scanopy) (point-in-time, v0.12.8)  
**Goal:** MSP-focused network discovery platform with certificate monitoring, VPN detection, change alerting, and AI-assisted service identification  
**Approach:** Build it right, solo dev pace  

---

## Key Decisions

| Aspect | Decision |
|--------|----------|
| **Base project** | Scanopy v0.12.8 (fork, not track) |
| **Branding** | Full rename to VantageNet |
| **Backend** | Rust (Axum) — keep from Scanopy |
| **Frontend** | SvelteKit + Tailwind — keep, restyle to dark theme |
| **Database** | PostgreSQL + TimescaleDB (for metrics) |
| **AI Search** | EXA API |
| **AI Vision** | OpenRouter (Claude/GPT-4o/Gemini) |
| **Daemon packaging** | Native .deb for Debian 12/13 |
| **UI aesthetic** | Reconya-inspired dark cyberpunk |

---

## Pre-Phase: Foundation & Orientation

**Duration:** 1-2 weeks  
**Goal:** Understand what we're working with before writing code

### Tasks

- [x] **Fork & Setup** ✅
  - Fork scanopy/scanopy to N2con-Inc/VantageNet
  - Full rebrand to VantageNet completed
  - Clone locally, get dev environment running
  - Run through the full user flow (create org, add network, run discovery)
  
- [ ] **Codebase Orientation**
  - Map backend structure (`backend/src/` - Rust/Axum)
  - Map frontend structure (`ui/src/` - SvelteKit)
  - Understand database schema (`backend/migrations/`)
  - Identify daemon ↔ server communication protocol
  
- [ ] **Document Findings**
  - Create `docs/ARCHITECTURE.md` with your learnings
  - Note extension points for each planned feature
  - Identify potential conflicts with upstream development

- [ ] **UI Aesthetic Assessment**
  - Compare Scanopy vs Reconya UI side-by-side
  - Document specific UI changes wanted (colors, fonts, components)
  - Decide: gradual restyle vs. big-bang theme change

### Deliverables
- Running local dev environment
- Architecture documentation
- UI change list
- Go/no-go decision on fork viability

---

## Phase 1: MSP Data Model

**Duration:** 2-3 weeks  
**Goal:** Transform Scanopy's Organizations into MSP Client/Site hierarchy

### Why First?
This is the foundational data model change. Everything else (certs, VPN, alerts) hangs off this structure. Get it right early.

### Current State (Scanopy)
```
Organization
└── Network
    └── Subnet
        └── Host
            └── Service
```

### Target State (VantageNet)
```
Organization (your MSP)
└── Client (tenant - isolated)
    └── Site (physical/logical location)
        └── Network
            └── Subnet
                └── Host
                    ├── Interfaces
                    ├── Services
                    ├── Certificates  ← Phase 3
                    └── SSH Keys      ← Phase 3
```

### Tasks

- [ ] **Database Schema**
  - Add `clients` table (under organizations)
  - Add `sites` table (under clients)
  - Modify `networks` to reference `site_id`
  - Add Row-Level Security or application-layer client isolation
  
- [ ] **Backend API**
  - New endpoints: `/api/v2/clients/{client_id}/...`
  - Modify existing endpoints to be client-scoped
  - Update daemon registration to include client assignment
  
- [ ] **Frontend UI**
  - Client selector in nav/header
  - Site management UI
  - Breadcrumb: Client → Site → Network → Host
  
- [ ] **Daemon Changes**
  - Config: `client_id` assignment
  - Registration payload update

### Deliverables
- Working multi-client isolation
- Site-based network grouping
- UI navigation reflecting hierarchy

---

## Phase 2: Discovery Enhancements

**Duration:** 3-4 weeks  
**Goal:** Expand discovery beyond port scanning

### What Scanopy Has
- ARP scanning
- Port scanning
- Service fingerprinting (200+ definitions)
- Docker socket integration
- Partial SNMP support

### What We Add

| Protocol | Value | Complexity |
|----------|-------|------------|
| **mDNS/Bonjour** | Find Apple devices, printers, smart home | Medium |
| **LLDP/CDP** | Physical switch topology | Medium |
| **DHCP monitoring** | New device detection | Low |
| **DNS validation** | PTR record consistency | Low |
| **Enhanced SNMP** | Switch port mapping, VLANs | Medium |

### Tasks

- [ ] **mDNS Discovery Module**
  - Listen for `_http._tcp.local`, `_https._tcp.local`, etc.
  - Parse TXT records for version info
  - Correlate with existing hosts by IP/MAC
  
- [ ] **LLDP/CDP Listener**
  - Raw socket capture (requires CAP_NET_RAW)
  - Parse TLVs: Chassis ID, Port ID, System Name
  - Build physical topology layer
  
- [ ] **DHCP Lease Monitor**
  - Passive DHCP traffic monitoring
  - Extract hostname (option 12), vendor class (option 60)
  - Alert on new MAC addresses
  
- [ ] **DNS Validation**
  - Forward lookup: hostname → IP match?
  - Reverse lookup: PTR exists and matches?
  - Flag mismatches in UI

### Deliverables
- Richer device metadata
- Physical topology from LLDP
- New device alerts from DHCP

---

## Phase 3: Certificate & SSH Key Monitoring

**Duration:** 3-4 weeks  
**Goal:** Track TLS certificates and SSH host keys across all services

### Why This Phase?
High MSP value. Certificate expiry surprises are painful. SSH key changes can indicate compromise.

### Tasks

- [ ] **TLS Certificate Scanner**
  - Scan all discovered HTTPS/TLS services
  - Handle STARTTLS (SMTP, LDAP, PostgreSQL, etc.)
  - Extract: subject, issuer, SANs, expiry, chain
  - Assess trust against Mozilla root store
  
- [ ] **Database Schema**
  - `certificates` table with full chain storage
  - `ssh_host_keys` table with fingerprints
  - History tracking (when did cert change?)
  
- [ ] **SSH Key Fingerprinting**
  - Probe SSH services (like `ssh-keyscan`)
  - Store ed25519, ECDSA, RSA fingerprints
  - Detect key changes
  
- [ ] **Expiry Alerting**
  - 30/14/7/1 day warnings
  - Per-client alert configuration
  - Dashboard widget: "Certificates expiring soon"
  
- [ ] **UI Components**
  - Certificate detail view (chain visualization)
  - SSH key list per host
  - Expiry calendar/timeline

### Deliverables
- Full certificate inventory per client
- SSH key tracking with change detection
- Expiry warnings in UI

---

## Phase 4: Change Detection & Alerting

**Duration:** 3-4 weeks  
**Goal:** Monitor for changes and surface them to users

### Why This Phase?
"What changed on my network?" is a core MSP question. This is the diff engine.

### Change Types

| Entity | Changes Detected |
|--------|------------------|
| Host | New, removed, IP changed, MAC changed |
| Service | New, removed, version changed |
| Certificate | New, renewed, changed, expired |
| SSH Key | New, changed |
| Network | New subnet detected |

### Tasks

- [ ] **Change Detection Engine**
  - Compare current scan to previous state
  - Generate change events with before/after
  - Store in `change_events` table
  
- [ ] **Alert Rules**
  - Per-client configurable rules
  - Filter by entity type, severity, tags
  - Suppression windows (don't re-alert for X hours)
  
- [ ] **In-App Alert UI**
  - Alert feed (chronological)
  - Filter/search
  - Acknowledge flow (individual + bulk)
  - Unread count badge in nav
  
- [ ] **Real-time Updates**
  - WebSocket push for new alerts
  - Toast notifications in UI

### Deliverables
- Change history for all entities
- Configurable alert rules
- Real-time alert feed

---

## Phase 5: VPN & Connectivity Monitoring

**Duration:** 2-3 weeks  
**Goal:** Detect VPN tunnels, monitor latency, alert on degradation

### Tasks

- [ ] **VPN Detection**
  - Analyze routing table for tunnel interfaces (tun0, wg0, etc.)
  - Identify routes to private ranges via non-default gateways
  - Cross-daemon correlation (Daemon A sees route to Daemon B's subnet)
  
- [ ] **Latency Probes**
  - Configurable probe targets per daemon
  - MTR-style traceroute with latency/loss metrics
  - Store time-series data (consider TimescaleDB extension)
  
- [ ] **VPN Link Model**
  - `vpn_links` table: network_a ↔ network_b
  - Status: up/down/degraded
  - Latency/loss thresholds for alerting
  
- [ ] **UI Components**
  - VPN status dashboard
  - Latency graphs over time
  - Topology view showing VPN connections

### Deliverables
- VPN tunnel inventory
- Latency monitoring with history
- VPN health alerts

---

## Phase 6: AI-Assisted Service Identification

**Duration:** 3-4 weeks  
**Goal:** Intelligently identify unknown services with learning loop

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Identification Pipeline                   │
├─────────────────────────────────────────────────────────────┤
│  1. Local Pattern DB     │ Fingerprints from past confirms  │
│  2. Scanopy Definitions  │ 200+ built-in service patterns   │
│  3. EXA Search           │ Web search for unknown banners   │
│  4. LLM Analysis         │ Screenshot + context → ID        │
│  5. User Confirmation    │ Human-in-the-loop validation     │
│  6. Pattern Learning     │ Save confirmed → local DB        │
└─────────────────────────────────────────────────────────────┘
```

### Tasks

- [ ] **Screenshot Capture**
  - Headless Chrome/Chromium integration
  - Capture HTTP/HTTPS services
  - Store screenshots with service records
  
- [ ] **EXA Search Integration**
  - Query: "service port {port} banner {banner}"
  - Parse results for service identification
  - Cache results to reduce API calls
  
- [ ] **LLM Integration (OpenRouter)**
  - Vision model for screenshot analysis
  - Structured output: product, vendor, version, category
  - Cost tracking and daily limits
  
- [ ] **Learning Loop**
  - User confirmation UI: "Is this [suggestion]?" [Yes/Edit/No]
  - Store confirmed patterns in `service_patterns` table
  - Priority: local patterns > Scanopy > EXA > LLM
  
- [ ] **Cost Controls**
  - Per-client API budget
  - Tiered approach (free local → cheap EXA → expensive LLM)
  - Usage dashboard

### Deliverables
- Multi-tier identification pipeline
- Learning from user confirmations
- Cost-controlled API usage

---

## Phase 7: Native Daemon Packaging

**Duration:** 2 weeks  
**Goal:** Debian package for L2/MAC visibility without Docker

### Why?
Docker networking limits MAC address visibility. Native daemon = full L2 access.

### Tasks

- [ ] **Debian Package Structure**
  ```
  vantagenet-daemon/
  ├── DEBIAN/
  │   ├── control
  │   ├── postinst
  │   └── prerm
  ├── etc/vantagenet/daemon.toml
  ├── lib/systemd/system/vantagenet-daemon.service
  └── usr/bin/vantagenet-daemon
  ```
  
- [ ] **Build Pipeline**
  - Cross-compile Rust daemon for amd64/arm64
  - GitHub Actions workflow for .deb generation
  - GPG-signed packages
  
- [ ] **Auto-Update Mechanism**
  - Server tracks daemon versions
  - Push update notification via WebSocket
  - Daemon self-updates from GitHub releases
  
- [ ] **Installation Script**
  - One-liner bootstrap: `curl ... | bash`
  - Interactive configuration wizard

### Deliverables
- `.deb` package for Debian 12/13
- Apt repository setup
- Auto-update from server

---

## Phase 8: UI Restyling

**Duration:** 2 weeks (can be interleaved with other phases)  
**Goal:** Apply Reconya-inspired dark cyberpunk aesthetic

### Reconya UI Elements to Adopt

| Element | Implementation |
|---------|----------------|
| Dark-first theme | Tailwind dark mode as default |
| Green accent (#10b981) | Replace Scanopy's accent color |
| Orbitron font | Brand/headers |
| Roboto Condensed | Body text |
| Cyberpunk cards | Subtle borders, backdrop blur |
| Loading spinners | Consistent green ring animation |

### Tasks

- [ ] **Theme System**
  - CSS custom properties for theming
  - Dark/light toggle (dark default)
  
- [ ] **Typography**
  - Import Orbitron, Roboto fonts
  - Apply hierarchy: brand → headers → body → mono
  
- [ ] **Component Restyling**
  - Navigation sidebar
  - Dashboard cards
  - Tables
  - Modals
  - Forms
  
- [ ] **Polish**
  - Consistent hover states
  - Smooth transitions (0.2-0.3s)
  - Loading states everywhere

### Deliverables
- Cohesive dark cyberpunk theme
- Consistent component library
- Light mode option

---

## Phase 9: Advanced Features

**Duration:** Ongoing  
**Goal:** API, reports, and optional vulnerability awareness

### Tasks

- [ ] **REST API**
  - OpenAPI documentation
  - API key authentication
  - Rate limiting
  
- [ ] **Scheduled Reports**
  - PDF/CSV generation
  - Report types: Network Summary, Certificate Report, Change Report
  - Email delivery (future)
  
- [ ] **Asset Lifecycle Tags**
  - Built-in: production, development, staging, decommissioned
  - Custom tags with colors
  - Tag-based filtering
  
- [ ] **Vulnerability Awareness (Opt-in)**
  - Nmap NSE scripts integration
  - Nuclei templates (safe subset)
  - Explicit opt-in per client

### Deliverables
- Documented API
- Exportable reports
- Asset tagging system

---

## Dependency Graph

```
Pre-Phase (Foundation)
    │
    ▼
Phase 1 (MSP Data Model) ◄── Everything depends on this
    │
    ├──────────┬──────────┬──────────┐
    ▼          ▼          ▼          ▼
Phase 2    Phase 3    Phase 5    Phase 8
(Discovery) (Certs)    (VPN)      (UI)
    │          │          │
    └────┬─────┴──────────┘
         ▼
    Phase 4 (Change Detection) ◄── Needs entities to detect changes on
         │
         ▼
    Phase 6 (AI Identification) ◄── Needs services to identify
         │
         ▼
    Phase 7 (Debian Package) ◄── Can be done earlier if needed
         │
         ▼
    Phase 9 (Advanced)
```

---

## Recommended Order (Solo Dev)

1. **Pre-Phase** — Understand the codebase
2. **Phase 1** — MSP data model (foundation)
3. **Phase 3** — Certificate monitoring (high MSP value, visible wins)
4. **Phase 4** — Change detection (builds on Phase 3)
5. **Phase 8** — UI restyling (can interleave)
6. **Phase 2** — Discovery enhancements
7. **Phase 5** — VPN monitoring
8. **Phase 6** — AI identification
9. **Phase 7** — Debian packaging
10. **Phase 9** — Advanced features

### Rationale
- Phase 1 first because everything depends on it
- Phase 3 (certs) before Phase 2 (discovery) because certs provide immediate MSP value
- Phase 4 after Phase 3 so you have meaningful changes to detect
- UI restyling interleaved to keep motivation high (visible progress)
- AI identification later because it's powerful but not blocking

---

## Tech Stack Summary

| Component | Technology |
|-----------|------------|
| **Server Backend** | Rust (Axum) — inherited from Scanopy |
| **Server Database** | PostgreSQL 17 |
| **Server Frontend** | SvelteKit 2 + TypeScript + Tailwind |
| **Daemon** | Rust — inherited from Scanopy |
| **AI Search** | EXA API (via MCP) |
| **AI Vision/Reasoning** | OpenRouter (Claude/GPT-4o/Gemini) |
| **Packaging** | Debian .deb, Docker (optional) |

---

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Upstream sync** | Point-in-time fork (diverge) | Simpler maintenance, full control over direction |
| **Branding** | Full rename to **VantageNet** | Clean break, MSP-focused identity |
| **Time-series DB** | TimescaleDB | Route/latency metrics need time-series queries |

### TimescaleDB Strategy

1. **Phase 1-4:** Use plain PostgreSQL, but design schemas with TimescaleDB in mind
   - Use `TIMESTAMPTZ` for all time columns
   - Structure tables for hypertable conversion (time-based primary organization)
2. **Phase 5:** Enable TimescaleDB extension when VPN/latency monitoring lands
   - Convert metrics tables to hypertables
   - Set up retention policies (e.g., 90 days raw, 1 year aggregated)
3. **Benefit:** No deployment complexity until we actually need time-series features

### Rename Scope (Pre-Phase) ✅ COMPLETED

Renamed throughout:
- [x] Repository name (`N2con-Inc/VantageNet`)
- [x] Package names (Cargo.toml: `vantagenet-server`, package.json: `vantagenet-ui`)
- [x] Binary names (`vantagenet-server`, `vantagenet-daemon`)
- [x] Systemd service files (`vantagenet-daemon.service`)
- [x] Docker image names (`ghcr.io/n2con-inc/vantagenet/...`)
- [x] Config file paths (`/etc/vantagenet/`, `com.vantagenet.daemon`)
- [x] UI branding (title: VantageNet)
- [x] Documentation (README.md)
- [x] Environment variable prefixes (`VANTAGENET_*` with backwards compat)
- [x] Service definitions (`VantageNetDaemon`, `VantageNetServer`)
- [x] Database names (`vantagenet`)
- [x] Docker compose networks and containers

## Open Questions (Remaining)

1. **Multi-region/cloud support?**
   - Design doc mentions AWS sites
   - Daemons work anywhere with network access to server
   - Cloud metadata integration (AWS/Azure/GCP) could be future phase

2. **Mobile/responsive priority?**
   - Full mobile UI or desktop-focused?
   - Affects Phase 8 (UI) effort

---

*Roadmap created: December 30, 2025*  
*Pre-Phase rebrand completed: December 30, 2025*
