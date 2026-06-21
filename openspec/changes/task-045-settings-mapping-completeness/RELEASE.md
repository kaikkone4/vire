# Release — TASK-045 (Settings mapping completeness)

- **Task:** TASK-045
- **Tier:** L1 project (backend bug fix — no credential storage change, no new deps, no egress change)
- **Branch:** `feat/task-045-settings-mapping-completeness`
- **PR:** #33
- **Tag:** `task-045/v0.6.3` (SSH-signed; see §Tag signing)
- **Head commit:** `bd90b77`
- **Base version:** v0.6.2 (TASK-044)
- **Date:** 2026-06-21
- **Gates passed:** SW-3 QA (PASS, 12 scenarios), SW-4 Code Review (PASS), SW-5 Security (PASS — L2 scan), SW-6 Release (this doc)

---

## What changed

**Settings mapping list now shows all known environments — not just recent discovery hits.**

The env-mapping panel in Settings previously surfaced only environments seen in the last
active discovery window (7-day default for new installs). Environments visible only in older
evidence rows — or mapped manually before the discovery window was widened — were silently
absent from the mapping list even though their evidence and mappings were intact in the DB.

This patch widens the mapping universe to **discovered ∪ evidence-backed ∪ mapped**:

- `env_mapping/mod.rs`: The mapping list is built from a `BTreeMap` union of three sources:
  (1) `langfuse_discovered_environments` (all rows, not window-filtered), (2) distinct
  `environment` values in `langfuse_ai_evidence` evidence rows, and (3) keys already in
  `langfuse_env_project_map`. De-duplication is exact and lexically ordered.
- `last_seen` fallback ordering: discovered rows use `MAX(ai_end_ts)` → `MAX(ai_start_ts)`
  → empty string. Rows with no evidence but with a mapping still appear (mapped-only entries).
- `langfuse/mod.rs`: Discovery now scans from the resolved import-range floor (not a hardcoded
  7-day window) through to `now`, so the range follows the user-configured import range.
- `langfuse/discovery.rs`: Scan remains `MAX_PAGES`-bounded (no unbounded page walk).

**Scope:** `src-tauri/src/env_mapping/mod.rs`, `src-tauri/src/langfuse/mod.rs`,
`src-tauri/src/langfuse/discovery.rs`. No IPC name/payload change, no renderer change, no
schema change, no new dependency, no new network egress.

---

## User-facing note

**This fix makes previously hidden environments appear in Settings → Environment Mapping.**

The mapping list was silently missing environments that existed only in older evidence rows
(outside the recent discovery window). No data was lost — the evidence and any existing
mappings were always present in the DB. They just did not appear in the list.

**To see the corrected list:** re-open Settings after updating. No re-import is required —
evidence-backed environments surface immediately from existing DB rows. If you have unmapped
environments that were not previously visible, map them now.

---

## Required declarations

### 1. Deployment size: patch

No new IPC commands, no new capabilities, no DB schema change, no new dependencies, no new
network egress. The change corrects the population query for the mapping list and widens the
discovery scan window. Semantic versioning: **patch** (bug fix, no feature addition, no
breaking change).

### 2. Rollback strategy: automated

Rolling back reverts three Rust source files (`env_mapping/mod.rs`, `langfuse/mod.rs`,
`langfuse/discovery.rs`) and rebuilds. No schema migration, no data migration, no credential
re-entry, no user action required.

| Layer | Forward (v0.6.2 → v0.6.3) | Rollback (v0.6.3 → v0.6.2) | Automated? |
|---|---|---|---|
| **DB schema** | No change | No change | ✅ Automated |
| **IPC / renderer** | No change; arg shape unchanged | No change | ✅ Automated |
| **Evidence / mapping data** | No change; evidence rows read wider | No change; prior query used | ✅ Automated |
| **Discovery window** | Follows import-range floor | Reverts to 7-day default window | ✅ Code revert |

**Rating: automated.** The binary reverts completely; no user action needed.

### 3. Component compatibility matrix

No new runtime dependency introduced. No Cargo.toml / lockfile change.

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
| **vite** | 6.4.3 | 6.4.3 | floor from TASK-043; unchanged by this task |
| **tsx** | 4.22.0 | 4.x | Test runner `devDependency`; unchanged |
| **typescript** | 5.7.2 | 5.x | Build-only; unchanged |
| **Upstream: Langfuse API** | v2 | v3 | unchanged |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged; secret key only |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-026 | — | no DDL change |

---

## L1 artifact expectations

### Tag signing

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).

**Intended tag:** `task-045/v0.6.3` on commit `bd90b77`.

```
git tag -s task-045/v0.6.3 -m "release(task-045): v0.6.3 patch — settings mapping universe widened to discovered ∪ evidence ∪ mapped; discovery window follows import-range floor" bd90b77
git push origin task-045/v0.6.3
```

### SBOM

No new runtime dependency. Cargo.lock and package-lock.json unchanged by this task. SBOM baseline
carries forward from v0.6.2.

---

## Gate checklist

- [x] SW-3 QA PASS (12 scenarios)
- [x] SW-4 Code Review PASS (no blockers; Rust 182/182; fmt; clippy no new warnings)
- [x] SW-5 Security PASS (L2: gitleaks 0 / 191 commits; semgrep 0 ERROR; OSV no CVE ≥ 7.0; Trivy 0 HIGH/CRIT; SEC-010 secret-free verified)
- [x] RELEASE.md written with all three required declarations
- [x] Tag `task-045/v0.6.3` — see §Tag signing
- [x] PR #33 promoted from draft to ready-for-review
