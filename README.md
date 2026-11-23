# GXE Search Queue

> Intelligent search queue and scheduling system for the *Arr ecosystem.
>
> **GXE Republic** — open source, autonomous, community-driven.

GXE Search Queue is an open source project developed under the **GXE Republic** brand. It introduces an intelligent system for managing and scheduling media content searches across the *Arr ecosystem — **Seerr**, **Radarr**, **Sonarr**, and **Prowlarr**.

The software is designed to be **autonomous, reusable, and distributable** outside of any specific infrastructure: it contains no mandatory dependencies on particular IPs, services, or configurations.

## Why

- Reduce excessive load on public indexers.
- Manage searches through a smart, global queue.
- Eliminate duplicate search requests.
- Automate retries, indexer cooldowns, and error handling.
- Provide an authenticated administrative dashboard.
- Be simple to install and update (Docker-first).
- Enable community contributions and development.

## Key capabilities

| Capability | Details |
|---|---|
| Centralized Seerr requests | Single entry point for movie and series requests |
| Global queue | One global search at a time; minimum 10-minute interval between runs |
| Deduplication | Duplicate requests are detected and collapsed |
| Separated queues | Movies and series are managed independently |
| Progressive retries | Configurable backoff per failed request |
| Indexer cooldowns | Per-indexer cooldown enforcement |
| Manual search detection | Distinguishes user-initiated searches |
| State persistence | Full state recovery after restarts |
| Dashboard | Authenticated admin UI with 90-day history |
| Notifications | Alerts for important events |

## Architecture principles

Integrations with Seerr, Radarr, Sonarr, and Prowlarr happen **exclusively through their public APIs and webhooks**. GXE Search Queue never modifies the internal databases of integrated applications.

The architecture keeps the following concerns strictly separated, so each can be replaced or extended independently:

- Queue engine
- Scheduler
- Retry management
- State management
- Database
- External integrations
- Internal API
- Dashboard

See [docs/architecture.md](docs/architecture.md) for details and [docs/PROJECT_SPEC.md](docs/PROJECT_SPEC.md) for the full preliminary specification.

## Status

> ⚠️ **This project is in preliminary design (v0.1.0 target).**
> The repository currently contains scaffolding, CI/CD, and documentation.
> The technology stack is **not yet finalized** — see [docs/PROJECT_SPEC.md](docs/PROJECT_SPEC.md), section 4.

| Area | Status |
|---|---|
| Project identity | ✅ Defined |
| License | ✅ Apache-2.0 |
| Functional requirements | ✅ First definition complete |
| Technology stack | ⏳ Under evaluation |
| Internal architecture | ⏳ To be designed |
| Database | ⏳ To be designed |
| Integration APIs | ⏳ To be validated |
| Dashboard | ⏳ To be designed |
| Docker | 📐 Planned |
| CI/CD | ⏳ To be implemented |
| First release | 📅 Planned: 0.1.0 |

## Distribution

GXE Search Queue is **Docker-first**. Official images are published to GitHub Container Registry:

```
ghcr.io/kiritogxe/gxe-search-queue
```

Images are versioned (Semantic Versioning), multi-architecture (AMD64 / ARM64), built reproducibly by CI/CD, and configurable through environment variables and configuration files. Persistent volumes are supported for the database and application data.

Installation options:

1. Official Docker image (recommended)
2. Self-build from this repository
3. Custom image based on the official one

See [docs/installation.md](docs/installation.md) for instructions.

## Versioning

The project follows [Semantic Versioning](https://semver.org/):

| Version | Meaning |
|---|---|
| `0.1.0` | First experimental release |
| `0.2.0` | New compatible features |
| `0.2.1` | Bug fixes |
| `1.0.0` | First stable release (after validation of core features and reliability) |

## Documentation

- [Project specification (preliminary 0.1.0)](docs/PROJECT_SPEC.md)
- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Installation](docs/installation.md)
- [Development](docs/development.md)
- [Integrations](docs/integrations/)

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md) before submitting changes.

## Security

Found a vulnerability? See [SECURITY.md](SECURITY.md) for our disclosure policy.

## License

[Apache License 2.0](LICENSE) — see the full text in the [LICENSE](LICENSE) file.
