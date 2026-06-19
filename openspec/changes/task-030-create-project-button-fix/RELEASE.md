# Release — TASK-030 Create Project button fix (in-app input for env create-and-map)

- **Task:** TASK-030
- **Tier:** L2 project (this task L1-equivalent: frontend-only, no new backend/egress/deps)
- **Branch:** `feat/task-030-create-project-button-fix`
- **PR:** #25
- **Tag:** `task-030/v0.3.1` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `db29bae`
- **Base version:** v0.3.0 (TASK-029)
- **Date:** 2026-06-19
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

Replaced the `window.prompt()` dialog in the environment → project mapping panel with an in-app
`<input data-create-name>` element + **Create & map** button. Native `prompt()` returns `null`
silently in the macOS WKWebView (packaged app), making the Create Project action a no-op in
production. The inline input is pre-filled with the environment name (`maxlength="120"`) and
persists in the DOM so the handler can read it directly.

**Files changed (3):**

- `src/env-mapping-ui.ts` — `mappingRow()`: replaced single button with input + button pair;
  environment name is HTML-escaped via `escapeHtml` for all attribute slots (XSS closed).
- `src/main.ts` — `bindEnvMapping()`: `[data-create-map]` handler reads `data-create-name` input
  value (trimmed); empty name rejected client-side (`alert` + refocus); IPC sequence unchanged
  (`create_project` → `set_env_mapping` → `rerender()`); selector uses `CSS.escape` (injection closed).
- `tests/envMappingUi.test.mjs` — markup assertions for the new input/button pair (11/11 pass).

No backend change. No new IPC command. No config or dependency change. No DB schema change.

---

## Required declarations

### 1. Deployment size: patch

Frontend-only UI fix. Three source/test files changed. No new runtime dependency (Cargo.toml,
Cargo.lock, and package-lock.json unchanged across this diff — zero output from
`git diff 9de66e5..2c86111 -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json`).
No breaking change. No DB schema change. No new IPC command or capability.

### 2. Rollback strategy: partial-automated

All rollback layers are schema-free for this patch (no new DB table or settings row).

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-029 `.app` — no cleanup step needed | ✅ Automated |
| DB schema | No new table or settings row in this patch; TASK-029 build unaffected | ✅ Automated (N/A) |
| env-mapping UI | TASK-029 build renders the old single-button row (relies on `prompt()`) | ✅ Automated |
| IPC sequence | Unchanged; `create_project` + `set_env_mapping` both registered in TASK-029 build | ✅ Automated |
| Forward-path (un-rollback) | Reinstall the TASK-030 `.app` — no migration step; DB intact | ✅ Automated |

**Rollback note:** macOS packaged `.app` reinstall is the rollback mechanism; no automated CI/CD
rollback pipeline exists (TASK-028 updater is pending). Full packaged-app smoke (T6) requires a
physical Mac and remains outstanding as a human-only UAT step (documented in `tasks.md`). The
rollback to v0.3.0 is safe at code/DB level; T6 manual verification is the outstanding gate.

Rated **partial-automated** (consistent with prior releases) because no automated deployment
pipeline exists for this desktop app.

### 3. Component compatibility matrix

No new runtime dependency introduced. The matrix inherits v0.3.0 (TASK-029) unchanged; confirmed
by `git diff 9de66e5..2c86111 -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json`
returning no output.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; 142 Rust tests pass |
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
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | No new endpoint or egress host; unchanged |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-029 | — | No new table or column; fully compatible |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment (`~/.ssh/id_ed25519: No such file`) — same
constraint as TASK-029, TASK-027, and TASK-026 (see prior release artifacts).

**Dry-run record:** the intended tag is `task-030/v0.3.1` (annotated, SSH-signed) on commit
`db29bae`. Role definition prohibits falling back to an unsigned tag at L2+. Action required
before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-030/v0.3.1 -m "release(task-030): v0.3.1 patch — in-app input for env create-and-map" db29bae
git push origin task-030/v0.3.1
```

### SBOM (L2 requirement)

No new dependency was introduced; the SBOM baseline is unchanged from v0.3.0. A full SBOM
regeneration is not required for a patch that changes zero Cargo or npm dependencies. If a fresh
SBOM is required per policy, generate with:

```
cargo cyclonedx --manifest-path src-tauri/Cargo.toml --format json -o openspec/changes/task-030-create-project-button-fix/sbom.json
```

---

## Pre-existing advisories (non-blocking for this patch)

- **`vite` ≥ 6.4.3** — GHSA-fx2h-pf6j-xcff (CVSS 8.2, HIGH), dev-only build dependency, not
  in shipped binary. Introduced before TASK-030; not introduced or worsened by this patch. Track
  at repo level (dedicated dep-bump task recommended).
- **`esbuild` ≥ 0.28.1** — GHSA-g7r4-m6w7-qqqr (CVSS 2.5), dev-only. Same posture.

---

## Gate checklist

- [x] SW-3 QA PASS — 11/11 focused env-mapping tests; 72/74 frontend (2 pre-existing unrelated);
  142/142 Rust; no `prompt()` in `src/`
- [x] SW-4 Code Review PASS — no blocking findings; two non-blocking suggestions logged in `review.md`
- [x] SW-5 Security PASS — XSS/attribute-injection closed; no new IPC/egress/dep; gitleaks clean;
  semgrep 0 findings; OSV delta: zero new vulns
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-030/v0.3.1` — **pending SSH private key** (dry-run record above)
- [x] PR #25 promoted from draft to ready-for-review
- [ ] Packaged macOS T6 manual UAT — **outstanding, human-only** (see `tasks.md`)

---

## Merge ordering

PR #24 (recovery: `fix/task-029-release-docs-recovery`) — **already merged to main** on 2026-06-19.
PR #25 (`feat/task-030-create-project-button-fix`) — base branch updated to `main`; ready to merge.
No merge ordering dependency remains. Janne merges PR #25 at will.

**Note on PR #25 diff:** The branch carries TASK-029 gate-artifact commits (handoff.md,
qa.md updates — created locally after PR #23/24 were merged, not included in those PRs). These are
visible in the PR diff as branch noise. The actual TASK-030 production change is commits
`2c86111` and `db29bae`. No rebasing is strictly required for merge correctness, but a rebase
onto origin/main would produce a cleaner diff.
