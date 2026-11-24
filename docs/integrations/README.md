# Integration validation

Adapters now call documented API surfaces, but they have not been validated
against live service versions. This is implementation status, not a claim of
supported compatibility. Record product versions and observed behavior before
shipping support.

| Service | Documented surface found | Still to validate |
|---|---|---|
| Seerr | Polling adapter calls paginated `GET /api/v1/request`; official [OpenAPI specification](https://github.com/seerr-team/seerr/blob/develop/seerr-api.yml). | Webhooks are not implemented. Validate request shape, approvals, modification/cancellation, and how automatic search interacts with GXE. The administrator must disable Seerr automatic search manually or searches can bypass GXE. |
| Radarr | Adapter calls movie, queue, system status, and command routes under `/api/v3`; official [API reference](https://radarr.video/docs/api/) documents `GET /api/v3/command/{id}`. | Validate command request bodies and status semantics, completion/result timing, queue/import checks, and manual search detection against supported versions. |
| Sonarr | Adapter calls series, episode, queue, system status, and command routes under `/api/v3`; official [API reference](https://sonarr.tv/docs/api/) documents command lookup for the v3 API. | Validate installed API version, command request/status semantics, episode availability/monitoring, season-pack satisfaction, import state, and manual search detection. |
| Prowlarr | Adapter checks `/api/v1/system/status` and reads `/api/v1/indexer`, `/api/v1/indexerstatus`, and `/api/v1/indexerstats` for the dashboard. The read-only response is normalized to a safe field allowlist. Official [API reference](https://prowlarr.com/docs/api/) documents these API surfaces. | Read-only shape was smoke-tested against Prowlarr 3.0.2.5780. No 429 event/cooldown ingestion, `Retry-After` handling, or indexer control is implemented. Aggregate query failures are not classified as 429. Validate search fan-out and per-indexer pacing before claiming indexer protection. |

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

The project owner has reported live smoke tests for single-movie searches, a
full Arcane season request, and a large mixed movie/TV queue that ran for hours
without an observed HTTP 429. A three-episode season pack was also observed and
handled by Sonarr. The exact service versions and command response/status
payloads have not been recorded, so these reports do not yet establish a
supported-version matrix or prove per-indexer pacing. No external service
database is accessed or modified.
