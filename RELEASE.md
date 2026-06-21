# Vire — Release Notes

## v0.6.0 — Suggestions UAT polish: cost, normalization, trackability (TASK-034)

**Branch:** `feat/task-034-suggestions-uat-polish`
**PR:** #29

### What changed

Polish pass on the AI time-entry Suggestions view (TASK-032), covering four workstreams:

**A — Same-minute / 23:59 span normalization (bug fix).** AI-suggested blocks whose start and
end timestamps fall in the same minute are now normalized to a non-zero span before accept:
forward-bump if room exists, or anchor on day-end for the `23:59` edge case
(`23:59:10 → 23:59:50` stores as `23:58 → 23:59`, 1 min, no midnight cross). The frontend
suggestion row mirrors both branches so the UI defaults are always consistent with the backend.
Manual entry rejection of `start == end` is unchanged.

**B — AI cost persistence, reporting, and CSV (new capability).** Two additive nullable columns
(`cost_total REAL`, `cost_currency TEXT`) are added to `time_entries` via idempotent
`add_column_if_absent`. Accepting a suggestion copies its cost verbatim; manual entries write
`NULL` (absence ≠ zero, DEC-004). The summary query computes `ai_cost_total` separately from
human minutes (DEC-003). The Reports/Summary view gains an AI cost sub-line per project card and
a mixed-currency guard ("—" when currencies differ). CSV export gains `cost_total`/`cost_currency`
columns; manual-entry rows emit empty strings.

**C — Trackability and disabled-source notices (new UI).** Unmapped-environment suggestions now
show a "not trackable until mapped" notice with a direct link to Settings. Untimed blocks carry
a badge "not auto-trackable — add time manually". The empty-state view lists all causes
(nothing imported, unmapped, untimed, source-down) with actionable links — no blank table, no
bare "0".

**D — 30-minute gap spec correction (no code change).** `GAP_MINUTES = 30` in the engine was
always a compile-time constant; the spec incorrectly described it as configurable. Spec corrected.

### Compatibility and rollback

Two additive nullable columns on `time_entries` (`cost_total`, `cost_currency`); no IPC, schema
breaking change, or dependency. Rolling back to v0.5.0 leaves the columns inert — older builds
neither read nor write them. AI-accepted entries created under v0.6.0 persist as plain entries in
older builds (cost not displayed; no data loss). See
[openspec/changes/task-034-suggestions-uat-polish/RELEASE.md](openspec/changes/task-034-suggestions-uat-polish/RELEASE.md)
for the full rollback table and component compatibility matrix.

### Tests

**Rust** (`cargo test --lib`): **165 passed / 0 failed**
(new tests: A4 ×4 — same-minute, no-duration, 23:59 day-end DEC-035, manual reject unchanged;
B6 ×2 — cost copy, cost/manual separation)

**Frontend** (`npm run test:frontend` with `LANGFUSE_*` unset): **105 passed / 0 failed**
(new tests: `suggestionsUi.test.mjs` — `subMinutesHHMM` unit, DEC-035 23:59 day-end, C1–C4
trackability/notices, SEC-012 secret-free render; `summaryCards.test.mjs` (new) — 7 B5 cost
and aggregate tests)

### Manual smoke steps before shipping

- M1 — Suggestions view: accept a same-minute block (if available); confirm it stores without
  error and appears in Today with a non-zero duration.
- M2 — Accept a suggestion with a known AI cost; open Reports → confirm the project card shows
  the AI cost sub-line; export CSV and verify `cost_total`/`cost_currency` columns are populated.
- M3 — With an unmapped environment: open Suggestions; confirm the "not trackable until mapped"
  notice and "Map in Settings" link appear.
- M4 — With no suggestions at all: confirm the empty-state panel lists causes with actions, not
  a blank table.

(Human-only; outstanding UAT gate — requires packaged `.app` on physical Mac.)

---

## v0.5.0 — Reports quick-range presets (TASK-033)

**Branch:** `feat/task-033-reports-quick-ranges`
**PR:** #28

### What changed

Added **four quick-range preset buttons** to the Reports view — Last 7 days, Last 14 days,
Last 30 days, Last 90 days. Clicking a preset populates the start/end date inputs and
re-renders the report immediately, including honouring the active project filter for CSV export.

