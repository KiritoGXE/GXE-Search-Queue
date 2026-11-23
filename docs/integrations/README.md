# Integration validation

Adapters now call documented API surfaces, but they have not been validated
against live service versions. This is implementation status, not a claim of
supported compatibility. Record product versions and observed behavior before
shipping support.

| Service | Documented surface found | Still to validate |
|---|---|---|
| Seerr | Polling adapter calls paginated `GET /api/v1/request`; official [OpenAPI specification](https://github.com/seerr-team/seerr/blob/develop/seerr-api.yml). | Webhooks are not implemented. Validate request shape, approvals, modification/cancellation, and how automatic search interacts with GXE. The administrator must disable Seerr automatic search manually or searches can bypass GXE. |
| Radarr | Adapter calls movie, queue, system status, and command routes under `/api/v3`; official [API reference](https://radarr.video/docs/api/). | Validate command request bodies and status query behavior, completion/result semantics, queue/import checks, and manual search detection against supported versions. |
| Sonarr | Adapter calls series, episode, queue, system status, and command routes under `/api/v3`; official [API reference](https://sonarr.tv/docs/api/). | Validate installed API version, command request/status, episode availability/monitoring, season-pack satisfaction, import state, and manual search detection. |
| Prowlarr | Adapter currently checks `/api/v1/system/status`; official [API reference](https://prowlarr.com/docs/api/) also lists indexer status and management surfaces. | No 429 event/cooldown ingestion, `Retry-After` handling, or indexer control is implemented. Validate fan-out concurrency and safe per-indexer pacing before claiming indexer protection. |

## Current conclusions

- Public API documentation exists for the core services, but endpoint presence
  does not validate request bodies, state semantics, or compatibility with the
  installed versions.
- Seerr automatic search can trigger Radarr/Sonarr searches on approval. The
  administrator must disable it before resuming GXE; GXE does not change it.
- Prowlarr exposes indexer status and indexer update routes. This does not by
  itself establish that GXE can safely control cooldowns or pace requests per
  indexer.
- Serializing GXE commands does not prove per-indexer request pacing. Do not
  claim indexer protection before experimental validation.
- Timeout recovery must query the external command state before retrying.

## Local validation status

No Seerr, Radarr, Sonarr, or Prowlarr container was visible during read-only
inspection of the Docker daemon, so installed versions and live behavior could
not be validated. No service was modified and no credentials were requested.
The application must be treated as experimental until these integration checks
are repeated against supported versions.
