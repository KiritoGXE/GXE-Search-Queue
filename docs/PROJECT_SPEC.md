# GXE Search Queue
## Preliminary project specification

**Document version:** 0.1.0
**Planned software version:** 0.1.0
**Status:** Preliminary design
**Organization:** GXE Republic
**License:** Apache License 2.0
**Planned repository:** `KiritoGXE/gxe-search-queue`
**Distribution:** Docker / GitHub Container Registry
**Technology stack:** To be evaluated

---

## 1. Project identity and vision

GXE Search Queue is an open source project developed under the GXE Republic brand.

It is intended to introduce an intelligent system for managing and scheduling media content searches within the *Arr ecosystem.

The software must be autonomous, reusable, and distributable outside of the GXE Republic infrastructure.

It must not contain mandatory dependencies on configurations, IP addresses, services, or resources specific to the original infrastructure.

### Main objectives

- Reduce excessive requests to public indexers.
- Manage searches through smart queues.
- Avoid duplicate searches.
- Automate retries, cooldowns, and error handling.
- Provide an administrative dashboard.
- Be simple to install and update.
- Allow community contributions and development.

## 2. Open source and license

The project will be published on GitHub under the **Apache License 2.0**.

The license allows use, modification, and redistribution of the software, including in commercial contexts, in accordance with its terms.

The repository will include:

- `LICENSE`
- `README.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- Issue and pull request templates.

Project management will be centralized in the GXE Republic GitHub repository.

## 3. Architecture and independence

GXE Search Queue will be an autonomous service.

Integrations with Seerr, Radarr, Sonarr, and Prowlarr will happen through supported APIs and webhooks.

The software must not directly modify the internal databases of the integrated applications.

The architecture must keep separated:

- Queue engine.
- Scheduler.
- Retry management.
- State management.
- Database.
- External integrations.
- Internal API.
- Dashboard.

This separation will allow individual components to be replaced or extended without rewriting the entire project.

## 4. Technology stack

**Not yet defined.**

The choice will take into account:

- Development and maintenance simplicity.
- Performance and resource consumption.
- Async operations handling.
- Availability of libraries for the *Arr ecosystem APIs.
- Ease of building the dashboard.
- Automated testing.
- Distribution via Docker.
- Support for Linux AMD64 and ARM64.

The technology choice will be made before implementation.

## 5. GitHub repository

Planned repository:

`github.com/KiritoGXE/gxe-search-queue`

Preliminary structure:

```text
gxe-search-queue/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   ├── workflows/
│   └── PULL_REQUEST_TEMPLATE.md
│
├── docs/
│   ├── architecture.md
│   ├── configuration.md
│   ├── installation.md
│   ├── development.md
│   └── integrations/
│
├── src/
│   ├── core/
│   ├── scheduler/
│   ├── integrations/
│   ├── database/
│   ├── api/
│   ├── dashboard/
│   └── notifications/
│
├── tests/
│   ├── unit/
│   ├── integration/
│   └── fixtures/
│
├── docker/
├── config/
├── .dockerignore
├── .env.example
├── compose.yaml
├── Dockerfile
├── LICENSE
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
└── SECURITY.md
```

The final structure will depend on the selected stack.

## 6. Docker distribution

The project will be Docker-first.

Official distribution will happen through GitHub Container Registry:

`ghcr.io/kiritogxe/gxe-search-queue`

Images must be:

- Versioned.
- Reproducible via CI/CD pipeline.
- Multi-architecture, targeting AMD64 and ARM64.
- Accompanied by tags consistent with releases.
- Configurable through environment variables and configuration files.
- Provided with persistent volumes for database and application data.

Users must be able to choose between:

1. Installation via the official Docker image.
2. Self-build from the repository.
3. Optional customization of the image.

GitHub Actions will be used for testing, building, and automatic publishing of images.

## 7. Versioning

The project will use Semantic Versioning.

| Version | Meaning |
|---|---|
| `0.1.0` | First experimental release |
| `0.2.0` | New compatible features |
| `0.2.1` | Fixes |
| `1.0.0` | First stable release |

Version 1.0.0 will only be reached after validation of the main features and system reliability.

## 8. Planned features

Queue engine features remain those defined in the original functional specification:

- Centralized management of Seerr requests.
- Global queue for movies and episodes.
- One global search at a time.
- Minimum 10-minute interval between runs.
- Request deduplication.
- Separate management of movies and series.
- Progressive retries.
- Indexer cooldown management.
- Detection of manual searches.
- State recovery after restarts.
- Administrative dashboard.
- 90-day history.
- Notifications for important events.

The already-approved functional rules remain unchanged.

## 9. Distribution and security principles

The project must not require proprietary cloud services to function.

Integration credentials must be stored securely and must not appear in logs.

The dashboard must provide authentication.

Configuration must be documented through examples free of real credentials.

The database must be persistent and compatible with progressive schema upgrades.

## 10. Project status

| Area | Status |
|---|---|
| Project identity | Defined |
| License | Apache-2.0 |
| Functional requirements | First definition complete |
| Technology stack | To be evaluated |
| Internal architecture | To be designed |
| Database | To be designed |
| Integration APIs | To be validated |
| Dashboard | To be designed |
| Docker | Architecture planned |
| CI/CD | To be implemented |
| Public repository | To be created |
| First release | Planned: 0.1.0 |

---

**GXE Search Queue — Preliminary specification 0.1.0**

Reference document for the technical design and development of the GXE Republic open source project.
