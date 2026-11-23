# GXE Search Queue
## Preliminary project specification

**Document version:** 0.1.0
**Planned software version:** 0.1.0
**Status:** Experimental v0.1.0 preview; integrations not live-validated
**Organization:** GXE Republic
**License:** Apache License 2.0
**GitHub repository:** `KiritoGXE/GXE-Search-Queue`
**Distribution:** Docker / GitHub Container Registry
**Technology stack:** Rust/Tokio/Axum + SQLx/SQLite backend; React/TypeScript frontend (approved)

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

**Approved:** Rust with Tokio and Axum, SQLx and SQLite; React with TypeScript
for the dashboard. This is a modular monolith. API contracts, detailed module
boundaries, and integration behavior remain under design and validation.

The choice will take into account:

- Development and maintenance simplicity.
- Performance and resource consumption.
- Async operations handling.
- Availability of libraries for the *Arr ecosystem APIs.
- Ease of building the dashboard.
- Automated testing.
- Distribution via Docker.
- Support for Linux AMD64 and ARM64.

Avoid microservices and additional infrastructure unless validation shows they
are needed.

## 5. GitHub repository

Planned repository:

`github.com/KiritoGXE/GXE-Search-Queue`

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

The planned image name for a future release is:

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

## 8. Functional requirements and status

The complete approved behavior is captured in the project owner's master
prompt. This repository document is a summary; it does not replace those rules.
Approved requirements include the global queue, one active GXE search, 10-minute
start-to-start minimum, configurable timeout and capacity, movie/episode job
granularity, fairness, deduplication, Seerr reconciliation, retry schedule,
SQLite persistence, authenticated dashboard, dashboard-only notifications,
and 90-day history.

### 8.1 Queue and scheduling

- One global queue and at most one active GXE-managed search.
- The minimum interval is 10 minutes from the start of the prior search. If a
  search runs longer than the interval, the next may begin immediately after it
  finishes. A search timeout is dashboard-configurable and does not prove the
  external operation failed.
- Queue capacity is unlimited by default and configurable. State survives
  restarts.
- Track command submission, start, completion, interval expiry, timeout, and
  service availability separately. Reconcile external state before retrying an
  ambiguous operation.
- If both movies and episodes are eligible, schedule at most two consecutive
  searches of one category before giving the other a turn. Avoid series
  monopolization; process a series' episodes oldest first.
- An administrator may prioritize an unstarted job, but cannot interrupt an
  active search or bypass the interval or indexer cooldown.

### 8.2 Media work

- Each search concerns one movie or one episode. Radarr/Sonarr retain control
  of release selection, quality profiles, season packs, downloads, and imports.
- Do not search a movie already available, downloading, grabbed, or awaiting
  import. A found/grabbed result completes the search job; download/import
  continues independently.
- For movies, use the digital release date when known; otherwise use theatrical
  date plus a configurable delay. The proposed initial 30-day delay is not
  approved. If no usable date exists, wait and refresh metadata without an
  indexer search.
- Only episodes in seasons requested through Seerr are followed. Only episodes
  Sonarr considers available are added; future episodes are not active jobs.
  Specials monitored by Sonarr are included. Add newly available episodes
  automatically, including already-aired missing episodes in ended/cancelled
  series. Sonarr decides between episode searches and season packs.
- Remove episodes satisfied by a season-pack grab. When a TV request changes,
  add only newly requested missing episodes and preserve existing work. Retry
  synchronization if Sonarr has not yet added the requested series.
- If the last active Seerr request for a series is cancelled, stop following
  future episodes and cancel its unstarted work. Do not interrupt active
  searches or delete downloads/imported media.

### 8.3 Requests and reconciliation

- Only approved Seerr requests create work. Manual Radarr/Sonarr additions are
  observed to avoid duplicate searches but never imported into the GXE queue.
- Deduplicate movies, series, and episodes. Keep one work item with links to all
  active Seerr requests and requesters; preserve first-request ordering.
- Cancelling one duplicate request keeps shared work while another active
  request remains. Remove unstarted work when none remain; do not interrupt
  started work.
- Use Seerr webhooks and periodic reconciliation. Webhooks must be idempotent.

### 8.4 Retries and failures

No-result searches increment the content retry counter using this schedule:
1 hour, 6 hours, 24 hours, 3 days, 7 days; later attempts wait 7 days and then
repeat the cycle. Technical failures have a separate configurable policy that
is not yet finalized and must not increment this counter.

For HTTP 429, respect `Retry-After` when present, otherwise use progressive
backoff. Cool down only the affected indexer when technically possible. Do not
count 429 as no-results or let manual retry bypass cooldown. Notify admins after
several hours of blockage; the exact threshold is open. Suspend global search
when all usable indexers are unavailable. Do not claim per-indexer protection
until demonstrated experimentally.

On service outage, suspend only affected work: Radarr for movies, Sonarr for
episodes, Prowlarr for all searches, and Seerr for request synchronization.
Resume automatically after recovery. On restart, inspect persisted external
commands before resuming; never blindly repeat an operation.

### 8.5 Dashboard, security, and retention

Provide a standalone responsive dashboard with authentication. It must show
application/service/indexer health, current work, queue counts, cooldowns,
history, and relevant statistics. Admins can pause/resume, prioritize, remove
unstarted jobs, retry, suspend supported indexers, inspect history, and change
settings. Emptying the queue requires confirmation. Never expose controls that
delete torrents or media.

Notifications are dashboard-only, important-event-only, and grouped to avoid
flooding. Retain search/event history for 90 days without deleting active jobs
or operational state. Do not commit or log integration secrets.

Technical retry policy and the proposed 30-day theatrical release delay are
not finalized. Per-indexer cooldown implementation and manual-search detection
must be validated against supported APIs. A serial GXE queue does not prove
per-indexer request pacing. Initial persistence and scheduler primitives exist;
external integrations and user-facing application behavior are not yet
implemented or validated.

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
| Technology stack | Approved: Rust/Tokio/Axum, SQLx/SQLite, React/TypeScript |
| Internal architecture | Modular monolith implemented for experimental preview; recovery and API behavior need validation |
| Database | SQLite schema and initial migration implemented; migration policy needs review before stable release |
| Integration APIs | Seerr/Radarr/Sonarr/Prowlarr adapters implemented from API surfaces; live behavior not validated |
| Dashboard | Authenticated React/TypeScript dashboard implemented for core queue controls |
| Docker | Local multi-stage Compose build available; no GHCR publication |
| CI/CD | To be implemented |
| Public repository | Planned; publication not confirmed |
| First release | Experimental target: 0.1.0; not released |

---

**GXE Search Queue — Preliminary specification 0.1.0**

Reference document for the technical design and development of the GXE Republic open source project.
