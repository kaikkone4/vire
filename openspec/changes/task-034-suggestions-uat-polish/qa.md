# QA Report — TASK-034 Suggestions UAT polish

**Gate**: SW-3  **Tier**: L1  **Branch**: `feat/task-034-suggestions-uat-polish`  **Date**: 2026-06-21 (recheck after DEC-035 SW-4 escalation fix)

## Verdict: PASS

All spec scenarios have observable test coverage. All gates green. No blockers.

---

## Scenario coverage matrix

| # | Scenario | Location | Result |
|---|----------|----------|--------|
| A1 | `normalize_same_minute_span` in lib.rs: positive span → unchanged; forward bump → `(start, start+n)`; midnight cross → day-end anchor `(23:59−n, 23:59)` (DEC-035 re-do, replaces `bump_end_if_not_after`) | `lib.rs:204` | PASS |
| A2 | Called in `accept_suggestion_repo` after derive, before `parse_duration`; rebinds both ends; manual path unchanged | `lib.rs:613` | PASS |
| A3 | `suggestionRow`: same-minute block → `forward = addMinutesHHMM`; if `forward !== start` → End default = forward (non-boundary); if `forward === start` (23:59 clamped) → End = start, Start = `subMinutesHHMM(start, n)` (DEC-035 echo) | `suggestions-ui.ts:143–152` | PASS |
| A3-sub | `subMinutesHHMM` helper: subtracts mins, floors at 00:00 | `suggestions-ui.ts:78` | PASS |
| A3-normal | Normal multi-minute block keeps its real End — no bump applied | `suggestionsUi.test.mjs` | PASS |
| A4-rust-same | Accept of same-minute block (09:00:10→09:00:50) bumps end, stores `09:01`, `end > start` | `lib.rs:1941` | PASS |
| A4-rust-no-dur | Same-minute block without engine duration bumps by minimum 1 | `lib.rs:1971` | PASS |
| A4-rust-2359 | **DEC-035**: same-minute block at 23:59:10→23:59:50 anchors on end → stores `23:58 → 23:59`, duration 1, no error, no midnight cross | `lib.rs:1991` | PASS |
| A4-rust-manual | Manual entry with equal start and end is still rejected | `lib.rs:2024` | PASS |
| A4-fe-sub | `subMinutesHHMM('23:59', 1) == '23:58'`; floor at 00:00 | `suggestionsUi.test.mjs:74` | PASS |
| A4-fe-same | A3: 09:00 same-minute → End `09:01`, not `09:00` | `suggestionsUi.test.mjs:81` | PASS |
| A4-fe-2359 | **DEC-035**: 23:59 same-minute → Start `23:58` / End `23:59`; never `23:59/23:59` | `suggestionsUi.test.mjs:90` | PASS |
| B1 | `add_column_if_absent` cost_total/cost_currency in init_db; manual path writes NULL (absence ≠ zero) | `lib.rs:139–140` | PASS |
| B2 | `accept_suggestion_repo` INSERT carries `suggestion.cost_total`/`cost_currency` verbatim | `lib.rs:625–627` | PASS |
| B3 | `SummaryRow` ai_cost_total via `SUM(CASE WHEN origin='ai_suggested' THEN cost_total END)` — NULL when no AI cost, never 0 | `lib.rs:443–444` | PASS |
| B4 | CSV header + rows carry `cost_total,cost_currency` (empty string when NULL); origin column kept | `lib.rs:505,517–518` | PASS |
| B5 | `summaryCards` project card: `AI-suggested {h} · {cost}` — "—" when NULL; AI sub-line absent when `ai_minutes == 0` | `summary-cards.ts:54–57` | PASS |
| B5-lead | Lead "Total tracked" card aggregates AI cost via `aggregateAiCost`; mixed-currency → "—", reported separately | `summary-cards.ts:44–48` | PASS |
| B6-cost-copy | Accept copies 1.5/USD onto entry verbatim | `lib.rs:2044` | PASS |
| B6-summary-sep | Summary ai_cost_total separate from human; manual-only range → ai_cost_total NULL | `lib.rs:2063` | PASS |
| B6-csv | CSV header has cost columns; AI row has cost/currency; manual row cells empty | `lib.rs:2063` | PASS |
| B6-fe-cost | B5: AI sub-line carries cost | `summaryCards.test.mjs:18` | PASS |
| B6-fe-null | B5: AI cost absent → "—", never "0" | `summaryCards.test.mjs:25` | PASS |
| B6-fe-no-ai | B5: manual-only project shows no AI sub-line | `summaryCards.test.mjs:31` | PASS |
| B6-fe-mixed | B5: mixed-currency aggregate → "—" | `summaryCards.test.mjs:49` | PASS |
| C1 | `unmappedNotice`: copy "not trackable until mapped"; action button "Map in Settings" | `suggestions-ui.ts:199` | PASS |
| C2 | Untimed row carries `.hint` badge "not auto-trackable — add time manually" in summary `<tr>`, before edit panel; timed rows carry no badge | `suggestions-ui.ts:118–119` | PASS |
| C3 | `emptyState` lists all causes: nothing-imported, unmapped (when present), untimed, source-down (when `sourceDegraded`) — each with action | `suggestions-ui.ts:219` | PASS |
| C4 | No bare empty table; no "0"; source-down cause only when `sourceDegraded`; XSS-safe | `suggestionsUi.test.mjs:190` | PASS |
| D1 | `const GAP_MINUTES: i64 = 30` at `engine.rs:21`; no settings field, no IPC, no UI — fixed gap, spec corrected to match | `engine.rs:21` | PASS |
| D2 | `clustering_respects_the_gap_boundary` at `suggestions/tests.rs:145` still covers at/over/under-30-min boundary | `tests.rs:145` | PASS |
| SEC | SEC-012 render test: no `sk-`, `Bearer`, `Authorization`, `payload`, `prompt` etc in rendered HTML | `suggestionsUi.test.mjs:225` | PASS |

