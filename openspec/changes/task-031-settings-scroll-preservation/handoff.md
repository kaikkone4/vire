<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-031 settings-scroll-preservation

- **Change dir**: `openspec/changes/task-031-settings-scroll-preservation/`
- **Branch / PR**: `feat/task-031-settings-scroll-preservation` — draft PR #26
- **Tier**: L1-equivalent (frontend-only; no backend/IPC/schema/egress/deps)
- **Phase / gate**: SW-4 code review PASS + SW-5 security PASS (2026-06-20); ready for SW-6.

## Gate results

- SW-3 QA: PASS. Build clean; scoped tests 14/14. Manual macOS UAT remains human-only and is not an
  SW-4/SW-5 blocker. See `qa.md`.
- SW-4 Code Review: PASS. No craft, convention, complexity, dead-code, or architecture blockers. See
  `review.md`.
- SW-5 Security: PASS. semgrep 0, gitleaks (branch) clean, Trivy 0 HIGH/CRITICAL runtime. No XSS/secret/
  IPC/capability/auth regression. See `sec.md`. Pre-existing dev-dep vite 8.2 CVE (GHSA-fx2h-pf6j-xcff,
  lockfile untouched, dev-server-only, not shipped) escalated as a SEPARATE bump task — not blocking.

## Checks carried forward

- `git diff --check main...HEAD`: PASS
- `npm run build`: PASS
- Scoped frontend tests: PASS (14/14)
- Full frontend suite in SW-4 sandbox: 71/75; four unrelated `pi-observe.security` tests hit local-listen
  `EPERM`. SW-3 recorded 73/75 with two pre-existing Langfuse-dependent failures.

## Active blockers

- None for pipeline gates.
- Human macOS UAT (`tasks.md` §4) remains unverified.
- Out-of-scope follow-up (do NOT fold into this PR): bump vite ≥6.4.3 + esbuild ≥0.28.1 (pre-existing
  dev-dep CVEs). Open as its own task.

## Exact next action

Route to SW-6 Release Manager (SW-4 + SW-5 both PASS). Confirm draft PR #26 body before marking ready.

## Notes

- Scroll capture/restore is correctly centralized in `src/main.ts:43`; decision helper is
  `src/scroll.ts:9-11`.
- GitHub PR body was not inspectable in SW-4 because `api.github.com` was unreachable. Local commit
  subject/body are complete; confirm draft PR description before marking ready.
