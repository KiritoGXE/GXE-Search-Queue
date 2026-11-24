# Frontend audit — GXE Search Queue

**Audit date:** 2026-11-24
**Repository branch:** `main` (tracks `origin/main`)
**Project state:** experimental v0.1.0 preview
**Scope:** read-only repository audit, apart from this document. No application code, dependencies, tests, or external services were changed or run.

> This audit describes the current working tree, including pre-existing uncommitted work. It does not treat those changes as part of the committed baseline.

## 1. Current repository state and safeguards

At audit start, `git status --short --branch` reported `main...origin/main` plus local changes in:

- `README.md`
- `docs/architecture.md`
- `docs/integrations/README.md`
- `src/integrations/mod.rs`
- `src/main.rs`
- `web/index.html`
- `web/src/main.tsx`
- `web/src/style.css`

These files were preserved. The backend/documentation changes concern persisted-command recovery; the frontend files contain the latest uncommitted Operations Console iteration. No `FRONTEND_AUDIT.md` existed before this audit.

Repository instructions in `AGENTS.md` require the approved stack (Rust/Tokio/Axum, SQLx/SQLite, React/TypeScript), preserve Apache-2.0, avoid external-database access, keep integrations on documented APIs, and clearly distinguish implemented behavior from unvalidated guarantees.

## 2. Architecture and stack

The application is a Docker-first modular monolith:

```text
Browser (React + TypeScript, Vite-built static assets)
                 │ same-origin /api/v1, Authorization: Bearer …
                 ▼
Rust service (Tokio + Axum) ─── SQLite (SQLx, migrations, WAL)
          │
          ├── Seerr polling and request reconciliation
          ├── Radarr movie commands and state checks
          ├── Sonarr episode commands and state checks
          └── Prowlarr availability check
```

The multi-stage `Dockerfile` builds the frontend with Node 22, builds the Rust service, and copies the static `web-dist` into the runtime image. Axum serves both the authenticated API and the SPA fallback. Compose publishes port 8080 by default and persists SQLite in a named volume. The application does not require a separate frontend server in production.

### Frontend packages

Runtime dependencies are React 18 and React DOM. Development/build dependencies are TypeScript 5.7, Vite 6, the React Vite plugin, and React type definitions. There is no frontend router, component library, icon package, animation library, data-fetching/cache layer, or browser-test dependency. No dependency installation was performed for this audit.

The frontend entry point is `web/src/main.tsx`; the stylesheet is `web/src/style.css`; `web/index.html` is the Vite shell. The current implementation puts API calls, types, session state, polling, action handlers, navigation, and all page markup in one React `App` component. Styling is a single global stylesheet. The present navigation is a scroll-to-section shell, rather than distinct routed pages.

## 3. Current frontend behavior and feature inventory

### Authentication and data refresh

- The dashboard asks for the single administrator token and stores it in `sessionStorage` under `gxe-token`.
- API requests send `Authorization: Bearer <token>` and JSON content type.
- A 401 clears the token and returns the UI to sign-in.
- On sign-in, and every 10 seconds thereafter, the client fetches status, queue, history, settings, and dashboard events concurrently.
- A manual refresh is available. A global busy flag is used for mutations.
- The fetch helper assumes a JSON response and has no explicit timeout, cancellation, retry/backoff, or stale-response protection. A failure in the parallel refresh currently fails the whole refresh batch.
- There is no WebSocket, Server-Sent Events, or other push channel. The backend has no frontend live-update stream.

### Current user-facing capabilities

- Overview of current scheduler activity, movie/episode queue counts, suspended count, service connection state, and active cooldown count.
- Global queue pause/resume.
- One active alerts list with acknowledgement.
- Queue listing with title search and movie/episode filter.
- Queue actions: prioritize, force retry, remove unstarted item, and empty unstarted work (browser confirmation).
- Recent activity plus up to 30 reconciliation records for active/ambiguous runs.
- Settings form for minimum interval, capacity, search timeout, and theatrical delay.
- Responsive CSS rules and reduced-motion handling in the current working-tree UI.

The queue client does not currently expose sorting, pagination, expandable request details, requester information, grouped series/season/episode views, attempt history per job, or an operation drawer. The backend API returns all active/nonterminal jobs in one response, so a frontend table can paginate locally, but very large queues have no server paging contract today.

