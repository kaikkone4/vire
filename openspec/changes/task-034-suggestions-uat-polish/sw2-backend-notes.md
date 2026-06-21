# SW-2 Backend implementation notes — TASK-034 (Workstreams A + B)

Backend-led scope only. Frontend UI polish (A3 edit-panel End default, B5 Reports card cost line,
Workstream C copy) is **deferred** per the SW-2 instruction — only the backend DTO/type additions the
contract requires were made on the frontend. Workstream D was **verify-only** (no code change).

## What changed

### Workstream A — accept never stores a zero/negative span (DEC-034; day-end boundary DEC-035)
> **SW-4 escalation re-do (2026-06-21, DEC-035):** the original forward-bump-with-clamp design hard-
> errored at exactly `start == 23:59` (clamp → `start == end` → `parse_duration` rejects). Re-done to
> anchor the span on its **end** at the day's last minute, keeping the same-day `date + HH:MM` model.
- `src-tauri/src/lib.rs`
  - **A1** `bump_end_if_not_after` **replaced** by `normalize_same_minute_span(date, start, end,
    duration_minutes) -> Result<(String, String), String>` (placed next to `parse_duration`). Returns the
    `(start, end)` pair. When the derived `end` is **not strictly after** `start`, normalize to
    `minutes = duration_minutes.filter(|m| *m > 0).unwrap_or(1)`:
    - positive span (incl. an explicit user edit) → `(start, end)` untouched;
    - `start + minutes` same day → `(start, start + minutes)` (forward bump, as before);
    - `start + minutes` crosses midnight (i.e. `start == 23:59`) → anchor on the end:
      `(23:59 - minutes, 23:59)`, flooring the derived start at `00:00`. Realistic case `23:58 → 23:59`.
    A malformed `start`/`end` still surfaces its validation error (not silently rewritten). Uses the
    existing `chrono::NaiveDateTime` + `chrono::Duration::minutes`.
  - **A2** `accept_suggestion_repo` calls the helper after `validate_date`, before `parse_duration`, and
    **rebinds both** `start_time` and `end_time` from its result. `parse_duration` and the manual
    `create_entry_repo`/`update_entry_repo` paths are **unchanged** — manual entry still rejects
    `start == end` (human typo). Frontend echo (A3) is **not touched** in this re-do per the SW-2 scope
    (backend Workstream A only).

### Workstream B — AI cost reaches Reports and CSV (DEC-003 completion)
- `src-tauri/src/lib.rs`
  - **B1** `init_db`: idempotent `add_column_if_absent(time_entries, cost_total REAL)` +
    `(cost_currency TEXT)`. `TimeEntry` gains `cost_total: Option<f64>` / `cost_currency: Option<String>`;
    `get_entry` + `list_entries_repo` SELECTs and `row_to_entry` read the new columns. Manual paths
    write neither column → both stay `NULL` (absence ≠ zero).
  - **B2** `accept_suggestion_repo` INSERT carries `suggestion.cost_total` / `suggestion.cost_currency`
    verbatim onto the entry (cost is not user-editable — no field on `SuggestionEdit`).
  - **B3** `SummaryRow` gains `ai_cost_total: Option<f64>` / `ai_cost_currency: Option<String>`;
    `summary_repo` SQL adds `SUM(CASE WHEN origin='ai_suggested' THEN cost_total END)` (bare `SUM` →
    `NULL` when no AI cost, never 0) and `MAX(CASE WHEN origin='ai_suggested' THEN cost_currency END)`.
    `duration_minutes` / `ai_minutes` meanings are unchanged so prior numbers never shift.
  - **B4** `export_csv_repo` header + rows gain `cost_total,cost_currency` (between `origin` and
    `total_duration_hours`); empty cells when `NULL`; currency reuses `csv_escape`.
- `src/main.ts` (DTO/type additions only — no rendering)
  - `Summary` gains optional `ai_cost_total` / `ai_cost_currency`; `Entry` gains optional `origin` /
    `cost_total` / `cost_currency`, mirroring the backend DTOs for the deferred B5/C frontend work.

### Workstream D — inter-trace gap contract (verify-only, NO code)
- **D1** confirmed `src-tauri/src/suggestions/engine.rs:21` still reads `const GAP_MINUTES: i64 = 30;`
  (fixed gap, no settings/persistence/IPC/UI). `engine.rs` was **not** touched. Spec delta already
  states the fixed 30-min policy (design §7).
- **D2** confirmed `src-tauri/src/suggestions/tests.rs:145` `clustering_respects_the_gap_boundary` still
  pins the at/over/under-30-min boundary; passes. No new test required.

## Tests added (`src-tauri/src/lib.rs` unit tests)
- `accept_of_same_minute_block_bumps_end_and_never_stores_zero` — same-minute block (09:00:10→09:00:50,
  dur 1) accepts with end `09:01`, duration ≥ 1, `end > start`.
- `accept_of_same_minute_block_without_engine_duration_bumps_by_minimum_one` — same-minute, no engine
  duration → bumps by minimum 1.
- **`accept_of_day_end_same_minute_block_anchors_on_end_at_2359`** (DEC-035 re-do) — same-minute block at
  `2026-06-12 23:59:10→23:59:50`, dur 1 → stores `23:58 → 23:59`, duration 1, `end > start`; no zero span,
  no error, no midnight cross.
- `manual_entry_with_equal_start_and_end_is_still_rejected` — manual `start == end` still errors.
- `accept_copies_suggestion_cost_provenance_onto_the_entry` — accept copies `1.5/USD` onto the entry.
- `summary_and_csv_carry_ai_cost_separately_and_manual_cost_is_null` — summary `ai_cost_total` separate
  from human; manual entry cost `NULL`; a manual-only range keeps `ai_cost_total` `NULL`; CSV header +
  AI row carry cost/currency; manual row cost cells empty.
- "Normal accept unchanged" is covered by the pre-existing
  `accept_creates_exactly_one_ai_entry_marks_accepted_and_is_decided_once` (09:00→10:00, 60 min).

## Gate results (run on `feat/task-034-suggestions-uat-polish`, base = merged TASK-032 + TASK-033)
- **G1** `cargo test --lib` → **165 passed, 0 failed** (DEC-035 re-do adds the `23:59` day-end test).
  `cargo fmt --check` clean. `cargo clippy --lib --all-targets` → only pre-existing warnings at
  `lib.rs:1145` (app-data-dir `io_other_error`) and `lib.rs:1703` (AUTO_IMPORT `assertions_on_constants`
  test); **no new warnings** from this change (neither line is in the touched helper/call-site/test).
- **G2** `npm run build` (tsc + vite) → green. `npm run test:frontend` → **88 pass / 2 fail**. The 2
  failures are in `tests/pi-observe.security.test.mjs` (remote-Langfuse-host / dotenv-allowlist) and are
  **pre-existing & environmental** (Langfuse env contamination, same class flagged in the TASK-003 QA
  handoff): reproduced 8-pass/2-fail on the clean base with my code stashed; my-domain
  `tests/suggestionsUi.test.mjs` passes 10/10.
- **G3** `openspec validate task-034-suggestions-uat-polish --strict` → **valid (exit 0)**.

## Guarantees held
never-zero (A); absence ≠ zero — `NULL` cost stays `NULL`/"—" (B); AI ≠ human — AI cost summed
separately, never folded into the human total (B); no new egress / no secrets (only aggregate numbers
added; engine untouched); additive & reversible (idempotent `ADD COLUMN`).
</content>
</invoke>
