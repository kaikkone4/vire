# TASK-033 — Reports view: quick-range preset buttons (Last 7 / 14 / 30 days)

## Why

In the **Reports** view, choosing a date range today means typing both a start and an end date into two
`<input type="date">` controls (`#rStart`, `#rEnd`) and pressing **Apply** (`src/main.ts:59`,
`renderReports`). The most common reporting questions — "what did I do in the last week / two weeks /
month?" — therefore take several clicks and manual date arithmetic every time. Janne asked for quick
preset buttons (Last 7 days / 14 / 30, etc.) so a common range is one click.

## What changes

Add quick-range preset buttons to the Reports view that set the report's start and end dates to a
relative window ending **today** and refresh the report — **alongside**, not replacing, the existing
custom start/end inputs. A preset is a convenience that fills the same two dates the user could type by
hand; everything downstream (totals, entry table, CSV export) is unchanged and reuses the dates.

Concrete presets: **Last 7 days**, **Last 14 days**, **Last 30 days**, **Last 90 days**. Each is an
*inclusive* window — end = today, start = today minus (N − 1) days — so "Last 7 days" covers today plus
the previous six calendar days. The window is computed in **local** time, consistent with how the rest of
the app derives "today" (`localDateInputValue`, `src/date.ts`), so there is no UTC off-by-one near
midnight.

No backend, schema, IPC, or dependency change. The Reports data commands (`list_time_entries`,
`get_summary`, `export_report_csv`) already accept arbitrary `YYYY-MM-DD` start/end dates and validate
`start ≤ end` (`src-tauri/src/lib.rs:181,611,640,650`); presets only feed them values the user could
already enter. See `arch-review.md` for the recommended mechanism, the inclusive-window definition, and
rejected alternatives.

## Scope

In scope:
- A row of quick-range preset buttons in the Reports view (`src/main.ts` `renderReports`).
- A small **pure** date-window helper module (`src/report-ranges.ts`) that turns a preset into a concrete
  `{ start, end }` pair using local-date math — unit-testable, mirroring the `langfuse-settings.ts`
  pure-helper precedent.
- A unit test for the pure helper (`tests/reportRanges.test.mjs`).

Out of scope (explicitly not this change):
- Any backend, schema, IPC, credential, or CSV-format change (the existing commands already take the
  range).
- Calendar-aware presets ("This week", "This month", "This quarter", "Year to date"): a different math
  shape (month/quarter boundaries, not a fixed day count). Note as a possible follow-up; do not build here.
- Persisting the last-used preset across app restarts, or adding presets to the Manual Entry view.
- Changing the existing custom start/end inputs, the project filter, or the default range (still today).

## Owner

SW-2 (frontend developer) implements; SW Architect (this doc + `arch-review.md`) owns the design.

## Impact

- Affected code: `src/main.ts` (`renderReports` only) and a new `src/report-ranges.ts`.
- Affected specs: new capability `reports-quick-ranges` (this change).
- Tier: L1-equivalent — frontend-only, no new backend/egress/deps, no data-model change.
- Risk: low. Presets write into the existing two date inputs and trigger the existing Apply path; custom
  entry and CSV export are unchanged. The only correctness concern is the date math, which is isolated in
  a pure, tested helper.