The date ranges are computed in local calendar time (no UTC conversion) and are correct across
DST boundaries and the date line. No backend, IPC, schema, or dependency change.

### Compatibility and rollback

Fully compatible with v0.4.0 (no DB migration, no IPC change). Rollback: reinstall the v0.4.0
`.app` — no cleanup step. See [openspec/changes/task-033-reports-quick-ranges/RELEASE.md](openspec/changes/task-033-reports-quick-ranges/RELEASE.md)
for the full rollback table and component compatibility matrix.

### Tests

**Frontend** (`npm run test:frontend`): **88 passed / 2 pre-existing failures (unrelated)**
(new tests: `tests/reportRanges.test.mjs` — 5/5 across `America/Los_Angeles` and
`Pacific/Kiritimati`; the 2 failures are in `tests/pi-observe.security.test.mjs`,
unrelated and present on `main`)

### Manual smoke steps before shipping

- M1 — Open Reports view; confirm four preset buttons appear above the date inputs.
- M2 — Click **Last 7 days**; confirm start/end populate and the report re-renders.
- M3 — Select a project filter, click **Last 30 days**; confirm the project filter is preserved
  in the subsequent CSV export.
- M4 — Manually edit the date range after clicking a preset; confirm no preset stays highlighted.

(Human-only; outstanding UAT gate — requires packaged `.app` on physical Mac.)

---

## v0.4.0 — AI time-entry suggestions (TASK-032)

**Branch:** `feat/task-032-ai-time-suggestions`
**PR:** #27

### What changed

Added an **AI time-entry suggestion system** — a local engine that reads Langfuse evidence from
SQLite and proposes time-entry blocks per project-day, without any new network egress.

**Workstream A — Suggestion engine + persistence:** new `src-tauri/src/suggestions/` module
(engine, store, tests). Reads evidence at call time via LEFT JOIN (no evidence rewrite, DEC-001).
New `time_entry_suggestions` table (additive, idempotent). IPC: `list_time_entry_suggestions`
(atomic regeneration — delete-then-insert in one transaction, SW-4 fix).

**Workstream B — Accept/dismiss + origin separation:** `accept_time_entry_suggestion` (atomic tx:
insert entry + guarded-UPDATE suggestion) and `dismiss_time_entry_suggestion` (guarded pending-only
update). New `origin TEXT NOT NULL DEFAULT 'manual'` column on `time_entries` (additive; backfills
existing rows). Today/Reports SQL and CSV split on `origin` — AI minutes never counted in the
human/billable total (DEC-003).

**Workstream C — Suggestions review UI:** new `src/suggestions-ui.ts` pure builders,
`'Suggestions'` view route in `src/main.ts`, `tests/suggestionsUi.test.mjs` (10 tests).

No new crate or npm dependency. No new egress host or Tauri capability. Suggestion surface is
secret-free (SEC-012); all UI output XSS-escaped; accept is TOCTOU-safe atomic transaction.

### Compatibility and rollback

Two additive schema changes: `time_entry_suggestions` table and `time_entries.origin` column
(both `CREATE TABLE/ADD COLUMN IF NOT EXISTS`; rollback to v0.3.2 leaves them inert — TASK-031
binary never reads them). AI-accepted entries created before rollback persist as plain entries
in older builds. No data loss. See [openspec/changes/task-032-ai-time-suggestions/RELEASE.md](openspec/changes/task-032-ai-time-suggestions/RELEASE.md)
for the full rollback table and component compatibility matrix.

### Tests

**Rust** (`cargo test`): **159 passed / 0 failed**
(includes new TASK-032 tests: suggestion engine scoring, atomic regeneration rollback, accept
atomicity, dismiss guard, secret-free surface assertion `surfaces_carry_no_secrets`)

**Frontend** (`npm run test:frontend`): **73 passed / 2 pre-existing failures (unrelated)**
(new tests: `tests/suggestionsUi.test.mjs` — 10/10; the 2 failures are in
`tests/pi-observe.security.test.mjs`, unrelated and present on `main`)

### Manual smoke steps before shipping

