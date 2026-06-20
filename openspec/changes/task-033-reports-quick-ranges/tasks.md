# Tasks — TASK-033 reports-quick-ranges

Owner: SW-2 (frontend). Design + constraints: `arch-review.md`. Do the minimum below; no extra refactors.

## 1. Pure date-window helper (`src/report-ranges.ts` — new)

- [ ] 1.1 Create `src/report-ranges.ts` with **no DOM and no IPC** (mirrors `src/langfuse-settings.ts`).
- [ ] 1.2 Export `REPORT_RANGE_PRESETS`: an ordered, read-only list of `{ key, label, days }` for
  **Last 7 / 14 / 30 / 90 days** (`days` = 7, 14, 30, 90; `label` = `Last N days`).
- [ ] 1.3 Export a pure `reportRange(days: number, today = new Date()): { start: string; end: string }`
  that returns an **inclusive** window ending today: `end = localDateInputValue(today)` and
  `start = localDateInputValue(today − (days − 1) days)`. Use **local-date** math — subtract days on a
  local `Date` (e.g. `new Date(y, m, d - (days-1))`) and format via `localDateInputValue` from
  `./date`. Do NOT use `toISOString().slice(0,10)` (UTC → off-by-one). `start ≤ end` must always hold.

## 2. Quick-range buttons in Reports (`src/main.ts`, `renderReports` only)

- [ ] 2.1 Import `REPORT_RANGE_PRESETS` and `reportRange` from `./report-ranges`.
- [ ] 2.2 In `renderReports()`, render a button row (each `data-range="<key>"`) near the existing
  `.filters` row, labelled per preset. Keep the `#rStart` / `#rEnd` inputs, `#rProject`, **Apply**, and
  **Export CSV…** controls exactly as they are.
- [ ] 2.3 Wire each preset button: on click compute `{ start, end } = reportRange(days)`, set
  `#rStart.value = start` and `#rEnd.value = end`, then trigger the existing apply path
  (`run(renderReports)`) so totals + entries refresh and the CSV-export handler closes over the new
  start/end. Preserve the current project filter (`#rProject`). Do NOT add any new IPC command.
- [ ] 2.4 (Optional, nice-to-have) Mark the active preset (e.g. `aria-pressed` / a class) when the
  current `#rStart`/`#rEnd` equal a preset window. Skip if it adds noticeable complexity.

## 3. Tests

- [ ] 3.1 Add `tests/reportRanges.test.mjs` (pure-helper style, like `tests/langfuseSettings.test.mjs`):
  with a fixed reference date, assert `reportRange(7|14|30|90)` returns the correct inclusive
  `{ start, end }` (end = reference day; start = reference − (N−1) days), and that `start ≤ end`. Include
  a month/year-boundary case (e.g. reference 2026-01-03, days 7 → start 2025-12-28).
- [ ] 3.2 Run `npm run test:frontend` and `npm run build` (tsc + vite). Both must pass.

## 4. Manual acceptance (packaged/dev macOS app — human-only)

- [ ] 4.1 In Reports, press **Last 7 days**: `#rStart`/`#rEnd` update to the 7-day inclusive window
  ending today and the totals + entry table refresh to that range.
- [ ] 4.2 Press **Last 30 days** then **Export CSV…**: the exported file covers the 30-day window.
- [ ] 4.3 After a preset, type a custom start/end and **Apply**: still works (presets did not disable
  manual entry).
- [ ] 4.4 With a project selected in the filter, pressing a preset keeps that project filter applied.

## Notes

- Backend is unchanged: `list_time_entries` / `get_summary` / `export_report_csv` already take arbitrary
  `start_date` / `end_date` and reject `start > end` (`src-tauri/src/lib.rs:181`). Inclusive windows always
  satisfy `start ≤ end`, so no new validation/error path is introduced.
- Keep all date math in the pure helper so it is testable without a webview (the frontend suite has no DOM
  harness). The button wiring in `renderReports` is verified by manual UAT, consistent with prior
  frontend tasks (TASK-030/031).
