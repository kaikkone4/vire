# Release — TASK-043 Stream A (npm dev-dep advisory bump)

- **Task:** TASK-043
- **Tier:** L2 project (task L1 — dev-dependency advisory patch, no runtime change)
- **Branch:** `chore/task-043-vite-esbuild-advisory-bump`
- **PR:** #30
- **Tag:** `task-043/v0.6.1` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `7d845b9`
- **Base version:** v0.6.0 (TASK-034)
- **Date:** 2026-06-21
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

**npm devDependency advisory bump only (2 package files)**

- `vite` 6.4.2 → **6.4.3** (within declared `^6.0.7` range; floor pinned to `^6.4.3` in `package.json`)
- `esbuild` (transitive under `tsx`) 0.28.0 → **0.28.1**; 26 `@esbuild/*` platform binaries updated in lockfile
- Top-level `esbuild` 0.25.12 — unchanged (not affected by the advisory)

**Advisories cleared (all dev-server-class, none shipped in `.app`):**

| Advisory | Pkg | CVSS | Before | After |
|---|---|---|---|---|
| GHSA-fx2h-pf6j-xcff (`server.fs.deny` Windows alt-path bypass) | vite | 8.2 HIGH | 6.4.2 | **6.4.3 ✓** |
| GHSA-v6wh-96g9-6wx3 (launch-editor NTLMv2 hash disclosure) | vite | 5.5 MED | 6.4.2 | **6.4.3 ✓** |
| GHSA-g7r4-m6w7-qqqr (dev-server arbitrary file read) | esbuild | 2.5 LOW | 0.28.0 | **0.28.1 ✓** |

`npm audit --omit=dev` = 0 before **and** after — no shipped-runtime exposure.

**Scope guard:** diff vs `main` = `package.json` + `package-lock.json` + OpenSpec artifacts only.
Zero `src/`, `src-tauri/`, `Cargo.*`, `tauri.conf.json`, or capability-file changes.
0 packages added / 0 packages removed. All 107 packages resolve via `registry.npmjs.org`.

---

## Required declarations

### 1. Deployment size: patch

This is a dev-only advisory bump. No source code, runtime dependency, Tauri config, Rust crate,
IPC surface, DB schema, or user-facing behaviour changes. The shipped `.app` is byte-identical to
v0.6.0. Semantic versioning: **patch** (risk-reduction maintenance, no feature, no breaking change).

### 2. Rollback strategy: automated

The change does not affect the shipped binary artifact. The vite and esbuild versions are consumed
only at build time (dev server + bundler); nothing from them is included in the Tauri `.app`.

| Layer | Rollback action | Automated? |
|---|---|---|
| Shipped `.app` | No change; v0.6.0 binary remains valid | ✅ N/A |
| Dev tooling | Revert `package.json` + `package-lock.json` and `npm ci` | ✅ Automated (`git revert` + one command) |
| DB / IPC / capability | No change; identical to v0.6.0 | ✅ N/A |

**Rating: automated** — no manual steps required. Revert is a single `git revert` on a two-file
commit. Stream B (Tauri/GTK RUSTSEC advisories, TASK-044) is independent and untouched.

### 3. Component compatibility matrix

No new runtime dependency introduced. No Rust crate, IPC command, schema, or capability change.
Only dev toolchain versions changed.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; unchanged |
| **rusqlite** | 0.32 | 0.32.x | `bundled`; unchanged |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; unchanged |
| **serde / serde_json** | 1.0 | 1.x | unchanged |
| **chrono** | 0.4 | 0.4.x | unchanged |
| **uuid** | 1.0 | 1.x | unchanged |
| **thiserror** | 2.0 | 2.x | unchanged |
| **url** | 2.0 | 2.x | unchanged |
| **Node.js** | 18.0 LTS | 26.x | Build tooling only; not shipped |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; unchanged |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings; no new commands |
| **@tauri-apps/plugin-dialog** | 2.2.0 | 2.2.x | unchanged |
| **tsx** | 4.22.0 | 4.x | Test runner `devDependency`; unchanged |
| **typescript** | 5.7.2 | 5.x | Build-only; unchanged |
| **vite** | **6.4.3** | **6.4.3** | `devDependency` floor raised from `^6.0.7`; not in shipped `.app` |
| **esbuild** (under tsx) | **0.28.1** | **0.28.1** | `devDependency` transitive; not shipped |
| **esbuild** (top-level) | 0.25.12 | 0.25.12 | unchanged; no advisory |
| **Upstream: Langfuse API** | v2 | v3 | unchanged |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-034 | — | no change |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment — same constraint as TASK-026 through TASK-034.
Role definition prohibits falling back to an unsigned tag at L2+.

**Dry-run record:** the intended tag is `task-043/v0.6.1` (annotated, SSH-signed) on commit
`7d845b9`. Action required before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-043/v0.6.1 -m "release(task-043): v0.6.1 patch — npm dev-dep advisory bump: vite 6.4.3 + esbuild 0.28.1 (clears GHSA-fx2h HIGH, GHSA-v6wh-96g9 MED, GHSA-g7r4 LOW)" 7d845b9
git push origin task-043/v0.6.1
```

### SBOM (L2 requirement)

Only `devDependency` package versions changed. The Cargo.lock is unchanged; the runtime npm
dependency graph is identical to v0.6.0. The SBOM baseline carries forward from v0.6.0.
No new component entry is required for dev-only version bumps.

---

## PR #20 — close as superseded (action required)

Dependabot PR #20 proposes `vite` → **8.0.16**, a major-version jump:
- Outside the `^6.0.7` declared range
- Requires Node ≥ 20.19 engine floor
- Breaking plugin/config API changes vs v6
- Warrants its own SW-1 architectural scope evaluation

PR #30 clears the same HIGH + LOW advisories via the minimal, in-range Vite 6.4.3 patch.
**Recommendation: close PR #20 as superseded by PR #30. Do not merge.**

---

## Gate checklist

- [x] SW-3 QA PASS — `npm ci`, `npm audit` (0 vulns), `npm run build` PASS, 103/105 frontend tests (2 pre-existing env failures); advisory cleared; scope guard confirmed
- [x] SW-4 Code Review PASS — no blocking findings; diff scope limited to package files + OpenSpec artifacts
- [x] SW-5 Security PASS — OSV/npm-audit/Trivy 0, gitleaks 0 leaks, semgrep 0 ERROR; CVSS 8.2 HIGH cleared; `--omit=dev` audit 0
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-043/v0.6.1` — **pending SSH private key** (dry-run record above)
- [x] PR #30 promoted from draft to ready-for-review
- [ ] PR #20 closed as superseded — **action required**
