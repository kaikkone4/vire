# Tasks — TASK-032 ai-time-suggestions

Design + constraints: `design.md`, `arch-review.md`. Three sequenced workstreams; **A gates B and C**.
Do the minimum; no capture, no import-behaviour change, no extra refactors. Check off as you go.

## Workstream A — Suggestion engine + persistence  (backend)  [gates B, C]

- [x] A1. New module `src-tauri/src/suggestions/` (`mod.rs`, `engine.rs`, `store.rs`, `tests.rs`);
  declare `mod suggestions;` in `lib.rs`.
- [x] A2. `suggestions::store::migrate` creates `time_entry_suggestions` (schema: `design.md` §1.1);
  call it from `init_db` AFTER `projects`/`time_entries`/`env_mapping` migrate (`lib.rs:110`).
- [x] A3. Engine `generate(conn)` (`design.md` §2): join mapped `langfuse_ai_evidence`, bucket by
  `(project, local date)`, cluster by the 30-min `GAP` constant, aggregate tokens/cost/counts/health,
  compute confidence/source/reason. Idempotent: replace only `pending`, preserve `accepted`/`dismissed`.
- [x] A4. absence ≠ zero: all-untimed bucket → `duration NULL` "needs manual time" (never 0);
  unmapped-env evidence returned in a separate `unmapped` summary (never dropped/zeroed).
- [x] A5. Unit tests (`suggestions/tests.rs`) per `design.md` §7-A. `cargo test` + `fmt`/`clippy` clean.
- [x] **A-checkpoint.** Verify engine output against a seeded fixture before starting B/C.

## Workstream B — Accept / dismiss IPC + AI-origin entry + reporting separation  (backend)

- [x] B1. `add_column_if_absent(conn, "time_entries", "origin", "TEXT NOT NULL DEFAULT 'manual'")` in
  `init_db`; add `origin` to `TimeEntry` (manual path defaults `'manual'`).
- [x] B2. IPC `list_time_entry_suggestions(regenerate)`, `accept_time_entry_suggestion(id, edits?)`,
  `dismiss_time_entry_suggestion(id)` (`design.md` §3); register in `generate_handler!`.
- [x] B3. Accept = only writer of an `origin='ai_suggested'` `time_entries` row (+ secret-free
  provenance note), single transaction, marks suggestion `accepted` w/ `accepted_entry_id`.
  Unknown-duration block without start/end edits → error (never invent a duration). Dismiss writes
  nothing. Re-deciding a decided suggestion → rejected.
- [x] B4. DEC-003 reporting: `get_summary` + `export_report_csv` separate `'manual'` vs `'ai_suggested'`
  totals (human-minutes unchanged in meaning; AI-minutes a distinct figure/column). No silent conflation.
- [x] B5. Tests per `design.md` §7-B. `cargo test` + `fmt`/`clippy` clean.

## Workstream C — Review/Accept UI  (frontend)

- [x] C1. Add `'Suggestions'` to `View`/`views` (`src/main.ts:12,21`); `renderSuggestions()` + bind;
  route in `render()`.
- [x] C2. List grouped by project → date; each row: duration (or "needs manual time"), span,
  tokens/cost ("—" when unknown), trace/session counts, health, confidence, reason; Accept / Edit /
  Dismiss. Edit reveals inline date/start/end/note (reuse `forms.ts`); Accept submits `edits`.
- [x] C3. "Refresh suggestions" → regenerate=true. Unmapped notice links to Settings. Absence/empty
  states explain the cause (no evidence / nothing mapped / all dismissed). Reuse `sourceBanner()`.
- [x] C4. Secret-free render (SEC-012): names, project refs, time, aggregate numbers, counts, health
  only — no payload/session-id/prompt/metadata.
- [x] C5. Frontend tests per `design.md` §7-C. `npm run test:frontend` + `npm run build` green.

## Cross-cutting verification (before SW-3 QA)

- [x] V1. Re-read all changed files; confirm the §5 guarantees checklist holds end to end.
- [x] V2. No new network egress (engine is SQLite-only); Langfuse untouched.
- [x] V3. Update `handoff.md` with phase, gate result, next action.

## Manual acceptance (packaged/dev macOS app — human-only)

- [ ] M1. With `veronavi` mapped and evidence imported, the Suggestions view shows blocks; Accept
  creates an entry visible in Today/Reports; the AI total is reported separately from manual time.
- [ ] M2. Dismiss removes a suggestion and posts nothing. Refresh does not duplicate decided suggestions.
- [ ] M3. An unmapped environment with evidence shows the "map it" prompt, not an empty/zero view.
</content>
