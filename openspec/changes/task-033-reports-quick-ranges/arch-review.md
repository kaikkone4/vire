# Architecture review — TASK-033 reports-quick-ranges

**Verdict: PASS** — design is ready for SW-2. Single-view, frontend-only, additive UX change; no
component-boundary crossing, no schema/IPC/NFR impact, no BA escalation. Not split-required (one new pure
helper module + wiring in one render function + one unit test, one PR).

## 1. Current behaviour (confirmed from code)

The Reports view is built by one function, `renderReports()` (`src/main.ts:59`). It renders a `.filters`
row with two date inputs `#rStart` / `#rEnd` (both default to `today()`), a `#rProject` select, an
**Apply** button, and an **Export CSV…** button. Apply re-runs `renderReports`, which reads the two input
values and calls `loadRange(start, end, pid)` → `list_time_entries` + `get_summary`; Export calls
`export_report_csv` with the same `start`/`end`/`pid` captured at render time.

There are no range presets today — every range is hand-typed into the two date inputs. The default
(today→today) and the per-input behaviour are otherwise fine.

## 2. Why this is frontend-only (backend already supports arbitrary ranges)

The three Reports data commands already take arbitrary `YYYY-MM-DD` start/end:

- `list_time_entries(start_date, end_date, project_id)` — `src-tauri/src/lib.rs:611`
- `get_summary(start_date, end_date, project_id)` — `src-tauri/src/lib.rs:640`
- `export_report_csv(start_date, end_date, project_id)` — `src-tauri/src/lib.rs:650`, which calls
  `validate_date_range` (`:181`) rejecting only `start > end`.

A preset just supplies a `{start, end}` pair the user could already type. So **no backend, schema, IPC,
or dependency change** is needed. An inclusive window always has `start ≤ end`, so it never trips
`validate_date_range` — no new error path is introduced.

## 3. Recommended design (SW-2 implements; do not implement here)

Two pieces, kept deliberately small:

**(a) A pure date-window module `src/report-ranges.ts`** — mirrors the `src/langfuse-settings.ts`
precedent (pure helpers, no DOM, no IPC, unit-tested):
- `REPORT_RANGE_PRESETS`: ordered read-only `{ key, label, days }` list for **Last 7 / 14 / 30 / 90
  days**.
- `reportRange(days, today = new Date()): { start, end }` returning an **inclusive** window ending today:
  `end = localDateInputValue(today)`, `start = localDateInputValue(today − (days − 1) days)`.

**(b) Button wiring in `renderReports()` only**: render a button row (`data-range="<key>"`) beside the
existing `.filters`; on click, set `#rStart.value`/`#rEnd.value` from `reportRange(days)` and trigger the
existing apply path (`run(renderReports)`). The existing inputs, project filter, Apply, and Export
controls stay exactly as they are.

Why this placement:
- It reuses the **one** existing query/export path. Presets are pure input convenience; nothing downstream
  changes. This is the least code and the lowest regression risk.
- Isolating the date math in a pure helper makes the only non-trivial logic (relative-window arithmetic)
  testable without a webview — the frontend suite has no DOM harness (the gap that hid TASK-030's
  `prompt()` no-op).

### Three constraints SW-2 must honour

1. **Inclusive window semantics.** "Last N days" = N calendar days ending today inclusive →
   `start = today − (N − 1)`. So "Last 7 days" = today + the previous 6 days. State this in code; it is the
   user-intuitive reading and guarantees `start ≤ end`.
2. **Local-date math, not UTC.** Compute by subtracting days on a *local* `Date`
   (`new Date(y, m, d - (days-1))`) and format with `localDateInputValue` (`src/date.ts`) — the same path
   `today()` uses. Do **not** use `toISOString().slice(0,10)`, which converts to UTC and shifts the date by
   a day for users behind/ahead of UTC near midnight. This is the single most likely defect; the unit test
   must include a month/year-boundary case to lock it.
3. **Preset must re-render so export stays consistent.** The Export handler in `renderReports` closes over
   `start`/`end`/`pid` captured at render time. A preset must set the inputs **and** trigger a re-render
   (`run(renderReports)`) so the next render captures the new dates and Export uses them. Do not set the
   inputs without re-rendering.

## 4. Alternatives considered and rejected

- **Reuse the Langfuse import-range model (`IMPORT_RANGE_PRESETS` / `since:`).** That model targets a
  persisted backend setting (`langfuse_import_range`) with a keyword/`since:date` vocabulary and a single
  cursor, not a concrete `{start, end}` pair for a two-input report filter. Reports needs an explicit end
  date and an inclusive start. Borrow the *pure-helper style*, not the data shape. Rejected as a direct
  reuse.
- **A `<select>` dropdown of ranges instead of buttons.** Workable, but the request is for one-click
  buttons; a row of small buttons is fewer interactions and matches the existing `.lf-form .row` button
  styling already in `style.css`. Minor; SW-2 may use a select if it reads better, but buttons are the
  recommendation.
- **Calendar-aware presets (This week / month / quarter / YTD).** Different math (month/quarter
  boundaries, not a fixed day count) and more UI. Out of scope for this change; note as a possible
  follow-up if Janne wants them. Not deferred work owed by this task.
- **Persisting the chosen preset across restarts.** Adds a settings/IPC surface for a convenience control.
  Over-engineered for this change. Rejected.

## 5. Edge cases SW-2 should honour

- **Boundary dates** (start crosses into the previous month/year): handled by `Date` arithmetic +
  `localDateInputValue`; covered by the required unit-test boundary case.
- **Empty result for the window**: the existing `summaryCards(..., 'No report data for this range.')`
  empty state already covers it — no new handling.
- **Active-preset highlight**: optional polish only; if the current inputs match a preset window it may be
  marked. Not required for acceptance.
- **Project filter**: a preset changes only the dates; `#rProject` is preserved by the existing re-render
  reading `#rProject.value`.

## 6. Architecture decisions / contracts

- No new ADR / `DEC-###` required. No data-model change vs `artifacts/ba/03_architecture_plan.md`
  (the Reports/summary capability and `APPROVED_SUMMARY`/`EXPORT_RECORD` model are untouched); no IPC
  command added or changed; no credential or egress surface touched (presets are client-side date math,
  rendered as plain HTML via the existing `esc()` path).
- No `feedback_to_ba[]` entry: this is additive UX convenience within the existing Reports component
  boundary (desktop UI), not a divergence from BA architecture.

## 7. NFR / security

- Performance: negligible — a few subtractions and a re-query the user could already trigger by typing.
- Privacy/security: unchanged — no new network, no new persistence, no new input that reaches the backend
  beyond the already-validated `start_date`/`end_date`/`project_id`. SEC posture identical to the existing
  Reports filters.