The four product areas requested for the reconstruction are **Overview, Queue, Indexers, and Settings**. The current working-tree navigation instead has **Command/Overview, Queue, Activity/History, and Systems/Settings**. There is no dedicated Indexers screen. The API exposes a cooldown read endpoint, but current UI polling does not call it.

## 4. API map and frontend contracts

All `/api/v1/*` endpoints below require the administrator bearer token. `/api/health` is unprotected and returns a small health response; it is used by the container health check. Static frontend assets use the Axum fallback route.

| Method and path | Current behavior / response shape | Frontend use and constraints |
|---|---|---|
| `GET /api/health` | `{ status: "ok" }` | Container health only; not an overall dependency-health verdict. |
| `GET /api/v1/status` | `{ paused, counts: { movies, episodes, suspended }, active_search, services: [{service, available, checked_at, detail}], active_cooldowns }` | Used by Overview. No derived overall `online/degraded/offline` field, completed/failed totals, search duration, title, or next-start countdown. `active_search` is a persisted run; content metadata is in queue data. |
| `GET /api/v1/queue` | `{ items: QueueJob[] }` for states other than completed, satisfied, cancelled, dismissed | Used by Queue and metrics. Each item includes `id`, `media_type`, `media_key`, `title`, `series_key`, `episode_sort_key`, manager ID, state, priority, eligibility/next-attempt timestamps, no-result attempt count, and first-request time. No pagination/filter parameters or requesters/linked Seerr request objects. |
| `GET /api/v1/history` | `{ items: [...] }`, newest 500 runs; each has run ID, job ID, state, start/completion timestamps, detail | Used for recent history and manual reconciliation. It does not include job title/type or expose query pagination. Backend stores/prunes completed history after 90 days. |
| `GET /api/v1/events` | `{ items: [...] }`, newest 100, including severity, message, created/acknowledged timestamps | Used for admin alerts. No pagination/filter parameters. |
| `POST /api/v1/events/:id/ack` | `{ acknowledged: boolean }` | Acknowledge an event. |
| `GET /api/v1/cooldowns` | `{ items: [{ id, name, cooldown_until, reason }] }` for currently active SQLite cooldown rows | Available to a future Indexers view, but the Prowlarr adapter does not populate/refresh 429 cooldowns in this preview. Empty data must not be presented as proof that indexers are healthy. |
| `GET /api/v1/settings` | Object of four values: minimum interval, queue capacity, search timeout, theatrical delay | Settings values loaded from SQLite are serialized as strings; defaults originate as JSON numbers. Frontend should normalize types at the API boundary. |
| `PUT /api/v1/settings` | Accepts the same four integer fields; `{ saved: true }` | Validation enforces interval ≥600, capacity ≥0, timeout >0, release delay ≥0. Current updates write settings sequentially rather than as one transaction. |
| `POST /api/v1/queue/pause` | `{ paused: boolean }` | Body `{ paused: boolean }`; active external operation is not interrupted. |
| `POST /api/v1/queue/empty` | `{ dismissed: number }` | Dismisses all unstarted jobs in eligible/nonterminal states; current UI confirms with `window.confirm`. |
| `POST /api/v1/queue/:id/priority` | `{ updated: boolean }` on success | Sets an unstarted item to high priority; interval, active operation, and scheduler fairness still apply. |
| `POST /api/v1/queue/:id/retry` | `{ queued: boolean }` on success | Clears retry delay for eligible unstarted work; does not reset the no-result counter and does not bypass the global scheduler. |
| `DELETE /api/v1/queue/:id` | `{ removed: boolean }` on success | Removes only unstarted work. Missing/started work returns an error. |
| `POST /api/v1/runs/:id/reconcile` | `{ reconciled: boolean }` on success | Body outcome must be `found`, `no_results`, `satisfied`, or `technical_error`, with optional detail. Current UI presents this for active/ambiguous runs. |

Database-backed API errors generally return JSON `{ error }`; bad input is 400, queue-capacity conflict 409, authentication failure 401, and other failures 500. The API does not currently publish an OpenAPI contract or generated frontend types.

## 5. Models and operational semantics relevant to UX

### Queue jobs

