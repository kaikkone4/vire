<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-2 Workstreams A + **B** — **DONE** (A-checkpoint + B-checkpoint passed). C not started.
- **Tier**: L2 (new capability; additive schema; backend + frontend; no egress)

## Last gate result
SW-1 PASS. SW-2 A+B complete. **SW-3 QA PASS on B (2026-06-20)**: all B1–B5 verified; 4 B tests
correct + passing; 158/0 cargo test; guarantees held (DEC-003/004/006, SEC-012, DEC-001/017).
QA evidence: `qa.md` (B section). B-impl evidence: `sw2-b-notes.md`.

## Active blockers
- none.

## Exact next action
**SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel** on the full branch (A+B commits).
Then **SW-2 Workstream C** (frontend, tasks.md C1–C5 / design §4) — Review/Accept UI, secret-free
render, absence/empty states.

## Required files (read these, not the whole tree)
- sw2-b-notes.md — B-checkpoint evidence + interpretation decisions + what's NOT in B-scope
- design.md §3/§7-B — IPC contract + B test list; §5 guarantees; sw2-a-notes.md for A context
- tasks.md — A + B done; C (C1–C5) remains; arch-review.md — DEC-003 rationale; spec.md — contract

## Notes carried forward
- B delivered: `time_entries.origin` + `TimeEntry.origin` + `SummaryRow.ai_minutes`; `accept`/`dismiss`
  repos + 3 IPC commands; `suggestions::current` (non-regenerate list), `store::get_by_id`,
  `SuggestionEdit`; `add_column_if_absent` now `pub(crate)`. Details/evidence: `sw2-b-notes.md`.
- For C (frontend, secret-free): consume `list/accept/dismiss_time_entry_suggestion`; render unmapped
  notice + absence/empty states; never render payload/session-id/prompt (SEC-012). `TimeEntry.origin` /
  `SummaryRow.ai_minutes` are additive — C surfaces AI-vs-human in Today/Reports.
- Guarantees held A+B: absence≠zero (DEC-004), no auto-post (DEC-006), AI≠human (DEC-003), secret-free
  (SEC-012), no egress (DEC-001/017). Accept is the **only** writer of `ai_suggested` (2 inserts total;
  the other defaults `'manual'`). B interpretation calls (dismiss idempotency, human-minutes) in notes.
