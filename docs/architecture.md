# Architecture Overview

`gxe-search-queue` is an autonomous service that sits between **Seerr** (request manager) and the *Arr search chain (**Prowlarr** as indexer aggregator, **Radarr**/ **Sonarr** as content managers). It receives movie and series requests, queues them intelligently, and triggers searches under strict rate and deduplication constraints.

> ⚠️ *The internal architecture is still in preliminary design (see [PROJECT_SPEC.md](PROJECT_SPEC.md)). This document describes the target architecture; the technology stack is not yet finalized.*

## System diagram

```
 +----------+  webhook   +-------------------------+   API    +-------------+
 |  Seerr   | ---------- |                         | -------- |  Prowlarr   |
 +----------+            |     gxe-search-queue    |          | (indexers)  |
                         |                         |          +-------------+
                         |  +-------+ +-----------+ |
                         |  |queue  | |scheduler  | |
                         |  +-------+ +-----------+ |          +-----------+
                         |  +-------+ +-----------+ |          |  Radarr   |
                         |  |retry  | |state mgmt | |          |  Sonarr   |
                         |  +-------+ +-----------+ |          +-----------+
                         |  +-------+ +-----------+ |
                         |  | api   | |dashboard  | |
                         |  +-------+ +-----------+ |
                         |  +---------------------+ |
                         |  | notifications       | |
                         |  +---------------------+ |
                         +-------------+-------------+
                                         |
                                         v
                                  +-----------+
                                  |  database |
                                  +-----------+
```

## Components

Each component is isolated so it can be replaced or extended without rewriting the whole system (spec, section 3).

- **Queue engine (`src/core`)** — the global search queue. Enforces:
  - One global search at a time
  - Minimum 10-minute interval between search runs
  - Request deduplication
  - Separate queues for movies and series
- **Scheduler (`src/scheduler`)** — schedules queue execution windows and recurring maintenance jobs (cooldown expiry, state cleanup, history pruning).
- **Retry management** — progressive (backoff) retries for failed requests, with per-request retry budgets.
- **State management** — tracks request/job lifecycle; guarantees full state recovery after restarts (no in-memory-only state).
- **Database (`src/database`)** — persistent store for requests, jobs, indexers, cooldowns, and history (90-day retention). Schema must support progressive migrations.
- **Integrations (`src/integrations`)** — adapters for Seerr, Radarr, Sonarr, and Prowlarr. **API/webhook only**: the software never writes to the internal databases of integrated apps.
- **Internal API (`src/api`)** — REST control plane for the dashboard and external tools (submit/query/cancel, status, indexer cooldown state).
- **Dashboard (`src/dashboard`)** — authenticated administrative UI with 90-day history.
- **Notifications (`src/notifications`)** — alerts for important events (job success/failure, cooldowns, queue overflow).

## Behavioral rules (from the approved functional spec)

1. **One search at a time** — the global queue serializes searches.
2. **10-minute minimum interval** — between the start of two searches.
3. **Deduplication** — identical requests collapse into one queue entry.
4. **Movie/series separation** — distinct pipelines, shared scheduling constraints.
5. **Progressive retries** — exponential/stepped backoff until retry budget exhausted.
6. **Indexer cooldowns** — per-indexer cooldown windows pause that indexer, not the whole queue.
7. **Manual search detection** — user-initiated (e.g. from Radarr) searches are detected and handled separately from queued ones.
8. **State recovery** — any restart resumes from persisted state; no lost jobs.
9. **90-day history** — completed/failed entries are retained for 90 days, then pruned.

## Security and independence

- No mandatory dependency on GXE Republic infrastructure (no hardcoded IPs/services).
- Integration credentials stored securely (env/secrets); never logged.
- Dashboard requires authentication.
- No proprietary cloud services required to run.
