# TASK-027 — Langfuse import that works, environment discovery + mapping, desktop UX polish

## Why

Janne installed the merged TASK-026 build on his Mac and tested it end-to-end. Settings and the
Keychain credential flow work, and **Test connection reports `reachable` / authenticated** — but a
**manual import returns no useful data and shows nothing actionable**: the AI-evidence panel stays
empty, the "latest trace" is blank, and environments are something the user has to type by hand. Four
concrete gaps stop the importer from being usable, plus desktop chrome rough edges and a request for an
in-app update affordance.

### A — The import is silent and (for Janne's data) finds nothing

Two distinct problems compound:

1. **The result is silent by construction.** `import_langfuse_now`
   (`src-tauri/src/lib.rs:303`) runs `langfuse::run_blocking_import`, which collapses the rich
   per-environment `ImportSummary` (pages, traces seen, unique, duplicates, schema issues, warnings —
   `src-tauri/src/langfuse/importer.rs:26`) down to `Result<(), String>` in `import_result`
   (`src-tauri/src/langfuse/mod.rs:58`) and then returns only a one-line `SourceHealthSnapshot`. Every
   count the importer computed is **discarded** before it reaches the UI, so a run that imported,
   skipped, or errored on traces looks identical to one that did nothing. There is no
   "imported N / skipped M / K errors" feedback — the exact "silently shows nothing" Janne saw.

2. **The environment filter almost certainly misses Janne's traces.** The importer queries
   `GET /api/public/traces?environment=<env>` (`src-tauri/src/langfuse/config.rs:226`) for each
   configured environment, defaulting to `["vire"]` (`config.rs:15`). Any trace whose Langfuse
   `environment` is something else (e.g. `default`, `production`, a per-repo name) is invisible: the
   `vire` query returns an empty page → `Missing`, and only the synthetic `default` probe
   (`importer.rs:83`) surfaces `default`. The architecture plan already warned this is the failure mode
   ("detect traces landing in `default`"; "pi-langfuse needs local patching for environment
   propagation" — `03_architecture_plan.md:144,152`). The user has no way to see *which* environments
   actually exist, so they cannot know what to type.

3. **Payload-shape drift is unverified against Janne's live Langfuse.** Token/cost are read from
   observation fields `usage{input,output,total}`, top-level `promptTokens/...`, and
   `calculatedTotalCost` (`src-tauri/src/langfuse/model.rs:152`). Current Langfuse also emits
   `usageDetails`/`costDetails` maps; if Janne's integration populates those instead, every generation
   trips `lacks_usage_and_cost()` → `schema_changed`, totals withheld. The importer must be **verified
   against, and tolerant of, the payload shape Janne's stack actually returns**.

### B — Import is manual-only

The only way to refresh AI evidence is the explicit "Import from Langfuse now" button. There is no
import on app startup and no periodic/background refresh, so the panel is stale until the user
remembers to click. Janne wants automatic import (startup + periodic/background or on refresh) **while
keeping manual import as an explicit action**.

### C — Environments are typed by hand

The environment list is a free-text CSV field in Settings (`src/main.ts:46`). Vire has no discovery:
it cannot tell the user which environments exist in their Langfuse. The user should not have to type
the env list.

### D — No environment → Vire-project mapping

Vire imports per environment but never maps an environment to a Vire **project**. The architecture
plan's `PROJECT_MAPPING` boundary ("map traces to Vire projects by environment first" —
`03_architecture_plan.md:141`) is unrealized. Janne wants to map each discovered Langfuse environment
to a Vire project, and for unmapped environments wants Vire to **suggest creating a project per env**.

### E — Desktop chrome rough edges

- **Fake macOS traffic-light buttons.** `shell()` renders a decorative `.traffic` cluster of three
  coloured dots (`src/main.ts:35`, styled `src/style.css` `.traffic`). The Tauri window keeps native
  macOS decorations (no `decorations:false` in `tauri.conf.json`), so the **real** close/minimise/zoom
  controls already exist in the native titlebar — these are duplicate, non-functional fakes and must be
  removed.
- **Dock icon too large.** The placeholder icon mark fills the full 1024px canvas
  (`src-tauri/icons/source/generate-vire-mark.mjs` — rounded square covers the whole canvas), so in the
  Dock it reads larger than neighbouring apps, which follow Apple's ~80% content-in-canvas safe-area
  convention. The mark needs transparent safe-area padding.
- **Titlebar / window chrome** should be reviewed for consistency once the fakes are gone.

### F — In-app app-update affordance (evaluated → split out, see below)

Janne wants an "Update available — click to install" affordance. A true click-to-**install** updater
requires the Tauri updater plugin, minisign update-artifact signing, **macOS code signing +
notarization** (explicitly out of scope as of v0.1 — `RELEASE.md:32`, `README.md:49`), a release
pipeline publishing to GitHub Releases (`origin` = `github.com/kaikkone4/vire`), and a **new network
egress host** that changes the SEC-002 network boundary. This is a different competency and
disproportionate to this task's cohesive theme. **It is split into a new task — TASK-028 — not built
here.** See the ADR (DEC-029) and `arch-review.md` §F.

