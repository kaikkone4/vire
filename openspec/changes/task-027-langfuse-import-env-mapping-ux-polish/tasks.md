# Tasks — TASK-027

Five workstreams. Implement/review **A first** (correctness core), then **C → D** (discovery enables
mapping), then **B** (auto-import builds on A), then **E** (independent polish). F is **not here** —
it is TASK-028 (DEC-029).

Role hints are for Pi-Assistant routing.

## Workstream A — Import works + secret-free diagnostics  *(backend developer)*

- [x] A1. Capture the **live payload shape** from Janne's local Langfuse: a non-secret sample of
  `GET /api/public/traces` and `GET /api/public/observations?traceId=`; confirm where token counts and
  cost live (`usage` vs `usageDetails`/`costDetails`, `calculatedTotalCost`). Record findings.
  *(Verified against live local stack v3.178.0 @ 127.0.0.1:3010 + TASK-007 spike, shape-only; see
  `workstream-a-payload-verification.md`. No authenticated body capture — keys stay in Keychain.)*
- [x] A2. Add a serializable **import report** (per-env + total counts: pages, seen, unique, duplicates,
  skipped-for-schema, per-env health, secret-free warnings) built from the existing
  `Vec<ImportSummary>`; stop discarding it in `import_result` (`langfuse/mod.rs:58`).
  *(`ImportReport`/`EnvImportLine` in `langfuse/importer.rs`; `skipped_schema` added to `ImportSummary`.)*
- [x] A3. Return the report from `import_langfuse_now` (`lib.rs:303`) alongside the health snapshot;
  keep the bounded path (`run_bounded`).
  *(`run_blocking_import` → `Result<ImportReport,String>`; command returns `ImportOutcome{snapshot,report}`
  via `run_bounded_result`, preserving the bound + TASK-021 in-band persist-failure surfacing.)*
- [x] A4. Widen `Observation` parsing (`langfuse/model.rs`) to also read current Langfuse token/cost
  locations; preserve absence-≠-zero; keep `schema_changed` only for genuinely unknown shapes.
  *(reads `usageDetails["input"|"output"|"total"]` + `costDetails["total"]` in legacy→current precedence.)*
- [x] A5. Frontend: render the import result in the source panel (`src/main.ts:45,49`) — counts,
  per-env health, and the empty/skipped explanation; never blank.
  *(`ImportOutcome` types + `importReportLine()` in `src/main.ts`; tsc clean.)*
- [x] A6. Tests: report counts per health; **report is secret-free** (no `sk-`/`pk-`/`Bearer`/
  `Authorization`/raw payload); payload-tolerance fixtures for legacy AND current shapes.
  *(10 new tests in `langfuse/tests.rs`; full `cargo test` green — 104 unit + 3 adversarial.)*

## Workstream C — Environment discovery  *(backend developer + frontend)*  *(after A)*

- [x] C1. Add a read-only discovery path: query `GET /api/public/traces` over the recent window
  **without** the `environment` filter; collect distinct non-empty `Trace.environment` values. Keep the
  URL allowlist + loopback gate intact (optional `environment`, still `/api/public/` + `validate_target`).
  *(`ApiPath::TracesAllEnvironments` + `LangfuseApi::get_traces_any_env`; `langfuse/discovery.rs`
  `discover_environments` paginates + collects distinct names; URL stays under `/api/public/traces`,
  loopback-gated, env param dropped.)*
- [x] C2. **VERIFY** the discovery assumption against the live stack (omitting `environment` returns
  cross-env traces and `environment` is populated on the list payload). If not, fall back to the trace/
  observation detail. **Stop and flag** if neither works.
  *(Verified secret-free at SW-2: live stack reachable `127.0.0.1:3010` (health 200); `/api/public/traces`
  present + auth-gated (401), so omit-vs-required cannot be distinguished by status — auth precedes param
  validation, and authenticated body capture is forbidden by SEC-003/A1. `Trace.environment` confirmed
  populated on the live **list** payload in TASK-007 §2.2 (same stack, shape-only), where `environment` is
  one of the optional list filters. Safe fallback exists (trace-detail read + advanced hand-entered CSV),
  so discovery degrading to empty never blocks the user → proceed, not blocked.)*
