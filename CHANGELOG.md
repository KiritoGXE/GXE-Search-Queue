# Changelog

All notable changes to this project are documented in this file, following the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format and
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Authenticated Axum API and responsive React/TypeScript dashboard.
- Docker Compose installation with persistent SQLite and health check.
- Seerr polling, Radarr/Sonarr service and command adapters, queue controls,
  settings, run history, and manual reconciliation for ambiguous commands.
- Scheduler worker with a safe paused first start, global serialization,
  minimum interval, category fairness, and no-result retry delays.
- Initial SQLite schema and embedded migrations for requests, jobs, runs,
  service health, cooldown records, settings, and dashboard events.

### Changed

- Stack selected as Rust/Tokio/Axum, SQLx/SQLite, and React/TypeScript.
- Documentation now describes the experimental preview and its unvalidated
  integration and per-indexer rate-limiting limits.
