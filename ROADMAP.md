# Roadmap to v0.1.0

## Current preview

The repository now contains an installable Docker Compose application with a
Rust/Axum API, SQLite persistence, React dashboard, Seerr polling, Radarr/Sonarr
search adapters, and a serialized scheduler. Fresh installs start paused.
This is an experimental preview and is not production-ready.

## Work required before v0.1.0

1. Validate Seerr, Radarr, Sonarr, and Prowlarr API behavior against explicit
   supported versions. In particular validate command payload/status, manual
   search detection, import and season-pack state, request changes, and restart
   recovery.
2. Implement and experimentally validate indexer 429 detection, `Retry-After`,
   cooldown state, and Prowlarr controls where supported. Do not claim indexer
   protection until the measured behavior supports that claim.
3. Complete the technical specification and agree remaining technical retry,
   notification, and release-date policies.
4. Add webhook ingestion, reliable season/request modification reconciliation,
   structured dashboard notifications, and full settings/health details.
5. Add focused unit and integration coverage for scheduler fairness, request
   deduplication/cancellation, 50+ concurrent requests, large queues, timeout
   ambiguity, HTTP 429, imports, and restart recovery.
6. Validate Compose health, upgrade, backup/restore, and multi-architecture
   image builds; prepare release notes and known limitations.

Do not publish `latest`, call the project stable, or mark it production-ready
before the required validation is complete.
