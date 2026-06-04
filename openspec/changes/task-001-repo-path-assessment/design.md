# Design — TASK-001 Repo/path assessment

## Context

TASK-001 is the first Phase A spike. Its purpose is to replace assumption with evidence about the
existing repo before TASK-003 chooses an implementation path (existing Tauri shell + native helper
vs. Swift/AppKit-first vs. ActivityWatch import/reference). The BA exit gate is explicit: produce a
clear salvage/reuse inventory and assume **neither** wipe **nor** reuse.

The current repo is `vire v0.1`: a local-only Tauri v2 desktop time tracker with manual project and
time-entry CRUD, simple summaries, CSV export, and an explicitly **deferred** automatic-capture
posture ("Manual Mode"). Alongside the app, the repo carries a substantial local observability
toolchain (`pi-observe` wrapper + a pinned local Langfuse stack) that is **not** part of the Tauri
runtime but is directly relevant to the AI-evidence epics (EPIC-003).

## Goals / Non-goals

**Goals**
- Document what exists and its architectural fit against the BA evidence model and APP-005 controls.
- Classify each asset (reuse-as-is / reuse-with-changes / reference-only / retire-replace).
- Surface design tensions as decision inputs for downstream tasks.

**Non-goals**
- No implementation, migration, deletion, or rewrite.
- No implementation-path decision (that is TASK-003).
- No resolution of the DEC-017 / `pi-observe` tension (that is TASK-003/006/007).

## Preliminary salvage/reuse inventory (architect's read)

| Area | Asset | BA/EPIC/SEC fit | Classification | Note |
| --- | --- | --- | --- | --- |
| Shell | Tauri v2 config, capabilities, plugin-dialog | §4 review/export shell | reuse-as-is | CSP currently allows **no** outbound HTTP (`connect-src ipc:` only) — privacy-aligned; importer must add a controlled backend-side path. |
| Frontend | TS SPA, 5 views, `escapeHtml` XSS guard | §4 Review UI candidate | reuse-with-changes | Review/approval workflow (EPIC-004) and summary-only export UX are not present. |
| Backend | `lib.rs` repo functions + Tauri commands | §4 SQLite store | reuse-with-changes | Clean repo pattern, parameterized SQL; needs the evidence/review/summary/retention surface. |
| Data | SQLite `projects`, `time_entries`, `settings` | §8 data model | partial / decision needed | `projects` maps to BA `projects`; `time_entries` (manual) is **not** in the BA evidence model; 11+ BA entities absent. No migration framework. |
| Export | `export_csv_repo`, `csv_escape`, `csv_formula_neutralized` | SEC-006, EPIC-005 | reuse-with-changes | Escaping/neutralization is strong and tested → high reuse. Default exports **raw rows**; BA requires reviewed-summary-only default. |
| Tests | Rust unit + `adversarial.rs`; frontend `*.test.mjs`; `pi-observe` suites | §12 test strategy | reuse-as-is (patterns) | Good adversarial/privacy posture; fixtures avoid real secrets. |
| Tooling | `pi-observe` runtime wrapper | EPIC-003 / TASK-006 | reference / reclassify | Local runtime observation, idle/orphan reconciliation, redaction, loopback-only network gating — strong TASK-006 signal source. See tension below. |
| Tooling | Local Langfuse stack + scripts | TASK-007 validation env | reuse-as-is (dev infra) | Pinned `langfuse:3.63.0`; local-only; useful to validate TASK-007 import schema/cost. |
| Privacy | README/Settings non-collection statements, `.gitignore` (sqlite/db/.env) | SEC-001/002/003 | reuse-as-is | Posture documented and enforced; no credentials committed. |

## Key design tensions (decision inputs only)

1. **DEC-017 vs. `pi-observe` role.** DEC-017 mandates Langfuse **import** (REST pull, pagination,
   dedup, health states) as the primary AI time/usage/cost evidence path, restricts local runtime
   observation to **reconciliation/health only** (no duplicate time/cost ledger), and rules a new
   Vire-specific pi/Claude adapter **out of MVP**. The existing `pi-observe` is a trace **emitter**
   (it POSTs to Langfuse ingestion) — i.e. closer to the very "adapter" DEC-017 defers. There is
   **no** Langfuse importer in the repo; TASK-007 is greenfield. Recommendation: treat `pi-observe`
   as a **TASK-006 runtime-reconciliation/health signal source** (local `events.jsonl` sessions,
   idle states), and route its emission role to TASK-003 as a `feedback_to_ba[]` clarification.

2. **Evidence data model gap.** Current schema is a generic-tracker shape (manual `time_entries`).
   The BA model separates raw / normalized / runtime / import / suggestion / review / approved-summary
   / export / retention lifecycles (13 entities). TASK-004 must decide migrate-vs-retire for
   `time_entries` and introduce a migration framework (none exists today).

3. **Network boundary location.** The Langfuse importer (Boundary C, SEC-002) should run in the Rust
   backend, not the webview, so the locked CSP stays intact and the outbound allowlist is enforced
   server-side. Architectural note for TASK-007/TASK-012.

4. **Export default policy.** Reuse the escaping/neutralization, but change the default export shape
   from raw per-entry rows to reviewed-summary-only (SEC-006, EPIC-005).

## Risks / trade-offs

- Over-reusing the manual-tracker framing could re-import the "generic time tracker" scope the BA
  explicitly narrowed away from. The inventory must keep reuse evidence-driven, not momentum-driven.
- `pi-observe` is mature and tempting to wire in directly; doing so before TASK-003/006/007 would
  risk creating the duplicate AI-cost ledger DEC-017 forbids.

## Open questions → downstream

- TASK-003: existing-shell-reuse vs. rebuild, and `pi-observe` emitter disposition.
- TASK-004: `time_entries` migrate-vs-retire; migration framework choice.
- TASK-007: Langfuse importer is net-new; validate schema/cost using the local stack.
