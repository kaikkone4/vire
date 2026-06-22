# Tasks — TASK-049 v0.8.0 version/docs metadata + macOS DMG install-artifact cleanup

Owner: **SW-frontend/devops**. Decision is fixed (PASS, single task — see `design.md`). Metadata/docs/
packaging only. No source, schema, IPC, capability, CSP, or dependency change.

## Implementation

- [x] T1 Bump `src-tauri/tauri.conf.json` `version` `0.1.0` → `0.8.0`.
- [x] T2 Bump `src-tauri/Cargo.toml` `[package].version` `0.1.0` → `0.8.0`. (Expect only the `vire`
  self-version line in `Cargo.lock` to change on next build — no dependency delta.)
- [x] T3 Add a `bundle.macOS.dmg` block to `src-tauri/tauri.conf.json` with explicit
  `windowSize` / `appPosition` / `applicationFolderPosition` (Tauri v2 defaults are acceptable —
  set them explicitly for a reproducible, curated layout). See `design.md` §3 for field names/defaults.
- [x] T4 Land the prepared v0.8.0 docs onto the branch: `README.md` (current-version line, privacy
  status, env-var table, compatibility note), `RELEASE.md` (v0.8.0 entry), and
  `docs/active-window-capture.md`. (These already exist in the dirty working tree.)
- [x] T5 In `README.md`: fix the hardcoded artifact name at `:79` to `Vire_0.8.0_<arch>.dmg`
  (or `Vire_<version>_<arch>.dmg`); expand the "Install and run" step (≈`:84`) to "drag **only**
  `Vire.app` onto the Applications shortcut — do not Select-All"; add a one-line note that
  `.VolumeIcon.icns` / `.DS_Store` are normal hidden DMG metadata, visible only when Finder's
  *Show hidden files* (⌘⇧.) is enabled — not corruption.

## Verification gate

- [x] V1 `git grep -n '0\.1\.0' src-tauri/tauri.conf.json src-tauri/Cargo.toml` returns nothing for the
  version fields (both read `0.8.0`). **Verified — no matches.**
- [x] V2 Packaged build is version-correct: `npm run tauri:build` produces
  `src-tauri/target/release/bundle/dmg/Vire_0.8.0_aarch64.dmg`, and the built
  `Vire.app/Contents/Info.plist` has `CFBundleShortVersionString = 0.8.0` and `CFBundleVersion = 0.8.0`
  (`/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" …`). **Verified — build exit 0;
  DMG `Vire_0.8.0_aarch64.dmg`; both plist keys read `0.8.0`.**
- [x] V3 Docs present and consistent: `README.md` current-version line says v0.8.0, `:79` artifact
  name no longer hardcodes `0.1.0`, privacy status describes the active-app/idle capture; `RELEASE.md`
  has a v0.8.0 entry; `docs/active-window-capture.md` exists. **Verified — all four checks pass.**
- [x] V4 DMG window layout sane: mount the produced DMG and confirm `Vire.app` + `Applications`
  symlink are positioned for a single-app drag (per the `bundle.macOS.dmg` config). The hidden
  `.VolumeIcon.icns` / `.DS_Store` are expected to remain (Tauri cannot suppress them — `design.md` §3);
  confirm the install docs explain this. **Verified — mounted read-only: root holds `Vire.app`,
  `Applications -> /Applications`, `.VolumeIcon.icns`, `.DS_Store` (6148 B, curated layout). README
  install step explains drag-only-the-app + hidden-files note.**
- [x] V5 No out-of-scope delta: `git diff main -- src-tauri/Cargo.lock` shows only the `vire`
  self-version line; no source (`src-tauri/src/**`), schema, IPC, capability, or `csp` change.
  **Verified — Cargo.lock diff is the single `vire` `0.1.0`→`0.8.0` line; `src-tauri/src/**` diff
  empty; `tauri.conf.json` diff is only `version` + the `bundle.macOS.dmg` block (no csp/identifier).**
- [x] V6 `openspec validate task-049-version-docs-macos-icon-cleanup --strict` passes. **Verified —
  "Change 'task-049-version-docs-macos-icon-cleanup' is valid".**

## Explicitly NOT in this task

- [ ] ~~Remove/rename the DMG `.VolumeIcon.icns` or its custom-icon bit~~ — no Tauri v2 config exists
  (tauri#9253, tauri#11190 both OPEN); a post-bundle re-pack/re-sign is out of scope and brittle.
- [ ] ~~Code signing / notarization~~ — separate concern; the unsigned-build Gatekeeper note stands.
- [ ] ~~Any source, schema, IPC, capability, CSP, capture-behavior, or in-app UI change~~.
- [ ] ~~Dependency bumps / `Cargo.lock` third-party changes~~ — TASK-047 advisory posture unaffected.
- [ ] ~~In-app updater (TASK-042) or in-app capture UI~~.