SQLite stores approved Seerr requests, request-to-job links, deduplicated queue jobs, search runs, service health, cooldown rows, string-valued settings, dashboard events, and scheduler state. Queue job media types are `movie` or `episode`. States are `waiting_metadata`, `waiting_release`, `queued`, `retry_wait`, `running`, `suspended`, `satisfied`, `completed`, `cancelled`, and `dismissed`. The queue endpoint intentionally omits terminal satisfied/completed/cancelled/dismissed jobs; users need Search History/events to inspect past outcomes.

For episodes, the current integration stores `series_key` and an `episode_sort_key`, and uses a media key resembling `tvdb:<id>:SxxExx`. This is enough to build a modest series/episode grouping from active queue items, but the API does not return a normalized series object, season object, episode title/number fields, requested-season links, or episode-level historical runs as a grouped payload. Terminal episodes are not in the queue response. A complete `Series → Seasons → Episodes` tree with completed and failed siblings cannot be reliably built from the existing queue API alone.

### Search runs

A run tracks external command ID, state (`submitting`, `active`, `reconciling`, `completed`, `no_results`, `technical_error`, `satisfied`), start/completion time, and outcome detail. Timeout is not proof that a command did not execute. The UI must retain manual reconciliation for ambiguous commands and must not offer blind resubmission.

### Services and indexers

Service health is currently for Seerr, Radarr, Sonarr, and Prowlarr, with availability, last check time, and detail. Prowlarr integration is currently an availability check only. Indexer-level 429 detection, `Retry-After`, cooldown ingestion/control, request counters, last-used/error records, or fan-out information are not implemented. A separate Indexers section can show that indexer data is unavailable/unvalidated; it cannot claim per-indexer live health from current API data.

## 6. Authentication, polling, and security constraints

- Authentication is a single local admin token, not user accounts, roles, or session-cookie auth. It is compared in constant time by Axum middleware for protected API routes.
- The dashboard retains the token in tab-scoped `sessionStorage`; it is removed on 401 or sign-out. Browser JavaScript sends it in the Bearer header. There is no login endpoint or server-managed session.
- The application requires a strong configured token and is intended for trusted local networks or a TLS reverse proxy; no TLS is terminated by the application.
- No secrets are needed in frontend bundles. Do not expose API keys or admin token in UI text, logs, source, audit examples, or mock data.
- Polling is frontend-driven at 10 seconds for the dashboard and backend-driven at 60 seconds for Seerr sync / 30 seconds for service checks / 10 seconds for command-state reconciliation. The UI must distinguish the last successful refresh from live service state and avoid resetting scroll, keyboard focus, search/filter state, or expanded rows on poll.

## 7. Tests, build, and validation tooling

Frontend `package.json` defines only `npm run build` (`tsc -b && vite build`). There is no frontend unit, component, or browser test runner and no Playwright dependency/configuration. Existing tests are Rust:

- `tests/unit/queue_test.rs`: legacy in-memory `Queue` push/pop smoke test.
- `tests/integration/api_test.rs`: placeholder assertion only; it does not call the API.
- Three integration recovery tests are embedded in `src/integrations/mod.rs` in the current worktree.
- GitHub Actions runs `cargo fmt --check`, Clippy with warnings denied, and `cargo test --workspace --all-features`. It does not currently build/typecheck the frontend in CI.

The repository development guide documents a Docker build and a separate Node 22 `npm ci && npm run build`. During this audit, no test, build, dependency installation, browser automation, or external service request was performed.

## 8. Existing visual/product debt to address in later phases

These are audit findings, not changes made in this phase:

