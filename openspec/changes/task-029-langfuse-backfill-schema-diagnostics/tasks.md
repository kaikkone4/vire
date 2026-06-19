# Tasks — TASK-029

Four sequenced workstreams. **A first** (diagnose — it gates B), then **B** (widen the parser only for
proven shapes), then **C** (range + cursor + backfill), then **D** (grouped report UX). **No time-entry
suggestion is built here** — that is TASK-030 (see `proposal.md` *Out of scope* and `arch-review.md`
§Split). Role hints are for Pi-Assistant routing.

## Workstream A — Forensic, secret-free schema diagnostics  *(backend developer)*  *(gates B)*

- [x] A1. Add a fixed, secret-free `SkipReason` enum + `SkipReasonCount`/`SkipSample` types
  (`langfuse/model.rs`): `missing_trace_id`, `observations_not_embedded`, `field_type_mismatch`,
  `generation_lacks_usage_and_cost`, `observations_fetch_failed`. Classification is **structural** over
  the raw `serde_json::Value` — never a passed-through `serde` error string.
- [x] A2. Replace the per-trace repeated `warnings.push("a trace did not match the expected shape")`
  with the `SkipClassifier` (`langfuse/importer.rs`): aggregate counts per reason; keep at most a small
  fixed N (=3, `MAX_SAMPLES_PER_REASON`) **structural** samples per reason (top-level key names +
  offending field JSON type name only — no values/content).
- [x] A3. Thread `skip_reasons` + `skip_samples` onto `ImportSummary`/`EnvImportLine` and aggregate
  them as `total_skip_reasons` in `ImportReport::from_summaries`; dropped the repeated shape-skip
  warning strings (kept transport/auth/persist fixed warnings).
- [x] A4. **VF-1 CONFIRMED** (via safe deterministic test evidence — the live stack is not reachable
  from the backend agent and capturing raw bodies is itself forbidden by VF-1). Proof against the
  project's own `Trace` type: the v3 ID-string list shape is the **one** shape that fails the
  pre-widening `from_value::<Trace>` (`v3_idlist_parses=false`), while embedded-object and
  observation-less traces parse — the exact numeric fit of the field report (611 with-observations
  skipped, 29 observation-less duplicates). **Dominant reason = `observations_not_embedded`**, hypothesis
  confirmed; no credentials/bodies/prompt content captured. (Live count re-confirmation is a QA/Janne
  step, see VF-2.)
- [x] A5. Tests: `skip_reasons_aggregate_per_env_and_in_total`, `skip_samples_are_bounded_per_reason`,
  and **SEC-011 negative** `skip_diagnostics_are_secret_free` (serialized reasons/samples contain none
  of `sk-`/`pk-`/`Bearer`/`Authorization`/a session/`oat01`/a raw field value; the ID-list sample
  carries only `field_type="array"`, `element_type="string"`, never the ID value).

## Workstream B — Widen the v3 parser + decouple identification from usage  *(backend developer)*

- [x] B1. Made **trace identification total**: the import loop reads `id` structurally first
  (`model::trace_id`) and, when the strict typed parse fails on a peripheral field, falls back to
  `Trace::from_value_tolerant` (reads `id`/`timestamp`/`environment`/`sessionId` defensively) so a
  peripheral-field type mismatch never drops an identifiable trace. Only `missing_trace_id` still skips.
- [x] B2. Made `Trace.observations` tolerant of the v3 list shape via
  `#[serde(deserialize_with = "deserialize_tolerant_observations")]` (`model.rs`) — keeps embedded
  objects (fast path), ignores ID strings/non-objects → fetch fallback. Driven by the A4 finding
  (`observations_not_embedded`).
- [x] B3. `normalize_trace` reads usage/cost from fetched observations for the v3 list shape;
  `schema_changed` is now reserved for genuinely unreadable usage (`generation_lacks_usage_and_cost`,
  `observations_fetch_failed`) / unidentifiable (`missing_trace_id`) traces. Absence-≠-zero preserved
  (`None`, never `0`).
- [~] B4. **VF-2 (code-complete; live count is a QA/Janne step).** Proven via mock that previously-
  skipped traces now import as `healthy` (usage from the fetched observations) or `schema_changed`
  (genuinely unreadable) — **none silently dropped**. "new-trace count > 0 against the live stack" must
  be confirmed by QA/Janne on the real 9-env / 640-trace stack (the backend agent has no stack access).
- [x] B5. Tests: `identifiable_trace_with_idlist_observations_is_imported` (mock → imported, healthy,
  cost captured, `observations_not_embedded` informational); `generation_with_no_usage_anywhere_is_
  schema_changed_absence_preserved`; `peripheral_field_type_mismatch_still_imports_identifiable_trace`;
  `trace_with_no_id_is_missing_trace_id_counted_not_crashed`.