These are usability/correctness gaps for the private local prototype, not a change to the AI-evidence
model, data boundaries, or local-only posture. A–E operationalize behaviours the architecture already
specified (environment-first mapping, environment-health detection) but never realized.

## What Changes

One OpenSpec change, **five sequenced workstreams (A → E)**. Workstream A is the correctness core and
is implemented/reviewed first; B–E build on it. F is **out of this change** (TASK-028).

### Workstream A — Make import work + add secret-free diagnostics  *(backend Rust)*

- **Thread the import report to the UI.** Replace the unit return of `run_blocking_import` with a
  secret-free **import report**: per-environment and total counts (pages walked, traces seen, unique
  imported, duplicates suppressed, traces skipped for schema reasons, per-environment health) and the
  existing secret-free warning strings. `import_langfuse_now` returns this report alongside the health
  snapshot so the Settings panel can render "Imported N traces across M environments; K skipped; …"
  instead of nothing. The report MUST never contain credentials, raw response bodies, or trace
  prompt/session content (**SEC-010**).
- **Verify + tolerate the live payload shape.** Confirm the importer against the trace/observation
  shape Janne's local Langfuse actually returns and extend the observation parser to also read current
  token/cost locations (e.g. `usageDetails` / `costDetails`) so present usage/cost is captured rather
  than degraded to `schema_changed`. Absence stays absence (`None`, never `0`); a genuinely unknown
  shape still degrades to `schema_changed` with a surfaced count.
- Manual import stays the existing **bounded** (`run_bounded`, `lib.rs:296`) path; no new health state
  is added beyond the ten-state taxonomy (`model.rs:11`).

### Workstream C — Environment discovery  *(backend Rust + frontend)*  *(precedes D)*

- Add a read-only **discover environments** path: scan recent traces **without** an environment filter
  and collect the distinct `environment` values the stack reports, persisting them as discovered
  environments. (Langfuse's public API exposes no list-environments endpoint; discovery is by trace
  scan — see `design.md` §4 and the verification flag.)
- Surface discovered environments in Settings as a **picker** so the user selects from what exists
  instead of typing. The free-text CSV stays available as an advanced/fallback affordance; defaults
  (`vire`) unchanged.

### Workstream D — Environment → Vire-project mapping  *(backend Rust + frontend)*

- Add a Vire-authoritative mapping from a Langfuse environment to a Vire **project** (new additive
  table; no change to `projects`/`time_entries`). Imported evidence for a mapped environment is
  associated with its project; the Vire project record stays the source of truth (DEC-001).
- For a **discovered-but-unmapped** environment, Vire **suggests creating a project** for it; creation
  goes through the existing `create_project` path on **explicit user action** — never silent
  auto-creation (suggestion-first, DEC-006).

### Workstream B — Automatic import  *(backend Rust)*  *(builds on A)*

- Run an import **on app startup** and on a **periodic background interval**, in addition to the
  explicit manual button (which is preserved unchanged). Auto and manual imports use the **same**
  `run_blocking_import` path and are **serialized** (never two concurrent imports against the same DB).
- Auto-import respects the `langfuse_enabled` switch and the SEC-002 loopback boundary identically to
  manual import; a disabled integration runs nothing. Background errors degrade to the existing health
  states and are never surfaced as zero.

### Workstream E — Desktop UX polish  *(frontend + assets)*

- **Remove the fake traffic-light buttons** from `shell()` (`src/main.ts:35`) and their `.traffic` CSS
  (`src/style.css`); keep the native macOS window controls. Re-check the titlebar layout so the brand /
  version text still reads correctly without the dots.
- **Fix the Dock icon** by regenerating the placeholder mark with transparent safe-area padding
  (~80% content) so it sits at parity with other Dock icons; document the same safe-area requirement so
  the eventual branded asset inherits it. The placeholder generator lives in `code/`; the **branded**
  asset remains brand-owned (`artifacts/brand/` is read-only to SW).

## Impact

- **Affected code (A, B, C, D):** `src-tauri/src/langfuse/{importer.rs,mod.rs,model.rs,api.rs,store.rs,config.rs}`
  (import report, payload tolerance, env discovery scan, discovered-env + mapping persistence),
  `src-tauri/src/lib.rs` (new IPC commands: import report, list discovered environments, get/set
  env→project mappings, suggest-create; startup + interval auto-import trigger), `src-tauri/src/settings/`
  (surface discovered environments / mappings), `src/main.ts` + `src/style.css` (import-result display,
  env picker, mapping UI).
- **Affected code (E):** `src/main.ts` (`shell()`), `src/style.css` (`.traffic`),
  `src-tauri/icons/source/generate-vire-mark.mjs` + regenerated `src-tauri/icons/*`, possibly
  `tauri.conf.json` window block; `README.md` icon-replacement note updated.