- M1 — Open the Suggestions view; confirm the list renders (may be empty if no Langfuse evidence).
- M2 — Accept a suggestion; confirm it appears in Today as an AI-badged entry, not counted in
  billable human total.
- M3 — Dismiss a suggestion; confirm it disappears and cannot be re-dismissed.
- M4 — Export CSV; confirm the `origin` column shows `manual` / `ai_suggested` correctly.

(Human-only; outstanding UAT gate — requires packaged `.app` on physical Mac.)

---

## v0.3.2 — Settings scroll preservation + copy cleanup (TASK-031)

**Branch:** `feat/task-031-settings-scroll-preservation`
**PR:** #26

### What changed

Fixed the **Settings UX bug** where pressing any control (Test connection, Save range,
Import now, Map, Save settings, …) scrolled the panel back to the top.

`shell()` is the single render chokepoint that re-assigns `app.innerHTML`. That destroyed
and recreated the `<main>` scroll container, resetting its `scrollTop` to `0` on every
in-Settings action. The fix captures the outgoing scroll position and whether the view is
unchanged before the swap, and restores it after — a same-view re-render stays put; navigating
to a different view resets to the top. The decision logic is extracted to a pure, unit-tested
`nextScrollTop()` helper in `src/scroll.ts`.

Also corrects a leftover copy in the env-mapping help panel: `use "Create project for …"` →
`use "Create & map"` (matches the button label). No behaviour or markup change.

No backend change. No new dependency. No DB schema change.

### Compatibility and rollback

Fully compatible with v0.3.1 (no DB migration, no IPC change). Rollback: reinstall the v0.3.1
`.app` — no cleanup step. See [openspec/changes/task-031-settings-scroll-preservation/RELEASE.md](openspec/changes/task-031-settings-scroll-preservation/RELEASE.md)
for the full rollback table and component compatibility matrix.

### Tests

**Frontend** (`npm run test:frontend`): **73 passed / 2 pre-existing failures (unrelated)**
(new tests: `scroll.test.mjs` — 2/2 helper cases; `envMappingUi.test.mjs` +1 copy assertion;
14/14 focused tests pass; the 2 failures are in `tests/pi-observe.security.test.mjs`,
unrelated and present on `main` before any TASK-031 commit)

### Manual smoke steps before shipping

- M1 — In Settings, scroll down, press any re-rendering control: viewport should stay put.
- M2 — Switch to another view (e.g. Today) and back to Settings: should open at top.
- M3 — Open Settings → mapping panel: help text should read **"Create & map"**, not "Create project for…".

(Human-only; outstanding UAT gate — see `qa.md`.)

---

## v0.3.1 — Create Project button fix: in-app input for env create-and-map (TASK-030)

**Branch:** `feat/task-030-create-project-button-fix`
**PR:** #25

### What changed

The **Create & map** action in the environment → project mapping panel now works in the packaged
macOS app. Previously the action relied on `window.prompt()`, which returns `null` silently in the
macOS WKWebView; the project name was never captured and no project was created.

The fix replaces the single button with an inline `<input>` (pre-filled with the environment name,
`maxlength="120"`) + **Create & map** button. The handler reads the input directly, trims it, and
rejects empty names before calling `create_project` → `set_env_mapping` — the same IPC sequence as
before, now with a working name source.

No backend change. No new dependency. No DB schema change.

### Compatibility and rollback

Fully compatible with v0.3.0 (no DB migration, no IPC change). Rollback: reinstall the v0.3.0
`.app` — no cleanup step. See [openspec/changes/task-030-create-project-button-fix/RELEASE.md](openspec/changes/task-030-create-project-button-fix/RELEASE.md)
for the full rollback table and component compatibility matrix.

### Tests

**Rust** (`cargo test`): **142 passed / 0 failed** (backend unchanged)

**Frontend** (`npm run test:frontend`): **72 passed / 2 pre-existing failures (unrelated)**
(new test: `envMappingUi.test.mjs` — 11/11 markup assertions for the new input/button pair;
the 2 failures are in `tests/pi-observe.security.test.mjs`, unrelated and present since before
any TASK-030 commit)

### Manual smoke steps before shipping

