# QA Report — TASK-029 DEC-032 diagnostic re-run (SW-3 post-DEC-032 architect redesign)

**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
**PR:** #23 (draft, base main)
**Tier:** L2
**Date:** 2026-06-18
**Verdict:** FAIL — back to SW-2 (backend developer)

---

## Executive summary

The backend developer silently modified `importer.rs`, `api.rs`, `config.rs`, and `store.rs` for DEC-032
but **did not update `tests.rs`**. The Rust test suite **does not compile** (12 errors). Zero tests ran.
The old defective tests that §8.5 required to be deleted are still present and still assert the data-loss
behaviour. The four replacement tests required by §8.5 are completely absent. The blocker from the
previous SW-4 `review.md` remains unresolvable until the test suite compiles.

Production-code assessment (importer.rs / api.rs / store.rs): the DEC-032 mechanism appears to be
correctly implemented in non-test code — `TRACES_ORDER_BY = "timestamp.asc"`, inclusive
`backfill_resume_from` store API, `instant_saturated` terminal flag, saturation detection and distinct
terminal surfacing. Those changes cannot be verified because the suite does not compile.

---

## Blocker — test suite does not compile (12 errors)

`cargo test --lib` (cwd: `src-tauri/`) produces 12 compile errors, all in `tests.rs`. No tests ran.

### B-1 — Four mock `get_traces` implementations have wrong arity (4 errors)

`api.rs` added `order_by: &str` as the 7th parameter to the `LangfuseApi::get_traces` trait method.
Four mock implementations in `tests.rs` still declare only 6 parameters and do not accept `order_by`.

| Location | Mock type |
|---|---|
| `tests.rs:63` | `MockApi::get_traces` (the primary shared mock) |
| `tests.rs:2090` | Second mock for backfill tests |
| `tests.rs:2227` | `ThreeInstantDenseMock::get_traces` |
| `tests.rs:2337` | `SaturatedInstantMock::get_traces` |

Fix: add `_order_by: &str` as the 7th parameter to each implementation.

### B-2 — Seven calls to old store functions that no longer exist (7 errors)

`store.rs` was correctly updated: `backfill_resume_to`/`set_backfill_resume_to` were renamed to
`backfill_resume_from`/`set_backfill_resume_from`. `tests.rs` still calls the old names.

| Location | Old call |
|---|---|
| `tests.rs:2182` | `store::backfill_resume_to(&c)` |
| `tests.rs:2202` | `store::backfill_resume_to(&c)` |
| `tests.rs:2306` | `store::backfill_resume_to(&c)` |
| `tests.rs:2427` | `store::backfill_resume_to(&c)` |
| `tests.rs:2443` | `store::backfill_resume_to(&c)` |
| `tests.rs:2452` | `store::set_backfill_resume_to(&c, …)` |
| `tests.rs:2474` | `store::backfill_resume_to(&c)` |

Fix: replace all calls with `store::backfill_resume_from(&c)` / `store::set_backfill_resume_from(&c, …)`.

### B-3 — `ApiPath::Traces` initializer missing `order_by` field (1 error)

`tests.rs:618` constructs `ApiPath::Traces { … }` without the `order_by` field now required by the struct.

Fix: add `order_by: importer::TRACES_ORDER_BY` to the struct literal at `tests.rs:618`.

---

## Blocker — old defective tests not deleted (§8.5 violation)

Arch-review §8.5 explicitly required the following two tests to be **deleted** because they assert the
data-loss behaviour DEC-032 eliminates. Both are still present.