- **Data model:** additive only — a discovered-environments record and an `environment → project_id`
  mapping table. **No change** to `projects` / `time_entries`; existing `langfuse_*` tables unchanged
  except additive columns if needed. Forward/backward compatible (idempotent `init_db`,
  `CREATE TABLE IF NOT EXISTS`), per the TASK-026 rollback posture (`RELEASE.md:36`).
- **Affected specs:** **MODIFY `langfuse-importer`** (import diagnostics report, payload tolerance,
  automatic import, environment discovery); **ADD `project-env-mapping`** (env→project mapping +
  suggest-create); **ADD `desktop-ui`** (native window chrome, Dock icon safe-area). `app-configuration`,
  `csv-export`, `runtime-reconciliation` are **not** modified.
- **Security (SEC-010):** the new import report / counts / discovery / mapping surfaces are secret-free —
  no credentials, raw bodies, or prompt/session content; extends SEC-003. Auto-import preserves the
  SEC-002 loopback boundary and the disabled short-circuit. No new network egress host is added in this
  change (F, which would add one, is split out).
- **Out of scope (clean boundaries):**
  - **App self-update / updater / signing / notarization / releases (F)** → **TASK-028** (DEC-029).
  - Any change to the **AI-evidence taxonomy** (ten states stay), capture, the runtime observer, the
    classifier/suggestion engine internals, the review UI, or CSV export.
  - **OTEL trace emission** — Vire stays a read-only trace consumer.
  - **Auto-creating projects** for environments — creation stays an explicit, human-approved action.
  - Replacing the **branded** icon asset (brand-owned); only the placeholder padding + docs change here.

## ADR — DEC-027 (proposed): automatic Langfuse import

**Decision.** Vire imports AI evidence automatically **on app startup** and on a **periodic background
interval**, in addition to the explicit manual "Import from Langfuse now" action, which is retained.
Automatic and manual imports share the single `run_blocking_import` path and are **serialized** (no two
concurrent imports against the local DB). Auto-import obeys `langfuse_enabled` and the SEC-002 loopback
boundary identically to manual import, runs off the UI thread/lock, and is bounded; failures resolve to
the existing health taxonomy, never zero. The importer remains the **sole** AI cost/time authority
(DEC-003 / DEC-017 unchanged).

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log via
`feedback_to_ba[]`.

**Alternatives considered.** (1) *Manual-only (status quo)* — rejected: the panel goes stale; Janne
explicitly asked for automatic refresh. (2) *Aggressive polling / OS background scheduler* — rejected
as disproportionate for a single-user local prototype; a modest in-process interval plus a startup run
is enough and keeps everything in the existing Rust core.

## ADR — DEC-028 (proposed): environment discovery + environment→project mapping

**Decision.** Vire **discovers** Langfuse environments by scanning recent traces (the public API has no
list-environments endpoint), persists the discovered set, and lets the user **map** each environment to
a Vire **project**. Mapping is **Vire-authoritative** (DEC-001): the Langfuse environment is an
external signal, the Vire project record is the source of truth. A discovered-but-unmapped environment
produces a **suggestion** to create a project for it; creation is an explicit, human-approved action
(DEC-006), never silent. This realizes the `PROJECT_MAPPING` / "environment-first mapping" the
architecture plan specified (`03_architecture_plan.md:141,175`).

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log.

**Alternatives considered.** (1) *Keep manual env CSV only* — rejected: the user cannot know which
environments exist (Janne's core complaint). (2) *Auto-create a project per discovered environment* —
rejected: violates the suggestion-first / human-approval gate (DEC-006). (3) *Map by metadata/session
instead of environment* — deferred: environment-first is the architecture's primary mapping mechanism;
finer signals are a later refinement.

## ADR — DEC-029 (proposed): app self-update split to TASK-028

**Decision.** The in-app app-update capability (F) is **split into TASK-028** and is **not** part of
TASK-027. A true "click to install" updater requires the Tauri updater plugin (new capability +
`tauri.conf.json` plugin config + a bundled minisign public key), minisign signing of update artifacts
(a new private-key/secret-management concern), **macOS code signing + notarization** (explicitly out of
scope at v0.1 — `RELEASE.md:32`), a release-publishing pipeline (GitHub Releases + a `latest.json`
endpoint), and a **new network egress host** that changes the SEC-002 network boundary. These cross
into devops/release infrastructure and an Apple Developer ID — a different competency and release risk
from TASK-027's import/UX theme. **Recommendation:** TASK-028 Phase 1 = a lightweight, read-only update
**check** (compare running version to the latest GitHub release; show "Update available", open the
release/download page) so Janne gets the affordance soon; Phase 2 = full signed/notarized auto-install.
Designing the new egress boundary once, with the updater, is cleaner than bolting a one-off check onto
this UX task.

**Status.** Proposed (this change); the canonical decision and the new-task scope are routed to BA /
Pi-Assistant. See `arch-review.md` §F for the full split rationale.
