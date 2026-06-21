# SW-4 Code Review — TASK-034 recheck

## Verdict: FAIL

DEC-035 is correctly implemented: backend and frontend both normalize a `23:59` same-minute
suggestion to `23:58 → 23:59`. Cost persistence/reporting/CSV, the fixed 30-minute contract
correction, and accept transaction boundaries are otherwise sound. One trackability state remains
unexplained in a reachable Suggestions view.

## Blocking issues

1. **A disabled AI evidence source is hidden when pending suggestions exist.**
   `sourceBanner()` returns no banner for `health === 'disabled'` because it only accepts values in
   `degradedHealth` (`src/main.ts:37`, `src/main.ts:49`). `renderSuggestions()` does classify disabled
   as degraded, but passes that flag only into `suggestionsBody` (`src/main.ts:67`), where it is consumed
   exclusively by `emptyState`; any non-empty pending list renders groups instead and discards the flag
   (`src/suggestions-ui.ts:247-256`). Therefore previously imported pending suggestions can be shown
   while the source is disabled with no explanation or Settings action, contrary to the requirement
   that the Suggestions surface distinguish a disabled/unavailable source. Include `disabled` in the
   source banner path (or render an equivalent notice independent of list emptiness) and add a test for
   a disabled source with a non-empty suggestion list.

## Suggestions

- Update completed checkboxes in `openspec/changes/task-034-suggestions-uat-polish/tasks.md:34-70`;
  they currently contradict `qa.md` and `handoff.md`.
- Remove stale wording: `src/main.ts:23` says card rendering is deferred although it is active, and
  `src-tauri/src/suggestions/engine.rs:18-20` describes the fixed gap as a “default”/“tunable” policy.

## Escalations to SW Architect

None. The prior end-of-day escalation is resolved by DEC-035 and needs no further architecture work.

## Review coverage

- Reviewed full implementation paths: `src-tauri/src/lib.rs`, `src/main.ts`,
  `src/suggestions-ui.ts`, `src/summary-cards.ts`, `tests/suggestionsUi.test.mjs`, and
  `tests/summaryCards.test.mjs`, plus the TASK-034 spec/design and fixed-gap engine/test references.
- Accept remains one SQLite transaction: entry insert, guarded pending-status update, read-back, and
  commit (`src-tauri/src/lib.rs:566-644`). Errors before commit roll back both writes.
- Commit messages are scoped and descriptive. PR description metadata was not available locally.
- No new dead code, unused imports, or touched-line Clippy warnings found.

## Verification

- `cargo test --lib`: PASS, 165/165.
- `cargo fmt --check`: PASS.
- `cargo clippy --lib --all-targets`: PASS with pre-existing warnings outside TASK-034 changes.
- Focused frontend tests: PASS, 23/23.
- `npm run build`: PASS.
- `openspec change validate task-034-suggestions-uat-polish --strict`: PASS.
- `git diff --check`: PASS.
- Full `npm run test:frontend`: 98/103 in this sandbox; four unrelated tests cannot bind
  `127.0.0.1` (`EPERM`), and one unrelated parallel timing test passed when rerun alone. QA recorded
  103/103 with the required environment cleanup.

## Changed paths reviewed

- Backend: `src-tauri/src/lib.rs`
- Frontend: `src/main.ts`, `src/suggestions-ui.ts`, `src/summary-cards.ts`
- Tests: `tests/suggestionsUi.test.mjs`, `tests/summaryCards.test.mjs`
- Contract/handoff artifacts under `openspec/changes/task-034-suggestions-uat-polish/`