T6 — Packaged macOS app: launch the signed `.app`, open Settings → env mapping panel, click
**Create & map** for an unmapped environment, confirm a project is created and the environment
is mapped. (Human-only; outstanding UAT gate — see `tasks.md`.)

---

## v0.3.0 — Langfuse schema diagnostics, backfill, and tolerant v3 import (TASK-029)

**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
**PR:** #23 (draft)

### What changed

**Tolerant v3 trace identification (Workstream B)**

- The import loop no longer drops a whole trace because `observations` arrives as an ID-string array (the
  Langfuse v3 list-endpoint shape). Identification reads `id`, `timestamp`, `environment`, and `sessionId`
  from the raw JSON, tolerating unknown shapes in peripheral fields. A trace with a usable `id` is always
  imported; if usage/cost remain unreadable after the observations fetch it is imported as `schema_changed`
  (counted, surfaced for review) — never silently dropped.
- This resolves the previously observed 611/640 skip rate against a live v3 Langfuse stack.

**Schema diagnostics (Workstream A)**

- Import results now surface **grouped skip-reason counts** and bounded **structural shape samples** (key
  names and JSON type names only — no field values, no credentials, no raw `serde` error strings). The old
  repeated free-string warning is removed in favour of this classified breakdown.
- Diagnostic data is secret-free by construction (SEC-011): samples carry only the list of top-level JSON
  key names, the offending field name, and the JSON type name — nothing from field values, payloads, or
  session/prompt/credential material.

**Configurable import range + incremental cursor (Workstream C)**

- A new Settings row `langfuse_import_range` accepts `last_7d`, `last_30d` (default; was `last_7d`),
  `last_90d`, `all`, or `since:<RFC3339>`. The range floor is resolved at import time; each environment
  tracks its own persistent cursor so normal imports are incremental (resume from cursor, no redundant
  re-scan).
- New IPC commands: `get_langfuse_import_range` / `set_langfuse_import_range`.

**Backfill (Workstream C)**

- **Backfill now** re-scans floor→now regardless of the cursor. Large backfills are broken into ordered
  monthly chunks, each committed atomically (S-3 invariant preserved). An interruption loses at most the
  in-flight chunk; re-running resumes via the inclusive DEC-032 cursor and skips already-imported traces
  via durable `(environment, trace_id)` dedupe.
- New IPC command: `backfill_langfuse_now` (larger timeout bound than a manual import).

**DEC-032 inclusive-from cursor and page-limit continuation**

- All trace-list requests use `orderBy=timestamp.asc` (oldest→newest; explicit to avoid relying on an
  undocumented default). When a run hits `MAX_PAGES`, the inclusive resume cursor (`fromTimestamp`) is set
  to the chronological maximum timestamp returned; the next run re-reads the full boundary instant from
  page 1 and durable dedupe suppresses the overlap.
- **Single-instant saturation** (≥ 50 000 traces at one millisecond): detected, cursor is parked (never
  pushed past unread data), and a distinct **terminal/capped diagnostic** is surfaced — never an infinite
  re-run loop. The operative invariant is unconditional: every trace is eventually imported exactly once.

### Known limitations

- **N+1 observations fetch** — a backfill over thousands of traces issues one `GET /api/public/observations`
  per trace. A future windowed-scan optimization is documented (design.md §4.4) but not built here; the
  per-trace path is correct (just slower) and chunked backfill limits the concurrency.
- **`seen_trace_ids` memory** — all trace IDs for an environment are loaded into a `HashSet` per run; flag
  for a bounded-cursor approach if histories grow large (single-user prototype, acceptable now).
- **Pre-existing pi-observe test failures (2)** in `tests/pi-observe.security.test.mjs` — unrelated to this
  task, file unchanged from `main` since before any TASK-029 commit.

### Compatibility and rollback

- **Data model (additive):** one new `settings` row (`langfuse_import_range`, default `last_30d`) and one
  new table `langfuse_backfill_progress` (a single-row inclusive resume cursor, DEC-032). No existing
  table or column is altered. Both use the idempotent `CREATE TABLE IF NOT EXISTS` posture.
- **Default window change:** import range floor moves from 7 days to 30 days. Re-importing a trace already
  in the store is a durable-dedupe no-op — no duplicate rows, no data loss.
