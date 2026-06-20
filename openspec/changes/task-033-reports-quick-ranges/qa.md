# QA Report — TASK-033 reports-quick-ranges

**Gate**: SW-3  **Verdict**: PASS  **Date**: 2026-06-20  **Tier**: L1 (frontend-only)

## Scenario coverage matrix

| # | Scenario (from spec.md) | Coverage type | Result |
|---|---|---|---|
| S1 | Last 7 days → 7-day inclusive window ending today | Unit: `reportRanges.test.mjs` test 2 (fixed ref 2026-06-14→2026-06-20); Manual UAT §4.1 (human-only) | COVERED |
| S2 | Last 14/30/90 days → correct inclusive windows | Unit: test 2 (exact dates asserted per preset); test 3 (invariant: `start ≤ end`, inclusive day-count for all 4 presets) | COVERED |
| S3 | `start ≤ end` always holds (valid range, no error) | Unit: test 3 asserts the invariant across all presets | COVERED |
| S4 | Window near month/year boundary correct in local time | Unit: test 4 — `reportRange(7, 2026-01-03)` → `2025-12-28`..`2026-01-03`, crossing year boundary | COVERED |
| S5 | Preset keeps selected project filter | Code: `pid` re-read from `#rProject` at top of `renderReports`; preset click triggers `run(renderReports)` without touching `#rProject`; Manual UAT §4.4 (human-only) | COVERED |
| S6 | CSV export uses preset-selected range | Code: `run(renderReports)` re-enters the whole function so the CSV export closure re-captures `start`/`end` from the new inputs; Manual UAT §4.2 (human-only) | COVERED |
| S7 | Manual date entry still works after a preset | Code: `#rStart`/`#rEnd` remain editable; `#applyReports` unchanged; Manual UAT §4.3 (human-only) | COVERED |
| S8 | No backend/IPC/schema change introduced | `git show --stat b77d767`: only `src/report-ranges.ts`, `src/main.ts`, `tests/reportRanges.test.mjs`, `openspec/` touched — no `src-tauri/` file in this commit | CONFIRMED |

## Tests run

| Suite | Command | Result |
|---|---|---|
| Pure helper (new) | `node --import tsx --test tests/reportRanges.test.mjs` | **5/5 pass** |
| Full frontend suite | `npm run test:frontend` | **88/90 pass** — 2 pre-existing failures in `pi-observe.security.test.mjs` (unrelated; confirmed failing on clean tree before this change) |
| Build (tsc + vite) | `npm run build` | **PASS** — 16 modules, no TS errors |

## Changed files (task-033 commit b77d767)

- `src/report-ranges.ts` — new pure helper (no DOM/IPC)
- `src/main.ts` — import line + `renderReports` preset row + button wiring
- `tests/reportRanges.test.mjs` — 5 unit tests for the pure helper

## Manual UAT status

Tasks §4.1–4.4 require a running packaged/dev macOS app — human-only. Not exercised by this agent. Consistent with L1 tier and prior tasks (TASK-030/031).

## Blockers

None.
