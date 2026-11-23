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
                                  `-- Prowlarr (health check only in this preview)
```

The application polls Seerr every minute, checks service health every 30
seconds, and runs one scheduling worker. It begins globally paused. Search
commands are serialized, and an active or ambiguous persisted run prevents a
new GXE operation. Ambiguous results require administrator reconciliation; the
application does not blindly repeat a command after a timeout or restart.

## Scheduler

An eligible operation is claimed transactionally in SQLite. The default
minimum interval is 600 seconds from the preceding run start; it cannot be set
below 600 seconds. A command still running past that interval continues to
block the queue. When both categories have eligible items, the scheduler
alternates after at most two consecutive searches of one category. Episode
selection favors earlier episodes and rotates across series when possible.

No-result searches use the approved escalating delays. Technical failures do
not increment that content counter; because a technical retry policy has not
been approved, the job is suspended for administrator action. An external
command timeout is preserved as ambiguous instead of automatically retried.

## Integrations and limits

Seerr request polling, Radarr/Sonarr command adapters, service health checks,
and a preflight check for active searches are implemented from documented API
surfaces. Live compatibility is still unverified. Seerr automatic search must
be disabled by the administrator or it can bypass GXE.

The Prowlarr adapter only checks service availability. It does not ingest 429
events, respect `Retry-After`, apply indexer-specific cooldowns, or verify
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
