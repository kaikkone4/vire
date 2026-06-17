# Release — TASK-027 Langfuse import automation + environment mapping + desktop UX polish

- **Task:** TASK-027
- **Tier:** L2
- **Branch:** `feat/task-027-langfuse-import-env-mapping-ux-polish`
- **PR:** #22
- **Tag:** `task-027/v0.2.0`
- **Head commit:** `7601811` (`76018119f8f0cc5085c641fd3a7daa7af1bb8199`)
- **Date:** 2026-06-17
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## Required declarations

### 1. Deployment size: minor

New features shipped; no breaking changes; no destructive schema migrations; no new runtime dependency
(Cargo.lock and package-lock.json unchanged by TASK-027).

Changes included (Workstreams A–E; Workstream F split to TASK-028 per DEC-029):

- **A — Import report and schema_changed fix:** `ImportReport`/`EnvImportLine` thread
  pages/seen/unique/duplicates/skipped_schema counts to the UI; `usageDetails`/`costDetails` (current
  Langfuse payload shape) parsed alongside the legacy `usage`/`calculatedTotalCost` path so present
  token/cost is captured instead of degrading to `schema_changed`; per-environment health surfaced
  (never blank on empty/partial import); absence-≠-zero invariant preserved.
- **B — Automatic import:** startup import after `init_db` + periodic import (default 900 s,
  `VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS` configurable, floored at 30 s); shared `Arc<Mutex<()>>`
  serialises auto and manual imports; disabled short-circuit reads no Keychain, no network.
- **C — Environment discovery and picker:** `GET /api/public/traces` (no `environment` filter) collects
  distinct environment names during every import; new `langfuse_discovered_environments` table
  (`environment PK`, `first_seen`, `last_seen`); Settings env field replaced with a checkbox picker
  seeded from discovered envs (CSV advanced fallback preserved; `vire` always offered).
- **D — Environment → project mapping:** new `langfuse_env_project_map` table
  (`environment PK`, `project_id FK→projects`, `created_at`, `updated_at`); mapping panel in Settings
  shows discovered envs as mapped/unmapped; explicit "Create project for `<env>`" action in one step;
  evidence association is a read-time JOIN (no data rewrite on remap); no auto-create, no auto-map
  (DEC-006).
- **E — Desktop UX polish:** fake macOS traffic-light buttons removed from the titlebar (native controls
  only); CSS grid rebalanced (brand centred, version right-aligned); icon placeholder regenerated with
  `SAFE=0.8` (~80% canvas inset, Dock-parity scale).

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-026 `.app` — no cleanup step needed | ✅ Automated |
| `langfuse_discovered_environments` table | Created by `init_db` (idempotent); TASK-026 build ignores it — no schema conflict, no data loss | ✅ Automated (schema-free at rollback) |
| `langfuse_env_project_map` table | Created by `init_db` (idempotent); TASK-026 build ignores it — no data loss, env field reverts to hand-typed CSV | ✅ Automated (schema-free at rollback) |
| Import diagnostics (A) | TASK-026 build shows the prior coarse health enum only — no data loss, counts not surfaced | ✅ Automated |
| Auto-import thread (B) | Not started by TASK-026 build; import is manual-only again on rollback | ✅ Automated |
| Settings / Keychain | Unchanged from TASK-026; rollback has no effect on Keychain entries | ✅ Automated |
| Forward-path (un-rollback) | Reinstall the TASK-027 `.app` — both new tables are intact, discovery/mapping state preserved | ✅ Automated |

**Rollback tested (staging precondition):** QA scenario matrix (`qa.md`) verifies idempotent `init_db`,
additive discovery persistence, mapping CRUD, and disabled-import short-circuit. Full macOS smoke
(packaged `.app`, Dock icon scale, native controls, startup import, env mapping E2E) is documented in
`design.md §9` and must be completed on a physical Mac before any prod distribution.

### 3. Component compatibility matrix

No new runtime dependency was introduced (Cargo.lock and package-lock.json are byte-for-byte unchanged
in the TASK-027 diff — verified by `git diff df263ab..HEAD -- src-tauri/Cargo.lock package-lock.json`
returning no output). The matrix extends TASK-026's.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged from TASK-026 |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; `cargo build` clean, zero warnings |
| **tauri** (Rust crate) | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged from TASK-026 |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged from TASK-026 |
| **keyring** | 3.0 | 3.x | `apple-native` only; carried from TASK-026; no new usage in TASK-027 |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); new tables use same additive posture |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; new `TracesAllEnvironments` path uses same client |
| **serde / serde_json** | 1.0 | 1.x | |
| **chrono** | 0.4 | 0.4.x | |
| **uuid** | 1.0 | 1.x | |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped in `.app` |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; used for `tauri build` / `tauri icon` |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings in renderer (shipped) |
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | `/api/public/traces` (no `environment` filter for discovery); base URL user-configurable |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged from TASK-026 |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-022 | — | two new additive tables; no destructive migration |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `tag.gpgsign = true`,
`user.signingkey = ~/.ssh/id_ed25519.pub`). The key file is absent from this environment
(`~/.ssh/id_ed25519.pub: No such file or directory`) — same constraint as TASK-026 (see
`task-026-desktop-production-readiness/RELEASE.md §L2 artifact expectations`).

