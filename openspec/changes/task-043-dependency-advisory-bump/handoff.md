<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-043 dependency-advisory-bump (Stream A executed; Stream B = TASK-044)

- **Change dir**: openspec/changes/task-043-dependency-advisory-bump/
- **Branch / PR**: Stream A → `chore/task-043-vite-esbuild-advisory-bump` → **draft PR #30** (base main).
  Supersedes dependabot **PR #20** (vite→8 major; recommend close).
- **Phase / gate**: SW-2/DevOps Stream A executed + verified (2026-06-21). Awaiting review + merge.
- **Tier**: L1 (Stream A). Stream B (Tauri/GTK) = L2, separate task TASK-044, untouched.

## Last gate result
**SW-3 QA PASS (2026-06-21).** Verified on worktree `chore/task-043-vite-esbuild-advisory-bump`.
vite 6.4.2→**6.4.3** (in-range, floor `^6.4.3`), tsx esbuild 0.28.0→**0.28.1**.
`npm audit` → **0 vulns**; `npm run build` OK; test 103/2 (2 pre-existing).
QA fix committed: `953191c` — lockfile `name` restored to `"code"` (was `".wt-task043"` worktree artifact).
Scope guard confirmed: diff vs origin/main = package.json + package-lock.json + openspec artifacts only.
Full results: `qa.md`.
(Prior: DevOps SW-2 PASS, 2026-06-21; SW-1 SPLIT-REQUIRED, 2026-06-20.)

## Active blockers
- None. PR #30 is ready for SW-4 + SW-5.

## Exact next action
Pi-Assistant: (1) route PR #30 to **SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel**;
(2) **close dependabot PR #20** as superseded (vite 8 major, out-of-scope — see qa.md + ops-review.md);
(3) Stream B remains TASK-044 (untouched).

## Required files (read these, not the whole tree)
- `ops-review.md` — Stream A verification matrix, advisory table, why-not-PR#20 (PRIMARY)
- `arch-review.md` — SW-1 triage & split rationale; Stream A scoped to vite 6.4.3
- `tasks.md` §A — implementer checklist (A.1–A.6, all satisfied)

## Notes carried forward
- Stream A fix is dev-only; `npm audit --omit=dev` = 0 (not in shipped `.app`). Confirms reachability=none.
- PR #20 is 64 commits behind main + bumps vite to 8.0.16 (major, out of declared ^6.0.7 range) — do
  NOT merge as a cleanup; superseded by the scoped PR #30.
- No CI in repo (`.github/workflows/` empty) — A.3–A.5 gates are manual today; noted in ops-review as a
  future DevOps task (out of this scope).
- Streams A and B independent — A did not wait on B; Stream B (Tauri/GTK RUSTSEC) untouched here.
