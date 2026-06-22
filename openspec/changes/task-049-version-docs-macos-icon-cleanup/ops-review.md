# Ops Review — TASK-049 v0.8.0 version/docs + macOS DMG install-artifact cleanup

- **Role**: SW-2 DevOps · **Date**: 2026-06-22 · **Branch**: `feat/task-049-version-docs-macos-icon-cleanup` (off `main` @ `8c45c0b`)
- **Type**: release-hygiene — packaging metadata + docs only. No shipped source, schema, IPC, capability, CSP, or third-party dependency change.

## What changed

| File | Change |
|---|---|
| `src-tauri/tauri.conf.json` | `version` `0.1.0` → `0.8.0`; added `bundle.macOS.dmg` window layout (`windowSize 660×400`, `appPosition 180,170`, `applicationFolderPosition 480,170`). |
| `src-tauri/Cargo.toml` | `[package].version` `0.1.0` → `0.8.0`. |
| `src-tauri/Cargo.lock` | Only the `vire` self-version line `0.1.0` → `0.8.0` (build-regenerated; no dependency delta). |
| `README.md` | v0.8.0 current-version line, active-app/idle env-var table, privacy-status block, compatibility note (carried from TASK-048); artifact name de-hardcoded to `Vire_<version>_<arch>.dmg`; install step rewritten to drag **only** `Vire.app` + hidden-file (`.VolumeIcon.icns`/`.DS_Store`) explanation. |
| `RELEASE.md` | v0.8.0 release entry (carried from TASK-048). |
| `docs/active-window-capture.md` | New capture feature doc (carried from TASK-048). |

## Build evidence (real packaged build)

`npm run tauri:build` — **exit 0**, finished in ~25 s (incremental release compile).

| Check | Result |
|---|---|
| DMG filename | `src-tauri/target/release/bundle/dmg/Vire_0.8.0_aarch64.dmg` |
| `Info.plist CFBundleShortVersionString` | `0.8.0` (PlistBuddy) |
| `Info.plist CFBundleVersion` | `0.8.0` (PlistBuddy) |
| DMG root (mounted read-only, hidden shown) | `Vire.app`, `Applications -> /Applications`, `.VolumeIcon.icns` (93 455 B), `.DS_Store` (6 148 B — curated layout from the new config) |

The `.VolumeIcon.icns` / `.DS_Store` hidden files remain as designed — Tauri v2 has no config to suppress them (tauri#9253, tauri#11190, both OPEN). The README install step now explains they are normal DMG metadata and instructs dragging only `Vire.app` so nothing leaks into `/Applications`. No post-bundle re-pack attempted (explicitly out of scope).

## Verification gate (tasks.md V1–V6)

- **V1** `git grep '0\.1\.0'` over version fields → no matches. ✅
- **V2** Build version-correct (DMG name + both plist keys = `0.8.0`). ✅
- **V3** Docs present and consistent (README v0.8.0 line, de-hardcoded artifact, privacy block; RELEASE v0.8.0 entry; `docs/active-window-capture.md` present). ✅
- **V4** DMG layout sane (single-app drag target); hidden files explained in docs. ✅
- **V5** No out-of-scope delta — `Cargo.lock` diff is the single `vire` self-version line; `src-tauri/src/**` diff empty; `tauri.conf.json` diff is only `version` + the `bundle.macOS.dmg` block. ✅
- **V6** `openspec validate task-049-version-docs-macos-icon-cleanup --strict` → valid. ✅

## NFR / operational notes

- No runtime code path, startup, capture behavior, or schema change. `.app` payload is byte-equivalent apart from the version string; the DMG layout is cosmetic.
- Build remains **unsigned / un-notarized** — the existing Gatekeeper note in README stands; code signing is a separate, deferred concern (out of scope here).
- TASK-047 advisory posture (`deny.toml`, target-scoped) unaffected — no crate added or version-changed.

## Handoff

→ **SW-3 (QA)**: re-verify V1–V6 against a clean packaged build on a physical Mac (confirm `Vire_0.8.0_aarch64.dmg`, Info.plist `0.8.0`, and the drag-only-the-app DMG layout). → **Release Manager (SW-6)**: v0.8.0 artifact (`Vire_0.8.0_aarch64.dmg`) is version-correct and ready as the release cut.
