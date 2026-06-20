# Tasks — TASK-034 Suggestions UAT polish

Implemented as one change, three workstreams. Recommended order A → B → C. Each workstream is a commit.

## Workstream A — accept never stores zero (DEC-034; day-end boundary DEC-035)  *(backend-led)*

> Re-do after SW-4 escalation: the forward-clamp design failed at `start == 23:59` (clamp → `start==end`
> → `parse_duration` rejects → accept errors). Keep same-day model; anchor the span on its END at the
> day's last minute. See `arch-review.md` "SW-4 escalation resolution — DEC-035" and `design.md` §1.

- [x] A1. Add `normalize_same_minute_span(date, start, end, duration_minutes) -> Result<(String,String)>`
  helper in `lib.rs` (chrono `NaiveDateTime` + `Duration::minutes`). Positive span → unchanged; same-day
  bump → `(start, start+minutes)`; midnight cross → anchor on end: `(23:59 - minutes, 23:59)`, floor start
  at `00:00`. (Replaces the pair-less `bump_end_if_not_after`.) — done (`lib.rs:204`).
- [x] A2. Call it in `accept_suggestion_repo` after deriving start/end, before `parse_duration`, and
  rebind **both** `start_time` and `end_time` from its result; leave `parse_duration` and the manual
  `create_time_entry` path unchanged. — done (`lib.rs:613`).
- [x] A3. Frontend: in `suggestionRow` (`src/suggestions-ui.ts`), when start and end render equal compute
  `forward = addMinutesHHMM(start, n)`; if `forward !== start` set the End default to it; if it clamped
  (`forward === start`, day's last minute) set End to `start` and Start to `subMinutesHHMM(start, n)`. Add
  `subMinutesHHMM` (floor `00:00`); keep `addMinutesHHMM`. — done (`suggestions-ui.ts`: `subMinutesHHMM`
  added; `suggestionRow` mirrors both branches; stale `bump_end_if_not_after` comment updated to
  `normalize_same_minute_span`).
- [x] A4. Tests: Rust — same-minute accept stores `end > start`, duration ≥ 1, no edits; **`23:59`
  same-minute accept stores `23:58 → 23:59`** (not zero/error); normal accept unchanged; manual
  `start==end` still rejected. — Rust done (`accept_of_day_end_same_minute_block_anchors_on_end_at_2359` +
  existing 09:00/14:30/manual tests). Frontend — normal same-minute End default > Start; **`23:59`
  same-minute renders Start `23:58` / End `23:59`** — done (`tests/suggestionsUi.test.mjs`: kept the
  09:00 → `09:01` test, added the `23:59` day-end Start `23:58` / End `23:59` test + a `subMinutesHHMM`
  unit test). `suggestionsUi` suite 16/16 green; `npm run build` green.

## Workstream B — AI cost reaches Reports and CSV (DEC-003 completion)  *(backend-led)*

- [x] B1. `init_db`: idempotent `add_column_if_absent` for `time_entries.cost_total REAL` and
  `cost_currency TEXT`; extend `TimeEntry` + `row_to_entry`; manual path writes NULL.
- [x] B2. `accept_suggestion_repo`: copy `suggestion.cost_total`/`cost_currency` onto the new columns.
- [x] B3. `SummaryRow` + `summary_repo`: add `ai_cost_total` (+ `ai_cost_currency`) summed over
  `origin='ai_suggested'`; keep `duration_minutes`/`ai_minutes` meanings stable.
- [x] B4. `export_csv_repo`: add `cost_total,cost_currency` columns (empty when NULL); keep `origin`.
- [x] B5. Frontend: `summaryCards` (`src/main.ts`) shows AI cost on the AI-suggested line; "—" when null;
  extend `Summary` type.
- [x] B6. Tests: Rust — accept copies cost; summary separates AI cost; manual entry NULL AI cost; CSV has
  cost columns. Frontend — Reports card cost line + "—" absence.

## Workstream C — environment trackability explained (DEC-004 posture)  *(frontend)*

- [x] C1. `suggestions-ui.ts`: tighten `unmappedNotice` copy ("not trackable until mapped — Map in
  Settings").
- [x] C2. Surface the untimed "not auto-trackable — add time manually" state on the row (badge), not only
  inside the edit panel.
- [x] C3. Ensure `emptyState` names all causes (unmapped / untimed / source disabled-or-down / nothing
  imported) each with its action; reuse `sourceBanner()` on the view for disabled/down.
- [x] C4. Tests: each cause renders its named copy + action; no bare empty table; no "0".

## Workstream D — inter-trace gap contract correction (spec-only, NO code)  *(verification)*

- [x] D1. Confirm `engine.rs` still defines `const GAP_MINUTES: i64 = 30` (fixed gap, no config surface).
  Do **NOT** change `engine.rs`. The spec delta (MODIFIED "Imported AI evidence is aggregated into
  suggested time blocks") now states a *fixed* 30-minute gap, matching the code — see `design.md` §7.
- [x] D2. Confirm existing `suggestions/tests.rs:145` `clustering_respects_the_gap_boundary` still covers
  the at/over/under 30-min boundary; no new test required (the fixed-gap scenario maps to it).

## Gates (all workstreams)

- [x] G1. `cargo test` green; `cargo fmt --check` + `cargo clippy` clean.
- [x] G2. `npm run test:frontend` green; `npm run build` green.
- [x] G3. `openspec validate task-034-suggestions-uat-polish --strict` passes (only after TASK-032 is
  merged + archived to `openspec/specs/`; all four MODIFIED requirement headers must match the archived
  base spec exactly, incl. the gap requirement corrected in D1).
- [x] G4. Update `openspec/changes/task-034-suggestions-uat-polish/handoff.md` on completion.