**Dry-run record:** the intended tag is `task-027/v0.2.0` (annotated, SSH-signed) on commit `7601811`
(`76018119f8f0cc5085c641fd3a7daa7af1bb8199`). The role definition prohibits falling back to an
unsigned tag at L2+. Action required before this release is finalized: restore the SSH signing key and
run:

```sh
git tag -s task-027/v0.2.0 -m "task-027: Langfuse import automation, env discovery/mapping, desktop UX polish (v0.2.0, minor, partial-automated rollback)"
git push origin task-027/v0.2.0
```

Verify with: `git tag -v task-027/v0.2.0`.

**This is a local prototype — no production deployment pending.** The signing gap does not block PR
review or merge; it must be resolved before any distribution artifact is published.

### SBOM

No formal SBOM generated for this release. No new dependency was introduced; the existing
`src-tauri/Cargo.lock` (492 packages) and `package-lock.json` (106 packages) remain the machine-readable
manifests. OSV-scanner 2.3.8 ran against both at SW-5 (`sec.md §2`). A toolchain-generated SBOM
(`cargo-sbom` / `syft`) is recommended before any L3 or public distribution milestone — same standing
recommendation as TASK-026.

### Signed binary / notarization

The packaged `.app` is not code-signed or notarized (out of scope for this L2 prototype; documented in
root `RELEASE.md §Known limitations`). Unchanged from TASK-026.

### Tested rollback

Verified at SW-3 QA level: idempotent `init_db`, additive `discovered_environments`/`env_project_map`
tables, remap-does-not-destroy-evidence contract (`evidence_is_associated_to_a_project_at_read_time_without_rewrite`),
disabled-auto-import short-circuit, and import-slot serialisation. Full macOS smoke (including packaged
`.app` launch, Dock icon scale, native window controls, startup import, env mapping) requires a physical
Mac and is documented in `design.md §9`.

---

## Pre-existing issues (classified non-blocking)

| Issue | Classification | Recommended action |
|---|---|---|
| `tests/pi-observe.security.test.mjs` — 2 failures (sandbox EPERM on 127.0.0.1 bind) | Pre-existing; file unchanged from `main` in all TASK-027 commits (`git diff --name-only df263ab..HEAD -- tests/pi-observe.security.test.mjs` returns empty); tests the `pi-observe` CLI, not the Vire Tauri app | Separate issue against pi-observe observability tooling; not a TASK-027 blocker |
| `vite` 6.4.2 GHSA-fx2h-pf6j-xcff / CVE-2026-53571 (CVSS 8.2), `esbuild` GHSA-gv7w-rqvm-qjhr | Dev-server `devDependencies` only; absent from shipped `.app`; **not introduced by TASK-027** (package-lock.json unchanged in diff); pre-existing from TASK-026 | Follow-up housekeeping: `npm update vite` → latest 6.x (patches both CVEs via transitive esbuild bump) |
| `glib` 0.18.5 RUSTSEC-2024-0429 (CVSS 5.3 Medium — below 7.0 L2 auto-fail) | Pre-existing Tauri framework transitive dep; not reachable on macOS target path | Track at Tauri framework upgrade level |
| ~16 no-CVSS RustSec advisories (GTK/unic transitive) | Pre-existing; GTK Linux backend unused on macOS | Advisory only |
| gitleaks working-tree false positives (3 hits in `src-tauri/target/` `.rmeta` build artifacts) | `target/` is git-ignored; history scan clean (0 leaks across all 5 TASK-027 commits); same keyboard-accelerator string as TASK-026 gitleaks note | Add `target/` exclusion to working-tree scan config to suppress noise |

---

## Gate summary

| Gate | Verdict | Artifact |
|---|---|---|
| SW-3 QA | PASS (123 Rust unit+adversarial; 51 frontend; TypeScript clean; build clean; `cargo fmt` clean — blocker resolved `02f25c6`) | `qa.md` |
| SW-4 Code Review | PASS (A–E within approved boundaries; no CSP/capabilities/updater drift; RELEASE.md blocker resolved `7601811`) | `review.md` |
| SW-5 Security | PASS (gitleaks 0 history leaks; semgrep 0 ERRORs; Trivy 0 HIGH/CRIT; no in-artifact CVE ≥ 7.0; SEC-010 import-report secret-free verified) | `sec.md` |
| SW-6 Release | **PASS** | this document |

**Release approval = Janne merging PR #22.**
