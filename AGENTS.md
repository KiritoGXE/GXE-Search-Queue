# Repository instructions

- Treat the project owner's GXE Search Queue master prompt as the authoritative
  source for approved behavior.
- This repository contains an experimental v0.1.0 preview. Clearly distinguish
  implemented behavior from integrations and guarantees that are not validated.
- The approved stack is Rust/Tokio/Axum, SQLx/SQLite, and React/TypeScript.
  Ask before changing this stack or another fundamental approved requirement.
- SQLite is the approved initial database. Do not introduce PostgreSQL, Redis,
  microservices, unrelated search/crawler connectors, or external notifications
  without explicit approval.
- Integrate only through supported Seerr, Radarr, Sonarr, and Prowlarr APIs or
  webhooks. Never modify their internal databases or the personal media stack.
- Never commit credentials or expose secrets in logs, examples, or chat.
- Do not claim per-indexer rate limiting, integration support, or production
  readiness without experimental validation and documentation.
- Preserve Apache-2.0 licensing and keep documentation consistent with actual
  behavior.
