# Architecture Overview

`gxe-search-queue` acts as the primary background processing engine within the GX.Engine platform. It receives search, crawling, and indexing jobs, manages their execution state, handles retries, and forwards completed payloads to downstreams.

## System Diagram

```
                       +-------------------+
                       |    API Clients    |
                       +---------+---------+
                                 | REST
                                 v
                       +-------------------+
                       |     src/api       |
                       +---------+---------+
                                 |
                                 v
+-------------------+  +-------------------+  +-------------------+
|   src/scheduler   |->|     src/core      |<-| src/notifications |
+-------------------+  +---------+---------+  +-------------------+
                                 |
                                 +----------+
                                 |          |
                                 v          v
                        +------------+ +----------+
                        | PostgreSQL | |  Redis   |
                        +------------+ +----------+
```

## Subsystems

- **Core (`src/core`)**: Implements task queues, priority semantics, backoff logic, and worker lifecycle events.
- **Scheduler (`src/scheduler`)**: Periodically enqueues recurring search jobs (cron syntax) and evaluates dependent job graphs.
- **Integrations (`src/integrations`)**: Contains adapters for external search backends, crawlers, and third-party data providers.
- **Database (`src/database`)**: Handles SQL pool management, migrations, transaction helpers, and state updates.
- **API (`src/api`)**: Exposes REST endpoints to submit jobs, query status, update worker settings, and cancel tasks.
- **Dashboard (`src/dashboard`)**: Embedded or separate frontend dashboard interface.
- **Notifications (`src/notifications`)**: Dispatches status events via webhooks, emails, and channels.

## Guarantees

- **At-least-once delivery**: Jobs remain in queue until explicitly acknowledged by a worker.
- **Lease timeout**: Stalled workers forfeit their active lease, re-queuing the job for another worker.
- **Idempotency**: Downstreams should handle re-delivered jobs safely using unique job IDs.