## Workstream C — Configurable range + incremental cursor + resumable backfill  *(backend developer)*

- [x] C1. Added `langfuse_import_range` to `settings` (`settings/mod.rs`):
  `last_7d|last_30d|last_90d|all|since:<RFC3339>`, default `last_30d`; settings-first resolve + validate
  (malformed `since:`/unknown → default; the parse error is fixed and secret-free, never echoes input).
  IPC `get_langfuse_import_range` / `set_langfuse_import_range`. `ImportRange` lives in `langfuse/mod.rs`.
- [x] C2. Replaced the fixed `recent_window(7)` import window with a **per-environment** resolver
  (`importer::run_import_with` + `incremental_window`): incremental `from = max(range_floor,
  cursor_ts − OVERLAP)` (`OVERLAP_SECS = 3600`, chronological compare), backfill `from = range_floor`,
  `to = now`. Cursor still never regresses.
- [x] C3. Discovery stays on a bounded `recent_window(DISCOVERY_WINDOW_DAYS=7)` (no full-history scan
  per import).
- [x] C4. Chunked **resumable atomic backfill** (`importer::run_backfill` + `backfill_chunks`): a wide
  range is an ordered sequence of bounded sub-windows (≥30d, ≤24 chunks), each run through the single-window
  engine and persisted atomically as its own run (S-3 + TASK-021 surfacing preserved). A window hitting the
  `MAX_PAGES` backstop sets `reached_page_limit` (surfaced on the report — no silent truncation).
  *(Chunk order flipped to **oldest→newest** under DEC-032 C4a.)*
  - [x] C4a. **DEC-032 REDESIGN (supersedes the SW-4 second-oldest-instant fix — `arch-review.md` §8).**
    Replace the exclusive-`toTimestamp` / `min_ts2` continuation with an **ascending `orderBy=timestamp.asc`
    sweep + inclusive `fromTimestamp` resume-cursor**: persist `resume_from = max_reached` (the chronological
    max parseable `timestamp` of a page-limited run) to `langfuse_backfill_progress`; `run_backfill` resumes
    at `fromTimestamp = resume_from` (**inclusive**), re-reading the **whole boundary instant** so
    equal-timestamp traces are never skipped (dedupe ⇒ exactly once, no tie-breaker needed); a clean run
    clears the cursor. Add `orderBy` to `ApiPath::Traces` + `get_traces` (fixed literal `timestamp.asc`; no
    new egress, GET-only). **Delete** `note_oldest_instants`/`min_ts`/`min_ts2`/`page_limit_floor_ts`.
    Single-instant saturation (`≥ D = MAX_PAGES×PAGE_LIMIT = 50 000` traces at one instant; `max_reached ==
    resume_from`) is a **distinct secret-free terminal diagnostic** (count, no timestamp value) — never a
    silent skip, never a loop. In-band `PERSIST_FAILURE_MSG` for store faults retained. See design §4.3 +
    `arch-review.md` §8.3–§8.4.
- [x] C5. IPC `backfill_langfuse_now` (backfill mode + larger `BACKFILL_TIMEOUT_SECS=300` bound);
  honours `langfuse_enabled` short-circuit, SEC-002 loopback (same `build_url`/`get_traces` path),
  `import_lock` serialization, off-UI dedicated thread — identical posture to `import_langfuse_now`.
- [~] C6. *(efficiency, deferred — ACKNOWLEDGED not hidden)* The windowed-observations-scan-by-`traceId`
  optimization is deferred; the per-trace `get_observations` N+1 cost at backfill scale is explicitly
  documented at the fetch site in `normalize_trace` and bounded by the chunked-resumable design (a
  timeout is non-destructive). Recommended as a TASK-029 follow-up.
- [x] C7. Tests: `incremental_window_resumes_from_cursor_floored_by_range`;
  `first_import_uses_range_floor_then_resumes_from_cursor` (mock window capture);
  `backfill_imports_history_in_chunks_and_is_resumable` (**VF-3**: multi-chunk, dedup convergence on
  re-run, cursor non-regressing, no duplicate rows); `backfill_reports_bounded_run_rather_than_
  truncating_silently`; `import_range_parses_validates_and_floors` (incl. malformed `since:`);
  `import_range_setting_persists_validates_and_defaults`.
  - [x] C7a. **DEC-032 REDESIGN tests (`arch-review.md` §8.5).** **Delete** the tests that encode the defect
    (`page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping`,
    `page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears`,
    `page_limited_backfill_resumes_below_boundary_on_rerun`, and the second-oldest-instant test). **Add:**
    (1) `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` (union over runs == full set, each
    trace exactly once — no skip, no dup); (2) `backfill_equal_timestamp_block_at_boundary_is_fully_
    reimported_not_skipped` (the direct equal-timestamp regression); (3) `backfill_single_instant_at_or_
    above_page_depth_is_surfaced_terminal_not_looping` (mock returns a **full `limit`-sized page** so the run
    models ≥ `D = MAX_PAGES×PAGE_LIMIT = 50 000` distinct same-instant traces — the real §8.4 premise, not a
    one-per-page proxy; cursor not advanced past unread, distinct saturation diagnostic, bounded-iteration
    proof of no loop); (4) `backfill_boundary_timestamp_is_robustly_parsed_
    else_imported_but_excluded_from_cursor`. **Retain** `cursor_advances_by_instant_not_lexically_across_
    offsets_and_precision`, `delayed_classification_and_cursor_compare_instants_across_offsets`, and the
    `cmp_ts` `Option<Ordering>` no-lexical-fallback guarantee. Update VF-3 test to the inclusive cursor.

