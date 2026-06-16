# Design — TASK-027

Scope: make the Langfuse import actually return understandable results against Janne's live stack,
discover environments automatically, map environments to Vire projects, automate import, and polish the
desktop chrome. App self-update (F) is split to TASK-028 (DEC-029).

## 1. Component boundaries (no boundary is crossed)

Per `03_architecture_plan.md` §3 the components are: Capture · AI Runtime Observer · **Langfuse
Importer** · **Local SQLite Store** · Suggestion Engine · Review UI · Exporter · **Settings**.

| Workstream | Touches | In-boundary because |
| --- | --- | --- |
| A — import + diagnostics | Langfuse Importer, Store | The importer's own responsibility is "import trace timestamps, usage/cost fields … compute health" (`03_architecture_plan.md:89`). Surfacing counts and tolerating payload shape is internal to it. |
| B — auto-import | App runtime (lib.rs), Importer | A **trigger** for the existing importer, not a new evidence source. No new authority; importer stays the sole AI cost/time source (DEC-003/017). |
| C — env discovery | Importer (read), Store, Settings | Discovery is a read-only trace scan inside the importer; persistence is additive Store; surfacing is Settings. |
| D — env→project mapping | Store, Settings, Projects (read/create via existing path) | Realizes `PROJECT_MAPPING` (§3 data model) — Vire-authoritative env→project mapping; suggestion-first per DEC-006. |
| E — UX polish | Review/Settings UI (frontend shell), window config, icon asset | Pure presentation; no data-model impact. |

**No data-model component boundary is crossed.** The AI Runtime Observer, capture, classifier, review,
and exporter are untouched. The importer remains read-only and the single AI-evidence authority.

## 2. Root-cause analysis — why "reachable" but "no useful data"

Three independent causes; the fix addresses all three.

1. **Result is discarded (definite).** `import_langfuse_now` (`lib.rs:303`) → `run_blocking_import`
   (`mod.rs:37`) computes `Vec<ImportSummary>` (`importer.rs:58`) with full counts, then
   `import_result` (`mod.rs:58`) throws it away and returns `Ok(())`; the command returns only a
   `SourceHealthSnapshot` (one health enum + message). The UI literally cannot show counts because they
   never leave the Rust core. → **Workstream A** threads a report out.

2. **Environment filter mismatch (most likely cause of "empty").** Traces are queried with a hard
   `environment=<env>` param (`config.rs:226`) over `allowed_environments` (default `["vire"]`,
   `config.rs:15`). Janne's pi/Claude Code traces are very likely under a different environment (the
   plan flags `default` and pi-langfuse propagation issues — `03_architecture_plan.md:144,152`). A
   non-matching env returns an empty page → `Missing`; only the synthetic `default` probe
   (`importer.rs:78-85`) surfaces `default`. Any other environment is invisible, and the user has no
   way to learn its name. → **Workstreams C (discover) + D (map)**.

3. **Payload-shape drift (must verify).** Token/cost come from `usage{input,output,total}`, top-level
   `promptTokens/...`, and `calculatedTotalCost` (`model.rs:152-217`). Current Langfuse may instead
   populate `usageDetails`/`costDetails`. If so, every generation hits `lacks_usage_and_cost()`
   (`model.rs:222`) → `schema_changed`, totals withheld → "no useful data" even when traces are found.
   → **Workstream A** verifies against the live shape and widens the parser.

## 3. Workstream A — import report + payload tolerance

### 3.1 Secret-free import report

Add a serializable report type (illustrative; exact shape is the implementer's):

```
ImportReport {
  total_unique: usize,
  total_seen: usize,
  total_duplicates: usize,
  total_skipped_schema: usize,   // generations/traces dropped for shape reasons
  environments: Vec<EnvImportLine { environment, health, pages, traces_seen, unique, duplicates, skipped, warnings }>,
}
```

- Build it from the existing `Vec<ImportSummary>` already returned by `importer::run_import` — the data
  exists; A just stops discarding it. `schema_issues` is already counted (`importer.rs:149,188`); expose
  it as `skipped`.
- `import_langfuse_now` returns `{ snapshot, report }` (or the report carries the snapshot fields).
- **SEC-010:** the report is built only from counts, health enums, and the existing secret-free warning
  strings (`ApiError::message` / fixed sentinels — already secret-free, `model.rs:75`,
  `importer.rs:395`). It MUST NOT include raw trace payloads (which may carry prompts/sessions),
  response bodies, or any credential. Add a test asserting the report contains none of
  `sk-`/`pk-`/`Bearer`/`Authorization`/raw-payload text.
- The frontend (`sourcePanel` / post-import handler, `main.ts:45,49`) renders
  "Imported N (M skipped, D duplicates) across E environments" with per-env health, so an empty/partial
  result is **explained**, never blank.

### 3.2 Payload tolerance (verify-then-widen)

- The implementer MUST first capture the **actual** shape from Janne's local stack (a non-secret sample
  of `GET /api/public/traces` and `GET /api/public/observations?traceId=`) and confirm field locations.
