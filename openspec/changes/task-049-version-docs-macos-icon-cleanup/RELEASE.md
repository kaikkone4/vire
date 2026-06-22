# RELEASE gate artifact — TASK-049 / v0.8.0

- **Role**: Release Manager (SW-6) · **Date**: 2026-06-22
- **Branch**: `feat/task-049-version-docs-macos-icon-cleanup` · **PR**: #37
- **Version**: v0.8.0 (packaging metadata component — see deployment size)
- **Tag**: `task-049/v0.8.0`
- **Tier**: L2

---

## Gate status

| Gate | Status |
|---|---|
| SW-3 QA | **PASS** (`qa.md`) |
| SW-4 Code Review | **PASS** (`review.md`) |
| SW-5 Security | **PASS** (`sec.md`, Tier L2) |
| CI advisory gate | **SUCCESS** (`cargo-deny advisories, Apple-target-scoped`, 2026-06-22T07:18:51Z) |

All SW-3/SW-4/SW-5 blockers closed (post-doc-fix recheck 2026-06-22). CI advisory gate ran on PR and passed.

---

## 1. Deployment size

**patch** — packaging metadata and documentation only. Zero source, schema, IPC, CSP,
capability, or third-party dependency change. Specifically:

- `src-tauri/tauri.conf.json`: `version` field `0.1.0→0.8.0`; cosmetic `bundle.macOS.dmg`
  window-layout block added (Finder icon positions only — no permission, signing, or executable
  semantics).
- `src-tauri/Cargo.toml`: `[package].version` `0.1.0→0.8.0`.
- `src-tauri/Cargo.lock`: single `vire` self-version line regenerated; dependency graph
  byte-identical to `main` (492 packages, no crate added/removed/version-changed).
- `README.md`, `RELEASE.md`, `docs/active-window-capture.md`: documentation only.

`git diff main -- src-tauri/src/` = 0 lines. The built macOS `.app` binary is functionally
equivalent to the TASK-048 build; only the embedded version string and DMG layout metadata differ.

---

## 2. Rollback strategy

**automated** — revert branch `feat/task-049-version-docs-macos-icon-cleanup`. No data
migration, no DDL change, no IPC surface change. Prior builds on the same Mac run against the
same `vire.sqlite` without conflict. The version string reverts to `0.1.0` in metadata only;
the macOS DMG layout block is cosmetic packaging metadata consumed only at `npm run tauri:build`
time and has no runtime effect on the installed `.app`.

---

## 3. Component compatibility matrix

| Component | Min version | Pinned / locked | Notes |
|---|---|---|---|
| macOS | Ventura 13+ | — | Unchanged; no new entitlement, capability, or TCC grant |
| Tauri runtime | 2.11.2 | 2.11.2 | `version` string updated; zero runtime API change; `Cargo.lock` graph byte-identical to main |
| Rust toolchain | stable | stable (cargo 1.95.0 tested) | No constraint change |
| `src-tauri/Cargo.toml` `version` | 0.8.0 | — | Self-version aligned to TASK-048 semantic version; no new crate dependency |
| SQLite DB (`vire.sqlite`) | v0.7.1+ schema | — | No DDL change; packaging metadata has no DB effect |
| IPC / CSP / capabilities | unchanged | — | No surface change; `tauri.conf.json` identifier `dev.vire.app` unchanged |
| npm / frontend deps | unchanged | unchanged | No `package.json` / `package-lock.json` delta |
| macOS DMG bundler | hdiutil (system) | — | `bundle.macOS.dmg` block consumed only at bundle time; layout cosmetic |
| `cargo-deny` CI gate | 0.19.9 | 0.19.9 | TASK-047 advisory gate unaffected; no crate version-changed |

Zero new Rust crates or npm packages added. Upstream advisory posture (TASK-047 `deny.toml`)
unchanged — Cargo.lock graph is byte-identical to main.

---

## Changed paths

| Path | Change |
|---|---|
| `src-tauri/tauri.conf.json` | `version` `0.1.0→0.8.0`; `bundle.macOS.dmg` window-layout block added |
| `src-tauri/Cargo.toml` | `[package].version` `0.1.0→0.8.0` |
| `src-tauri/Cargo.lock` | Single `vire` self-version line regenerated |
| `README.md` | v0.8.0 version line; de-hardcoded artifact name; drag-only install step; hidden-file (`.VolumeIcon.icns`/`.DS_Store`) explanation; active-app/idle env-var table; privacy-status block |
| `RELEASE.md` | v0.8.0 release entry (version/packaging subsection for TASK-049) |
| `docs/active-window-capture.md` | New TASK-048 capture feature documentation (doc correction applied: `window_title` column invariant, default-OFF wording) |
| `openspec/changes/task-049-version-docs-macos-icon-cleanup/*.md` | Gate artifacts: handoff, ops-review, proposal, qa, review, sec, tasks, arch-review, design, code-to-spec |

---

## Build artifact (SW-2 / ops-review evidence)

| Artifact | Value |
|---|---|
| DMG filename | `Vire_0.8.0_aarch64.dmg` |
| `CFBundleShortVersionString` | `0.8.0` |
| `CFBundleVersion` | `0.8.0` |
| DMG root (hidden shown) | `Vire.app`, `Applications → /Applications`, `.VolumeIcon.icns` (93 455 B), `.DS_Store` (6 148 B) |
| Build exit | 0 (~25 s incremental release compile) |

Build remains unsigned/un-notarized — unchanged from prior releases and documented in README
(Gatekeeper right-click→Open / Privacy & Security → Open Anyway). Code signing is a separate
deferred concern.

---

## Notes

- **Signed tag**: SSH signing key present (`/Users/kaikkonen/.ssh/id_ed25519.pub`,
  `gpg.format=ssh`, `tag.gpgsign=true`). Tag `task-049/v0.8.0` created after RELEASE.md commit.
- **CI**: `cargo-deny (advisories, Apple-target-scoped)` — SUCCESS (2026-06-22T07:18:51Z,
  run #27936199623). Only CI gate on this PR; passed.
- **Relation to TASK-048**: The root `RELEASE.md` v0.8.0 entry covers both TASK-048 (minor
  feature: zero-permission capture loop) and TASK-049 (packaging metadata). The overall v0.8.0
  deployment size is **minor** (driven by TASK-048); TASK-049's specific contribution is **patch**.
  This artifact records TASK-049's release unit independently per the one-release-per-TASK rule.
