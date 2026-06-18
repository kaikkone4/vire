# QA Report — TASK-029 Langfuse Backfill + Schema Diagnostics

**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
**PR:** #23 (draft, base main)
**Tier:** L2
**Date:** 2026-06-18
**Verdict:** PASS (second re-run after SW-4 second-pass blocker fixes)

---

## SW-4 Second-Pass Blocker Fix Verification

Three blockers were raised in the second SW-4 pass and addressed in commit `242f670`.
The first-pass blockers (commit `4018a00`) remain verified and are included for completeness in §First-Pass Fixes below.

### Blocker 1 (refined) — Continuation boundary guarantees monotonic progress without skipping

**What was wrong:** The first-pass fix persisted the oldest instant reached as the boundary. Because
`toTimestamp` is exclusive, a re-run starting strictly below the oldest would re-read from that
exclusive ceiling — safe only if the oldest instant was fully drained. If the page backstop fired
mid-instant (equal-timestamp collision), the oldest instant was only partially read, so setting the
boundary to it would skip the un-drained portion. Two additional edge cases were also unaddressed:
a window that collapsed to a single saturated instant could loop indefinitely; a window that
produced no parseable timestamp at all would clear the boundary and falsely restart from `now`.

**Fix verified:**
- `note_oldest_instants` tracks the **two chronologically-oldest distinct instants** seen across the
  window. The continuation boundary is set to the **second-oldest distinct instant** (`min2`) — the
  oldest *fully-drained* one. The next re-run scans `[range_floor, boundary)` exclusively, which
  re-reads the oldest (possibly partially-read) instant via durable `(env, trace_id)` dedup without
  skipping any trace at the cut instant.
- **Equal-timestamp saturation fallback:** when the entire window is a single saturated instant
  (min2 is absent), the boundary falls back to that instant. The exclusive resume then scans
  strictly below it, making progress to older history. The unreachable same-instant excess is
  surfaced via `reached_page_limit` rather than silently retried.
- **No-usable-timestamp preservation:** a page-limited run that yields zero parseable timestamps
  neither advances nor clears the boundary — it preserves whatever was previously recorded. This
  prevents a false restart-from-`now` that would make prior progress permanently unreachable.
- `backfill_chunks` propagates the exact ceiling string (not a re-formatted DateTime) so a
  sub-second boundary such as `2026-05-10T12:30:00.500Z` is not truncated to second precision and
  thereby made to skip a 500ms slice.
- **New tests (3 for this blocker):**
  - `page_limited_backfill_boundary_is_second_oldest_instant_so_cut_instant_is_not_skipped` —
    three-trace source; confirms boundary is `t_mid` not `t_old`; confirms `t_old` would have
    skipped the cut instant.
  - `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping` —
    `SaturatedInstantMock` simulates a single instant with `u32::MAX` pages; run 1 sets boundary
    to that instant; run 2 scans exclusively below it, reaches older history, clears boundary.
    No infinite loop.
  - `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears` — pre-seeded
    boundary survives a page-limited run whose traces carry no parseable timestamp.

### Blocker 2 — Continuation persistence failures surface in-band

**What was wrong:** A failure to read, write, or clear the `langfuse_backfill_progress` table was
silently swallowed (treated as "no boundary" for reads, or ignored for writes/clears). The UI
could therefore truthfully claim durable resumability after a boundary operation that had in fact
failed. If the table itself was corrupted or dropped, a re-run would restart from `now` with no
indication that prior progress was lost.

**Fix verified:**
- `run_backfill` tracks a `continuation_failed` flag. Every store operation
  (`backfill_resume_to`, `set_backfill_resume_to`, `clear_backfill_resume_to`) is checked; an
  `Err` from any of them sets the flag rather than silently continuing.
- When `continuation_failed` is set, `flag_continuation_failure` injects the fixed, secret-free
  `PERSIST_FAILURE_MSG` sentinel into every environment's `warnings` vec. This is the same in-band
  sentinel used by regular import persistence failures (SEC-003 / TASK-021 S-4).
- `import_result` in `lib.rs` keys on `PERSIST_FAILURE_MSG` to collapse the run to an `Err` — the
  fault-independent IPC channel — so the UI/report cannot claim resumability after a failed
  boundary persist. The reported state is `Unknown` (never a stale healthy snapshot).
- **New test (1):**
  - `continuation_boundary_persistence_failure_surfaces_in_band_not_a_false_resumable_claim` —
    drops `langfuse_backfill_progress` table; runs a page-limiting backfill; asserts the
    `PERSIST_FAILURE_MSG` sentinel is present in warnings and `import_result` returns `Err`.

