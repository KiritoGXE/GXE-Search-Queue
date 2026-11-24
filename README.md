# GXE Search Queue

GXE Search Queue is an experimental, self-hosted search scheduler for Seerr,
Radarr, Sonarr, and Prowlarr. It persists approved Seerr requests in SQLite,
deduplicates work, and schedules one movie or episode search at a time. It does
not replace the *Arr applications and does not manage downloads.

**This is an early preview, not a stable or production-ready release.** The
per-indexer rate-limiting behavior has not been validated. Radarr and Sonarr
search commands can fan out across several Prowlarr indexers, so a 10-minute
gap between GXE commands does not guarantee a 10-minute gap per indexer.

## Quick start

1. Copy `.env.example` to `.env`, set a unique `GXE_ADMIN_TOKEN` of at least
   32 characters, then fill in service URLs and Radarr/Sonarr/Prowlarr API
   keys. Seerr, like the *Arr services, needs its API key for request
   synchronization; its public status endpoint does not validate that access.
2. Make sure Seerr's automatic search on approval is disabled. Otherwise its
   searches can bypass GXE. GXE never changes Seerr settings automatically.
3. Run `docker compose up -d --build` and open `http://localhost:8080`.
4. Sign in with `GXE_ADMIN_TOKEN`. The queue starts paused; inspect service
   status and known limitations before choosing **Resume queue**.

The Compose file builds locally and stores SQLite in the `app-data` volume.
The app container does not install or modify the media services. See
[installation](docs/installation.md) and [configuration](docs/configuration.md).

## What works in this preview

- Authenticated dashboard and API, using a bearer token.
- Persistent SQLite requests, deduplicated jobs, search history, and scheduler
  state; 90-day history pruning.
- Polling reconciliation of approved Seerr requests every minute.
- Movie and monitored, aired episode jobs sourced from Radarr and Sonarr.
- One GXE-managed search at a time, 10-minute minimum start interval, category
  fairness, episode ordering, no-result retry delays, and manual priority.
- Search command polling, download/library checks, and restart reconciliation
  for persisted manager command IDs. Unknown outcomes remain blocked for manual
  reconciliation; GXE never blindly resubmits an uncertain command.
- Persistent dashboard alerts for technical search failures and uncertain
  external outcomes, plus service outage/restoration transitions. Repeated
  active alerts are grouped until acknowledged.
- Dashboard controls for pausing, prioritizing, retrying, removing unstarted
  jobs, and changing core scheduler settings.

## Known limitations

- API behavior has not been validated against live Seerr/Radarr/Sonarr/Prowlarr
  versions. See the [integration validation record](docs/integrations/README.md).
- No Seerr webhooks yet; polling is used as the fallback synchronization.
- Per-indexer HTTP 429 detection, `Retry-After`, cooldown controls, and safe
  indexer-specific pacing are not implemented. Cooldown display is groundwork,
  not proof of indexer protection.
- Technical errors and uncertain command results are held for administrator
  reconciliation because safe automatic retry behavior is not finalized.
- Dashboard notifications cover technical search failures, ambiguous outcomes,
  and observed service availability transitions. Indexer cooldown and queue
  completion alerts are not yet wired into the event feed.
- TV request modifications, season-pack satisfaction, and comprehensive
  recovery scenarios still need integration validation.
- Authentication is a single local administrator token. Deploy only on a
  trusted network; use a TLS reverse proxy for remote access.
- No image has been published to GHCR. No stable release or multi-architecture
  release has been produced.

## Project documents

- [Specification](docs/PROJECT_SPEC.md)
- [Architecture](docs/architecture.md)
- [Roadmap](ROADMAP.md)
- [Development guide](docs/development.md)
- [License](LICENSE)

Apache-2.0. Maintained by GXE Republic.