- Extend `Observation` parsing so `prompt()/completion()/total()/cost()` also read current Langfuse
  locations (`usageDetails`, `costDetails`) in addition to the existing fields. Preserve the
  absence-≠-zero invariant: a field absent everywhere stays `None`; only a value of `0` reads as `0`.
- Keep `schema_changed` for a genuinely unrecognized shape, but ensure recognized-but-newer shapes are
  parsed, not degraded. Add fixture tests for both the legacy and current shapes against the in-memory
  mock `LangfuseApi` (the trait seam at `api.rs:14` makes this network-free).

## 4. Workstream C — environment discovery

- **Mechanism:** the Langfuse public API has **no list-environments endpoint** (verified scope: traces,
  observations, health are the paths in use — `config.rs:80-93`). Discovery = call
  `GET /api/public/traces` over the recent window **without** the `environment` query param (the param
  is a filter; omitting it returns all environments) and collect the distinct non-empty `environment`
  values from `Trace.environment` (`model.rs:135`).
- **VERIFICATION FLAG (assumption to confirm at SW-2):** that omitting `environment` returns
  cross-environment traces, and that `Trace.environment` is populated on the list payload. If the list
  payload omits `environment`, fall back to reading it from the observations/trace detail. The
  implementer MUST confirm against the live stack before relying on it. This is the one external
  assumption in the change.
- A new `ApiPath::TracesAllEnvironments` (or make `environment` optional on the existing `Traces`
  variant) keeps the URL allowlist intact — still rooted under `/api/public/`, still loopback-gated by
  `validate_target` (`config.rs:168`). No new host.
- Persist discovered environments (additive table/column) with a last-seen timestamp; surface in
  Settings as a multi-select picker that seeds `allowed_environments`. Free-text CSV remains as an
  advanced fallback. Discovery runs as part of import (cheap: it reuses the trace scan).

## 5. Workstream D — environment → project mapping

- **Data model (additive):**
  ```
  langfuse_env_project_map (
    environment TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
  )
  ```
  Created in `init_db` via `CREATE TABLE IF NOT EXISTS` (idempotent, additive — same rollback posture as
  `RELEASE.md:36`). No change to `projects`/`time_entries`/existing `langfuse_*` tables.
- **Authority (DEC-001):** the Vire project record is source of truth; the environment is the external
  key. Mapping associates imported evidence (already keyed by `environment` in
  `langfuse_ai_evidence`/`langfuse_raw_traces`, `store.rs:38,30`) with a project via the map — no
  rewrite of evidence rows is required (join at read time).
- **Suggest-create (DEC-006):** for a discovered environment with no map row, Settings shows a "Create
  a project for environment `<env>`" suggestion. Clicking it calls the **existing** `create_project`
  (`lib.rs:161`) and then writes the map row. Vire never auto-creates a project or auto-maps silently —
  the human approves each.
- IPC: `list_env_mappings`, `set_env_mapping(environment, project_id)`, `clear_env_mapping(environment)`,
  and discovered-environments are returned with their mapping state so the UI can render mapped /
  unmapped / suggest-create per environment.

## 6. Workstream B — automatic import

- **Triggers:** (1) once on app startup (after `init_db`, in `run()` setup, `lib.rs:330`), off the UI
  thread; (2) a periodic interval (e.g. a dedicated OS thread sleeping a configurable interval, or a
  Tauri async task spawning the blocking import on the blocking pool). Both call the **same**
  `run_blocking_import` used by the manual button — no parallel importer.
- **Serialization:** guard concurrent runs (an import-in-progress flag / mutex) so a periodic tick
  cannot overlap a manual click or a prior tick against the same SQLite DB. Each import already uses its
  own connection (`mod.rs:38`) and persists atomically (`store.rs:128`); the guard prevents redundant
  concurrent work and lock contention.
