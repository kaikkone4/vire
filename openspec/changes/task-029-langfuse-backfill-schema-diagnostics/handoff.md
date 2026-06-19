# Handoff — TASK-029 task-029-langfuse-backfill-schema-diagnostics

## Current phase
SW-6 Release — COMPLETE

## Last gate result
PASS — all three required declarations present in RELEASE.md; PR #23 promoted to ready-for-review.

## What was done (SW-6)
- Created `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/RELEASE.md` (v0.3.0, minor,
  partial-automated rollback, full compatibility matrix).
- Updated root `RELEASE.md` TASK-029 heading from "v0.1" to "v0.3.0".
- Committed gate artifacts (`RELEASE.md`, `review.md`, `sec.md`, `qa-diag-sw6.md`) in commit `85aa74c`.
- Promoted PR #23 from draft → ready-for-review via `gh pr ready 23`.

## Blockers
- **Tag signing deferred** (same as TASK-026/027): SSH private key `~/.ssh/id_ed25519` absent.
  Intended tag: `task-029/v0.3.0` on `0d6037e4a85faf9e9d5b6b647258caaeaeef44c1`.
  Must be signed and pushed before any distribution artifact is published (not a merge blocker).

## Next action
- Release approval = Janne merging PR #23.
- If L2+: restore SSH signing key; run `git tag -s task-029/v0.3.0 ...` + `git push origin task-029/v0.3.0`.
- Verification flow (Flow 3, stubbed): no further SW steps required at L1.
- Follow-up: Documentation Engineer (if L2+); TASK-030 (time-entry suggestion from AI evidence).

## Required files
- `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/RELEASE.md` — gate document
- `RELEASE.md` (root) — updated to v0.3.0
- `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/review.md` — SW-4 PASS
- `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/sec.md` — SW-5 PASS (DEC-032 regen)
- `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/qa-diag-sw6.md` — SW-3/QA artifact
