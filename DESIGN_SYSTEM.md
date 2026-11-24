# GXE Search Queue — Operations Console Design System

**Status:** approved for implementation by the project owner on 2026-11-24
**Product stage:** experimental v0.1.0 preview
**Reference:** Marathon by Bungie; visual language is reinterpreted for an operations product. No game logos, screenshots, copied layouts, or proprietary assets are used.

## 1. Design intent

GXE should feel like a precise, rugged control surface: purposeful typography, modular information, bright signals used sparingly, and a distinct visual rhythm. The user should be able to answer three questions immediately: **Is GXE healthy? What is it doing? What needs my attention?**

The product interface is not a game HUD. Every line, indicator, motion, and panel must communicate an actual state, an action, or a navigational relationship. Legibility and dependable operation take priority over atmosphere.

## 2. Color system

| Token | Value | Meaning and use |
|---|---|---|
| `void` | `#101214` | Main canvas and global shell |
| `surface` | `#191D20` | Primary content surfaces |
| `elevated` | `#22282B` | Raised controls, selected rows, overlays |
| `signal-orange` | `#FF5A24` | Primary actions, active navigation, priority and keyboard focus accents |
| `signal-cyan` | `#50D9E8` | Synchronization, timestamps, technical metadata, live refresh |
| `off-white` | `#E8E9E5` | Primary text |
| `muted` | `#92999B` | Secondary text; not used for small critical text without a contrast check |
| `success` | `#A4D66D` | Confirmed completion and service availability |
| `danger` | `#F06464` | Errors, failed jobs, dangerous/destructive actions |
| `warning` | `#E9B85C` | Degraded, delayed, retry-wait, or attention states |
| `separator` | `#343A3D` | Hairlines and table separators |

Use orange only for actions and selected/focused items, not as a general decoration. Cyan is reserved for system synchronization and technical data. Green means a confirmed good state. Red means an error or critical condition. Warning amber is distinct from both. Every non-neutral state also includes a word or icon; color alone never conveys status. Avoid text on colored surfaces until contrast is measured in the rendered UI.

## 3. Typography

- **Space Grotesk Variable:** display headings, navigation titles, short high-emphasis labels.
- **Inter Variable:** controls, body copy, forms, and tables.
- **IBM Plex Mono:** identifiers, timestamps, counters, technical values, and compact metadata.

Use locally bundled Fontsource packages under their OFL-1.1 licenses; do not load fonts from Google Fonts or another runtime host. Text defaults: body 14–16 px; primary table text at least 13 px; secondary metadata at least 11 px; technical labels at least 10 px. Display headings may use uppercase and tracking; body copy remains sentence case and normally mixed case. Use tabular numerals for counters and time.

## 4. Geometry and spacing

- Base spacing unit: 4 px. Common steps: 4, 8, 12, 16, 24, 32, 40, 48.
- Main desktop navigation: fixed/rail width around 220–240 px; content max width around 1500 px; page gutters 28–48 px.
- Cards/panels use 2–4 px corner radii; compact controls may use 2 px. Avoid fully rounded pills except small status chips where the shape improves scanning.
- Borders are thin and neutral. Use surface contrast and alignment to define hierarchy; avoid glow and decorative gradient dependence.
- Mobile touch targets: at least 44×44 px for primary interactive controls; icon-only controls require accessible names and a visible focus state.

## 5. Application shell and navigation

Four first-class sections, each with a stable URL/hash or route and its own heading, context, and focused content:

1. **Overview** — system state, active operation, queue summary, indexer summary, actionable alerts, recent events.
2. **Queue** — the primary workspace; search/filter/sort and grouped TV episodes; compact details expand on demand.
3. **Indexers** — observed Prowlarr configuration and status, tracked cooldowns, recent statistics where available, and an explicit explanation of the 429 validation limit.
4. **Settings** — supported GXE settings, grouped and described; no controls for settings GXE cannot save.

Search history remains available as a secondary Queue view so the four primary destinations stay focused without dropping the previous history workflow.

