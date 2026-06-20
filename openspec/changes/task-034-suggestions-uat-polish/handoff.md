<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste. -->

# Handoff — TASK-034 Suggestions UAT polish

- **Change / branch / PR**: `task-034-suggestions-uat-polish` /
  `feat/task-034-suggestions-uat-polish` / #29
- **Phase / gate**: SW-3 **PASS** (2026-06-21) → route to SW-4 ∥ SW-5
- **Tier**: L1

## Gate result

Full SW-3 recheck after consolidated fix-loop commit 51d52fb. All 30 spec scenarios
have observable test coverage. All gates green. No blockers.

## What was verified

- **A1/A2/A3 (DEC-034/035):** `normalize_same_minute_span` in lib.rs handles positive span
  (unchanged), forward bump (`start → start+n`), and 23:59 day-end anchor (`23:58 → 23:59`).
  `accept_suggestion_repo` calls it after derive. Frontend `suggestionRow` mirrors both branches
  via `subMinutesHHMM`; DEC-035 23:59 pre-fill test passes.
- **B1–B6 (AI cost):** `add_column_if_absent` for cost columns; accept INSERT carries cost;
  `SUM(CASE WHEN origin='ai_suggested')` returns NULL (not 0) when no AI cost; CSV header +
  rows carry cost columns; `summaryCards` shows "—" for absent cost, no AI sub-line for
  manual-only projects, mixed-currency → "—".
- **C1–C4 (source/trackability):** `unmappedNotice` copy and Settings link; `.hint` badge for
  untimed rows; `emptyState` names every cause; `sourceDisabledNotice()` surfaces above a
  non-empty pending list when `sourceDisabled=true` (gap not covered by `sourceBanner`).
- **D1–D2 (fixed 30-min gap):** `GAP_MINUTES = 30` in engine.rs — no settings, no IPC, no UI;
  clustering boundary test still passes.
- **SEC:** No auto-post (accept is sole writer); absence ≠ zero (NULL cost renders "—");
  SEC-012 render test passes (no sk-, Bearer, Authorization, payload, prompt in HTML).

## Checks

- Rust (`src-tauri/`): `cargo test --lib` → **165/0**; `cargo fmt --check` → clean;
  `cargo clippy --lib --all-targets` → 3 pre-existing warnings only.
- Frontend (`LANGFUSE_*` unset): `npm run test:frontend` → **105/0**; `npm run build` → green.
- OpenSpec: `openspec validate task-034-suggestions-uat-polish` → valid.
- `git diff --check HEAD` → clean.

## LANGFUSE_* note

`tests/pi-observe.security.test.mjs` lines 50 and 82 fail when `LANGFUSE_*` env vars are set
(pre-existing, unrelated to TASK-034). With vars unset: 105/0.

## Exact next action

Route to **SW-4 (Code Reviewer)** ∥ **SW-5 (Security Agent)** in parallel.

## Artifacts

- `openspec/changes/task-034-suggestions-uat-polish/qa.md` — scenario coverage matrix
- `openspec/changes/task-034-suggestions-uat-polish/review.md` — SW-4 review artifact
