# QA Report — TASK-021 `fix/task-021-langfuse-persist-failure-ipc-surfacing`

**Tier:** L2  
**Commit audited:** `99e1118`  
**PR:** #13  
**Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing`  
**Date:** 2026-06-12  

---

## Verdict: PASS

All scenarios have observable test coverage. `cargo test --lib` is green (40/40). No regressions introduced.

---

## Test execution

```
cargo test --lib
running 40 tests
… [all pass] …
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

---

## Scenario coverage matrix

| Scenario | Spec reference | Test | Result |
|---|---|---|---|
| Primary persist failure **and** marker insert both fail → secret-free in-band `Err` via `run_blocking_import` / `import_langfuse_now` | spec §"A persistence failure reaches the import command even when the failure-marker write also fails" | `persist_failure_surfaces_in_band_even_when_marker_write_also_fails` | PASS |
| Stale prior `healthy` snapshot is NOT returned as the surfacing channel when store is unwritable | same scenario, stale-healthy assertion | `persist_failure_surfaces_in_band_even_when_marker_write_also_fails` (asserts `after.health == "healthy"` proving DB snapshot is not the channel; `import_result()` returns `Err`) | PASS |
| Marker-succeeds path (trigger on evidence only) still records `Unknown` marker and snapshot is non-healthy | spec §"A failed write is visible, not swallowed" | pre-existing `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` | PASS |
| No new health state | proposal §"Unchanged surfaces" | no new `HealthState` variant in diff; `Unknown` reused | PASS |
| No schema change | proposal §"Unchanged surfaces" | no `ALTER`/`DROP` in diff; schema PRAGMA test unchanged | PASS |
| `import_result` keys on exact sentinel, not `health == Unknown` | tasks.md §2, proposal §"What Changes" | `unknown_when_response_is_indeterminate` (legitimately-persisted Indeterminate returns `Ok`, not `Err`) + new test trigger design | PASS |
| Absence-never-zero preserved | spec §absence-≠-zero | `absence_is_never_zero_cost_when_stack_down`, `time_only_trace_has_null_cost_not_zero` | PASS |
| No credential/secret exposure in surfaced error | spec §"Surfaced persistence errors carry no secrets"; SEC-003 | `persist_failure_surfaces_in_band_even_when_marker_write_also_fails` (needle scan: `sk-`, `Bearer`, `Authorization`, `password`, `token`, `canary`, `forced`, `RAISE`, `ABORT`) | PASS |
| No raw-activity egress | TASK-019 read-only posture | `importer_only_issues_read_calls` (unchanged) | PASS |
| Marker stays best-effort (`let _`) | proposal §"Keep the marker best-effort" | diff: `let _ = store::insert_import_run(conn, &marker)` line unchanged | PASS |
| `lib.rs` needs no edit — existing `?` propagates | tasks.md §3 | diff confirms `lib.rs` is not touched; `run_bounded(…)?` at lib.rs:212-214 propagates the new `Err` | PASS |
| Pre-existing CSV adversarial failure is unrelated and out of scope | tasks.md §5 note | `csv_export_neutralizes_formula_like_project_names_and_notes` fails identically on base commit `dd5d3b9` — CSV export, unrelated to TASK-021 | CONFIRMED UNRELATED |

---

## Clippy

```
cargo clippy --lib
warning: this `map_or` can be simplified  [importer.rs:182 — pre-existing TASK-020 code]
warning: unnecessary `if let` …           [importer.rs:305-316 — pre-existing TASK-020 code]
warning: unnecessary `if let` …           [importer.rs:315 — pre-existing TASK-020 code]
warning: this can be `std::io::Error::other`  [lib.rs:224 — pre-existing TASK-020 code]
warning: `vire` (lib) generated 4 warnings
```

All 4 warnings are in untouched TASK-020 code, identical to the base commit. The TASK-021 changes (`pub const PERSIST_FAILURE_MSG`, `import_result()` helper, doc comments, new test) introduce **zero new warnings**.

---

## Code correctness notes (L2 negative-path checks)

- **Sentinel key correctness:** `import_result()` compares `w == importer::PERSIST_FAILURE_MSG` (exact string equality), not `health == Unknown`. The existing `unknown_when_response_is_indeterminate` test confirms a legitimately-persisted Indeterminate run returns `Ok(())` from `import_result` — the two `Unknown` paths are correctly distinguished.
- **`import_result` visibility:** Declared `fn` (private to the `langfuse` module); tests call it as `super::import_result()` from the `langfuse::tests` child module — valid in Rust without exposing a public surface.
- **Regression test fault coverage:** Trigger installed on `langfuse_import_runs` aborts both the `persist_import_run` run-record insert (inside the atomic transaction) and the best-effort `insert_import_run` marker call — both failure paths exercised by a single trigger, matching the exact gap described in the proposal.
- **False-healthy prevention verified by design:** The test asserts `after.health == "healthy"` (proving the DB snapshot is stale) while simultaneously asserting `import_result(&summaries).is_err()` (proving the in-band channel fired). The two assertions together prove the in-band `Err`, not the snapshot, is the authoritative channel.

---

## Blockers

None.

## Non-blockers

- Pre-existing CSV adversarial test failure (`csv_export_neutralizes_formula_like_project_names_and_notes`) — out of scope for TASK-021; tracked separately.
- 4 pre-existing clippy warnings in TASK-020 code — out of scope for TASK-021.