Persistent shell elements: compact project identity, primary navigation, current section, connection/refresh status, global queue pause/resume, notification count, and sign-out. At mobile widths, navigation becomes an accessible horizontal tab rail or a labelled compact menu; it does not keep a desktop sidebar squeezed onto the screen.

The shell must preserve filters, sort, scroll position, expanded rows, and keyboard focus during background refreshes and section changes where appropriate.

## 6. Page composition

### Overview

Prioritize a derived system state (`Online`, `Degraded`, `Offline`, `Unknown`) and state its basis. Show current active work with a content title joined from queue data, operation state, elapsed time if it can be derived reliably, and recovery guidance when ambiguous. Place queue totals and indexer availability nearby. Display only counts supported by API data; no fabricated throughput charts or download progress. Recent events and actionable alerts are separate, compact lists.

### Queue

Use a dense, readable work list with sticky column labels on large screens and responsive row cards on small screens. Main row content: queue position/order, title, movie/episode type, state and text, retry count, next eligible/attempt time when set, and a contextual action menu. Search and filters stay visible; sorting must reflect actual backend ordering and clearly identify the active sort. Manual priority remains a discrete action, never drag/drop implying arbitrary ordering or interval bypass.

Episodes are grouped by stable `series_key`; display a series heading and season/episode derived from stable episode metadata or the existing media key. The expanded group contains episode-level state. Never invent completion/failure for episodes missing from the active-queue payload. Expansion is keyboard operable and announced as expanded/collapsed. Detail drawer shows only available fields and run history explicitly linked to that job.

### Indexers

Rows identify Prowlarr indexers and distinguish: enabled/disabled configuration, Prowlarr-reported cooldown, ordinary recent failures, GXE cooldown records, and unknown/unavailable data. A Prowlarr status endpoint reporting `disabledTill` is not labelled HTTP 429 unless the response/history contains reliable 429 evidence. The page communicates that serialized GXE searches do not prove per-indexer request pacing. Read-only view first; no Prowlarr configuration controls unless a later change is explicitly validated and safely implemented.

### Settings

Group the four current persisted values under Queue/Scheduling and Release eligibility. Show current values, constraints, unit labels, inline validation, dirty/saved state, and a clear save result. Keep global queue pause as an operational control in the shell, not a duplicated setting. No fictitious synchronization/indexer/notification settings.

## 7. Shared components

Create small, typed components with clear responsibilities:

- `AppShell`, `PrimaryNav`, `ConnectionStatus`, `QueuePauseControl`.
- `Button`, `IconButton`, `TextInput`, `Select`, `SegmentedFilter`.
- `StatusBadge`, `ServiceIndicator`, `IndexerState`, `Timestamp`, `MetricValue`.
- `PanelHeader`, `EmptyState`, `Skeleton`, `InlineError`, `Toast/SaveFeedback`.
- `QueueTable`/`QueueRow`, `SeriesGroup`, `JobDetailsDrawer`.
- `ConfirmationDialog` for empty/remove actions and `RunReconciliationDialog` for explicit outcome reconciliation.

Components support hover, keyboard focus, disabled, pending, success, and error states as applicable. Destructive actions are visually distinct and require confirmation when affecting multiple items. A confirmation includes the exact scope and clarifies that active external searches are not interrupted.

Use Lucide icons with the ISC license, imported individually/tree-shaken. Icons supplement visible text and are not the only status label. Avoid decorative icon clusters.

## 8. Data, loading, and interaction states

Every page/async region supports:

- Initial loading skeleton matching its real content geometry.
- Empty state with a concrete next step where one exists.
- Stale data with a visible last-updated time if refresh fails.
- Inline retry for an isolated panel/API failure; one failed endpoint must not blank all other panels.
- Success feedback after mutations, plus field-level save validation.
- Clear 401/session-expiry handling and no credential echo.
- Pending/disabled button feedback during mutation; prevent duplicate destructive submissions.
- Background refresh that reconciles changed rows without remounting the page or stealing focus.
- Explicit `Unknown` where source data is absent; never substitute a healthy/zero state for loading or request failure.