- [x] C3. Persist discovered environments (additive table/column, last-seen ts); run discovery as part
  of import.
  *(`langfuse_discovered_environments(environment PK, first_seen, last_seen)` in `langfuse/store.rs`;
  `upsert_discovered_environment` advances `last_seen` additively; `run_blocking_import` runs
  `discover_and_record` best-effort after the import.)*
- [x] C4. Settings: replace/augment the free-text env CSV with a **picker** seeded from discovered
  environments (CSV stays as advanced fallback; default `vire` unchanged). *(`src/env-mapping-ui.ts`
  `envPickerCheckboxes`/`envPickerOptions`/`mergeSelectedEnvironments`; Settings form ticks discovered +
  configured envs, `vire` always offered, Advanced CSV adds undiscovered envs; save = union of both.)*
- [x] C5. Tests: discovery collects distinct envs from a multi-env page; allowlist/loopback unchanged.
  *(`langfuse/tests.rs`: distinct-across-pages, empty/error, additive persistence, allowlist+loopback URL.)*

## Workstream D — Environment → Vire-project mapping  *(backend developer + frontend)*  *(after C)*

- [x] D1. Add additive `langfuse_env_project_map(environment PK, project_id FK→projects, created_at,
  updated_at)` in `init_db` (`CREATE TABLE IF NOT EXISTS`).
  *(`env_mapping::migrate` called from `init_db` after `projects` exists so the FK resolves.)*
- [x] D2. IPC: `list_env_mappings`, `set_env_mapping`, `clear_env_mapping`; return discovered
  environments with mapping state (mapped / unmapped).
  *(All in `lib.rs` + registered in `generate_handler`; `list_discovered_environments` returns
  `DiscoveredEnvState{mapped, project_id, project_name}`. `set_env_mapping` maps an existing project
  only — never creates.)*
- [x] D3. Associate imported evidence with a project via the map at **read time** (join) — no evidence
  rewrite. Vire project record stays authoritative (DEC-001).
  *(`list_evidence_projects_repo` LEFT JOINs evidence→map→projects; `list_evidence_projects` IPC.
  Clearing a mapping changes only the join, evidence rows untouched — asserted by test.)*
- [x] D4. Settings UI: per environment show mapped project / a project picker / a **"Create a project
  for `<env>`"** suggestion that calls the existing `create_project` then writes the map row (explicit
  user action only — DEC-006). No silent auto-create. *(`src/env-mapping-ui.ts` `mappingPanel`/
  `mappingRow` + `bindEnvMapping` in `src/main.ts`: mapped row shows project + Clear; unmapped row shows
  a project picker (Map) and Create-project-for-`<env>` which prompts, calls `create_project` then
  `set_env_mapping`. Clear calls `clear_env_mapping`. No auto-create/auto-map.)*
- [x] D5. Tests: map persistence + idempotent init; suggest-create writes a row only on explicit action;
  evidence-to-project join. *(`env_mapping/tests.rs`: idempotent migrate, set/list/remap/clear,
  missing-project refusal creates nothing, suggest-create explicit-only with no auto-create, read-time
  join + no-rewrite, secret-free surfaces.)*

## Workstream B — Automatic import  *(backend developer)*  *(after A)*

- [x] B1. Run an import once **on app startup** (after `init_db` in `run()` setup, `lib.rs:330`), off
  the UI thread. *(Dedicated OS thread spawned in `run()` setup; first loop iteration imports at startup.)*
- [x] B2. Run a **periodic background import** at a (configurable) interval via the same
  `run_blocking_import`; manual button preserved unchanged. *(Same thread sleeps `auto_import_interval()`
  — default 900s, env override `VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS` floored at 30s — and re-runs the
  same `run_blocking_import`; manual `import_langfuse_now` unchanged.)*
