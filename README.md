# VantageNet

**MSP-focused network discovery and monitoring platform.**

VantageNet is a fork of [Scanopy](https://github.com/scanopy/scanopy), extended with enterprise MSP features including multi-tenant client management, certificate monitoring, VPN detection, change alerting, and AI-assisted service identification.

## Key Features

- **Automatic Discovery**: Scans networks to identify hosts, services, and their relationships
- **200+ Service Definitions**: Auto-detects databases, web servers, containers, network infrastructure, monitoring tools, and enterprise applications
- **Interactive Topology**: Generates visual network diagrams with extensive customization options
- **Distributed Scanning**: Deploy daemons across network segments to map complex topologies
- **Docker Integration**: Discovers containerized services automatically
- **Organization Management**: Multi-user support with role-based permissions
- **Scheduled Discovery**: Automated scanning to keep documentation current

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the development plan including:
- MSP Client/Site hierarchy
- Certificate & SSH key monitoring
- Change detection & alerting  
- VPN connectivity monitoring
- AI-assisted service identification
- Reconya-inspired dark UI theme

## Quick Start

```bash
docker compose up -d
```

Access the UI at `http://<your-server-ip>:60072`, create your account, and wait for the first discovery to complete.

## Development

```bash
# Install dependencies
make install-dev-mac  # or install-dev-linux

# Start development environment
make dev-container

# Or run components separately
make setup-db
make dev-server    # Terminal 1
make dev-ui        # Terminal 2  
make dev-daemon    # Terminal 3
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       VantageNet Server                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   Axum API  │  │  PostgreSQL │  │  SvelteKit Frontend     │ │
│  │  (Rust)     │  │  + Timescale│  │  + Tailwind (dark)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
        ▲                                        ▲
        │ Discovery Updates                      │ Work Requests
        │ (Push or Pull)                         │
        ▼                                        ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ VantageNet    │    │ VantageNet    │    │ VantageNet    │
│ Daemon        │    │ Daemon        │    │ Daemon        │
│ (Site A)      │    │ (Site B)      │    │ (Site C)      │
└───────────────┘    └───────────────┘    └───────────────┘
```

## License

AGPL-3.0 - See [LICENSE.md](LICENSE.md)

## Contributing

Contributions welcome! See [contributing.md](contributing.md) for guidelines.

---

**Based on [Scanopy](https://github.com/scanopy/scanopy)** - Forked with love for MSP use cases
