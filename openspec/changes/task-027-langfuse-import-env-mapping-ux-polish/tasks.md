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

- [ ] C1. Add a read-only discovery path: query `GET /api/public/traces` over the recent window
  **without** the `environment` filter; collect distinct non-empty `Trace.environment` values. Keep the
  URL allowlist + loopback gate intact (optional `environment`, still `/api/public/` + `validate_target`).
- [ ] C2. **VERIFY** the discovery assumption against the live stack (omitting `environment` returns
  cross-env traces and `environment` is populated on the list payload). If not, fall back to the trace/
  observation detail. **Stop and flag** if neither works.
- [ ] C3. Persist discovered environments (additive table/column, last-seen ts); run discovery as part
  of import.
- [ ] C4. Settings: replace/augment the free-text env CSV with a **picker** seeded from discovered
  environments (CSV stays as advanced fallback; default `vire` unchanged).
- [ ] C5. Tests: discovery collects distinct envs from a multi-env page; allowlist/loopback unchanged.

## Workstream D — Environment → Vire-project mapping  *(backend developer + frontend)*  *(after C)*

- [ ] D1. Add additive `langfuse_env_project_map(environment PK, project_id FK→projects, created_at,
  updated_at)` in `init_db` (`CREATE TABLE IF NOT EXISTS`).
- [ ] D2. IPC: `list_env_mappings`, `set_env_mapping`, `clear_env_mapping`; return discovered
  environments with mapping state (mapped / unmapped).
- [ ] D3. Associate imported evidence with a project via the map at **read time** (join) — no evidence
  rewrite. Vire project record stays authoritative (DEC-001).
- [ ] D4. Settings UI: per environment show mapped project / a project picker / a **"Create a project
  for `<env>`"** suggestion that calls the existing `create_project` then writes the map row (explicit
  user action only — DEC-006). No silent auto-create.
- [ ] D5. Tests: map persistence + idempotent init; suggest-create writes a row only on explicit action;
  evidence-to-project join.

## Workstream B — Automatic import  *(backend developer)*  *(after A)*

- [ ] B1. Run an import once **on app startup** (after `init_db` in `run()` setup, `lib.rs:330`), off
  the UI thread.
- [ ] B2. Run a **periodic background import** at a (configurable) interval via the same
  `run_blocking_import`; manual button preserved unchanged.
- [ ] B3. **Serialize** imports (in-progress guard/mutex) so auto and manual never overlap on the DB.
- [ ] B4. Respect `langfuse_enabled` + SEC-002 loopback identically to manual; disabled runs nothing.
  No new capability/CSP; **stop and flag** if one seems needed.
- [ ] B5. Tests: serialization guard prevents overlap; disabled short-circuit; failure lands in the
  health taxonomy, never zero.

## Workstream E — Desktop UX polish  *(frontend + assets)*  *(independent)*

- [ ] E1. Remove the fake `.traffic` cluster from `shell()` (`src/main.ts:35`) and the `.traffic` CSS
  (`src/style.css`); keep native window controls.
- [ ] E2. Re-balance `.titlebar` layout so brand/version still align without the dots.
- [ ] E3. Regenerate the placeholder icon mark with ~80% safe-area padding
  (`src-tauri/icons/source/generate-vire-mark.mjs`), re-run `npx tauri icon …`, regenerate
  `src-tauri/icons/*` incl. `icon.icns`.
- [ ] E4. Docs: note the safe-area requirement so the branded asset (brand-owned) inherits it; do **not**
  write to `artifacts/brand/`.
- [ ] E5. Frontend test/snapshot: `shell()` no longer emits `.traffic`.

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