- **Rollback:** revert the importer/settings/UI changes. The new `settings` row is inert to older builds
  (unknown key ignored). `langfuse_backfill_progress` is inert to older builds (unknown table never read)
  and re-created idempotently on re-upgrade. No destructive migration.
- **Security:** SEC-011 holds (diagnostic/sample surfaces are key/type names and counts only — no field
  values or credentials). SEC-002 loopback boundary, GET-only contract, disabled short-circuit, and
  absence-≠-zero invariant are all preserved. No new crate, egress host, permission, or capability added.

### Tests

**Rust** (`cargo test --manifest-path src-tauri/Cargo.toml`): **142 passed / 0 failed**
(includes new TASK-029 tests covering tolerant v3 identification, skip-reason classifier, SEC-011 secret-free
diagnostics, inclusive DEC-032 cursor, page-limit continuation, saturation terminal diagnostic and fixture,
backfill chunked resumability, equal-timestamp boundary dedup, multi-environment page-limit, and
continuation-store fault surfacing)

**Frontend** (`npm run test:frontend`): **71 passed / 2 pre-existing failures (unrelated)**
(new TASK-029 tests in `tests/importReport.test.mjs` covering range/backfill settings UI, grouped
diagnostics rendering, saturation terminal/capped note, mixed-env distinct rendering, and SEC-011
no-secret assertion; the 2 failures are in `tests/pi-observe.security.test.mjs`, unchanged from `main`)

### Manual smoke steps before shipping

1. With a live v3 Langfuse stack: confirm a manual import imports all traces (not 611/640 skipped).
2. Set Import range to `last_90d` and run **Backfill now**; confirm progress is durable (interrupt and
   re-run — no duplicate rows, run continues where it left off).
3. Confirm the import report shows grouped skip reasons (if any) instead of repeated warning strings.
4. Check Rollback smoke: open a prior Vire build on the same Mac — confirm no crash and the new settings
   row / `langfuse_backfill_progress` table are silently ignored.

---

## v0.1.0 — Desktop production readiness (TASK-026)

**Branch:** `feat/task-026-desktop-production-readiness`

### What changed

**In-app Langfuse settings with secure secret storage (Workstream A)**

- The Settings view now includes a **Langfuse integration panel** where you can configure base URL, source (local/cloud), environments, public key, secret key, and the enable/disable switch — without editing shell environment variables or restarting into a sourced shell.
- The **secret key is stored in the macOS Keychain** (service `dev.vire.app`), never in SQLite, logs, evidence rows, or exports. The settings form shows only a `set / not set` indicator and never renders the stored value back (SEC-009).
- The **public key is also stored in the Keychain** (same service) for a single clean credential surface.
- Non-secret settings (`base_url`, `source`, `environments`, `langfuse_enabled`) are stored as additive rows in the existing SQLite `settings` key/value table — no schema change to `projects` or `time_entries`.
- **Config precedence:** in-app settings override process environment variables. Env vars (`VIRE_LANGFUSE_*`) are retained as a clearly-marked developer fallback (the TASK-025 `.env.example` template remains valid for dev workflows).
- **Test connection:** a bounded action (20 s ceiling) that reports a coarse verdict (`reachable` / `auth_or_network_error`) without echoing any secret or raw response body. The button is disabled when the integration is turned off.
- **Disabled state:** with the integration toggle off, no import runs and no health probe fires; the source panel shows an explicit `disabled` state — never zero AI usage or cost.
- New Rust `keyring` crate dependency (`apple-native` feature, macOS Keychain Services).

**Mac application icon (Workstream B)**

- The packaged app now shows a **Vire icon** in the Dock and app switcher (generated into `src-tauri/icons/` including `icon.icns`; referenced by `bundle.icon` in `tauri.conf.json`).
- The current mark is a **temporary placeholder**. To replace with a branded asset: drop a PNG (≥ 1024 × 1024) at `src-tauri/icons/source/vire-icon.png`, run `npx tauri icon src-tauri/icons/source/vire-icon.png`, rebuild — no code change required.
- **Safe area (updated in TASK-027 E3):** the mark must occupy ~80% of the canvas (≈10% margin per side) — a full-bleed PNG renders oversized in the Dock. The placeholder was regenerated with this inset in TASK-027. The branded replacement PNG must keep the same safe area.

