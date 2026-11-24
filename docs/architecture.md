# Architecture and current status

GXE is a single Rust/Tokio/Axum application with a React/TypeScript dashboard
and SQLite persistence. Its purpose is to coordinate requests with existing
services; it does not manage media releases, downloads, or playback.

```text
Seerr --periodic request sync--> GXE Search Queue
                                  |-- SQLite (queue, run state, settings, history)
                                  |-- authenticated dashboard/API
                                  |-- Radarr (one movie search command)
                                  |-- Sonarr (one episode search command)
                                  `-- Prowlarr (health plus read-only indexer telemetry)
```

The application polls Seerr every minute, checks service health every 30
seconds, and runs one scheduling worker. It begins globally paused. Search
commands are serialized, and an active or ambiguous persisted run prevents a
new GXE operation. On startup and periodically, GXE queries the manager for
persisted command IDs and reconciles completed or failed operations. A command
submission with no persisted ID, an unavailable manager, or an unknown command
status remains blocked for administrator reconciliation; GXE never blindly
repeats it.

## Scheduler

An eligible operation is claimed transactionally in SQLite. The default
minimum interval is 600 seconds from the preceding run start; it cannot be set
below 600 seconds. A command still running past that interval continues to
block the queue. When both categories have eligible items, the scheduler
alternates after at most two consecutive searches of one category. Episode
selection favors earlier episodes and rotates across series when possible.

No-result searches use the approved escalating delays. Technical failures do
not increment that content counter; because a technical retry policy has not
been approved, the job is suspended for administrator action. If a persisted
manager command reports completion after a restart, GXE checks queue/library
state before recording a result and allows the same visibility window used by
normal searches.

## Integrations and limits

Seerr request polling, Radarr/Sonarr command adapters, service health checks,
and a preflight check for active searches are implemented from documented API
surfaces. Live compatibility is still unverified. Seerr automatic search must
be disabled by the administrator or it can bypass GXE.

The Prowlarr adapter checks service availability and exposes a normalized,
read-only view of configured indexers, Prowlarr-reported disabled windows, and
aggregate query statistics from the previous 24 hours. These aggregate failures
do not reliably identify HTTP 429 events. GXE does not ingest 429 events, respect
`Retry-After`, apply indexer-specific cooldowns, control indexers, or verify
query fan-out. Therefore the 10-minute GXE interval is not a guarantee of
per-indexer protection. Do not claim that protection until experimentally
validated. No external service database is accessed or modified.

## Persistence

The initial migration includes requesters through Seerr request rows, queue
jobs and request links, external search commands, service health, cooldown
records, settings, and dashboard events. Transactions protect idempotent
request/job insertion and scheduler claims. The database is mounted in the
Compose named volume; completed history is pruned after 90 days while active
operations and queue state are retained.
