# TASK-049 — v0.8.0 version/docs metadata + macOS DMG install-artifact cleanup

## Why

TASK-048 (PR #36) shipped the zero-permission active-app + idle capture loop, but its
post-merge code-to-spec **DIVERGED**: the v0.8.0 README/RELEASE/feature-doc changes were
prepared only in the dirty working tree and never landed on `main`, and the build version
fields were never bumped. Two visible drifts remain:

1. **Version drift (verified — `design.md` §1).** `src-tauri/tauri.conf.json` (`version`) and
   `src-tauri/Cargo.toml` (`[package].version`) still say `0.1.0`, so a fresh build stamps
   `CFBundleShortVersionString=0.1.0` / `CFBundleVersion=0.1.0` and ships
   `Vire_0.1.0_aarch64.dmg` — while the docs say **v0.8.0**. `README.md:79` also hardcodes the
   `Vire_0.1.0_<arch>.dmg` artifact name.
2. **Docs not landed (verified).** The v0.8.0 `README.md` current-version line + privacy-status
   update + env-var table + compatibility note, the `RELEASE.md` v0.8.0 entry, and
   `docs/active-window-capture.md` exist only in the dirty working tree, not on `main`.

Separately, Janne reports that after building and moving `Vire.app` to `/Applications`, Finder
shows a **faded/grey extra icon** named like `.VolumeIcon.icns` / `.icon`. Verified root cause
(mounted the produced DMG — `design.md` §2): the DMG volume root carries hidden
`.VolumeIcon.icns` (byte-identical to the app icon, sha `241793f0…`) and `.DS_Store`. These are
standard Tauri/`create-dmg` volume metadata; they are visible only when Finder *Show hidden
files* is on, and are copied into `/Applications` if the user Select-All-drags the DMG window
instead of dragging only `Vire.app`. Tauri v2 exposes **no config option to suppress** them
(tracking issues #9253 and #11190 both **OPEN** — `design.md` §3).

This is a **release-hygiene / packaging-metadata change only**. No feature, UI, capture, schema,
IPC, capability, or CSP change.

## What Changes

- **Version sync.** Set `version` in `src-tauri/tauri.conf.json` and `[package].version` in
  `src-tauri/Cargo.toml` to **`0.8.0`** so the built `.app`/`.dmg` version metadata matches the
  documented release; fix the hardcoded artifact name in `README.md:79`.
- **Land the v0.8.0 docs** prepared by TASK-048: `README.md` (current-version line, privacy
  status, env-var table, compatibility note), `RELEASE.md` (v0.8.0 entry), and
  `docs/active-window-capture.md`.
- **Install-artifact UX.** Update the README "Build and run the packaged app" install steps to
  instruct dragging **only `Vire.app`** onto the Applications symlink (never Select-All) and to
  explain that `.VolumeIcon.icns` / `.DS_Store` are normal hidden DMG metadata, not an error —
  visible only when *Show hidden files* is enabled. Add the supported `bundle.macOS.dmg`
  window-layout config (`windowSize` / `appPosition` / `applicationFolderPosition`) so the
  intended single-app drag is unambiguous.

## Scope

In scope: `src-tauri/tauri.conf.json` (version + dmg window layout), `src-tauri/Cargo.toml`
(version), `README.md`, `RELEASE.md`, `docs/active-window-capture.md`, and this OpenSpec change.

Out of scope (and **not available** via Tauri v2 config — `design.md` §3): removing or renaming
the DMG `.VolumeIcon.icns`; a custom post-bundle DMG re-pack to delete hidden files; code
signing / notarization; any source, schema, IPC, capability, CSP, capture, or UI change;
`Cargo.lock` dependency changes; the in-app updater (TASK-042) or in-app capture UI.

## Recommendation

**PASS — single coherent task, no split, no BA escalation.** All three workstreams are
metadata/docs/packaging for the same v0.8.0 release cut and touch overlapping files
(`README.md`); splitting would fragment one release-hygiene change. No component-boundary
crossing per `03_architecture_plan.md`; no BA-architecture divergence. Implementation owner:
**SW-frontend/devops** (text/TOML/JSON edits + one packaged-build verification).