**Production packaged build, no dev server (Workstream C)**

- `npm run tauri:build` produces a self-contained `Vire.app` (and `.dmg` where the toolchain supports it) that **does not require a Vite dev server or `npm run tauri:dev` at runtime**.
- Artifacts: `src-tauri/target/release/bundle/macos/Vire.app` and (where supported) `src-tauri/target/release/bundle/dmg/Vire_0.1.0_<arch>.dmg`.

### Known limitations

- **Not code-signed or notarized.** On first launch, macOS Gatekeeper may block the app — right-click → Open (or *System Settings → Privacy & Security → Open Anyway*). Signing/notarization is out of scope for v0.1.
- **DMG generation** depends on toolchain support; `.app` is the primary artifact.
- **macOS only.** Cross-platform packaging (Windows/Linux) is out of scope.

### Compatibility and rollback

This release is safe to install alongside or after the prior v0.1 dev build on the same Mac:

- **Database:** uses the same `app_data_dir()/vire.sqlite` as all prior builds. The `init_db` schema init is idempotent (`CREATE TABLE IF NOT EXISTS` + `INSERT OR IGNORE`). The new Langfuse settings are additive key/value rows in the existing `settings` table — no destructive migration, no column changes to `projects` or `time_entries`.
- **Keychain:** entries are app-scoped (`dev.vire.app`) and persist across reinstall. They are not bundled in the artifact.
- **Rollback:** reverting to the immediately prior build opens the same `vire.sqlite` and ignores the unknown additive `settings` rows (key/value table, no schema dependency). No data loss. The prior build falls back to `VIRE_LANGFUSE_*` env vars for Langfuse configuration.

### Tests

**Rust** (`cargo test --manifest-path src-tauri/Cargo.toml`): **94 passed / 0 failed**
(includes 24 new TASK-026 tests covering config resolution precedence, SEC-009 secret non-leak, Test connection coarse verdict + bounded timeout, atomic Keychain pair rollback on failed replacement, and SEC-002 loopback boundary for settings-sourced values)

**Frontend** (`npm run test:frontend`): **39 passed / 2 failed**
(2 pre-existing failures in `tests/pi-observe.security.test.mjs` — unrelated to this task, file unchanged from `main`)

### Manual smoke steps before shipping

