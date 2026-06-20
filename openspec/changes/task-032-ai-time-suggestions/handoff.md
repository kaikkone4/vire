<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)
- **Phase / gate**: SW-2 Workstream A — **DONE** (A-checkpoint passed). B/C not started.
- **Tier**: L2 (new capability; additive schema; backend + frontend; no egress)

## Last gate result
SW-1 Architect PASS (2026-06-20). SW-2 A complete (2026-06-20): engine + table + 12 unit tests;
`cargo test` 154 pass / 0 fail, `fmt --check` clean, `clippy` 0 findings in module.

## Active blockers
- none. (4 non-blocking interpretation notes in sw2-a-notes.md §"Interpretation decisions" for QA/BA.)

## Exact next action
SW-2 Workstream **B** (sw-backend-developer): implement accept/dismiss IPC + `time_entries.origin` +
DEC-003 reporting separation per tasks.md B1–B5 / design.md §3, on the same branch. Then Workstream C
(frontend) per tasks.md C1–C5 / design.md §4. (Or route to SW-3 QA on the A slice if gating per-WS.)

## Required files (read these, not the whole tree)
- sw2-a-notes.md — A-checkpoint evidence table + interpretation decisions + what's NOT in A-scope
- design.md — data model (§1), engine algorithm (§2), IPC (§3 → B), UI (§4 → C), guarantees (§5)
- tasks.md — A done; B (B1–B5) and C (C1–C5) remain
- arch-review.md — DEC-003 constraint, scope/split rationale
- specs/ai-time-suggestions/spec.md — capability contract (5 requirements)

## Notes carried forward
- A delivered: `src-tauri/src/suggestions/{mod,engine,store,tests}.rs`; `mod suggestions;` +
  `suggestions::store::migrate` wired into `lib.rs` `init_db` (after env_mapping/runtime_observer).
- Public `generate`/DTOs are `#[allow(dead_code, unused_imports)]` until B/C consume them.
- HARD for B (DEC-003): accepted blocks → `time_entries.origin='ai_suggested'`; `get_summary`
  (lib.rs:472) + `export_report_csv` must report AI vs human duration separately. Accept is the **only**
  writer of an `ai_suggested` entry; unknown-duration block w/o start/end edits → error.
- Guarantees verified in A: absence≠zero (DEC-004), no auto-post (DEC-006), secret-free (SEC-012),
  no egress (DEC-001/017). AI≠human (DEC-003) deferred to B.
- `cost_currency` reserved/NULL from importer today; mixed-currency path implemented + tested.
