<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-2 Workstreams A + B + **C** — **DONE** (A/B/C-checkpoints passed). All of SW-2 complete.
- **Tier**: L2 (new capability; additive schema; backend + frontend; no egress)

## Last gate result
SW-1 PASS; SW-3 QA PASS on A+B. **C built + self-verified (2026-06-20)**: C1–C5 done; 10 builder tests
pass; `test:frontend` 85/83/2 (the 2 fails = pre-existing network `pi-observe.security`, also fail on
`main`); `npm run build` green; §5 guarantees held. C evidence: `sw2-c-notes.md`.

## Active blockers
- none.

## Exact next action
**SW-3 QA on Workstream C**, then **SW-4 (Code Review) + SW-5 (Security)** on the full A+B+C branch.
M1–M3 (packaged macOS app) stay human-only/unchecked.

## Required files (read these, not the whole tree)
- sw2-c-notes.md — C evidence + reporting-separation surface + not-in-scope; sw2-b/a-notes.md — B/A
- design.md §4/§7-C — frontend spec + C tests; §5 guarantees; tasks.md — A+B+C done

## Notes carried forward
- C (frontend, secret-free): `'Suggestions'` view in `src/main.ts` + pure builders `src/suggestions-ui.ts`;
  consumes `list/accept/dismiss_time_entry_suggestion`; grouped list, Accept/Edit/Dismiss,
  Refresh→regenerate, unmapped→Settings notice, absence/empty copy. `Summary.ai_minutes` + `summaryCards()`
  show AI-vs-human separately in Today/Reports (DEC-003). No new CSS / no backend change. See `sw2-c-notes.md`.
- A+B: `time_entries.origin`/`TimeEntry.origin`/`SummaryRow.ai_minutes`; 3 IPC cmds + accept/dismiss
  repos; `suggestions::current`, `SuggestionEdit`. See `sw2-b-notes.md`.
- Guarantees A+B+C: absence≠zero (DEC-004), no auto-post (DEC-006 — accept is the only writer), AI≠human
  (DEC-003), secret-free (SEC-012), no egress (DEC-001/017).
