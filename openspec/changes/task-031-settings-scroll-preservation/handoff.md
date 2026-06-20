<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-031 settings-scroll-preservation

- **Change dir**: `openspec/changes/task-031-settings-scroll-preservation/`
- **Branch / PR**: `feat/task-031-settings-scroll-preservation` — PR #26 (ready-for-review)
- **Tier**: L1-equivalent (frontend-only; no backend/IPC/schema/egress/deps)
- **Phase / gate**: SW-6 Release COMPLETE (2026-06-20)

## Gate results

- SW-3 QA: PASS. Build clean; 14/14 focused tests. See `qa.md`.
- SW-4 Code Review: PASS. No blockers. See `review.md`.
- SW-5 Security: PASS. semgrep 0, gitleaks clean, Trivy 0 HIGH/CRITICAL. See `sec.md`.
- SW-6 Release: COMPLETE. RELEASE.md written; gate artifacts committed (76c1ad4); PR promoted.

## Active blockers

- None for pipeline gates.
- Tag `task-031/v0.3.2` pending SSH private key (`~/.ssh/id_ed25519` absent in this env).
  Dry-run record + manual command in `RELEASE.md` §Tag signing.
- Human macOS UAT M1–M3 remains unverified (DOM/webview-bound, human-only).
- Out-of-scope follow-up (do NOT fold into this PR): bump vite ≥6.4.3 + esbuild ≥0.28.1.

## Exact next action

Janne merges PR #26. After merge: apply SSH signing key and run the `git tag` command from
`RELEASE.md` §Tag signing. Then open a separate dep-bump task for vite/esbuild CVEs.

## Notes

- RELEASE.md at `openspec/changes/task-031-settings-scroll-preservation/RELEASE.md`.
- Root `RELEASE.md` updated with v0.3.2 section at top.
- Deployment size: **patch**. Rollback: **partial-automated** (no migration; reinstall .app).