## Workstream D — Grouped, actionable import summary  *(frontend developer)*

- [x] D1. Extended the report types (`ImportReport`/`EnvImportLine` + `SkipReasonCount`/`SkipSample`)
  with `skip_reasons`/`skip_samples`/`total_skip_reasons` and the `reached_page_limit` indicator.
  Moved out of `src/main.ts` into a pure, Tauri-free `src/import-report.ts` so the renderer is
  unit-testable; `main.ts` imports the types/renderer from there.
- [x] D2. Replaced `importReportLine()` with `renderImportReport()` (`src/import-report.ts`): per-env
  **seen / new / duplicate / skipped** + a grouped reason breakdown (one line per reason, e.g.
  *"611 observations not embedded"*) + a bounded structural-sample `<details>` (key/type names only) +
  an incremental-vs-backfill headline + a `reached_page_limit` "re-run to continue, nothing truncated"
  note; never the repeated per-trace string. `aria-live="polite"` preserved. All env/reason/sample
  text escaped (SEC-011). **(DEC-032)** The renderer also surfaces the `instant_saturated` single-instant
  **saturation terminal** as a *distinct* capped/terminal note — it suppresses the rerunnable
  "re-run to continue" continuation claim for the saturated environment (and per-env line), so a terminal
  cap is never presented as ordinary progress. Flag-driven and secret-free (no timestamp value, SEC-011).
- [x] D3. Settings: import-range control (`last_7d|last_30d|last_90d|all|custom since date`, default
  display `last_30d`) wired to `get_langfuse_import_range`/`set_langfuse_import_range`, plus a
  **"Backfill now"** button wired to `backfill_langfuse_now` alongside the kept "Import from Langfuse
  now"; both buttons show progress text and disable while in flight and honour the disabled-source
  short-circuit. Pure range-control helpers (`canonicalImportRange`/`parseImportRangeControl`/
  `importRangeLabel`/`IMPORT_RANGE_PRESETS`) in `src/langfuse-settings.ts`; report + range styling in
  `src/style.css`. No CSP/capability change, no renderer network calls.
- [x] D4. Tests: `tests/langfuseSettings.test.mjs` range-helper canonicalization + round-trip + label;
  `tests/importReport.test.mjs` — report groups reasons (no repeated-warning spam), shows per-env
  seen/new/dup/skipped, distinguishes backfill, surfaces the page-limit note, escapes injected text,
  and **SEC-011** asserts the rendered string carries no secret/prompt/value/`oat01`/`Bearer` material
  and samples show only key/type names. `tsc --noEmit` + `vite build` green; 21 focused frontend tests
  pass (11 new report + 10 settings incl. 7 new range).

## Cross-cutting

- [x] X1. `cargo test --lib` green (133 passed; 60 langfuse incl. 13 new); `cargo fmt` applied;
  `cargo clippy --all-targets` introduces **zero** net new warnings (baseline task-027 = 7, after
  TASK-029 = 7, all 7 pre-existing and outside this diff). *(Frontend `tsc`/tests are Workstream D.)*
- [x] X2. `openspec validate task-029-langfuse-backfill-schema-diagnostics --strict` passes.
- [x] X3. Confirmed: **no** new `HealthState` variant (only `SkipReason` added, a diagnostic enum);
  **no** new egress host (backfill/incremental/discovery all via `ImporterConfig::build_url` under
  `/api/public/*`, GET-only); diff touches only `langfuse/{model,importer,mod,tests}.rs`,
  `settings/mod.rs`, `lib.rs` — **no** change to capture, `runtime_observer`, the classifier,
  `env_mapping`, or CSV export.
- [ ] X4. RELEASE.md (SW-6): additive settings row, range-default 7d→30d behaviour note, partial-automated
  rollback — consistent with TASK-026/027 posture. *(SW-6 gate; not SW-2 backend scope.)*
