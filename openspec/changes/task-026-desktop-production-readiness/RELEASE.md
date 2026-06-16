# Release — TASK-026 Desktop Production Readiness

- **Task:** TASK-026
- **Tier:** L2
- **Branch:** `feat/task-026-desktop-production-readiness`
- **PR:** #21
- **Tag:** `task-026/v0.1.0`
- **Date:** 2026-06-16
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## Required declarations

### 1. Deployment size: minor

New features shipped; no breaking changes; no destructive schema migrations.

Changes included:
- **Workstream A — In-app Langfuse settings with Keychain secret storage:** 5 new IPC commands (`get_langfuse_settings`, `set_langfuse_settings`, `set_langfuse_secret`, `clear_langfuse_secret`, `test_langfuse_connection`); new Settings panel; new Rust `settings` module; macOS Keychain-backed secret key (DEC-026 / SEC-009); additive `settings` key/value rows in SQLite; settings-first config resolver with env dev fallback.
- **Workstream B — Mac application icon:** Vire icon generated into `src-tauri/icons/` (incl. `icon.icns`); `bundle.icon` populated in `tauri.conf.json`. Placeholder mark; replacement path documented.
- **Workstream C — Production packaged build:** `npm run tauri:build` produces a self-contained `Vire.app` (and `.dmg` where toolchain supports it) that requires no Vite dev server at runtime. Packaging and rollback compatibility documented in README.

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the prior `.app` — no cleanup step needed | ✅ Automated |
| SQLite `settings` table | Additive rows stay in place; prior builds ignore unknown key/value rows and fall back to `VIRE_LANGFUSE_*` env vars | ✅ Automated (schema-free key/value) |
| Keychain entries | `dev.vire.app` entries (`langfuse_public_key`, `langfuse_secret_key`) persist across reinstall; prior builds do not read them (env-only) — no harm, no data leak | ⚠️ Manual cleanup if desired via Keychain Access.app or `security delete-generic-password -s dev.vire.app -a langfuse_public_key` |
| Forward-path (un-rollback) | Reinstall the TASK-026 `.app` — Keychain entries and `settings` rows are intact | ✅ Automated |

**Rollback tested:** QA scenario matrix (`qa.md §C`) verifies DB round-trip and the prior-build ignores-unknown-settings contract. Keychain-path rollback verified via the `failed_replacement_restores_the_prior_pair_and_never_mixes_keychain_with_env` regression test.

**Staging precondition (L2):** macOS smoke checklist (`qa.md §Manual smoke checklist`) must be completed on a physical Mac before prod release. The checklist covers: launch without dev server, Keychain set/clear cycle, Test connection coarse verdict, disabled-state behavior, and Keychain Access.app verification.

### 3. Component compatibility matrix

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; Keychain Services available on all macOS 11+ |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; `cargo build` verified clean, zero warnings |
| **tauri** (Rust crate) | 2.2 | 2.2.x | `src-tauri/Cargo.toml` |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | `src-tauri/Cargo.toml` |
| **keyring** (new — TASK-026) | 3.0 | 3.x | `apple-native` feature only; macOS Security framework; no Linux secret-service path compiled; no OSV advisory against this crate or its +5 transitive deps |
| **rusqlite** | 0.32 | 0.32.x | `bundled` feature (SQLite 3.46+ included) |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; no OpenSSL dependency |
| **serde / serde_json** | 1.0 | 1.x | |
| **chrono** | 0.4 | 0.4.x | `serde` + `clock` features |
| **uuid** | 1.0 | 1.x | `v4` feature |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped in `.app` |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; used for `tauri build` / `tauri icon` |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings in the renderer (shipped) |
| **vite** | 6.0.7 | 6.4.2 | Build tooling `devDependency`; **not in shipped `.app`** — see advisory |
| **Upstream: Langfuse API** | v2 | v3 | `/api/public/traces`; base URL user-configurable |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | via `keyring` 3.x `apple-native` |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-022 | — | additive `settings` key/value rows only; no destructive migration |

---

## L2 artifact expectations

### Tag signing (dry-run — signing unavailable in this environment)