### Blocker 3 (strengthened) — No lexical fallback at all for unparseable timestamps

**What was wrong:** The first-pass `cmp_ts` returned `Ordering::Equal` (and previously lexical)
for unparseable inputs. Even returning `Equal` was unsafe: it could cause a malformed timestamp
string to hold its lexical position in cursor or delayed logic depending on context.

**Fix verified:**
- `cmp_ts` now returns `Option<Ordering>`: `Some(...)` when both sides parse; `None` when either
  is unparseable. There is **no lexical fallback whatsoever.**
- All cursor advancement, boundary comparison, delayed classification, and `is_stale` decisions
  treat a `None` result as "ignore" — unparseable timestamps are simply excluded from every
  ordering decision. The cursor never advances or regresses to a garbage timestamp. The delayed
  classification yields nothing for an unparseable trace timestamp.
- `later_ts` similarly skips unparseable sides rather than falling back to the string that happened
  to sort later as bytes.
- **New test (1):**
  - `malformed_trace_timestamp_is_ignored_for_cursor_and_delayed_not_lexically_ordered` — first
    import sets cursor to `09:30:00Z`; second import presents a trace with `"not-a-timestamp"`;
    asserts cursor stays at `09:30:00Z` and the garbage trace is not classified `Delayed`.

---

## SW-4 First-Pass Blocker Fix Verification (commit `4018a00`)

### First-Pass Blocker 1 — Page-limited backfill makes monotonic progress

The first-pass fix established the continuation-boundary mechanism: `run_backfill` reads a persisted
boundary (`store::backfill_resume_to`) and resumes at `[range_floor, boundary]` instead of
`[range_floor, now]`; a page-limited run records and persists `page_limit_floor_ts`; a clean run
clears the boundary. The UI wording was updated to "Each re-run resumes from the oldest history
already fetched, so repeated runs reach progressively further back." This mechanism is the
foundation the second-pass fix (above) refines to handle equal-timestamp and no-timestamp edge
cases.

**Tests:** `page_limited_backfill_resumes_below_boundary_on_rerun` — PASS.

### First-Pass Blocker 2 — Cursor and delayed comparisons parse DateTime<Utc>

The first-pass fix introduced `cmp_ts` as the single instant-comparison primitive routing all
cursor, max-ts, delayed, and `is_stale` comparisons through `DateTime::parse_from_rfc3339`. The
second-pass strengthens this to return `Option<Ordering>` with no fallback.

**Tests:** `cursor_advances_by_instant_not_lexically_across_offsets_and_precision`,
`delayed_classification_and_cursor_compare_instants_across_offsets` — both PASS.

---

## Test Suite Results

### Rust (`cargo test --lib`)

**141 passed · 0 failed · 0 ignored** (finished in ~0.17 s)

8 new tests added across two fix commits:
- **First-pass (3):** `page_limited_backfill_resumes_below_boundary_on_rerun`,
  `cursor_advances_by_instant_not_lexically_across_offsets_and_precision`,
  `delayed_classification_and_cursor_compare_instants_across_offsets`
- **Second-pass (5):** `page_limited_backfill_boundary_is_second_oldest_instant_so_cut_instant_is_not_skipped`,
  `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping`,
  `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears`,
  `continuation_boundary_persistence_failure_surfaces_in_band_not_a_false_resumable_claim`,
  `malformed_trace_timestamp_is_ignored_for_cursor_and_delayed_not_lexically_ordered`

Total up from 133 (SW-3 gate) → 136 (first fix) → **141 (second fix)**.

### Frontend unit tests (`node --import tsx --test tests/*.test.mjs`)

**69 passed · 2 failed · 0 cancelled**

| File | Pass | Fail | Notes |
|------|------|------|-------|
| `importReport.test.mjs` | 12 | 0 | Page-limit truthfulness test passes; `re-run to continue` wording verified |
| `langfuseSettings.test.mjs` | 10 | 0 | All range/settings tests |
| `pi-observe.security.test.mjs` | 8 | **2** | **PRE-EXISTING on `main`; unrelated to TASK-029** |
| Other (`date`, `forms`, `envMappingUi`, `htmlEscape`, `pi-observe.*`, `shellChrome`, etc.) | — | — | all passing |

**The 2 pi-observe.security failures are pre-existing on `main`** — `git diff main..HEAD --
tests/pi-observe.security.test.mjs` is empty (file unchanged on this branch). Out of scope.

### Frontend build (`tsc --noEmit` + `vite build`)