1. **Page structure:** the current UI is one scrollable screen, not four task-focused sections. History is currently promoted into navigation while Indexers is not.
2. **Overview truthfulness:** the working-tree UI labels the node `ONLINE` independently of API data and presents `READY` as a fallback even before status has loaded. A system state must be derived from availability and loading/error state, or shown as unknown.
3. **Queue hierarchy:** the queue can filter media type and title but has no series grouping, sort controls, row expansion, detail drawer, attempt trail, next-attempt display, or added-at display. It renders episodes and movies in a flat table.
4. **Indexer claims:** status exposes only a cooldown count, and `/cooldowns` returns local cooldown rows that are not currently populated by the documented adapter path. Do not turn these into simulated operational metrics.
5. **Active operation context:** status has a run/job ID but no title or media type. A UI can join the current job ID to the queue response where present, but terminal/missing job cases need a careful empty/unknown representation.
6. **Metrics limits:** the backend status counts only nonterminal movies/episodes and suspended jobs. It does not provide completed/failed counts, throughput, retry totals, queue-age percentiles, or indexer rates. Avoid fake charts and derive only metrics that the available data can support.
7. **Polling UX:** a refresh error is global and the current frontend's `Promise.all` discards all refresh updates when any one endpoint fails. Poll responses can race with actions. No skeleton/loading distinction or per-section retry state is currently provided.
8. **Settings feedback:** current fields are real and constrained by the backend, but there is no inline validation/status per field; values should normalize string/number response types and visibly report save/error outcome.
9. **History/detail relationship:** history provides `job_id` but not a joined title/type, and is capped at 500 records per request. Join against queue data only when safe; older completed jobs are absent from queue and will need backend support if history must show names.
10. **Responsive table:** CSS currently allows horizontal table overflow on narrow screens. The later design should choose compact row cards or prioritized columns for mobile instead of relying on horizontal scrolling.
11. **Accessibility and interaction:** markup has a few accessible names but no shared component primitives; focus, keyboard navigation, contrast, live-region announcements, confirmation patterns, and touch target sizing need a systematic QA pass. Current scroll navigation does not represent URL routes and can get out of sync with manual scrolling.
12. **Content language:** much of the current visible copy is English. The product's intended localization strategy is not specified; do not introduce an internationalization dependency or translate every label without an agreed product direction.

## 9. Proposed frontend architecture for the reconstruction

Keep the approved React/TypeScript application and the existing same-origin Axum API. No backend changes are required for the first visual prototype. Split the current monolithic component by responsibility before wiring the reconstructed presentation:

```text
web/src/
  app/                 app shell, navigation, section selection
  api/                 typed API client, response normalization, error mapping
  features/
    overview/          status summary, active operation, events
    queue/             filters/search, responsive queue, episode grouping
    indexers/          explicitly supported health/cooldown data + limitations
    settings/          typed fields, validation, save feedback
  components/           buttons, status, badges, tables/cards, dialogs, skeletons
  hooks/                refresh lifecycle and action state (or feature-local hooks)
  styles/               design tokens, foundations, component/page styles
  types/                API/domain types aligned to actual Rust responses
```

Use a small in-app section state with accessible links/buttons and scroll/focus handling if sections remain a single SPA shell; four routed views are also possible, but no routing library is currently present and should not be added unless it solves a concrete navigation/history requirement. Keep API access in one typed same-origin client. Normalize endpoint payloads once (not in presentation components), preserve useful stale data when an individual poll fails, and keep mutation feedback separate from refresh status. Keep polling at the current rate initially; do not add WebSocket/SSE without a backend feature and demonstrated need.

The API boundary should reflect limitations honestly: derive system health from known service availability and API reachability with an explicit `unknown/loading/error` state; show Indexers as unvalidated/limited if the endpoint has no live data; show only server-supported queue actions; and do not fabricate counts, progress, cooldowns, or series completion states. A later API change for richer series/history models requires separate review and approval because the current request's phase 0 forbids application changes.

## 10. Migration risks and mitigations

| Risk | Why it matters | Phase-gated mitigation |
|---|---|---|
| Pre-existing dirty worktree | Recovery and dashboard files are already modified locally | Preserve those edits; isolate new edits; do not reset or overwrite. This audit only adds its own file. |
| UI state can overstate health | No overall system status contract; current visual fallback says online/ready | Define UI state from API availability and distinguish unknown from healthy. |
| Indexers screen could imply unsupported control | Prowlarr per-indexer metrics/cooldowns are not integrated | Restrict first implementation to facts currently returned and label validation gaps; do not create suspend controls. |
| TV grouping can be incomplete | Active jobs have series and sort keys, but no normalized season/request/terminal hierarchy | Group only active returned items using stable keys; do not imply completed episode history. Record any backend contract request separately. |
| History loses descriptive metadata | History returns job IDs only, while completed jobs disappear from queue | Preserve IDs/state/times now; request a minimal history projection in a later backend change if titles are required. |
| Polling can disrupt interaction | 10-second whole-screen refresh can race with actions and any future open detail state | Keep UI state local; reconcile data without remounting/resetting; handle endpoint errors independently. |
| API errors are coarse | 400/409/500 JSON error strings; no field-error schema | Map known statuses/messages into helpful UI feedback and retain raw non-secret details for diagnosis. |
| Frontend quality is not enforced in CI | Current workflow tests Rust only | Add frontend type/build and browser QA in a later approved implementation phase. |
| Stacked changes may be accidentally deployed/published | Docker currently builds the entire dirty worktree | Do not commit/push or publish until the owner explicitly requests it and the pre-existing modifications are accounted for. |

