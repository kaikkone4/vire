<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-2 fix loop 1 done (2026-06-20) — **both SW-4 blockers fixed**. Re-route to SW-4.
- **Tier**: L2

## Last gate result
**SW-4 FAIL (2026-06-20)**, now remediated. B1: `generate` is an atomic replace-set
(`unchecked_transaction` wraps delete + guarded inserts + final read; rollback on insert failure) +
new failure-path test. B2: dropped stale module-wide `allow(dead_code,unused_imports)` + the dead
`EvidenceRow.trace_id` field/projection. Checks all green. Details: `fix-sw4-loop1.md`.

## Active blockers
- None. Awaiting SW-4 recheck.

## Exact next action
**SW-4 Code Review:** re-review HEAD against the two fixed blockers (see `fix-sw4-loop1.md`). SW-5 may
continue independently. Local changes are committed on `feat/task-032-ai-time-suggestions`.

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
