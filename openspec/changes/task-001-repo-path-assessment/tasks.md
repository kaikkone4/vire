# Tasks — TASK-001 Repo/path assessment

> Spike/assessment only. No source, schema, or config changes. Deliverable is the inventory
> artifact and gap/control-coverage map. Implementer: **backend-developer** (Rust/Tauri/SQLite
> primary), with **devops** consulted for build/release/observability tooling (SEC-008).

## 1. Frontend / shell inventory
- [x] 1.1 Inventory the Tauri v2 shell (`src-tauri/tauri.conf.json`, `capabilities/default.json`,
      `Cargo.toml`) and record the current CSP/network posture and plugin set.
- [x] 1.2 Inventory the TypeScript SPA (`src/main.ts`, `date.ts`, `forms.ts`, `html.ts`) and its
      views (Today, Projects, Manual Entry, Reports, Settings); note XSS-escaping coverage.
- [x] 1.3 Classify shell/UI reuse vs. the review/export shell role in `04_technical_plan.md` §4.

## 2. Backend / data inventory
- [x] 2.1 Inventory the Rust command surface and repo functions in `src-tauri/src/lib.rs`.
- [x] 2.2 Inventory the SQLite schema (`projects`, `time_entries`, `settings`) and persistence
      approach (`init_db`, no migration framework).
- [x] 2.3 Map current schema against the BA evidence data model (`04_technical_plan.md` §8,
      13 entities) and record which entities exist, partially exist, or are absent.
- [x] 2.4 Record the `time_entries` (manual stopwatch) divergence from the evidence-driven model
      as a TASK-004 decision input — do not migrate or retire here.

## 3. Summary / export inventory
- [x] 3.1 Inventory `summary_repo`, `export_csv_repo`, `csv_escape`, `csv_formula_neutralized`.
- [x] 3.2 Record reuse value of CSV escaping/formula neutralization against SEC-006/EPIC-005.
- [x] 3.3 Record the export-default divergence: current export emits raw per-entry rows;
      BA requires reviewed-summary-only defaults (no raw logs by default).

## 4. Tests inventory
- [x] 4.1 Inventory Rust unit tests (`lib.rs` `mod tests`) and `src-tauri/tests/adversarial.rs`.
- [x] 4.2 Inventory frontend `tests/*.test.mjs` (date, forms, htmlEscape) and the `pi-observe`
      test suites (lifecycle, security, adversarial, examples, langfuse healthcheck).
- [x] 4.3 Record reusable test patterns and the test-fixture privacy posture (no real secrets).

## 5. Observability / tooling inventory
- [x] 5.1 Inventory `observability/pi-observe` (runtime wrapper, idle/orphan reconciliation,
      project resolution, credential redaction, loopback-only Langfuse gating).
- [x] 5.2 Inventory the local Langfuse stack (`observability/langfuse`, `scripts/langfuse-*.sh`,
      `setup-local-observability.sh`) as TASK-007 validation infrastructure.
- [x] 5.3 Record the DEC-017 tension: `pi-observe` is a trace **emitter/adapter**, while DEC-017
      mandates Langfuse **import** as primary AI evidence and "no new pi/Claude adapter in MVP".
      Capture as TASK-003/TASK-006/TASK-007 input only — do not resolve here.

## 6. Privacy / security posture inventory
- [x] 6.1 Map existing coverage against APP-005 controls: SEC-001 (capture allowlist — N/A, capture
      deferred), SEC-002 (network boundary), SEC-003 (credential handling), SEC-005 (retention),
      SEC-006 (CSV safety), SEC-008 (release integrity).
- [x] 6.2 Run a secret-scan baseline over the repo and confirm no credentials are committed
      (verify `.gitignore` excludes `*.sqlite`, `*.db`, `observability/langfuse/.env`).
- [x] 6.3 Record the absence of an outbound network client and SBOM/signing/notarization tooling
      as known gaps for later tasks (SEC-002, SEC-008).

## 7. Deliverable
- [x] 7.1 Produce the salvage/reuse inventory artifact with per-asset classification, rationale,
      and BA/EPIC/SEC references.
- [x] 7.2 Confirm the exit gate: clear salvage/reuse inventory recorded with **no assumption of
      wipe or reuse**; replacement and reuse both remain open for TASK-003.
- [x] 7.3 Verify the artifact by re-reading it; ensure no credentials, raw titles, or secrets
      appear in any produced document.