- **Respect the switches:** auto-import checks `langfuse_enabled` (`settings/mod.rs:127`) and resolves
  config settings-first exactly like manual; a disabled integration runs nothing and probes nothing
  (mirrors `import_langfuse_now`'s short-circuit, `lib.rs:307`). The SEC-002 loopback boundary applies
  unchanged.
- **No UI freeze, no zero:** background imports never block the UI (off-thread, bounded by the existing
  reqwest ceilings); failures land in the health taxonomy and the import report, never as zero.
- **No new capability:** this is pure Rust-core scheduling; `capabilities/default.json` and the CSP are
  unchanged (the renderer still makes no network call). If any capability/CSP change appears necessary,
  **stop and flag** — it would be architecture-level.

## 7. Workstream E — desktop UX polish

- **Fake traffic lights:** delete the `<div class="traffic">…</div>` from `shell()` (`main.ts:35`) and
  the `.traffic` rules from `style.css`. The Tauri window keeps native decorations (no change to
  `tauri.conf.json` decorations), so the real controls remain. Re-balance the `.titlebar`
  `grid-template-columns` (currently `180px 1fr 180px`, `style.css`) so the centered brand and
  right-aligned version still align without the left dots.
- **Dock icon padding:** the placeholder mark currently fills the canvas
  (`generate-vire-mark.mjs` — `roundedRectSDF` spans full `N`). Regenerate with the rounded square
  inset to ~80% of the canvas (transparent safe-area margin) so the Dock renders it at parity with
  other apps, then re-run `npx tauri icon …` to regenerate `src-tauri/icons/*` incl. `icon.icns`.
  Update the README/RELEASE note that the **branded** asset (brand-owned, `artifacts/brand/`,
  read-only to SW) must keep the same safe-area when it lands.
- **Window chrome:** confirm `title`, min sizes, and that removing the fakes leaves no visual gap; no
  functional window-behaviour change is in scope.

## 8. F — app self-update: why it is split (summary; full rationale in arch-review §F)

"Click to install" requires: Tauri updater plugin + new capability + bundled minisign **public** key;
minisign **private** key to sign artifacts (secret management); macOS **code signing + notarization**
(Apple Developer ID — out of scope at v0.1, `RELEASE.md:32`); a release pipeline (GitHub Releases +
`latest.json`); and a **new network egress host** altering SEC-002. Different competency, different
release risk, and a one-off network-boundary change that should be designed once with the updater.
→ **TASK-028** (DEC-029). Recommended: Phase 1 lightweight version check (open download page), Phase 2
signed/notarized auto-install.

## 9. Testing strategy (L2)

- **Rust (network-free via the `LangfuseApi` trait mock, `api.rs:14`):** import report counts/health
  for healthy / missing / wrong-env / schema-changed; report is secret-free (no `sk-`/`pk-`/`Bearer`/
  raw payload); payload-tolerance fixtures (legacy `usage` shape AND current `usageDetails`/`costDetails`
  shape both parse; absence stays `None`); environment discovery collects distinct envs from a
  multi-env page; env→project map persistence + idempotent `init_db`; suggest-create writes a map row
  only on explicit action; auto-import serialization guard prevents overlap; disabled short-circuit runs
  nothing.
- **Frontend:** import-result panel renders counts and per-env health (incl. the empty/skipped case);
  env picker reflects discovered environments; mapping UI shows mapped/unmapped/suggest states; the
  `shell()` snapshot no longer emits `.traffic`.
- **Manual macOS smoke:** packaged `.app` shows the icon at Dock parity (no oversize); native window
  controls present, no fake dots; startup + periodic import populates the panel; manual import still
  works; an environment with real traces shows usage/cost.

## 10. Compatibility & rollback (L2)

- **DB:** all new tables/columns are additive via idempotent `init_db` (`CREATE TABLE IF NOT EXISTS`);
  no change to `projects`/`time_entries`. A prior build reopens the same `vire.sqlite` and ignores the
  new tables — no data loss (same posture as `RELEASE.md:36`).
- **Settings/Keychain:** unchanged from TASK-026; env→project map and discovered envs carry no secrets.
- **Rollback:** reverting to the TASK-026 build leaves the importer manual-only and the env CSV
  hand-typed; the additive tables are simply unused. No destructive migration.
- **Docs:** README/RELEASE updated for auto-import behaviour, the env picker + mapping flow, the icon
  safe-area, and a forward pointer to TASK-028 for app updates.