- [x] B3. **Serialize** imports (in-progress guard/mutex) so auto and manual never overlap on the DB.
  *(Shared `AppState.import_lock: Arc<Mutex<()>>`; auto `try_acquire_import_slot` skips when busy, manual
  worker `acquire_import_slot` waits — bounded by the existing import ceiling. Poison-recovering.)*
- [x] B4. Respect `langfuse_enabled` + SEC-002 loopback identically to manual; disabled runs nothing.
  No new capability/CSP; **stop and flag** if one seems needed. *(`run_auto_import_cycle` checks
  `settings::langfuse_enabled` before any probe/Keychain and reuses `run_blocking_import`'s loopback gate;
  no capability/CSP change.)*
- [x] B5. Tests: serialization guard prevents overlap; disabled short-circuit; failure lands in the
  health taxonomy, never zero. *(`lib.rs` tests: slot serialization, disabled records nothing, busy-skip;
  failure-into-taxonomy is the existing `run_import` behavior reused unchanged.)*

## Workstream E — Desktop UX polish  *(frontend + assets)*  *(independent)*

- [x] E1. Remove the fake `.traffic` cluster from `shell()` (`src/main.ts:35`) and the `.traffic` CSS
  (`src/style.css`); keep native window controls. *(Titlebar extracted to `src/shell-chrome.ts`
  `titlebar()`; `shell()` calls it. `.traffic` div and all `.traffic` CSS rules deleted. Native window
  decorations unchanged in `tauri.conf.json`.)*
- [x] E2. Re-balance `.titlebar` layout so brand/version still align without the dots. *(`.titlebar`
  grid `1fr auto 1fr`: brand centered in col 2, version pinned to col 3 — symmetric without the left
  dots.)*
- [x] E3. Regenerate the placeholder icon mark with ~80% safe-area padding
  (`src-tauri/icons/source/generate-vire-mark.mjs`), re-run `npx tauri icon …`, regenerate
  `src-tauri/icons/*` incl. `icon.icns`. *(Generator now insets the mark to 80% of canvas (`SAFE=0.8`,
  centered rounded-square SDF + V geometry on the inset box); source PNG regenerated and `npx tauri
  icon` re-ran the desktop icon set incl. `icon.icns`. Extraneous ios/android dirs `tauri icon` emits
  removed — desktop-only bundle.)*
- [x] E4. Docs: note the safe-area requirement so the branded asset (brand-owned) inherits it; do **not**
  write to `artifacts/brand/`. *(Safe-area note added to the generator header and the README Application
  icon section. `artifacts/brand/` untouched.)*
- [x] E5. Frontend test/snapshot: `shell()` no longer emits `.traffic`. *(`tests/shellChrome.test.mjs`
  asserts `titlebar()` renders brand/version, contains no `traffic` and no `<span>`, and escapes input.)*

## Cross-cutting (L2 gates)

- [ ] X1. Docs: update `README.md` + `RELEASE.md` for auto-import, env picker + mapping, icon safe-area,
  and a forward pointer to TASK-028 for app updates.
- [ ] X2. Compatibility/rollback note for SW-6: all new tables additive + idempotent `init_db`; reverting
  to TASK-026 leaves the new tables unused; no destructive migration.
- [ ] X3. Confirm no secret in any new IPC result / log / UI surface (SEC-010); confirm no new network
  egress host is introduced (F is split out).
- [ ] X4. `cargo test` + frontend tests green; manual macOS smoke per `design.md` §9.

## Out of scope → TASK-028 (F, DEC-029)

- App self-update: Tauri updater plugin, minisign signing, macOS code signing + notarization, GitHub
  Releases pipeline, new network egress host. Recommended split: Phase 1 lightweight version check
  (open download page); Phase 2 signed/notarized auto-install.
