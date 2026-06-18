# QA Report — TASK-029 SW-3 post-fix re-run (commit 11c8e1c)

**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
**PR:** #23 (draft, base main)
**Tier:** L2
**Date:** 2026-06-18
**Verdict:** PASS — route to SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel

---

## Executive summary

Commit `11c8e1c` (fix(task-029): SW-2 SW-4 rerun — saturation terminal renderer + D-deep test) resolves both SW-4 blockers. The frontend renderer now correctly handles `instant_saturated` as a distinct terminal condition (not `reached_page_limit` rerun advice), and the Rust saturation test models D=50,000 using the live `MAX_PAGES × PAGE_LIMIT` constants (1,000 × 50) rather than a 1,000-trace proxy. All 142/142 Rust tests pass and 71/73 frontend tests pass; the 2 frontend failures are pre-existing `pi-observe.security.test.mjs` issues that also fail on `main` and are out of scope for this gate.

---

## SW-4 blocker resolution (prior FAIL → resolved)

| Blocker | Prior failure | Resolution |
|---|---|---|
| Frontend ignored `instant_saturated` | `import-report.ts` had no `instant_saturated` field; renderer never showed "capped" wording | `instant_saturated` added to `EnvImportLine` and `ImportReport` types; `envLine()` checks `env.instant_saturated` first for "capped" message; `renderImportReport()` excludes saturated envs from `ordinaryPageLimited` and renders a distinct `saturationNote` |
| Saturation test modeled D=1,000 | `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping` only generated 1,000 traces, not the actual D=50,000 threshold | Test now computes `d = MAX_PAGES * PAGE_LIMIT` (1,000 × 50 = 50,000) from live constants; `SaturatedInstantAscMock` generates a full `limit`-length page per call so run 1 sees ≥ D distinct same-instant traces |

---

## Changed paths (commit 11c8e1c)

- `src/import-report.ts` — `instant_saturated` field added to both exported types; renderer handles saturation-terminal distinct from page-limit continuation
- `tests/importReport.test.mjs` — 3 new saturation-focused tests: distinct capped terminal, mixed-run (both notes), and SEC-011 check on saturation output
- `src-tauri/src/langfuse/tests.rs` — `SaturatedInstantAscMock` reworked to generate MAX_PAGES × PAGE_LIMIT = 50,000 distinct traces; saturation assertions use live `d = MAX_PAGES * PAGE_LIMIT`
- `tasks.md` — status notes updated

---

## DEC-032 invariant verification

| DEC-032 requirement | Status | Evidence |
|---|---|---|
| `orderBy=timestamp.asc` wired through API trait | ✅ | `api.rs`, `importer.rs:TRACES_ORDER_BY`, URL assertion in `loopback_is_the_default_and_allowed_for_local` |
| Resume uses inclusive `fromTimestamp` cursor (`backfill_resume_from`) | ✅ | `store.rs`, `importer.rs`, confirmed in `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` |
| `instant_saturated` terminal distinct from `reached_page_limit` | ✅ | `importer.rs:83–92`, `import-report.ts:111,134,136`, confirmed in frontend and Rust saturation tests |
| Equal-timestamp block at boundary fully re-read on resume | ✅ | `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped` |
| Old exclusive-`toTimestamp` / `min_ts2` scheme removed | ✅ | No `set_backfill_resume_to` or `backfill_resume_to` in production or test code |
| Saturation test models real D=50,000 not a proxy | ✅ | `d = MAX_PAGES(1000) × PAGE_LIMIT(50) = 50,000`; mock generates full page per call; assertions use `vire1.traces_seen >= d` |
| Saturation diagnostic is secret-free (SEC-011) | ✅ | `import-report.ts`: flag-only, no timestamp value; test asserts against `sk-`, `pk-`, `Bearer`, `T00:`, `2026-` etc. |

---

## §8.5 replacement tests — all present and passing

| Test | Location | Status |
|---|---|---|
| `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` | `tests.rs:2177` | ✅ PASS |
| `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped` | `tests.rs:2338` | ✅ PASS |
| `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping` | `tests.rs:2482` | ✅ PASS — D=50,000 confirmed |
| `backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor` | `tests.rs:2554` | ✅ PASS |

---

## Frontend saturation tests — all new, all passing

| Test | File | Status |
|---|---|---|
| `a saturated single-instant run renders a distinct terminal/capped diagnostic, not "re-run to continue"` | `importReport.test.mjs` | ✅ PASS |
| `a mixed run (one env page-limited, another saturated) shows BOTH notes distinctly` | `importReport.test.mjs` | ✅ PASS |
| SEC-011 check on saturation output (no credential/timestamp leakage) | `importReport.test.mjs` (inline in saturation test) | ✅ PASS |

---

## Scenario coverage matrix

| Scenario | Coverage |
|---|---|
| Page-limited backfill resumes forward via inclusive cursor | `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` |
| Equal-timestamp block at boundary fully re-imported on resume | `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped` |
| Single instant ≥ D=50,000 surfaced as distinct terminal, not looped | `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping` |
| Millisecond/offset timestamps parsed; garbage excluded from cursor | `backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor` |
| `orderBy=timestamp.asc` in every trace request URL | `loopback_is_the_default_and_allowed_for_local` |
| Continuation persistence failure surfaces in-band | `continuation_boundary_persistence_failure_surfaces_in_band_not_a_false_resumable_claim` |
| Saturation diagnostic carries no timestamp value in report (SEC-011) | `backfill_saturation_diagnostic_and_cursor_carry_no_timestamp_value` + frontend saturation test |
| Frontend renders `instant_saturated` as distinct terminal (not re-run advice) | `a saturated single-instant run renders a distinct terminal/capped diagnostic` |
| Mixed run: both page-limit continuation and saturation surfaced independently | `a mixed run (one env page-limited, another saturated) shows BOTH notes distinctly` |
| All prior scenario coverage (pagination, health states, dedup, secret-free, etc.) | Unchanged — 69 langfuse lib tests, 73 other lib tests |

---

## Test runs

| Suite | Command | Result |
|---|---|---|
| Rust lib (langfuse) | `cargo test --lib langfuse` | **69/69 pass** |
| Rust lib (all) | `cargo test --lib` | **142/142 pass** |
| Frontend | `npm run test:frontend` | **71/73 pass** — 2 pre-existing failures in `pi-observe.security.test.mjs` (also fail on `main`; not DEC-032 scope) |

---

## Pre-existing frontend test failures (not a gate blocker)

`tests/pi-observe.security.test.mjs` — 2 tests fail on this branch AND on `main`:

- `safe dotenv parser loads only allowlisted Langfuse keys without shell execution`
- `remote Langfuse host is blocked unless explicitly opted in`

These are observability/pi-observe tests that pre-date TASK-029. DEC-032 made no changes to `pi-observe` or its tests. Both failures were documented and isolated in all prior QA runs.
