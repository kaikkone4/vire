<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-033 reports-quick-ranges

- **Change dir**: openspec/changes/task-033-reports-quick-ranges/
- **Branch / PR**: feat/task-033-reports-quick-ranges · PR #28 (draft)
- **Phase / gate**: SW-2 Frontend implementation (DONE) → ready for SW-3 QA
- **Tier**: L1-equivalent (frontend-only, no backend/deps/schema/IPC)

## Last gate result
SW-2 complete, 2026-06-20. Built on origin/main (task-032 already merged, PR #27). tsc+vite build PASS;
new pure-helper test 5/5 PASS. Implements all 3 arch-review constraints.

## Active blockers
- none

## Exact next action
sw-qa-engineer (SW-3): integration-test per `tasks.md` §4 (manual UAT, human-only) + verify the pure
helper. Confirm preset buttons set #rStart/#rEnd, refresh totals+table, preserve #rProject, and that CSV
export uses the preset window. No backend regression to check (no IPC/schema touched).

## Required files (read these, not the whole tree)
- `sw2-impl-notes.md` — what changed, constraints honoured, test/build results (PRIMARY)
- `arch-review.md` — 3 mandatory constraints + rejected alternatives
- `tasks.md` §4 — manual acceptance checklist
- `src/report-ranges.ts` (new pure helper) · `tests/reportRanges.test.mjs`
- `src/main.ts` `renderReports` (line ~60) — only render fn touched

## Notes carried forward
- Pre-existing test noise: `tests/pi-observe.security.test.mjs` has 2 failures on the clean tree
  (unrelated to TASK-033). `npm run test:frontend` will show them; not a SW-3 blocker for this change.
- TOP DEFECT RISK was UTC off-by-one — mitigated by local-date math; locked by month/year-boundary tests.
- Preset re-renders (`run(renderReports)`) so Export-CSV closure captures the new start/end.
- Presets: Last 7/14/30/90 days only. Calendar-aware presets explicitly out of scope (arch-review §4).
