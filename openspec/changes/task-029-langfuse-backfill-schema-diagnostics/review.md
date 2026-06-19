# Code review — TASK-029 SW-4 full recheck

**Verdict:** PASS  
**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`  
**Reviewed commits:** backend fix `11c8e1c`, QA recheck `0d6037e`

## Blocking issues

None.

## Prior blockers

1. **Resolved — saturation is represented and rendered as terminal.** The frontend contract now includes `instant_saturated` globally and per environment (`src/import-report.ts:34`, `src/import-report.ts:47`). Saturation takes precedence over ordinary page-limit continuation in each environment line (`src/import-report.ts:111`) and the global renderer emits a distinct terminal/capped note while calculating ordinary continuation only from non-saturated environments (`src/import-report.ts:133`, `src/import-report.ts:136`). The saturated-only and mixed-environment regressions are non-vacuous and verify suppression/preservation of the correct messages (`tests/importReport.test.mjs:112`, `tests/importReport.test.mjs:137`).

2. **Resolved — the saturation fixture models the real page depth.** The mock returns `limit` distinct trace IDs per page (`src-tauri/src/langfuse/tests.rs:2448`), walks all `MAX_PAGES`, and the test derives `D` from the production constants and asserts both seen and unique counts are at least `D` (`src-tauri/src/langfuse/tests.rs:2508`). It also verifies the parked cursor, bounded iteration, terminal state on rerun, and no false progress (`src-tauri/src/langfuse/tests.rs:2527`, `src-tauri/src/langfuse/tests.rs:2532`, `src-tauri/src/langfuse/tests.rs:2543`).

## DEC-032 verification

- Every trace-list request passes the fixed `timestamp.asc` order (`src-tauri/src/langfuse/importer.rs:904`, `src-tauri/src/langfuse/importer.rs:910`), and URL construction serializes it as `orderBy` (`src-tauri/src/langfuse/config.rs:248`).
- Page-limited runs derive the continuation cursor from the chronological maximum parseable timestamp returned (`src-tauri/src/langfuse/importer.rs:1044`). Backfill persists that cursor and resumes from it inclusively (`src-tauri/src/langfuse/importer.rs:524`, `src-tauri/src/langfuse/importer.rs:619`).
- Boundary-instant rereads are deduplicated by environment and trace ID before persistence (`src-tauri/src/langfuse/importer.rs:940`); the equal-timestamp regression proves the unread remainder is imported exactly once on rerun (`src-tauri/src/langfuse/tests.rs:2338`).
- Multiple page-limited environments use the earliest high-water, preventing a faster environment from causing a slower one to be skipped (`src-tauri/src/langfuse/importer.rs:563`, `src-tauri/src/langfuse/importer.rs:587`).
- Saturation parks the cursor rather than advancing past unread data and remains a distinct terminal state (`src-tauri/src/langfuse/importer.rs:610`, `src-tauri/src/langfuse/importer.rs:619`, `src-tauri/src/langfuse/importer.rs:632`).
- Continuation-store failures remain in-band failures rather than false resumability claims (`src-tauri/src/langfuse/importer.rs:626`, `src-tauri/src/langfuse/importer.rs:639`; regression at `src-tauri/src/langfuse/tests.rs:2687`).

## Suggestions

- Correct the ordinary continuation copy at `src/import-report.ts:140` and its assertions at `tests/importReport.test.mjs:100`: the ascending sweep resumes from the newest timestamp already reached and progresses toward newer history, not “from the oldest history” or “progressively further back.” The action (“re-run to continue”) is correct, so this is non-blocking.
- Align the proposal's data-model statement with the implementation. `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/proposal.md:132` says existing `langfuse_*` tables are unchanged, while the implementation and design add `langfuse_backfill_progress` (`src-tauri/src/langfuse/store.rs:58`; `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/design.md:248`).
- The parallel, untracked SW-5 artifact still describes the superseded exclusive resume-to/minimum-timestamp scheme (`openspec/changes/task-029-langfuse-backfill-schema-diagnostics/sec.md:57`, `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/sec.md:71`, `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/sec.md:82`). SW-5 should regenerate it against DEC-032 before release; SW-4 did not modify that owned artifact.

## Scope and craft

- The TASK-029 diff remains within the declared importer, settings, IPC, report/range UI, tests, and OpenSpec surfaces. No runtime-observer, environment-mapping, capability, dependency, updater, suggestion-engine, or renderer-network scope was added.
- Commit `11c8e1c` has a specific, complete message describing both fixes and their test evidence. QA commit `0d6037e` is appropriately scoped to `qa.md`.
- No dead code, commented-out implementation, or new unbounded loop was found in the reviewed continuation path.

## Verification

- `cargo test --lib langfuse`: 69 passed.
- `cargo test --lib`: 142 passed.
- Focused frontend report/settings tests: 23 passed.
- `npm run build`: passed.
- `cargo fmt --check`: passed.
- `cargo clippy --all-targets`: passed with six existing warnings; `-D warnings` fails on those baseline warnings.
- `openspec validate task-029-langfuse-backfill-schema-diagnostics --strict`: passed.
- `git diff --check 6f90661...HEAD`: passed.
