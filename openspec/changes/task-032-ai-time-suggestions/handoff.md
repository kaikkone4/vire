<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-2 Workstreams A + **B** — **DONE** (A-checkpoint + B-checkpoint passed). C not started.
- **Tier**: L2 (new capability; additive schema; backend + frontend; no egress)

## Last gate result
SW-1 PASS. SW-2 A complete + SW-3 QA PASS on A (all 2026-06-20). **SW-2 B complete (2026-06-20)**:
accept/dismiss IPC + `time_entries.origin` + DEC-003 reporting separation + 4 unit tests. `cargo test`
158 pass / 0 fail (+4 vs A, 0 regressions), `fmt --check` clean, `clippy` no new findings.
B-checkpoint evidence: `sw2-b-notes.md`.

## Active blockers
- none.

## Exact next action
**SW-3 QA on the Workstream B slice** (sw-qa-engineer) — verify B1–B5 vs design §3/§7-B + §5 guarantees
(accept = only writer of `ai_suggested`; no auto-post; absence≠zero; DEC-003 separation; secret-free;
local-only). Then SW-2 Workstream **C** (frontend, tasks.md C1–C5 / design §4). SW-4/SW-5 on A may run
in parallel.

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
