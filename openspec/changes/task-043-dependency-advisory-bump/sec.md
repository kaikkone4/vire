# Security review — TASK-043 Stream A (npm dev-dep advisory bump) · SW-5

- **Change**: `openspec/changes/task-043-dependency-advisory-bump/` (Stream A only)
- **PR / branch**: draft **PR #30** → `chore/task-043-vite-esbuild-advisory-bump` (base `main`)
- **Reviewed at**: PR head `7d845b9` vs base `origin/main`
- **Date**: 2026-06-21 · **Reviewer**: Security Agent (SW-5)
- **Tier**: project L2 / task L1 (dev-dependency bump). Full L2 scanner stack run.
- **Verdict**: **PASS** — no auto-fail conditions hit.

## Scope under review

npm **devDependency** advisory bump only:
- `vite` 6.4.2 → **6.4.3** (in-range; floor raised `^6.0.7` → `^6.4.3`)
- transitive `esbuild` (under `tsx`) 0.28.0 → **0.28.1**

Diff vs `main` = `package.json` + `package-lock.json` + 6 openspec artifacts. **Zero** source/config
files changed (`git diff --name-only` confirms no `src/`, `src-tauri/`, `Cargo.*`, `tauri.conf.json`,
or capability files). Stream B (Tauri/GTK RUSTSEC) = TASK-044, untouched.

## Tier 1 scanner stack — results

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **OSV-scanner** 2.3.8 | PR `package-lock.json` | **0 issues** | No |
| **npm audit** (npm 11.16.0) | PR lock, full | **0 vulnerabilities** | No |
| **npm audit `--omit=dev`** | shipped runtime surface | **0 vulnerabilities** | No |
| **Trivy** 0.71.1 `fs` | PR lock, incl. `--include-dev-deps` | **0** (HIGH/CRITICAL **and** all-severity) | No |
| **gitleaks** 8.30.1 | 4 PR commits + changed files | **no leaks found** | No |
| **semgrep** 1.166.0 | `p/secrets` + `p/supply-chain` on manifest/lock | **0 findings, 0 ERROR-severity** | No |

### Advisories — baseline vs PR (independently reproduced)

OSV-scanner on the **`main` baseline** lockfile reports 3 known vulns; the PR clears **all three**:

| Advisory | Pkg (tag) | CVSS | main | PR #30 |
|---|---|---|---|---|
| GHSA-fx2h-pf6j-xcff (`server.fs.deny` bypass, Windows alt-paths) | vite (dev) | **8.2 HIGH** | 6.4.2 | **6.4.3 ✓ cleared** |
| GHSA-v6wh-96g9-6wx3 (launch-editor NTLMv2 hash disclosure, Windows UNC) | vite (dev) | 5.5 MED | 6.4.2 | **6.4.3 ✓ cleared** |
| GHSA-g7r4-m6w7-qqqr (dev-server arbitrary file read) | esbuild (dev) | 2.5 LOW | 0.28.0 | **0.28.1 ✓ cleared** |

The 8.2 HIGH is the only finding ≥ CVSS 7.0 (L1 auto-fail threshold); it is **eliminated** by this PR
— this PR exists to clear it. All three are dev-server-class and marked `(dev)` by OSV.

## Manual / supply-chain review

- **No runtime-shipped risk.** `npm audit --omit=dev` = 0 before **and** after. The advisories live in the
  Vite/esbuild dev-server, not in the bundled Tauri `.app`. Reachability in the shipped artifact = none.
- **Lockfile integrity.** lockfileVersion 3; `name: "code"` (the prior `.wt-task043` worktree artifact was
  corrected by QA in `953191c` — verified). 107 packages before, 107 after.
- **No new packages.** Lockfile delta = **0 added / 0 removed**. Only `vite` 6.4.2→6.4.3 and `tsx`'s nested
  `esbuild` 0.28.0→0.28.1 (plus its 26 platform-specific `@esbuild/*` optional binaries) changed version.
- **No new egress / alternate registry.** Every `resolved` URL points to `registry.npmjs.org`. No git/http
  tarball or mirror sources introduced.
- **No new install scripts.** `hasInstallScript` set is **identical to main** (`esbuild`, `fsevents`,
  `tsx/esbuild` — all pre-existing). No new package executes lifecycle scripts.
- **Integrity hashes present.** sha512 `integrity` populated on all changed packages; zero
  resolved-but-no-integrity entries across the lockfile.
- **No capability/permission change.** `package.json` `scripts` block byte-identical; only the `vite` floor
  string changed. No new dependency, no `tauri.conf.json`/capability delta (Cargo untouched).
- **No secrets.** gitleaks clean across the full PR commit range and changed file contents.

## Tier coverage

- **L1** (secrets + CVE ≥ 7): covered — gitleaks clean; the single CVSS 8.2 cleared, 0 remaining ≥ 7.0.
- **L2** (+ Trivy + semgrep ERROR): covered — Trivy 0 HIGH/CRITICAL; semgrep 0 ERROR.
- **L3** (ASVS L3 + threat model): not required for an L1 dev-dep bump; not run.

semgrep SAST is inherently low-signal here (no source code changed — lockfile/manifest only); run completed
clean with zero ERROR findings regardless.

## Escalations

None. No design-level security issue. No code-level FAIL.

## Verdict

**PASS** → proceed. No auto-fail conditions hit per the triage rubric (no secret; no CVE ≥ 7.0 remaining;
no Trivy HIGH/CRITICAL; no semgrep ERROR). The bump strictly reduces risk and adds no new
dependency, registry, install script, egress path, capability, or runtime exposure.

Per gate flow: SW-5 PASS — release to SW-6 (Release Manager) **after** SW-4 (Code Reviewer) also passes.