## 11. Migration plan (proposal, not started)

1. **Phase 1 — Design system:** establish tokens, typography, spacing, layout rules, interaction/accessibility states, and responsive breakpoints in `DESIGN_SYSTEM.md`; no API integration changes.
2. **Phase 2 — Visual prototype:** implement shell, Overview, and Queue with clearly labeled mock fixtures in an isolated prototype mode or temporary page. Validate real browser layouts at desktop/tablet/mobile before replacing the connected UI. Avoid duplicating production actions or mixing mock and live data silently.
3. **Phase 3 — Connected sections:** move shared API/types and current logic behind typed feature modules; integrate Overview, Queue, Indexers (strictly limited to existing data), and Settings. Retain all existing pause, prioritize, retry, remove, empty, acknowledge, reconcile, and save flows.
4. **Phase 4 — QA:** add frontend build/typecheck to validation; add focused component/API tests and browser tests if Playwright can be added with reasonable maintenance cost. Verify keyboard/focus/reduced-motion, breakpoints, polling races, errors, and destructive confirmation. Compare screenshots at target sizes and correct visual defects.
5. **Phase 5 — Stabilization:** verify no backend contract changes, run existing Rust tests/build plus frontend checks, check Git diff and docs, and document unsupported indexer/TV/history details. Do not remove the old behavior until the connected replacement passes parity checks.

## 12. Decisions and gates

- **No implementation occurred in this phase.** Only this audit report is created.
- The approved stack and backend/API contracts should remain unchanged for the prototype and initial reconstruction.
- The future Indexers section must expose current limits instead of inventing per-indexer health.
- A complete TV hierarchy, rich history labels, completed/failed aggregate metrics, and indexer telemetry exceed current API data. They are not frontend-only promises; any required backend contract changes must be proposed separately.
- The next requested phase is Design System, documented in `DESIGN_SYSTEM.md`. Work on that phase should begin only after the project owner approves this audit.

## 13. Implementation follow-up — 2026-11-24

The owner subsequently authorized implementation through completion. The audit above remains a snapshot of the initial state; the following work has since changed those contracts and status statements:

- Replaced the monolithic dashboard with a four-destination operations console: Overview, Queue, Indexers, and Settings. Search history remains a secondary Queue tab.
- Split frontend types, API access, refresh lifecycle, primitives, and each page into feature modules. Retained bearer-token authentication and 10-second dashboard polling, with per-endpoint error retention.
- Added the read-only authenticated `GET /api/v1/indexers` endpoint. It normalizes Prowlarr's configured indexers, reported disabled windows, and trailing-24-hour counters into a safe field whitelist. It does not expose indexer configuration fields or credentials and does not classify ordinary failures as HTTP 429.
- Extended `/api/v1/status` with the active job title/type and 90-day run outcome totals. Extended `/api/v1/history` with nullable job title/type via a database join. Added authenticated `GET /api/v1/queue/:id/details` for linked requesters and that job's recent attempts.
- Settings updates now validate supported ranges server-side and write the four settings in one SQLite transaction.
- Added local Fontsource font assets and their OFL license texts, Lucide's ISC notice, responsive CSS, reduced-motion support, and Playwright browser checks.
- Current queue payload still contains only nonterminal work. The UI does not claim a full terminal TV season tree; the season/episode grouping is built from the returned active episode jobs. Download progress, true per-indexer 429 ingestion, per-indexer GXE pacing, Prowlarr controls, and unsupported settings remain unavailable.
- Browser validation used mock fixtures for section navigation, episode grouping, history, confirmation dismissal, and mobile overflow. The production bundle was also loaded against the local GXE container; the live Prowlarr view returned one configured indexer in the test stack. No Arr settings or searches were changed.

The implemented stack remains Rust/Tokio/Axum, SQLx/SQLite, React/TypeScript/Vite. No production-readiness or per-indexer rate-limit guarantee is made.