The initial implementation retains the established 10-second dashboard polling cadence, but can load lower-priority/indexer details on entering that section and refresh them at a slower cadence. Do not add WebSocket/SSE without backend support and a demonstrated need.

## 9. Motion

Use CSS transitions and small state animations; no animation dependency is required. Typical transitions are 150–220 ms, with a 250 ms ceiling for routine interface movement.

- Buttons and navigation: color/surface change and a 1 px press/hover response.
- Drawer: short opacity + horizontal translation, with focus trapping and restoration.
- Confirmation modal: short opacity + small scale/translation; backdrop does not flash.
- Running work: one restrained state pulse/indicator, not a full-card continuous animation.
- Queue changes: brief row highlight on changed data; preserve row position and avoid reordering visible content during an active interaction.
- Cooldown countdown: update text at a sensible cadence (e.g. once per second only while visible); do not animate the whole card.
- Save/error/completion feedback: immediate icon/text transition and polite live-region announcement.
- Skeleton shimmer only during initial loading, not perpetual while stale data exists.

All nonessential animation is disabled or reduced under `prefers-reduced-motion: reduce`. Motion never hides information or delays action availability.

## 10. Responsive behavior

- **Wide desktop (≥1280 px):** navigation rail, two-column Overview where useful, full queue table, comfortable gutters.
- **Laptop (1024–1279 px):** narrower rail and content columns; preserve readable table columns; hide only redundant metadata behind row details.
- **Tablet (768–1023 px):** compact navigation, Overview becomes one column or asymmetric stack; queue controls wrap without overlap; maintain full keyboard access.
- **Mobile (<768 px):** touch-first navigation, summary values in compact sections, queue rows become cards with priority title/state first and actions in a labelled menu, series groups collapse, dialogs become near-fullscreen sheets, settings become one column. No unintended page-level horizontal overflow.

Responsive behavior is validated at 1440×900, 1280×800, 768×1024, 390×844, and 320×640. Target variants may be refined after browser inspection.

## 11. Accessibility

- Semantic landmarks, one page-level `h1`, ordered headings, real links for navigation, and buttons only for actions.
- Visible focus ring in signal orange with adequate contrast and no clipping.
- Full keyboard navigation; no hover-only controls; Escape closes dialogs/drawers; focus is trapped/restored for modal interaction.
- Status is conveyed in text and icon in addition to color. Live updates use restrained `aria-live="polite"`; errors use assertive announcements only when action-blocking.
- Form fields have persistent labels, associated help/error text, input constraints, and focus on first invalid field after submission.
- Reduced motion honored. Tables use headers/scope and responsive layouts keep reading order logical.
- Contrast is measured on actual foreground/background pairs. Muted metadata is not used for critical instructions or values.

## 12. Assets and dependency licenses

- Fonts are self-hosted from Fontsource packages under SIL Open Font License 1.1; their license notices remain in `node_modules` and redistributed font files retain required notices.
- Lucide React is ISC-licensed. Keep its upstream notice available through the package/license inventory.
- No external image, game mark, trademark, proprietary font, remote stylesheet, or runtime asset host is required.
- Keep Apache-2.0 for project code; dependencies retain their own licenses.

## 13. QA acceptance criteria

The implementation is not accepted solely because it builds. It must:

1. Render and operate at the responsive sizes above in a real browser.
2. Show loading, stale, empty, error, success, cooldown, paused, active, and ambiguous-operation states using truthful source data.
3. Preserve every existing dashboard action supported by API.
4. Keep queue search, filters, sort, groups, and open details stable through polling.
5. Make all navigation, row actions, dialogs, and forms keyboard operable.
6. Pass `prefers-reduced-motion` checks and avoid continuous decorative motion.
7. Avoid page overflow, clipped focus, unreadable small text, and color-only state cues.
8. Build the frontend and Rust application; run existing Rust tests and newly added focused frontend/browser checks.
9. Make no claim that Prowlarr failure statistics or disabled windows specifically prove HTTP 429 or indexer protection.