---

## Guarantee verification

| Guarantee | Mechanism | Verified |
|-----------|-----------|---------|
| Never zero duration (DEC-034/035) | `normalize_same_minute_span`: positive span → unchanged; forward bump ≥ 1 min; 23:59 anchors on end (`23:58 → 23:59`); manual path still rejects `start==end` | ✔ |
| Absence ≠ zero (DEC-004) | NULL cost → `SUM(CASE…)` yields NULL; renders "—"; test `tokens/cost unknown render "—"` | ✔ |
| AI ≠ human (DEC-003) | `ai_cost_total` via `WHERE origin='ai_suggested'`; never folded into `duration_minutes`; separate sub-line | ✔ |
| No auto-post (DEC-006) | Accept is sole writer; C changes are render-only; untimed blocks require edited span to accept | ✔ |
| Secret-free / no egress (SEC-012) | SEC-012 render test passes; engine untouched; no new IPC channels; only aggregate numbers/labels rendered | ✔ |
| Manual path unchanged | `create_time_entry`/`update_time_entry` write NULL cost; `parse_duration` and manual validation unchanged | ✔ |
| Mixed currency → "—" | `aggregateAiCost` returns null on currency-set size > 1; renders "—"; B6-fe-mixed test passes | ✔ |
| Additive & reversible | Idempotent `add_column_if_absent`; revert leaves columns unused; no schema breaking changes | ✔ |

---

## Gate results

| Gate | Command | Result |
|------|---------|--------|
| G1 | `cargo test --lib` (from `src-tauri/`) | **165 pass / 0 fail** |
| G1 | `cargo fmt --check` | **clean (exit 0)** |
| G1 | `cargo clippy --lib --all-targets` | **3 pre-existing warnings** (unnecessary `if let` ×2, `io::Error::other`) — no new warnings in TASK-034 touched code |
| G2 | `npm run test:frontend` (`LANGFUSE_*` unset) | **103 pass / 0 fail** |
| G2 | `npm run test:frontend` (env not unset) | **101 pass / 2 fail** — 2 pre-existing pi-observe env failures (see note) |
| G2 | `npm run build` | **green** (tsc + vite, 32 kB JS) |
| G3 | `openspec change validate task-034-suggestions-uat-polish --strict` | **valid** |

### Pre-existing clippy warning note
3 warnings in lib.rs/langfuse code — unnecessary `if let`, `io::Error::other` pattern — all pre-existing, confirmed absent from TASK-034 touched lines (`normalize_same_minute_span`, `accept_suggestion_repo`, `init_db`, `summary_repo`, `export_csv_repo`, unit tests).

### LANGFUSE_* env note
`tests/pi-observe.security.test.mjs` lines 50 and 82 ("safe dotenv parser" / "remote Langfuse host") fail in the local dev environment due to Langfuse env contamination. Confirmed pre-existing: reproduced on the clean base branch with TASK-034 stashed; unrelated to this change. With `LANGFUSE_*` unset all 103 tests pass.

---

## Changed paths

**Backend (`src-tauri/src/lib.rs`)**
- `normalize_same_minute_span` helper (A1, DEC-035 re-do — replaces `bump_end_if_not_after`)
- `accept_suggestion_repo`: calls normalize helper (A2) + inserts cost_total/cost_currency (B2)
- `init_db`: `add_column_if_absent` for cost columns (B1)
- `TimeEntry` struct, `row_to_entry`, SELECT queries (B1)
- `SummaryRow` struct + `summary_repo` SQL with `SUM(CASE WHEN …)` (B3)
- `export_csv_repo`: cost_total/cost_currency in header + rows (B4)
- 6 new unit tests (A4 ×4 incl. DEC-035 23:59 test, B6 ×2)

**Frontend**
- `src/suggestions-ui.ts`: `subMinutesHHMM` helper (new, A3/DEC-035); `suggestionRow` mirrors both day-end branches; `unmappedNotice` C1 copy; C2 trackBadge; C3 emptyState rewrite
- `src/summary-cards.ts` (new): `summaryCards` + `aggregateAiCost` + AI cost sub-line (B5)
- `src/main.ts`: imports `summaryCards`; `Summary`+`Entry` DTO type additions (B5, B1)
- `tests/suggestionsUi.test.mjs`: extended — `subMinutesHHMM` unit test, A3 23:59 day-end test (DEC-035), C1–C4 tests
- `tests/summaryCards.test.mjs` (new): 7 B5 tests
