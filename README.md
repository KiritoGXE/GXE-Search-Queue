# GXE-Search-Queue

Distributed, fault-tolerant queue and processing engine for search workloads within the GX.Engine ecosystem. Built with Rust for high throughput and low latency.

## Features

- **Core queue engine** — durable queue with at-least-once delivery semantics
- **Scheduler** — cron-like and dependency-aware job scheduling
- **Integrations** — modular connectors for search sources and crawlers
- **Database layer** — Postgres for persistence, Redis for fast state and locks
- **REST API** — control plane for queue management and observability
- **Dashboard** — web UI for monitoring queues, jobs, and workers
- **Notifications** — webhooks, email, and channel alerts on job lifecycle events

## Status

This repository is currently a scaffolding stage. The directory layout, CI, and packaging are in place; application code is being added module by module.

## Repository layout

```
gxe-search-queue/
├── .github/          # issue templates, workflows, PR template
├── docs/             # architecture, configuration, installation, development
├── src/
│   ├── core/         # queue engine primitives
│   ├── scheduler/    # scheduling logic
│   ├── integrations/ # connector modules
│   ├── database/     # persistence and migrations
│   ├── api/          # REST control plane
│   ├── dashboard/    # web UI
│   └── notifications/# alerting and webhooks
├── tests/            # unit, integration, fixtures
├── docker/           # Docker helper assets
├── config/           # runtime configuration examples
├── compose.yaml      # local stack (app + Postgres + Redis)
└── Dockerfile        # multi-stage production build
```

## Quick start

See [docs/installation.md](docs/installation.md) for prerequisites and setup steps, or jump straight to local development with Docker:

```sh
docker compose up -d
```

## Documentation

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

MIT — see [LICENSE](LICENSE).
