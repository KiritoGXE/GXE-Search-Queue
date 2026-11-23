# Installation

This preview is installed from the repository with Docker Compose. A GHCR
image is planned but has not been published.

## Requirements

- Docker Engine and the Docker Compose plugin.
- A Docker network route to Seerr, Radarr, Sonarr, and Prowlarr.
- API keys created in those services. Do not paste keys into chat or commit
  them to source control.

## Start

```sh
# From a checked-out copy of the project:
cp .env.example .env
```

Edit `.env` with a unique random `GXE_ADMIN_TOKEN` (at least 32 characters),
the service URLs reachable from the container, and their API keys. On a shared
Docker network, use service DNS names. Keep `.env` private.

Before enabling GXE, disable Seerr's automatic search on approval yourself;
GXE does not change Seerr configuration. Review the documented rate-limiting
limitations because serial searches do not prove per-indexer pacing.

```sh
docker compose up -d --build
```

Open `http://localhost:8080` (or the host and port configured in `GXE_PORT`)
and sign in with the administrator token. The queue starts paused. Confirm the
integration health shown on the dashboard, review the preview limitations,
then resume it when you are ready.

## Data, updates, backup, restore

SQLite lives in the named `app-data` volume. Stop the service before making a
file-level backup so the database and WAL are consistent:

```sh
docker compose stop app
docker run --rm -v gxe-search-queue_app-data:/data -v "$PWD":/backup alpine \
  tar -czf /backup/gxe-search-queue-data.tgz -C /data .
docker compose start app
```

Restore by stopping the app, extracting the archive into the same volume, and
starting the app. Back up `.env` separately in a protected location. For an
update, fetch the intended source revision, inspect its changelog and migration
notes, then run `docker compose up -d --build`. Preserve a backup before
updating; SQLite migrations run automatically at startup.

Do not expose this preview directly to the public internet. Its single-token
authentication is intended for a trusted local network. Use a TLS reverse
proxy if accessing it remotely.
