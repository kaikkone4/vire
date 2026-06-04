# Salvage/Reuse Inventory — TASK-001 Repo/Path Assessment

- **Task:** TASK-001 / OpenSpec change `task-001-repo-path-assessment`
- **Tier:** L2 · **Gate context:** APP-005 controls (SEC-001, SEC-002, SEC-003, SEC-005, SEC-006, SEC-008)
- **Branch:** `feat/task-001-repo-path-assessment`
- **Type:** Read-only spike/assessment. No source, schema, config, or data changes were made under `src/`, `src-tauri/`, or `observability/`.
- **Date:** 2026-06-04
- **Author role:** Backend Developer (SW-2)

## Exit gate statement (up front)

This assessment produces a **clear salvage/reuse inventory** and **assumes neither a repo wipe nor a
repo reuse**. Both **replacement** and **reuse** remain open implementation-path options handed to
**TASK-003**. Every classification below is evidence-driven (file-level inspection), not momentum-driven.

## 1. Method and evidence base

Inspected the working tree on branch `feat/task-001-repo-path-assessment` (no modifications). Files read
in full for this inventory:

- Shell/config: `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`, `package.json`, `tsconfig.json`, `index.html`, `.gitignore`, `README.md`.
- Backend: `src-tauri/src/lib.rs` (full command + repo surface), `src-tauri/src/main.rs`.
- Frontend: `src/main.ts`, `src/html.ts`, `src/date.ts`, `src/forms.ts`.
- Tests: `src-tauri/src/lib.rs` `mod tests`, `src-tauri/tests/adversarial.rs`; frontend `tests/*.test.mjs` (date, forms, htmlEscape, langfuse healthcheck, pi-observe suites).
- Observability: `observability/pi-observe/bin/pi-observe.mjs`, `observability/pi-observe/README.md`, `observability/langfuse/{docker-compose.yml,README.md,.env.example}`, `scripts/*.sh`.

BA reference base: `artifacts/ba/04_technical_plan.md` (§3 path options, §4 component plan, §8 data model
13 entities, §10 export/CSV, §11 L2 controls), `artifacts/ba/11_security_review.md` (SEC-001…SEC-008,
APP-005 conditional pass), `artifacts/ba/05_project_plan_epics.md` (TASK-001 spike), and the
Langfuse-import-first AI-evidence decision (DEC-017, wiki `decisions/vire-uses-langfuse-first-ai-evidence-with-runtime.md`).

Classification vocabulary (per spec delta, exactly one per asset): `reuse-as-is`,
`reuse-with-changes`, `reference-only`, `retire/replace`.

## 2. Salvage/reuse inventory

### 2.1 Tauri shell

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| Tauri v2 config (`tauri.conf.json`) | `productName: Vire`, identifier `dev.vire.app`, frontendDist `../dist`, single window | `reuse-as-is` | Minimal, current v2 shell already matches the "review/export shell" role (04 §4 Review UI/CSV exporter). |
| Content-Security-Policy | `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' asset: data:; connect-src ipc: http://ipc.localhost` | `reuse-as-is` | CSP allows **no outbound HTTP** (only IPC) → privacy-aligned with SEC-002. The Langfuse importer (Boundary C) must add a controlled, backend-side outbound path; do not relax the webview CSP. |
| Capabilities (`capabilities/default.json`) | `core:default`, `dialog:default`, `dialog:allow-save` | `reuse-as-is` | Narrow permission set; save-dialog supports SEC-006 "write only to Janne-selected local path". |
| Plugin set (`tauri-plugin-dialog` 2.2) | `Cargo.toml` deps | `reuse-as-is` | Only dialog plugin present; no network/fs-broad plugins to retire. |

