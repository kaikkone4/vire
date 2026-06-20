# Release — TASK-031 Settings scroll preservation + copy cleanup

- **Task:** TASK-031
- **Tier:** L2 project (this task L1-equivalent: frontend-only, no new backend/egress/deps)
- **Branch:** `feat/task-031-settings-scroll-preservation`
- **PR:** #26
- **Tag:** `task-031/v0.3.2` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `4a8ea20`
- **Base version:** v0.3.1 (TASK-030)
- **Date:** 2026-06-20
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

Fixed the Settings UX bug where pressing any control (Test connection, Save range, Import now,
Map, Save settings, …) scrolled the panel back to the top.

**Root cause:** `shell()` in `src/main.ts` is the single chokepoint that re-assigns
`app.innerHTML` on every render. Destroying and recreating the `<main>` scroll container reset
its `scrollTop` to `0` on every in-Settings action.

**Fix:** Capture the outgoing `<main>.scrollTop` and whether the view is unchanged
(`lastRenderedView === current`) before the innerHTML swap; restore on a same-view re-render,
reset to `0` on a view change, after the swap. Decision extracted to a pure, unit-tested helper
`nextScrollTop(sameView, prevScroll)` in `src/scroll.ts`.

**Copy cleanup (bounded, same view):** `env-mapping-ui.ts:97` help text leftover from TASK-030
(`use "Create project for …"`) corrected to `use "Create & map"` to match the button label.
No behaviour or markup change.

**Files changed (5 source/test):**

- `src/scroll.ts` (new) — pure helper `nextScrollTop(sameView, prevScroll): number`; side-effect-free.
- `src/main.ts` — `shell()`: captures `prevScroll`/`sameView` before innerHTML swap, restores after.
- `src/env-mapping-ui.ts` — help copy updated to match button label (`Create &amp; map`).
- `tests/scroll.test.mjs` (new) — 2 unit cases: same-view returns `prevScroll`, view-change returns `0`.
- `tests/envMappingUi.test.mjs` — 1 assertion added at line 112: verifies new help copy, rejects old.

No backend change. No new IPC command. No config or dependency change. No DB schema change.

---

## Required declarations

### 1. Deployment size: patch

Frontend-only UX bug fix. Five source/test files changed. No new runtime dependency
(`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `package.json`, and `package-lock.json`
untouched — confirmed by zero output from `git diff main...HEAD -- src-tauri/Cargo.toml
src-tauri/Cargo.lock package.json package-lock.json`). No breaking change. No DB schema change.
No new IPC command or capability.

### 2. Rollback strategy: partial-automated

All rollback layers are schema-free for this patch (no new DB table, no new settings row,
no new IPC command).

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-030 `.app` — no cleanup step needed | ✅ Automated |
| DB schema | No new table or settings row; TASK-030 build unaffected | ✅ Automated (N/A) |
| Scroll behaviour | TASK-030 build reverts to `scrollTop = 0` on re-render (old behaviour) | ✅ Automated |
| Copy in env-mapping panel | TASK-030 build shows old help text — cosmetic only, no data impact | ✅ Automated |
| IPC sequence | Unchanged; no new command to remove | ✅ Automated (N/A) |
| Forward-path (un-rollback) | Reinstall the TASK-031 `.app` — no migration; DB intact | ✅ Automated |

**Rollback note:** macOS packaged `.app` reinstall is the rollback mechanism; no automated
CI/CD rollback pipeline exists (TASK-028 updater is pending). Packaged-app smoke (manual UAT
M1–M3) requires a physical Mac and remains outstanding as a human-only step. The rollback to
v0.3.1 is safe at code/DB level.

Rated **partial-automated** (consistent with prior releases) because no automated deployment
pipeline exists for this desktop app.

### 3. Component compatibility matrix

No new runtime dependency introduced. The matrix inherits v0.3.1 (TASK-030) unchanged;
confirmed by zero diff to `Cargo.toml`, `Cargo.lock`, `package.json`, and `package-lock.json`.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; backend unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; unchanged |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); no new table |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; unchanged |
| **serde / serde_json** | 1.0 | 1.x | unchanged |
| **chrono** | 0.4 | 0.4.x | unchanged |
| **uuid** | 1.0 | 1.x | unchanged |
| **thiserror** | 2.0 | 2.x | unchanged |
| **url** | 2.0 | 2.x | unchanged |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; unchanged |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings; no new command added |
| **@tauri-apps/plugin-dialog** | 2.2.0 | 2.2.x | unchanged |
| **tsx** | 4.22.0 | 4.x | Test runner `devDependency`; unchanged |
| **typescript** | 5.7.2 | 5.x | Build-only; unchanged |
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | No new endpoint or egress host; unchanged |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-029 | — | No new table or column |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment (`~/.ssh/id_ed25519: No such file`) — same
constraint as TASK-030, TASK-029, TASK-027, and TASK-026 (see prior release artifacts).

**Dry-run record:** the intended tag is `task-031/v0.3.2` (annotated, SSH-signed) on commit
`4a8ea20`. Role definition prohibits falling back to an unsigned tag at L2+. Action required
before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-031/v0.3.2 -m "release(task-031): v0.3.2 patch — Settings scroll preservation + copy cleanup" 4a8ea20
git push origin task-031/v0.3.2
```

### SBOM (L2 requirement)

No new dependency was introduced; the SBOM baseline is unchanged from v0.3.1. A full SBOM
regeneration is not required for a patch that changes zero Cargo or npm dependencies. If a fresh
SBOM is required per policy, generate with:

```
cargo cyclonedx --manifest-path src-tauri/Cargo.toml --format json -o openspec/changes/task-031-settings-scroll-preservation/sbom.json
```

---

## Pre-existing advisories (non-blocking for this patch)

- **`vite` ≥ 6.4.3** — GHSA-fx2h-pf6j-xcff (CVSS 8.2, HIGH) + GHSA-v6wh-96g9-6wx3 (5.5 Med),
  dev-only build dependency, not in shipped binary. Pre-existing; not introduced or worsened by
  this patch. Track at repo level — dedicated dep-bump task required (see `sec.md` §Escalations).
- **`esbuild` ≥ 0.28.1** — GHSA-g7r4-m6w7-qqqr (CVSS 2.5 Low), dev-only. Same posture.

---

## Gate checklist

- [x] SW-3 QA PASS — 14/14 focused tests (scroll helper + envMappingUi); 73/75 frontend
  (2 pre-existing Langfuse-network failures unrelated); build clean
- [x] SW-4 Code Review PASS — no blocking findings; PR body confirmed adequate
- [x] SW-5 Security PASS — no XSS/IPC/secret/egress/dep regression; semgrep 0; gitleaks clean;
  Trivy 0 HIGH/CRITICAL runtime; pre-existing dev-dep advisories documented and escalated
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-031/v0.3.2` — **pending SSH private key** (dry-run record above)
- [x] PR #26 promoted from draft to ready-for-review
- [ ] Manual macOS UAT M1–M3 — **outstanding, human-only** (scroll DOM / webview-bound; see `qa.md`)

---

## Merge ordering

No outstanding dependency on prior PRs. PR #25 (TASK-030) is merged to `main`.
PR #26 (`feat/task-031-settings-scroll-preservation`) — base branch is `main`; ready to merge.
Janne merges PR #26 at will.
