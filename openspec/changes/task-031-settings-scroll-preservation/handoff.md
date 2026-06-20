<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-031 settings-scroll-preservation

- **Change dir**: openspec/changes/task-031-settings-scroll-preservation/
- **Branch / PR**: `feat/task-031-settings-scroll-preservation` (off `main` @5458a59) — draft PR #26
- **Phase / gate**: SW-2 implementation (DONE) → next SW-3 QA
- **Tier**: L1-equivalent (frontend-only; no backend/IPC/schema/egress/deps)

## Last gate result
SW-2 Frontend: DONE (2026-06-20). Scroll preservation + copy cleanup implemented; see sw2-impl-notes.md.
Build clean; test:frontend 73/2 (the 2 fails pre-exist on main — Langfuse-network pi-observe tests).

## Active blockers
- none

## Exact next action
sw-qa-engineer (SW-3): integration/QA gate on branch `feat/task-031-settings-scroll-preservation`.
Primary acceptance = manual UAT in the macOS app (tasks.md §4; fix is DOM/webview-bound, suite tests pure
builders only): in Settings scroll down, press Test connection / Save range / Import now / Map → viewport
stays put; switch views and back → other view opens at top; mapping help reads "Create & map".

## Required files (read these, not the whole tree)
- sw2-impl-notes.md — what changed, why scroll.ts is its own module, edge cases, test results (read FIRST)
- arch-review.md — root cause, fix, rejected alternatives; tasks.md — checklist + manual UAT
- proposal.md + specs/settings-scroll-preservation/spec.md — why/what/scope; requirements + scenarios
- src/scroll.ts (`nextScrollTop`); src/main.ts (`shell()` :43, `lastRenderedView` :28); env-mapping-ui.ts:97

## Notes carried forward
- Root cause: `shell()` re-assigns `app.innerHTML`, recreating `<main>` → scrollTop resets to 0.
- Fix at the shared chokepoint (benefits all views; acceptance in Settings) — not scope creep. Do NOT
  convert to partial/diff rendering (rejected). test:frontend 2 fails pre-exist on main (pi-observe).