### 2.2 Frontend SPA

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| TS SPA + 5 views (Today, Projects, Manual Entry, Reports, Settings) | `src/main.ts` (render*/bind* per view) | `reuse-with-changes` | Viable review/export shell (04 §4), but the EPIC-004 review/approval workflow (approve/reject/split/merge/reclassify/mark-billable) and summary-only export UX are **absent**; current UI is a manual CRUD tracker. |
| `escapeHtml` XSS guard (`src/html.ts`) consistently applied via `esc(...)` | All interpolation in `main.ts` routes through `esc` | `reuse-as-is` | Strong, uniform output encoding for `& < > ' "`; covered by `tests/htmlEscape.test.mjs`. Carry forward into any reused UI. |
| Helpers `date.ts` (`localDateInputValue`), `forms.ts` (`optionalText`) | small pure functions | `reuse-as-is` | Generic, side-effect-free, unit-tested. |
| Manual-entry view (stopwatch-style start/end CRUD) | `renderManual`/`bindEntries` | `reference-only` | This is the generic-tracker surface the BA narrowed away from (01/04 §1). Keep as reference for the review UI's manual-correction affordances; do not assume it survives as a primary capture path. Decision routed to TASK-003/TASK-009. |

### 2.3 Rust backend (command + repo surface)

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| Repo-pattern functions (`create_project_repo`, `list_entries_repo`, `summary_repo`, `export_csv_repo`, etc.) | `lib.rs` | `reuse-with-changes` | Clean separation of repo logic from Tauri commands; fully parameterized SQL (`params![...]`, no string interpolation of values) → good SEC posture. Needs the evidence/review/summary/retention surface (04 §4/§8) that does not exist yet. |
| Tauri command layer + `AppState { db: Mutex<Connection> }` | `#[tauri::command]` handlers, `run()` | `reuse-with-changes` | Sound state/locking pattern; command set is tracker-shaped (projects/entries/summary/export) and must grow capture/import/review/retention commands. |
| Input validation (length caps, date/range parsing, active-project checks) | `validate_*`, `parse_duration`, `project_exists_active` | `reuse-as-is` (patterns) | Defensive validation with explicit error strings; reusable approach for the evidence model. |
| Error handling `CmdResult<T> = Result<T, String>` with user-facing messages | throughout | `reuse-as-is` (patterns) | Messages avoid leaking internals; aligns with SEC-002/003 "redacted error classes only". |

### 2.4 SQLite layer / persistence

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| `projects` table | `init_db` DDL | `reuse-with-changes` | Maps to BA `projects` (04 §8) but lacks customer/billing-type/mapping fields. |
| `time_entries` table (manual stopwatch rows) | `init_db` DDL | `reference-only` | **Not** an entity in the BA evidence model (04 §8). Manual-tracker divergence recorded as a **TASK-004 migrate-vs-retire decision input** — not migrated or retired here. |
| `settings` table (`capture_status=manual_mode_deferred`) | `init_db` DDL + `INSERT OR IGNORE` | `reuse-with-changes` | A generic key/value settings store is reusable; current single row reflects deferred capture, not the BA config surface. |
| `init_db` schema bootstrap (`CREATE TABLE IF NOT EXISTS …`) | `lib.rs:36` | `retire/replace` | No migration framework exists (idempotent create-if-not-exists only). The BA model needs reversible migrations (04 §8 lifecycle/retention). Net-new framework decision → TASK-004. |
| Index `idx_entries_date_project` | `init_db` | `reference-only` | Indexing pattern reusable; tied to `time_entries` whose fate is undecided. |

> SQLite engine choice itself (bundled `rusqlite`) is consistent with the BA "local SQLite store"
> direction (04 §4/§8) and the no-shortcuts production-target posture; the layer's **shape**, not the
> engine, is what needs to change.

### 2.5 Summary / export

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| `csv_formula_neutralized` (prefixes `=,+,-,@`, tab/CR/LF) | `lib.rs:133` | `reuse-as-is` | Directly satisfies SEC-006 formula-injection neutralization (04 §10). Tested in unit + `adversarial.rs`. |
| `csv_escape` (quote/comma/newline escaping, `"` doubling) | `lib.rs:142` | `reuse-as-is` | Correct RFC-style escaping; high-value reusable safety primitive (SEC-006, EPIC-005). |
| `export_csv_repo` + `validate_csv_destination` (.csv ext, not-a-dir, Janne-selected path) | `lib.rs:143`, `:169` | `reuse-with-changes` | Path-safety + escaping reusable, **but default emits raw per-entry rows** (`date,project,start,end,duration,note,…`). BA requires **reviewed-summary-only** default with no raw logs (04 §10, SEC-006). Default shape must change → TASK-010/TASK-011. |
| `summary_repo` (per-project duration totals) | `lib.rs:124` | `reuse-with-changes` | Aggregation pattern reusable; lacks human-approved/AI-cost/health columns the BA summary model needs (04 §8 `approved_summaries`). |