Git is configured for SSH signing (`gpg.format = ssh`, `tag.gpgsign = true`, signing key declared as `~/.ssh/id_ed25519.pub`). However, the key file is **absent** from this environment (`~/.ssh/id_ed25519.pub: No such file or directory`) — the SSH signing key is misconfigured for this local session.

**Dry-run record:** the intended tag is `task-026/v0.1.0` (annotated, SSH-signed) on commit `734fbc8` (the SW-6 gate commit). The role definition prohibits falling back to an unsigned tag at L2+. Action required before this release is finalized: restore the SSH signing key to `~/.ssh/id_ed25519.pub` (or reconfigure `user.signingkey` to the correct path) and run:

```sh
git tag -s task-026/v0.1.0 -m "task-026: Desktop production readiness — Langfuse Keychain settings, Mac icon, production .app (minor, partial-automated rollback)"
git push origin task-026/v0.1.0
```

Verify with: `git tag -v task-026/v0.1.0` (requires the signing key in the verifier's `allowedSignersFile`).

**This is a local prototype — no production deployment pending.** The tag signing gap does not block PR review or merge; it must be resolved before any distribution artifact is published.

### SBOM

A formal SBOM (SPDX / CycloneDX) has not been generated for this prototype-phase release. The machine-readable dependency manifests are `src-tauri/Cargo.lock` (492 packages) and `package-lock.json` (106 packages); OSV-scanner 2.3.8 ran against both at SW-5 (`sec.md §0′`). A toolchain-generated SBOM (e.g. `cargo-sbom` or `syft`) is recommended before any L3 or public distribution milestone.

### Signed binary / notarization

The packaged `.app` is **not code-signed or notarized** (out of scope for this L2 prototype; documented in root `RELEASE.md §Known limitations`). On first launch macOS Gatekeeper may block — right-click → Open, or System Settings → Privacy & Security → Open Anyway. Signing and notarization are designated as a future milestone prerequisite before any public distribution.

### Tested rollback

Verified at SW-3 QA level via the DB scenario coverage matrix and the atomic Keychain pair regression test. Full macOS smoke (including Keychain Access.app verification) requires a physical Mac and is documented in the manual checklist (`qa.md §Manual smoke checklist`).

---

## Pre-existing issues (classified non-blocking)

| Issue | Classification | Recommended action |
|---|---|---|
| `tests/pi-observe.security.test.mjs` — 2 failures (sandbox EPERM on 127.0.0.1) | Pre-existing; file unchanged from `main`; tests the `pi-observe` CLI, not the Vire Tauri app; out of TASK-026 scope | Separate issue against the pi-observe observability tooling |
| DMG generation fails in non-interactive CI runner | Runner-specific; confirmed successful `.app`+`.dmg` build on macOS host (SW-4 `review.md §Verification`); `.app` is the primary artifact | Not a blocker; document runner caveat in CI config |
| `vite` 6.4.2 GHSA-fx2h-pf6j-xcff (CVSS 8.2) and `esbuild` GHSA-gv7w-rqvm-qjhr (CVSS 8.1) | Dev-server `devDependencies` only; absent from shipped `.app`; pre-existing (not introduced by TASK-026); confirmed non-blocking by SW-5 | Follow-up housekeeping: `npm update vite` → 6.4.3 (fixes both CVEs) |
| `glib` 0.18.5 RUSTSEC-2024-0429 (CVSS 6.9 Medium) | Pre-existing Tauri framework transitive dep; below 7.0 auto-fail threshold | Track at Tauri framework upgrade level |
| 16 no-CVSS RustSec advisories (GTK/unic transitive) | Pre-existing; mostly Linux backend (unused on macOS target) | Advisory only; no action for macOS-only builds |

---

## Gate summary

| Gate | Verdict | Artifact |
|---|---|---|
| SW-3 QA | PASS (2 recheck cycles; 94 Rust + 39 frontend tests) | `qa.md` |
| SW-4 Code Review | PASS (3 recheck cycles; all blockers resolved) | `review.md` |
| SW-5 Security | PASS (final recheck; semgrep 0, gitleaks 0, Trivy 0 HIGH/CRIT, no in-artifact CVE ≥ 7.0) | `sec.md` |
| SW-6 Release | **PASS** | this document |

**Release approval = Janne merging PR #21.**
