# QA Report — TASK-043 Stream A (npm dev-dep advisory bump)

- **Change**: `openspec/changes/task-043-dependency-advisory-bump/` (Stream A only)
- **QA date**: 2026-06-21 · **Tier**: L1 · **Verdict**: **PASS**
- **Branch / PR**: `chore/task-043-vite-esbuild-advisory-bump` → draft PR #30 (base: origin/main @ `0700565`)
- **QA engineer fix committed**: `953191c` — restored `package-lock.json` `name` field (`".wt-task043"` → `"code"`; worktree artifact from DevOps run). Pushed to PR branch.

## Scenario coverage matrix (L1 gate)

| Scenario (tasks.md §A) | Check | Result |
|---|---|---|
| A.1 vite resolves to 6.4.3 | `node_modules/vite` version | **6.4.3** ✓ |
| A.1 tsx esbuild resolves to ≥ 0.28.1 | `node_modules/tsx/node_modules/esbuild` version | **0.28.1** ✓ |
| A.1 top-level esbuild not disturbed | `node_modules/esbuild` version | **0.25.12** (unchanged) ✓ |
| A.2 floor pin raised | `package.json devDependencies.vite` | `^6.4.3` ✓ |
| A.3 `npm run build` | `tsc && vite build` | **OK** — 17 modules, dist/ emitted ✓ |
| A.4 `npm run test:frontend` | test runner count | **103 pass / 2 fail (pre-existing)** ✓ |
| A.5 GHSA-fx2h-pf6j-xcff cleared | `npm audit` | **0 vulnerabilities** ✓ |
| A.5 GHSA-g7r4-m6w7-qqqr cleared | `npm audit` | **0 vulnerabilities** ✓ |
| A.5 `--omit=dev` | runtime-only audit | **0 vulnerabilities** (before and after) ✓ |
| A.6 No Cargo/Tauri/src changes | `git diff origin/main HEAD --name-only` | Only `package.json`, `package-lock.json`, `openspec/changes/task-043-*/` ✓ |

## Advisories verified cleared

| GHSA | Pkg | Sev | Before | After |
|---|---|---|---|---|
| GHSA-fx2h-pf6j-xcff (server.fs.deny Windows alt-path bypass) | vite | HIGH | 6.4.2 | 6.4.3 ✓ |
| GHSA-g7r4-m6w7-qqqr (dev-server arbitrary file read) | esbuild (under tsx) | LOW | 0.28.0 | 0.28.1 ✓ |

Both advisories are dev-server-class only; `npm audit --omit=dev` = 0 before and after (not in shipped `.app`).

## Scope guard

`git diff origin/main HEAD --name-only` returns exactly 7 paths: `package.json`, `package-lock.json`, and 5 `openspec/changes/task-043-*/` files. No `src/`, `src-tauri/`, `Cargo.*`, `tauri.conf.json`, or capability files changed.

## QA fix applied (cosmetic)

`package-lock.json` `name` field was `".wt-task043"` (worktree artifact from DevOps run in a temp dir). Fixed to `"code"` (matching project root and remote main). Committed as `953191c` and pushed to the PR branch. `npm audit` re-confirmed 0 vulns after the fix.

## Pre-existing test failures (not a regression)

`tests/pi-observe.security.test.mjs` — 2 cases fail identically on pristine main (network/env-dependent Langfuse calls return 401 instead of the asserted block message). Same 103/2 count as documented in `ops-review.md`. No new failures introduced. In a network-capable CI env these pass (105/105 recorded in TASK-034 SW-3).

## PR #20 superseded rationale

Dependabot PR #20 bumps vite to **8.0.16** (major, outside `^6.0.7` scope, Node ≥ 20.19 engine floor, 64 commits behind main). PR #30 clears the same HIGH+LOW advisories via a minimal in-range 6.4.3 bump. Recommend closing PR #20 as superseded. Full rationale: `ops-review.md §Why NOT dependabot PR #20`.

## Stream B

TASK-044 (Tauri/GTK RUSTSEC) is independent and untouched. Not in scope here.

## Gate verdict

**PASS** → route to SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel. No blockers for Stream A merge after `953191c`.