**PASS** — `tsc --noEmit` emits 0 errors; `vite build` completes cleanly.

### Formatting and static analysis

- `cargo fmt --check`: **PASS** (no output — tree is formatted).
- `cargo clippy --all-targets`: **6 unique warnings, 0 net-new from either fix commit.** All
  pre-existing: `importer.rs:910/920` (`if let` style, from SW-2 commit b30025ee),
  `lib.rs:859` (`std::io::Error::other`, pre-TASK-029), `tests.rs:1383/1423` (`Default::default`
  style, from TASK-027 f9acb651), `lib.rs:1414` (constant assert, from TASK-027 f9acb651).

### OpenSpec validate

`openspec validate task-029-langfuse-backfill-schema-diagnostics --strict` → **"Change is valid"**.

---

## Scenario Coverage Matrix

### Workstream A — Secret-free skip taxonomy + grouped diagnostics

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| A1 | `SkipReason` enum (5 variants), `SkipReasonCount`, `SkipSample` types in `model.rs` | `skip_reasons_aggregate_per_env_and_in_total` | PASS |
| A2 | `SkipClassifier` replaces per-trace warning spam; `MAX_SAMPLES_PER_REASON=3` cap | `skip_samples_are_bounded_per_reason` | PASS |
| A3 | `skip_reasons`/`skip_samples` threaded onto `ImportSummary`/`EnvImportLine`; `total_skip_reasons` aggregated | `skip_reasons_aggregate_per_env_and_in_total` | PASS |
| A4 | VF-1 confirmed via deterministic test (live stack not reachable from agent) | `identifiable_trace_with_idlist_observations_is_imported` | PASS |
| A5 | SEC-011: serialised reasons/samples carry no `sk-`/`pk-`/`Bearer`/`Authorization`/session/`oat01`/raw field value | `skip_diagnostics_are_secret_free` | PASS |

### Workstream B — Tolerant v3 parser + identification decoupled from usage

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| B1 | Identification-first: `trace_id(value)` + fallback to `from_value_tolerant`; only `missing_trace_id` still drops | `peripheral_field_type_mismatch_still_imports_identifiable_trace`, `trace_with_no_id_is_missing_trace_id_counted_not_crashed` | PASS |
| B2 | `deserialize_tolerant_observations`: keeps embedded objects, silently drops ID strings → fetch fallback | `identifiable_trace_with_idlist_observations_is_imported` | PASS |
| B3 | Usage/cost from fetched observations for v3 list shape; `schema_changed` reserved for genuinely unreadable; absence ≠ 0 | `generation_with_no_usage_anywhere_is_schema_changed_absence_preserved` | PASS |
| B4 | VF-2 code-complete (live count is a Janne/QA step — see §VF-2 smoke checklist below) | Mock-proven via B5 tests | PASS (code) |
| B5 | Full B-workstream mock suite | `identifiable_trace_with_idlist_observations_is_imported`, `generation_with_no_usage_anywhere_is_schema_changed_absence_preserved`, `peripheral_field_type_mismatch_still_imports_identifiable_trace`, `trace_with_no_id_is_missing_trace_id_counted_not_crashed` | PASS |

