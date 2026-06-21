# Release — TASK-044 (Langfuse public key → SQLite settings)

- **Task:** TASK-044
- **Tier:** L2 project (task L2 — credential-storage change, secrets audit required)
- **Branch:** `feat/task-044-keychain-public-key-to-settings`
- **PR:** #32
- **Tag:** `task-044/v0.6.2` (SSH-signed; see §Tag signing)
- **Head commit:** `f552b64`
- **Base version:** v0.6.1 (TASK-043)
- **Date:** 2026-06-21
- **Gates passed:** SW-3 QA (PASS ×2: initial + D1/D2 recheck), SW-4 Code Review (PASS after Architect fix), SW-5 Security (PASS — supersedes prior PASS-but-gated), SW-6 Release (this doc)

---

## What changed

**Credential storage refactor: Langfuse public key moved from macOS Keychain to SQLite settings table.**

The Langfuse Basic-Auth username (the "public key") is not a secret — it is the HTTP username
component of a key pair, not the password. It was held in the macOS Keychain for convenience
alongside the secret key, but this is unnecessary and produces a second Keychain access prompt.
This change stores the public key in the existing plaintext `settings` table, keeping the secret
key Keychain-only.

**Resolver (pair-level env fallback, DEC-026):** The credential resolver now treats the pair as a
unit. If both keys are present in storage the stored pair is used. If neither is present and both
`VIRE_LANGFUSE_PUBLIC_KEY` / `VIRE_LANGFUSE_SECRET_KEY` env vars (or `LANGFUSE_*` aliases) are
set, the env pair is used as a development override. Any one-store state (public in SQLite but
not secret in Keychain, or vice versa) resolves to `None` — the half-pair is inert and the
missing env side is not consulted to fill the gap.

**Two-store consistency contract (DEC-027):** `set` writes SQLite first; on Keychain failure the
prior SQLite value is restored (or the row deleted if it was absent). `clear` clears SQLite first
and aborts before Keychain mutation on SQLite failure; on Keychain-delete failure the captured
public key is restored to SQLite. All compensation failures surface as `INCONSISTENT_SET_ERR` /
`INCONSISTENT_CLEAR_ERR` (fixed strings, secret-free) — never swallowed.

**Scope:** `src-tauri/src/settings/mod.rs`, `src-tauri/src/settings/tests.rs`,
`src-tauri/src/lib.rs`. No IPC name/payload changes; renderer argument shape unchanged.

---

## Required declarations

### 1. Deployment size: patch

No new IPC commands, no new capabilities, no DB schema change (the `settings` table existed since
TASK-026), no new dependencies, no new network egress. The change relocates one credential
component between two already-present local stores. Semantic versioning: **patch** (security
posture improvement, no feature addition, no breaking schema change).

### 2. Rollback strategy: partial-automated

Code rollback is fully automated (revert commits, `cargo build`). However, the credential storage
location changes in both directions, so one credential re-save is required by the user after any
roll in either direction.

| Layer | Forward (v0.6.1 → v0.6.2) | Rollback (v0.6.2 → v0.6.1) | Automated? |
|---|---|---|---|
| **DB schema** | No change; `settings` table pre-existed | No change | ✅ Automated |
| **IPC / renderer** | No change; arg shape unchanged | No change | ✅ Automated |
| **Credential data** | Public key absent in SQLite on existing install → re-save once | Public key in SQLite but old code reads Keychain → re-save once | ⚠️ Manual (one re-save) |
| **Env override** | `VIRE/LANGFUSE_*` pair required for env-only dev setups | Single-key env fallback restored | Code revert only |

**Rating: partial-automated.** The binary reverts automatically; credential re-entry is required
for installs that stored credentials before v0.6.2 (either direction).

> **User-facing note — existing installs:** the Langfuse public key is not auto-migrated from
> Keychain to the settings table. On first launch after upgrade the app shows "no credentials"
> (the public key is absent from SQLite; the old Keychain entry is ignored by the new resolver).
> Open Settings → Langfuse and re-enter both keys once. This is a one-time action.

> **User-facing note — env var change (pairs only):** if you use env vars for dev overrides, both
> `VIRE_LANGFUSE_PUBLIC_KEY` and `VIRE_LANGFUSE_SECRET_KEY` (or `LANGFUSE_PUBLIC_KEY` /
> `LANGFUSE_SECRET_KEY`) must be set. A single env key alongside a stored key resolves to no
> credentials — the env fallback is a whole-pair override, not a per-field merge.

> **Pending verification:** the single fresh-launch macOS Keychain dialog (one prompt for the
> secret key on first use of a clean install) has not been manually observed on real macOS
> hardware in this release cycle. Functional path is covered by the unit test suite (33 settings
> tests PASS), but the dialog UX is pending manual verification.

### 3. Component compatibility matrix

No new runtime dependency introduced. No Cargo.toml / lockfile change.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; secret key remains Keychain-only |
| **rusqlite** | 0.32 | 0.32.x | `bundled`; public key now stored here |
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
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | secret key only; public key removed |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-026 | — | no DDL change; `settings` table pre-existed |

---

## L2 artifact expectations

### Tag signing

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).

**Intended tag:** `task-044/v0.6.2` on commit `f552b64`.

```
git tag -s task-044/v0.6.2 -m "release(task-044): v0.6.2 patch — public key moved from Keychain to SQLite settings (DEC-026 pair-level env fallback + DEC-027 two-store consistency)" f552b64
git push origin task-044/v0.6.2
```

### SBOM

No new runtime dependency. Cargo.lock and package-lock.json unchanged by this task. SBOM baseline
carries forward from v0.6.1. The change affects runtime data flow only (credential location), not
the compiled dependency graph.

---

## Gate checklist

- [x] SW-3 QA PASS (initial `8e50e0b` + D1/D2 recheck `f552b64`)
- [x] SW-4 Code Review PASS (after Architect fix `db2eeef`)
- [x] SW-5 Security PASS (supersedes prior PASS-but-gated; `db2eeef` resolves Architect escalation)
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-044/v0.6.2` — pending (see §Tag signing)
- [x] PR #32 promoted from draft to ready-for-review
- [ ] Manual macOS Keychain dialog verification — pending (non-blocking)
