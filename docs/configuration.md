# Configuration

Set secrets through the environment or a protected `.env` file. Never commit
service API keys or the dashboard token.

| Variable | Default | Purpose |
|---|---|---|
| `GXE_ADMIN_TOKEN` | Required | Dashboard/API bearer token; minimum 32 characters. |
| `GXE_PORT` | `8080` | Host port published by Compose. |
| `DATABASE_PATH` | `/app/data/gxe-search-queue.sqlite3` in Docker | Persistent SQLite database. |
| `RUST_LOG` | `info` | Rust tracing filter. |
| `SEERR_URL` | Unset | Seerr API URL; required for request synchronization. |
| `SEERR_API_KEY` | Empty | Required Seerr API key for the request API; the public status endpoint alone cannot validate request access. |
| `RADARR_URL`, `RADARR_API_KEY` | Unset | Radarr API connection. |
| `SONARR_URL`, `SONARR_API_KEY` | Unset | Sonarr API connection. |
| `PROWLARR_URL`, `PROWLARR_API_KEY` | Unset | Prowlarr availability check. |

All URLs must be reachable from the app container. With Docker Compose, use
service names on a shared network, not `localhost` unless the service runs in
the same container.

## Dashboard settings

| Setting | Default | Constraint |
|---|---:|---|
| Minimum interval | 600 seconds | Cannot be lower than 600 seconds. |
| Queue capacity | 0 (unlimited) | Zero means unlimited; otherwise positive. |
| Search timeout | 900 seconds | Positive; ambiguous runs require reconciliation. |
| Theatrical release delay | 0 days | Configurable; the previously discussed 30 days is not approved. |

The scheduler starts globally paused on a fresh database. Seerr synchronization
runs every 60 seconds and service health checks run every 30 seconds. Those
intervals are currently implementation defaults, not dashboard settings.

The dashboard token is stored in the browser's session storage and sent as a
Bearer token. It is not an account system and does not provide TLS; keep the
service on a trusted network or put it behind a TLS reverse proxy.