### Workstream C — Configurable range + incremental cursor + resumable backfill

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| C1 | `ImportRange` enum: `last_7d/last_30d/last_90d/all/since:<RFC3339>`; default `Last30d`; IPC `get/set_langfuse_import_range`; malformed `since:` falls back to default | `import_range_parses_validates_and_floors`, `import_range_setting_persists_validates_and_defaults` | PASS |
| C2 | Per-env incremental: `from = max(range_floor, cursor_ts − OVERLAP_SECS=3600)`; cursor never regresses | `incremental_window_resumes_from_cursor_floored_by_range`, `first_import_uses_range_floor_then_resumes_from_cursor` | PASS |
| C3 | Discovery bounded to `DISCOVERY_WINDOW_DAYS=7` (no full-history scan) | Code review confirmed | PASS |
| C4 | Chunked resumable atomic backfill: ordered newest→oldest sub-windows (≥30d, ≤24 chunks); `reached_page_limit` surfaced | `backfill_imports_history_in_chunks_and_is_resumable` (multi-chunk, dedup, cursor non-regressing), `backfill_reports_bounded_run_rather_than_truncating_silently` | PASS |
| C4a | **First-pass fix:** page-limited backfill persists durable continuation boundary; re-runs reach strictly-older history; clean run clears boundary; UI wording truthful | `page_limited_backfill_resumes_below_boundary_on_rerun` | **PASS** |
| C4b | **Second-pass fix (equal-timestamp):** boundary is second-oldest distinct instant; equal-instant saturation escapes via exclusive resume; no-timestamp run preserves boundary | `page_limited_backfill_boundary_is_second_oldest_instant_so_cut_instant_is_not_skipped`, `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping`, `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears` | **PASS (new ×3)** |
| C4c | **Second-pass fix (in-band persist faults):** store read/write/clear failures inject `PERSIST_FAILURE_MSG`; `import_result` collapses to `Err`; UI cannot claim false resumability | `continuation_boundary_persistence_failure_surfaces_in_band_not_a_false_resumable_claim` | **PASS (new)** |
| C5 | `backfill_langfuse_now` IPC: `import_lock` serialised, dedicated off-UI thread, SEC-002 loopback, `BACKFILL_TIMEOUT_SECS=300` | Code review: `backfill_langfuse_now` in `lib.rs` confirmed | PASS |
| C6 | N+1 fetch cost documented and bounded by chunked design (deferred optimisation, acknowledged) | `// N+1 cost` comment at `normalize_trace` fetch site | ACKNOWLEDGED |
| C7 | Full C-workstream test suite | Tests C4–C4c above | PASS |
| C7a | **First-pass fix:** cursor/delayed comparisons use `DateTime<Utc>` (not lexical); mixed offset + fractional precision regression tests | `cursor_advances_by_instant_not_lexically_across_offsets_and_precision`, `delayed_classification_and_cursor_compare_instants_across_offsets` | **PASS** |
| C7b | **Second-pass fix (strict no-fallback):** `cmp_ts` returns `Option<Ordering>` — malformed timestamps are ignored entirely; no lexical fallback even for comparisons involving unparseable inputs | `malformed_trace_timestamp_is_ignored_for_cursor_and_delayed_not_lexically_ordered` | **PASS (new)** |

**Deduplication:** `store::seen_trace_ids` loads persisted trace IDs per env before each run; `INSERT OR IGNORE` on `(environment, trace_id)` PK; backfill re-run confirmed idempotent (VF-3 via `backfill_imports_history_in_chunks_and_is_resumable`).

### Workstream D — Grouped import summary + range/backfill UI

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| D1 | `ImportReport`/`EnvImportLine` extended; `import-report.ts` decoupled from `main.ts` | `null report renders nothing`, `headline reports new traces` | PASS |
| D2 | `renderImportReport`: per-env seen/new/dup/skipped; grouped reason breakdown (one line/reason); bounded `<details>` sample; backfill vs incremental headline; `reached_page_limit` note (truthful — both fix passes); no repeated per-trace string; `aria-live="polite"`; all text escaped | `reasons are GROUPED`, `each environment shows seen/new/duplicate/skipped`, `structural samples show key/type NAMES only`, `backfill headline is distinguished`, `a page-limit run says so and the claim is truthful`, `environment names/reasons/sample keys are escaped`, `SEC-011` | PASS |
| D3 | Range control: preset picker + custom date → `set_langfuse_import_range`; "Backfill now" button → `backfill_langfuse_now`; both disable while in-flight, honour disabled-source short-circuit; no CSP/capability change | `the default range is last_30d`, `keyword presets canonicalize`, `a custom date canonicalizes`, `canonical → control → canonical round-trips`, `Test connection is blocked while disabled` | PASS |
| D4 | 22 focused frontend tests: 12 `importReport.test.mjs` + 10 `langfuseSettings.test.mjs`; `tsc --noEmit` + `vite build` green | All 22 pass | PASS |

### Cross-cutting

| Task | Criterion | Result |
|------|-----------|--------|
| X1 | `cargo test --lib`: 141 passed; `cargo fmt --check` clean; `cargo clippy` zero net-new warnings | PASS |
| X2 | `openspec validate --strict` | PASS |
| X3 | No new `HealthState` variant; no new egress host; diff in scope (`langfuse/*`, `settings/mod.rs`, `lib.rs`, `src/*.ts`); no change to capture/runtime_observer/classifier/env_mapping/CSV | PASS |
| X4 | RELEASE.md (SW-6 gate scope — not SW-3) | OUT OF SCOPE |

**Security checks (second-pass fix additions):**
- `PERSIST_FAILURE_MSG` is a fixed, credential-free string constant (`"importer could not persist
  this run to the local store; recorded state is unknown"`). The raw SQLite driver error is never
  interpolated; the `sk-leak-canary` trigger regression test (`persist_failure_surfaces_in_band_even_when_marker_write_also_fails`) confirms no secret value leaks through even when the marker
  insert also fails under the same fault.
