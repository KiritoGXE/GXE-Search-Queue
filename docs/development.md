# Development

The project uses Rust/Tokio/Axum, SQLx/SQLite, and React/TypeScript. Docker
Compose builds the dashboard and backend into one self-hosted application.

## Build

```sh
docker build -t gxe-search-queue:dev .
```

To build the dashboard separately, use Node 22 with `npm ci` and `npm run
build`. The Rust service uses Cargo with the committed `Cargo.lock`.

## Current implementation

- SQLite migrations and persistent requests, shared jobs, runs, history,
  settings, health, and cooldown records.
- Authenticated Axum API and React dashboard.
- Seerr polling synchronization and Radarr/Sonarr request/queue/search adapters.
- One-job scheduler worker with interval and fairness policy, service-health
  checks, startup pause, and manual handling for ambiguous external commands.
- Multi-stage Docker build and Compose configuration.

The installed service APIs and command details are not validated against live
instances. Per-indexer 429 handling and cooldown ingestion are not implemented.
Read the [integration validation record](integrations/README.md) before
connecting an instance. Never add secrets to source control or logs.

## Validation work before v0.1.0

Validate request modification/cancellation, Seerr automatic search, command
payloads and lifecycle, manual operation detection, import and season-pack
state, HTTP 429 / `Retry-After`, concurrent request fan-out, and restart during
an external operation. Add tests alongside each validated scheduler behavior;
the existing legacy tests do not establish these guarantees.