### 2.6 Tests

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| Rust unit tests (`lib.rs mod tests`, 8 tests) | CRUD/archive/active-filter, validation, summary+CSV escape, persistence-across-reopen, formula neutralization | `reuse-as-is` (patterns) | In-memory + tempfile fixtures, adversarial CSV payloads, persistence reopen. Strong patterns to carry into the evidence model (04 §12). |
| `src-tauri/tests/adversarial.rs` (3 tests) | archived-project historical edits, inverted-range rejection, formula-like name/note neutralization | `reuse-as-is` (patterns) | Good adversarial/privacy posture; reuse the style for capture/import/redaction tests. |
| Frontend `tests/*.test.mjs` (date, forms, htmlEscape) | node `--test` runner | `reuse-as-is` (patterns) | Lightweight, dependency-light; htmlEscape test exercises adversarial payloads. |
| `pi-observe` suites + `langfuse.compose.healthcheck.test.mjs` | lifecycle/security/adversarial/examples/healthcheck | `reference-only` | Exercise the observability tooling (see §2.7). Useful redaction/lifecycle test reference for TASK-006/007; tied to tooling whose role is undecided. |
| Test-fixture privacy posture | fixtures use synthetic names (`"A, Inc"`, `"Historical Client"`, `example.invalid`) | `reuse-as-is` | No real secrets/customer data in fixtures (04 §11, SEC-003). |

### 2.7 Observability tooling

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| `pi-observe` runtime wrapper (`bin/pi-observe.mjs`) | metadata-only event log, idle/orphan reconciliation, project resolution, credential redaction, loopback-only Langfuse gating | `reference-only` | Strong **TASK-006 runtime-reconciliation/health signal source** (local `events.jsonl` sessions, idle/orphan states). **Not** part of the Tauri runtime. See §4 DEC-017 tension — its emitter role is a decision input, not a reuse decision, here. |
| `pi-observe` redaction (`redact()`) + loopback gating (`isLoopbackLangfuseHost`) + data-only dotenv parser | `bin/pi-observe.mjs:32`, `:205`, `:51` | `reuse-as-is` (patterns) | Token/key/private-key/credential-URL/env-line redaction, loopback-only egress by default, no-shell-eval `.env` parser, session-ID hashing. Directly relevant to SEC-002/SEC-003 for the app-side importer. |
| Local Langfuse stack (`docker-compose.yml`, pinned `langfuse:3.63.0` + worker, Postgres 16, Redis 7, ClickHouse 24.8, MinIO) | `observability/langfuse/` | `reuse-as-is` (dev infra) | Local-only (binds `127.0.0.1`), pinned images → reproducible **TASK-007 validation environment** for import schema/time/usage/cost. Not shipped with the app. |
| Setup/up/down/smoke scripts (`scripts/*.sh`) | guarded setup, smoke ingest checks | `reuse-as-is` (dev infra) | Provide a repeatable local validation loop for TASK-007 without installing system deps silently. |

### 2.8 Privacy / security posture

| Asset | Evidence | Classification | Rationale + reference |
| --- | --- | --- | --- |
| Non-collection statements (README "Privacy status", Settings/Today capture banners) | `README.md`, `main.ts` `capture()`/`renderSettings()` | `reuse-as-is` | Explicit "no screenshots/keystrokes/active windows/idle/terminal/URLs/file contents" posture; aligns with SEC-001 deferred-capture and L2 transparency (04 §11, SEC-007). |
| `.gitignore` exclusions (`*.sqlite`, `*.db`, `observability/langfuse/.env`, `.env.*` with `!.env.example`) | `.gitignore` | `reuse-as-is` | Enforces no-DB/no-secret commits (SEC-003). Verified — see §5. |
| `.env.example` with **empty** secret fields | `observability/langfuse/.env.example` | `reuse-as-is` | Template only; all secret/password fields blank (SEC-003). Verified — see §5. |
| Deferred automatic capture ("Manual Mode") | README + `get_capture_status` command | `reference-only` | SEC-001 capture allowlist is **N/A today** (capture deferred); becomes greenfield in TASK-002/TASK-005. Recorded, not built. |