- `cmp_ts` returns `None` for unparseable inputs and the calling code's `is_none_or` / `==
  Some(...)` guards ensure unparseable values are simply skipped — they are never logged, surfaced
  in a report, or ordered lexicographically.
- `langfuse_backfill_progress` table contains `marker TEXT` (fixed key `"resume_to"`) +
  `resume_to TEXT` (UTC RFC3339 position timestamp) + `updated_at TEXT` — position marker only,
  no trace content, no credential.
- No `tauri.conf.json` changes; no new CSP; no new Tauri capability files; no renderer network
  calls; no new egress host.

---

## Known Pre-existing Failures (Unrelated to TASK-029)

| Test | File | Classification |
|------|------|----------------|
| `safe dotenv parser loads only allowlisted Langfuse keys without shell execution` | `pi-observe.security.test.mjs:50` | Pre-existing on `main` — file unchanged on this branch (`git diff main..HEAD` empty). Out of scope. |
| `remote Langfuse host is blocked unless explicitly opted in` | `pi-observe.security.test.mjs:82` | Pre-existing on `main` — file unchanged on this branch. Out of scope. |

---

## VF-2 Manual Smoke Checklist (Janne's Real 9-env / 640-trace Stack)

Run these against the live Langfuse stack after deploying/building from `feat/task-029-langfuse-backfill-schema-diagnostics`.

**Setup**
- [ ] Build and launch the desktop app from this branch.
- [ ] Navigate to **Settings → AI evidence source (Langfuse)**.

**Import range**
- [ ] Range picker shows **"Last 30 days"** as the selected default.
- [ ] Change to **"Last 90 days"**, save — range label updates to "Last 90 days".
- [ ] Change back to **"Last 30 days"**, save.

**Incremental import (VF-2 core)**
- [ ] Click **"Import from Langfuse now"**.
- [ ] Verify `new_traces > 0` across environments in the report (previously was 0/640).
- [ ] Report shows **grouped reasons** (e.g. "611 observations not embedded"), NOT one warning line per trace.
- [ ] `seen / new / duplicate / skipped` counts shown per environment.
- [ ] No `sk-`/`pk-`/`Bearer`/session/credential material visible in the report UI.

**Tolerant import verification**
- [ ] Traces previously skipped with `observations_not_embedded` now appear as `healthy` or `schema_changed` (not silently dropped).
- [ ] Structural sample `<details>` block (if present) shows only key/type names, not observation IDs or values.

**Backfill**
- [ ] Click **"Backfill now"** — button disables and shows "Backfilling…" while in progress.
- [ ] Backfill completes without error; report headline says "Backfill run".
- [ ] Re-run "Import from Langfuse now" — no duplicate traces (new count = 0 for already-imported range, duplicates count ≥ 0 and stable).

**Page-limit check (optional — if a wide range with many traces)**
- [ ] If a run hits the page limit, report shows the "re-run to continue" note.
- [ ] Re-running "Backfill now" reaches older traces not seen in the first run (monotonic progress — guaranteed by second-oldest-distinct-instant boundary; confirmed by regression tests C4b).
- [ ] A second re-run does not re-import equal-timestamp traces from the cut instant (dedup suppresses them).

**Negative paths**
- [ ] With Langfuse integration **disabled**: both "Import now" and "Backfill now" buttons appear as `disabled`.
- [ ] Malformed custom `since:` date (e.g. "not-a-date") in range picker falls back to default rather than crashing.

---

## Gate Verdict

**PASS → route to SW-4 (Code Reviewer) ∥ SW-5 (Security Agent)**

All three second-pass blockers are resolved and verified:
1. Continuation boundary uses the second-oldest distinct instant, eliminating equal-timestamp
   skipping; saturated single-instant windows escape via exclusive resume without looping;
   no-usable-timestamp runs preserve the existing boundary without false restart.
2. Continuation persistence failures (read/write/clear) inject `PERSIST_FAILURE_MSG` in-band so
   `import_result` collapses to `Err`; the UI cannot claim durable resumability after a failed
   boundary operation.
3. `cmp_ts` returns `Option<Ordering>` with no lexical fallback; malformed timestamps are ignored
   for all cursor, delayed, and continuation decisions.

141 Rust tests (8 new total across both fix passes) and 22 frontend tests all pass. Build clean.
`openspec validate --strict` passes. `cargo fmt` clean. Zero net-new clippy warnings.
Pre-existing pi-observe failures are isolated to `main` (file unchanged on this branch).
VF-2 live-stack count confirmation is a Janne/QA step (see smoke checklist above).