| Test | Location | What it still asserts |
|---|---|---|
| `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping` | `tests.rs:2405` | Comment at line 2432 says "resumes at `[floor, instant)` (exclusive) → the saturated instant is **skipped**"; asserts `older` history is reached by advancing the exclusive boundary past unread data. This is the exact data-loss the spec forbids. Also calls `store::backfill_resume_to` (doesn't compile — B-2). |
| `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears` | `tests.rs:2449` | Pre-seeds via `store::set_backfill_resume_to` (doesn't compile — B-2); asserts that the old exclusive-`to` boundary is preserved unchanged. Not meaningful under the inclusive-`from` cursor. |

Both must be deleted. They encode the defect, not the invariant.

---

## Blocker — four required §8.5 replacement tests absent

Arch-review §8.5 listed four replacement tests. None are present in `tests.rs`.

1. `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` — **ABSENT**
   Window > D across many instants; run 1 hits backstop, sets `resume_from = max_reached`; run 2
   resumes inclusive and imports strictly-newer history; asserts union = full source set, every trace
   imported exactly once.

2. `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped` — **ABSENT**
   A block of N (< D) traces sharing the boundary instant, only part fitting in run 1's page limit;
   run 2 (`from = boundary`, inclusive) re-reads the whole instant; asserts every equal-timestamp
   trace is imported exactly once. This is the direct regression for SW-4's unresolved blocker.

3. `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping` — **ABSENT**
   Mock instant with ≥ D traces at one timestamp; asserts (a) cursor not advanced past unread data,
   (b) `instant_saturated` flag distinct from `reached_page_limit`, (c) re-running does not falsely
   report convergence (bounded iteration).

4. `backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor` — **ABSENT**
   Millisecond/offset timestamp parses and advances cursor; a garbage value is imported but excluded
   from cursor; the all-unparseable degenerate does not restart from `now`.

---

## DEC-032 invariant verification against production code

Cannot be executed (test suite does not compile). Structural inspection of non-test code:

| DEC-032 requirement | Status | Evidence |
|---|---|---|
| `orderBy=timestamp.asc` wired through API trait | ✅ in production code | `api.rs:29`, `importer.rs:39`, `TRACES_ORDER_BY` constant, `get_traces` call at `importer.rs:902` |
| Resume uses inclusive `fromTimestamp` cursor (`backfill_resume_from`) | ✅ in production code | `store.rs:239–261`, `importer.rs:513,617–618` |
| `min_ts2` / `note_oldest_instants` / `page_limit_floor_ts` removed | ✅ | Only comment reference at `importer.rs:468`; no code definitions found |
| Saturation terminal: `instant_saturated` distinct from `reached_page_limit` | ✅ in production code | `importer.rs:83–92`, `importer.rs:1040–1075` |
| Saturation detection: `max_reached == resume_from` | ✅ in production code | `importer.rs:1045`, `importer.rs:608` |
| `toTimestamp` exclusive ceiling NOT used as the resume cursor | ✅ in production code | No `set_backfill_resume_to` call site in production code |
| Mock `get_traces` passes `order_by` | ❌ BROKEN | `tests.rs:64,2090,2227,2337` — wrong arity, do not compile |
| Old store API calls removed from tests | ❌ BROKEN | `tests.rs:2182,2202,2306,2427,2443,2452,2474` — don't compile |

---

## What was NOT run

- `cargo test --lib` — 0 tests; compile failed with 12 errors
- Frontend tests — not run; no UI/report wording changed in this DEC-032 attempt
- `openspec validate` — not re-run (spec itself unchanged per §8.6)

---

## Required actions for SW-2 (backend developer)

1. **Fix B-1** — Add `_order_by: &str` as 7th parameter to mock `get_traces` at `tests.rs:64`, `2090`, `2227`, `2337`.
2. **Fix B-2** — Replace `store::backfill_resume_to` → `store::backfill_resume_from` at `tests.rs:2182`, `2202`, `2306`, `2427`, `2443`, `2474`; replace `store::set_backfill_resume_to` → `store::set_backfill_resume_from` at `tests.rs:2452`.
3. **Fix B-3** — Add `order_by: importer::TRACES_ORDER_BY` to `ApiPath::Traces { … }` at `tests.rs:618`.
4. **Delete** `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping` (`tests.rs:2405`).
5. **Delete** `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears` (`tests.rs:2449`).
6. **Add** the four §8.5 replacement tests (`backfill_page_limited_resumes_forward_by_inclusive_from_cursor`, `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped`, `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping`, `backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor`).
7. Verify `cargo test --lib` compiles and all tests pass before returning to SW-3.

---

## Scenario coverage matrix (deferred — suite does not compile)

Scenario coverage cannot be assessed until the suite compiles and the §8.5 tests are added. All
coverage claims from prior QA runs are invalidated for the DEC-032 scope.
