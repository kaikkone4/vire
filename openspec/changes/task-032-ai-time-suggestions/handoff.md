<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-3 QA recheck PASS (2026-06-20) — route to SW-4 + SW-5.
- **Tier**: L2

## Last gate result
**SW-3 QA PASS (recheck, 2026-06-20)** on HEAD `dc60924`. Full A+B+C re-run:
- Rust `--lib` 159/159 PASS; suggestions 13/13 (+1 new failure-path test).
- `cargo clippy --all-targets` → 0 warnings in suggestions/; langfuse pre-existing unchanged.
- `cargo fmt --check` PASS. `git diff --check` clean.
- Suggestions builder 10/10 PASS. `npm run build` PASS.
- B1 atomic rollback verified: `failed_regeneration_preserves_the_original_pending_set` passes.
- B2 dead-code clean: no `allow(dead_code)` in suggestions/; `EvidenceRow.trace_id` field gone.
- All cross-cutting guarantees (absence≠zero, no-auto-post, AI≠human, secret-free, no egress) confirmed.
- Details: `fix-sw4-loop1.md`; scenario matrix + recheck section: `qa-032.md`.

## Active blockers
- None.

## Exact next action
**SW-4 Code Review + SW-5 Security Agent** (parallel). Both receive HEAD `dc60924`.

## Required files (read these, not the whole tree)
- fix-sw4-loop1.md — what changed + checks; review.md — original SW-4 blockers; sw2-*-notes.md — impl

## Notes carried forward
- C: `'Suggestions'` view in `src/main.ts` + pure builders `src/suggestions-ui.ts`;
  consumes `list/accept/dismiss_time_entry_suggestion`; grouped list, Accept/Edit/Dismiss,
  Refresh→regenerate, unmapped→Settings notice, absence/empty copy. `Summary.ai_minutes` + `summaryCards()`
  show AI-vs-human separately in Today/Reports (DEC-003). No new CSS / no backend change. See `sw2-c-notes.md`.
- A+B: `time_entries.origin`/`TimeEntry.origin`/`SummaryRow.ai_minutes`; 3 IPC cmds + accept/dismiss
  repos; `suggestions::current`, `SuggestionEdit`. See `sw2-b-notes.md`.
- Guarantees A+B+C: absence≠zero (DEC-004), no auto-post (DEC-006 — accept is the only writer), AI≠human
  (DEC-003), secret-free (SEC-012), no egress (DEC-001/017).
