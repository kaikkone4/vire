# Tasks — TASK-034 Suggestions UAT polish

Implemented as one change, three workstreams. Recommended order A → B → C. Each workstream is a commit.

## Workstream A — accept never stores zero (DEC-034)  *(backend-led)*

- [ ] A1. Add `bump_end_if_not_after(date, start, end, duration_minutes)` helper in `lib.rs` (chrono
  `NaiveDateTime` + `Duration::minutes`; clamps a midnight-crossing bump to `23:59`).
- [ ] A2. Call it in `accept_suggestion_repo` after deriving start/end, before `parse_duration`; leave
  `parse_duration` and the manual `create_time_entry` path unchanged.
- [ ] A3. Frontend: in `suggestionRow` (`src/suggestions-ui.ts`), default the End input to
  `start + duration_minutes` when start and end render equal; add `addMinutesHHMM` helper.
- [ ] A4. Tests: Rust — same-minute accept stores `end > start`, duration ≥ 1, no edits; normal accept
  unchanged; manual `start==end` still rejected. Frontend — same-minute edit End default > Start.

## Workstream B — AI cost reaches Reports and CSV (DEC-003 completion)  *(backend-led)*

- [ ] B1. `init_db`: idempotent `add_column_if_absent` for `time_entries.cost_total REAL` and
  `cost_currency TEXT`; extend `TimeEntry` + `row_to_entry`; manual path writes NULL.
- [ ] B2. `accept_suggestion_repo`: copy `suggestion.cost_total`/`cost_currency` onto the new columns.
- [ ] B3. `SummaryRow` + `summary_repo`: add `ai_cost_total` (+ `ai_cost_currency`) summed over
  `origin='ai_suggested'`; keep `duration_minutes`/`ai_minutes` meanings stable.
- [ ] B4. `export_csv_repo`: add `cost_total,cost_currency` columns (empty when NULL); keep `origin`.
- [ ] B5. Frontend: `summaryCards` (`src/main.ts`) shows AI cost on the AI-suggested line; "—" when null;
  extend `Summary` type.
- [ ] B6. Tests: Rust — accept copies cost; summary separates AI cost; manual entry NULL AI cost; CSV has
  cost columns. Frontend — Reports card cost line + "—" absence.

## Workstream C — environment trackability explained (DEC-004 posture)  *(frontend)*

- [ ] C1. `suggestions-ui.ts`: tighten `unmappedNotice` copy ("not trackable until mapped — Map in
  Settings").
- [ ] C2. Surface the untimed "not auto-trackable — add time manually" state on the row (badge), not only
  inside the edit panel.
- [ ] C3. Ensure `emptyState` names all causes (unmapped / untimed / source disabled-or-down / nothing
  imported) each with its action; reuse `sourceBanner()` on the view for disabled/down.
- [ ] C4. Tests: each cause renders its named copy + action; no bare empty table; no "0".

## Workstream D — inter-trace gap contract correction (spec-only, NO code)  *(verification)*

- [ ] D1. Confirm `engine.rs` still defines `const GAP_MINUTES: i64 = 30` (fixed gap, no config surface).
  Do **NOT** change `engine.rs`. The spec delta (MODIFIED "Imported AI evidence is aggregated into
  suggested time blocks") now states a *fixed* 30-minute gap, matching the code — see `design.md` §7.
- [ ] D2. Confirm existing `suggestions/tests.rs:145` `clustering_respects_the_gap_boundary` still covers
  the at/over/under 30-min boundary; no new test required (the fixed-gap scenario maps to it).

## Gates (all workstreams)

- [ ] G1. `cargo test` green; `cargo fmt --check` + `cargo clippy` clean.
- [ ] G2. `npm run test:frontend` green; `npm run build` green.
- [ ] G3. `openspec validate task-034-suggestions-uat-polish --strict` passes (only after TASK-032 is
  merged + archived to `openspec/specs/`; all four MODIFIED requirement headers must match the archived
  base spec exactly, incl. the gap requirement corrected in D1).
- [ ] G4. Update `openspec/changes/task-034-suggestions-uat-polish/handoff.md` on completion.
