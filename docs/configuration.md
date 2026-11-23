# Configuration Reference

`gxe-search-queue` reads configuration from environment variables or file-based configuration in `config/`.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `LOG_LEVEL` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`). |
| `API_BIND_ADDR` | `0.0.0.0` | IP address for the REST API server. |
| `API_PORT` | `8080` | Port for the REST API server. |
| `DASHBOARD_BIND_ADDR` | `0.0.0.0` | IP address for the dashboard interface. |
| `DASHBOARD_PORT` | `8081` | Port for the dashboard interface. |
| `DATABASE_URL` | — | Postgres connection string (`postgres://user:pass@host:port/db`). |
| `DATABASE_POOL_MAX` | `10` | Maximum connections in the Postgres pool. |
| `REDIS_URL` | — | Redis connection string (`redis://host:port`). |
| `SCHEDULER_TIMEZONE` | `UTC` | Timezone evaluated for scheduled cron jobs. |

## File Configuration

Runtime default settings live under `config/default.toml` (or `.json`, `.yaml`). Values provided through environment variables take precedence over file-based values.

Example `config/default.toml`:

```toml
[server]
api_port = 8080
dashboard_port = 8081

[queue]
default_lease_duration_secs = 60
max_retries = 3

[database]
max_connections = 10
```
