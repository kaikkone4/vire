# Ops review — TASK-043 Stream A (npm dev-dep advisory bump)

- **Change**: `openspec/changes/task-043-dependency-advisory-bump/` (Stream A only)
- **Owner**: DevOps · **Date**: 2026-06-21 · **Tier**: L1
- **Scope guard**: npm devDependencies only (`vite`/`esbuild`/`package-lock.json`). **Tauri/GTK (Stream B) untouched.**
- **Verdict**: **MERGE the scoped fix (draft PR #30). Do NOT merge dependabot PR #20 as-is — supersede/close it.**

## What was verified (worktree off current `main`, Node v26.3.0 / npm 11.16.0)

| Gate (tasks.md §A) | Result |
|---|---|
| A.1 `npm update vite esbuild` | `vite` 6.4.2 → **6.4.3** (in-range `^6.0.7`); `tsx`'s `esbuild` 0.28.0 → **0.28.1**; top-level esbuild stays 0.25.12 (not in vulnerable range) |
| A.2 pin floor | `package.json` `devDependencies.vite` `^6.0.7` → **`^6.4.3`** (pins the fix; only line changed) |
| A.3 `npm run build` | **OK** — `tsc && vite build` succeeds, `dist/` (assets + index.html) emitted |
| A.4 `npm run test:frontend` | **103 pass / 2 fail (105 total)** — see "Pre-existing failures" below |
| A.5 advisories | `npm audit` → **0 vulnerabilities** (baseline was 1 HIGH + 1 LOW); `--omit=dev` → 0 before & after (dev-only) |
| A.6 no Cargo change | **Confirmed** — branch diff vs `main` is `package.json` + `package-lock.json` only; no `Cargo.*`, `tauri.conf.json`, capability, `.rs`, or `src/` change |

### Advisories cleared
| Advisory | Pkg | Sev | Was | Now |
|---|---|---|---|---|
| GHSA-fx2h-pf6j-xcff (`server.fs.deny` bypass, Windows alt-paths) | vite | **HIGH** | 6.4.2 | 6.4.3 ✓ |
| GHSA-g7r4-m6w7-qqqr (dev-server arbitrary file read) | esbuild (under `tsx`) | LOW | 0.28.0 | 0.28.1 ✓ |

Both are **dev-server-class** and not present in the shipped Tauri `.app` (`npm audit --omit=dev` = 0). Matches the SW-1 arch-review reachability finding.

## Pre-existing test failures (NOT caused by this change)

`tests/pi-observe.security.test.mjs` fails 2 cases **identically on pristine `main`** (vite 6.4.2, before any change):
- "safe dotenv parser loads only allowlisted Langfuse keys without shell execution"
- "remote Langfuse host is blocked unless explicitly opted in"

They are **network/environment-dependent** (the sandbox makes a real Langfuse call that returns 401 instead of the asserted block message). SW-3 recorded 105/105 for TASK-034 in a network-capable CI env, so these pass there. **Out of scope for this dependency bump** — flagged only to explain the non-green local count; no regression introduced (103/2 before == 103/2 after).

## Why NOT dependabot PR #20 (the central recommendation)

PR #20 (`dependabot/npm_and_yarn/multi-efc897c91a`) bumps `vite` to **8.0.16** — a **major-version jump (6 → 8)** that:
- falls **outside the declared `^6.0.7` range** and **outside SW-1's Stream A scope** (arch-review specifies the minimal in-range 6.4.3);
- introduces vite 8 breaking changes (config/plugin API, **Node ≥ 20.19 / 22.12** engine floor) for an advisory that is **dev-only and absent from the shipped artifact** — disproportionate risk/benefit;
- is **64 commits behind `main`** (still mergeable, but its naive diff is dominated by `main`'s progress — its own change is 2 files).

A vite major upgrade is a legitimate but **separate** decision that should go back through SW-1, not land via an auto-bump. The scoped 6.4.3 fix (PR #30) clears the same advisories at L1 risk.

## Recommendation / handoff

1. **Merge draft PR #30** (`chore/task-043-vite-esbuild-advisory-bump`) once a reviewer (and a network-capable CI run) is happy — it is the SW-1-designed Stream A fix.
2. **Close dependabot PR #20** as superseded/out-of-scope (or, if vite 8 is genuinely wanted, route a fresh SW-1 review for a deliberate framework-level upgrade — do not merge it as a "cleanup").
3. **Stream B (TASK-044, Tauri/GTK RUSTSEC)** remains untouched and independent — not part of this PR.

## Operational observations (out of scope; for a future task)

- **No CI present** (`.github/workflows/` is empty). The arch-review's A.3–A.5 gates (build + frontend tests + OSV/audit) currently rely on manual runs. A minimal CI workflow running `npm ci && npm run build && npm run test:frontend && npm audit` on PRs would make these advisory bumps self-verifying and stop them riding along across gates. Recommend a separate DevOps task.