## 3. BA evidence data-model coverage (04 §8, 13 entities)

| BA entity | Status in current repo | Note |
| --- | --- | --- |
| `projects` | **partial** | `projects` table exists; missing customer/billing-type/mapping fields. |
| `project_mappings` | **absent** | No repo/window/Langfuse-environment mapping concept. |
| `raw_evidence` | **absent** | No capture path; nothing stored. |
| `capture_health` | **absent** | No permission/degraded/sampling state model. |
| `normalized_evidence` | **absent** | `time_entries` (manual) is **not** equivalent; it is a manual stopwatch row, not normalized activity/AI-trace evidence. |
| `ai_runtime_sessions` | **absent in app** | Conceptually approximated **only** by `pi-observe` local sessions (tooling, not the Tauri app). |
| `langfuse_import_runs` | **absent** | No importer, no cursor/checkpoint store; TASK-007 is greenfield. |
| `classification_suggestions` | **absent** | No classification engine. |
| `correction_history` | **absent** | No correction/learning store. |
| `review_states` | **absent** | No review/approval workflow. |
| `approved_summaries` | **absent** | `summary_repo` computes ad-hoc totals; no durable human-approved summary entity. |
| `export_records` | **absent** | `export_csv_repo` writes a file but records no audit row. |
| `retention_jobs` | **absent** | No retention/deletion lifecycle. |

**Coverage:** 1 partial (`projects`), 12 absent. The current schema is a generic-tracker shape; the
evidence-driven model (raw → normalized → runtime/import → suggestion → review → approved-summary →
export → retention) is essentially net-new. Recorded as TASK-004 input; **no migration performed here.**

## 4. Design tensions — decision inputs only (not resolved here)

1. **DEC-017 vs. `pi-observe` emitter role.** DEC-017 makes Langfuse **import** (REST pull, pagination,
   dedup, health states) the primary AI time/usage/cost path, limits local runtime observation to
   reconciliation/health (no duplicate cost/time ledger), and rules a new Vire-specific pi/Claude
   adapter **out of MVP** (04 §6/§7). The repo's `pi-observe` is a trace **emitter** (POSTs to the
   Langfuse ingestion API) — architecturally close to the deferred "adapter". **No importer exists.**
   *Routed to TASK-003 (emitter keep/retire/clarify) and TASK-006/007; not decided here.* Recommended
   framing: treat `pi-observe` as a TASK-006 health/reconciliation signal source, not as the AI-cost
   authority.
2. **`time_entries` migrate-vs-retire.** Manual stopwatch rows have no slot in the BA model. Decision →
   TASK-004. No migration framework exists today (`init_db` create-if-not-exists only).
3. **Network boundary placement.** The Langfuse importer (Boundary C, SEC-002) should run in the Rust
   backend so the locked webview CSP stays intact and the outbound allowlist is enforced server-side.
   Architectural note for TASK-007/TASK-012.
4. **Export default policy.** Reuse the escaping/neutralization primitives, but change the default
   export shape from raw per-entry rows to reviewed-summary-only (SEC-006, EPIC-005). → TASK-010/011.

## 5. APP-005 control coverage (L2: SEC-001, SEC-002, SEC-003, SEC-005, SEC-006, SEC-008)

| Control | Existing coverage | Gap for downstream |
| --- | --- | --- |
| **SEC-001** capture allowlist | N/A — capture deferred ("Manual Mode"); non-collection documented in README/Settings | Field-level allowlist is greenfield (TASK-002/005). |
| **SEC-002** network boundary / no raw egress | Webview CSP permits no outbound HTTP; **no network client in the app**; `pi-observe` (tooling) gates egress to loopback by default | App-side importer must add a controlled, backend-side Langfuse-only path with request-body assertions (TASK-007/012). |
| **SEC-003** credential handling | `.gitignore` excludes `.env`; `.env.example` secret fields empty; `pi-observe` redaction + data-only dotenv parser; no creds committed (verified §6) | App-side secure credential storage + log/export redaction is net-new (TASK-007/011/012). |
| **SEC-005** retention/deletion | None — no raw evidence stored yet | Net-new raw/normalized/approved lifecycle + retention audit (TASK-004/010). |
| **SEC-006** CSV safety | Formula neutralization + escaping implemented and tested (unit + adversarial) | Switch default to summary-only export; add export audit record (TASK-010/011). |
| **SEC-008** release integrity | None — no SBOM/signing/notarization tooling | Net-new release hardening (signing/notarization, SBOM, dep review) before any distribution beyond Janne's machine (TASK-012). |