See [README.md — Packaged app and Langfuse settings](README.md#packaged-app-and-langfuse-settings-task-026--required-before-release) for the full checklist.

---

## v0.1 — Langfuse import, environment mapping and desktop polish (TASK-027)

**Branch:** `feat/task-027-langfuse-import-env-mapping-ux-polish`

### What changed

**Import report and schema_changed fix (Workstream A)**

- The **import source panel now shows real results** — counts of traces imported (unique / duplicates / skipped), plus per-environment health. Previously the result was discarded in the Rust core and the UI showed only a coarse health enum; `ImportReport`/`EnvImportLine` now thread counts all the way to the frontend.
- **`schema_changed` fixed for the current Langfuse payload shape.** `Observation` parsing now reads `usageDetails` and `costDetails` (current Langfuse token/cost locations) in addition to the legacy `usage`/`calculatedTotalCost` fields. A generation that arrives in the current shape no longer downgrades to `skipped_schema`; a genuinely unrecognised shape still does. The absence-≠-zero invariant is preserved.
- An empty or partial import is **explained, never blank** — the panel surfaces `missing` health, counts, and warnings for every environment.

**Automatic import (Workstream B)**

- Vire now **imports automatically**: once on startup (off the UI thread, after `init_db`) and periodically thereafter (default every 900 s; configurable via `VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS`, floored at 30 s). The same `run_blocking_import` path used by the manual button handles both.
- A **shared mutex serialises auto and manual imports** — a periodic tick skips when an import is already in progress, and the manual button queues behind any running import. No concurrent DB writes.
- Auto-import **respects all existing switches**: `langfuse_enabled` and the SEC-002 loopback boundary. A disabled integration runs nothing and reads no credentials. No new capability or CSP change.

**Environment discovery and picker (Workstream C)**

- Vire **discovers environments automatically** by scanning `GET /api/public/traces` without an `environment` filter and collecting distinct non-empty `Trace.environment` values. Discovery runs as part of every import (additive, no extra network round-trip beyond the import page scans). Results persist in the new `langfuse_discovered_environments` table (`environment PK`, `first_seen`, `last_seen`).
- The **Settings environment field is now a checkbox picker** seeded from discovered environments; `vire` is always offered. An *Advanced* CSV field remains as a fallback for environments not yet surfaced by discovery. Saving stores the union of ticked boxes and any advanced entries.

**Environment → project mapping (Workstream D)**

- A new **Environment → project mapping panel** in Settings shows every discovered environment as mapped or unmapped. An unmapped environment can be mapped to an existing project, or you can use **Create project for `<env>`** to create a project and map it in one explicit action.
- Vire **never auto-creates a project or auto-maps an environment** (DEC-006). Every mapping is a deliberate user action.
- Imported evidence is associated with a project **at read time via a join** — evidence rows are never rewritten. Clearing a mapping changes only the link; no data loss.
- New additive table: `langfuse_env_project_map(environment PK, project_id FK→projects, created_at, updated_at)`.

**Desktop UX polish (Workstream E)**

- **Fake macOS traffic-light buttons removed.** The titlebar now shows only the native window controls. The layout is re-balanced with a three-column CSS grid (brand centred, version right-aligned) so the titlebar is coherent without the left dots.
- **Icon safe-area regenerated.** The placeholder mark is now inset to **~80% of the canvas** (`SAFE=0.8` in `src-tauri/icons/source/generate-vire-mark.mjs`) so macOS renders it at Dock parity with other apps. Icon set regenerated: `icon.icns`, `128x128.png`, `32x32.png`, `64x64.png`, and the full `src-tauri/icons/` set.
- **Safe-area requirement for the branded asset:** the final branded PNG (brand-owned, `artifacts/brand/`) **must keep the same ~80% safe area** — a full-bleed 1024×1024 PNG renders oversized in the Dock. The placeholder generator already applies this inset; the branded asset must too.

### Compatibility and rollback

This change is **fully additive** and safe to install on top of any prior Vire build:

- **DB:** two new tables (`langfuse_discovered_environments`, `langfuse_env_project_map`). No changes to `projects`, `time_entries`, or the existing `langfuse_*` tables. All new tables use `CREATE TABLE IF NOT EXISTS` (idempotent init, same posture as prior releases).
- **Rollback:** reverting to the TASK-026 build on the same Mac leaves the importer manual-only and the environment field hand-typed CSV; the two new tables are present in the SQLite file but ignored. No data loss, no destructive migration.
- **Settings / Keychain:** unchanged from TASK-026. The mapping and discovery tables carry no secrets.

### App self-update → TASK-028 (DEC-029)

In-app self-update (Tauri updater plugin, artifact signing, macOS code signing + notarisation, GitHub Releases pipeline) is **split to TASK-028** (DEC-029). It is fully out of scope here — no updater plugin, no minisign key, no new network egress host, no `capabilities/default.json` change. Recommended split: Phase 1 lightweight version check (opens the download page); Phase 2 signed/notarised auto-install.

### Tests

**Rust** (`cargo test --manifest-path src-tauri/Cargo.toml`): **120 unit + 3 adversarial = 123 passed / 0 failed**
(26 new TASK-027-specific tests covering import report counts + health, schema_changed fix, secret-free diagnostics, environment discovery, env→project mapping, auto-import serialisation, and disabled short-circuit)

**Frontend** (`npm run test:frontend`): **51 passed / 2 pre-existing failures (unrelated)**
(12 new TASK-027-specific tests in `tests/envMappingUi.test.mjs` and `tests/shellChrome.test.mjs`; the 2 failures are in `tests/pi-observe.security.test.mjs`, unchanged from `main` since before any TASK-027 commit)

### Manual smoke steps before shipping

See [README.md — Build and run the packaged app](README.md#build-and-run-the-packaged-app) install steps 5–7 for the environment picker, mapping, and import report flows. The full manual macOS smoke checklist (packaged app, icon Dock size, native controls, startup import, env mapping) is in [design.md §9](openspec/changes/task-027-langfuse-import-env-mapping-ux-polish/design.md).