The assessment change releases nothing, so APP-005 Gate D does not fire here; coverage is recorded so
the inventory is control-aware. (SEC-004 human-approval invariant and SEC-007 permission transparency
are out of this task's mandated L2 set but noted as downstream gates in 04 §11 / 11 §6.)

## 6. Secret-scan baseline and hygiene (no secrets printed)

Run on branch `feat/task-001-repo-path-assessment`. Commands reported filenames/counts only; no secret
values were printed or written.

- **Tracked DB/env artifacts:** none. `git ls-files` matched no `*.sqlite`, `*.db`, or `*.env` files;
  only `observability/langfuse/.env.example` is tracked.
- **`.gitignore` excludes confirmed present:** `*.sqlite`, `*.db`, `observability/langfuse/.env`
  (plus `.env.*` with a `!.env.example` allow).
- **`.env.example` secret fields:** all secret/password/key assignments are **empty** (0 non-empty
  matches for `LANGFUSE_SECRET_KEY`, `LANGFUSE_PUBLIC_KEY`, `*_PASSWORD`, `NEXTAUTH_SECRET`, `SALT`,
  `ENCRYPTION_KEY`).
- **Secret-pattern scan over tracked files** (`github_pat_`/`ghp_`/`sk-ant-`/`sk-proj-`/`xox[abprs]-`/
  `AKIA…`/`BEGIN … PRIVATE KEY`): 3 file matches — `observability/pi-observe/bin/pi-observe.mjs`,
  `tests/pi-observe.security.test.mjs`, `tests/pi-observe.test.mjs`. All three are **redaction-pattern
  definitions and their test fixtures** (the wrapper's `redact()` regexes and tests that prove
  redaction works), **not live credentials**. No real secret material is committed.

**Baseline verdict:** no committed credentials; DB/secret artifacts are correctly gitignored.

## 7. Gaps register (for downstream tasks)

- No outbound network client in the app (SEC-002 importer net-new) → TASK-007/012.
- No migration framework; schema is create-if-not-exists only → TASK-004.
- 12 of 13 BA evidence entities absent; `projects` partial → TASK-004.
- No review/approval workflow or approved-summary entity → TASK-008/009/010.
- Default export emits raw rows, not reviewed summaries → TASK-010/011.
- No SBOM/signing/notarization tooling (SEC-008) → TASK-012.
- `pi-observe` emitter vs. DEC-017 importer-first posture unresolved → TASK-003/006/007.

## 8. Tests / checks run for this assessment

- **Static inventory** of all Rust/TS/test/observability files (read-only; nothing executed against
  product code to preserve the non-mutating spike).
- **Secret-scan baseline + `.gitignore` hygiene** (§6) — passed.
- **Test suites were inventoried, not executed:** `node_modules/` and the Rust `target/` are not
  present in this read-only checkout, and the spike must not `npm install`/compile or otherwise mutate
  the tree. Test coverage is catalogued in §2.6 from source. Re-running `npm test` (Rust) and
  `npm run test:frontend` is a normal pre-merge step for any task that actually changes code.

## 9. Exit-gate confirmation

- A clear, evidence-driven salvage/reuse inventory is recorded (§2), with every asset carrying exactly
  one classification and a BA/EPIC/SEC reference.
- Data-model coverage is explicitly mapped: present / partial / absent (§3).
- APP-005 control coverage and gaps are recorded for the L2 set (§5).
- The DEC-017 tension is captured and **routed to TASK-003/006/007 without resolution** (§4).
- **No assumption of wipe or reuse:** both replacement and reuse remain open for **TASK-003**. No file
  under `src/`, `src-tauri/`, or `observability/` was created, modified, or deleted; no schema
  migration or data deletion was performed.

## 10. Artifact safety verification

This document was re-read after writing. It contains **no** Langfuse credentials or secret-shaped
values, **no** raw window/app titles, **no** prompt/response text, **no** terminal command bodies, and
**no** environment dumps. Secret-scan output was limited to filenames and counts (§6).
