# Spec delta — release-packaging

## ADDED Requirements

### Requirement: Build version metadata matches the documented release version

The macOS bundle SHALL be stamped with the same version the project documents as current. The
`version` field in `src-tauri/tauri.conf.json` and the `[package].version` field in
`src-tauri/Cargo.toml` SHALL equal the documented release version (`0.8.0` for this change), so
that the produced `Vire.app/Contents/Info.plist` carries `CFBundleShortVersionString` and
`CFBundleVersion` equal to that version and the produced disk image is named
`Vire_<version>_<arch>.dmg`. Documentation SHALL NOT hardcode a stale artifact version in the
build/install instructions.

#### Scenario: Packaged build is version-consistent

- **WHEN** `npm run tauri:build` is run on a clean checkout
- **THEN** the produced DMG is named `Vire_0.8.0_<arch>.dmg`
- **AND** the produced `Vire.app/Contents/Info.plist` has `CFBundleShortVersionString = 0.8.0`
  and `CFBundleVersion = 0.8.0`
- **AND** `README.md` reports the current version as v0.8.0 and does not name a `0.1.0` artifact.

### Requirement: Release documentation reflects the shipped v0.8.0 feature set

The repository SHALL carry, on the release branch, the v0.8.0 documentation describing the
zero-permission active-app and idle capture loop: a `README.md` current-version line and
privacy-status section, a `RELEASE.md` v0.8.0 entry, and a feature document at
`docs/active-window-capture.md`. The privacy documentation SHALL state that capture is default
OFF, uses only `NSWorkspace.frontmostApplication` and `CGEventSource` event age, requests no
Accessibility/Screen-Recording permission, and never stores a window title.

#### Scenario: v0.8.0 docs are present and accurate

- **WHEN** a reviewer reads the release branch
- **THEN** `README.md` states the current version is v0.8.0 and its privacy status describes the
  opt-in active-app/idle capture (default OFF, zero-permission, `window_title` always NULL)
- **AND** `RELEASE.md` contains a v0.8.0 entry for the capture loop
- **AND** `docs/active-window-capture.md` exists and documents the capture behavior and config.

### Requirement: The DMG install path is unambiguous and its hidden metadata is documented

The DMG bundle SHALL present a curated drag-to-Applications layout via the supported
`bundle.macOS.dmg` configuration (`windowSize`, `appPosition`, `applicationFolderPosition`), and
the install documentation SHALL instruct the user to drag **only** `Vire.app` onto the
Applications shortcut. The documentation SHALL explain that the DMG's hidden `.VolumeIcon.icns`
and `.DS_Store` entries are standard macOS volume metadata — visible only when Finder's *Show
hidden files* option is enabled — and not a defect. The change SHALL NOT attempt to remove or
rename `.VolumeIcon.icns`, because Tauri v2 provides no supported configuration to do so.

#### Scenario: Install instructions guide a single-app drag and explain hidden metadata

- **WHEN** a user follows the README "Build and run the packaged app" → install steps
- **THEN** the steps instruct dragging only `Vire.app` onto the Applications shortcut (not a
  Select-All of the DMG window)
- **AND** the docs note that `.VolumeIcon.icns` / `.DS_Store` are normal hidden DMG metadata shown
  only when *Show hidden files* is enabled, not corruption
- **AND** the produced DMG positions `Vire.app` and the Applications shortcut per the
  `bundle.macOS.dmg` layout.

#### Scenario: Volume-icon suppression is bounded out

- **WHEN** the change is reviewed for the "faded/grey icon" fix
- **THEN** the DMG `.VolumeIcon.icns` and the volume custom-icon attribute remain present (they are
  standard `create-dmg` output)
- **AND** no post-bundle re-pack or re-sign step is introduced to delete them, because Tauri v2
  exposes no configuration for it (tracking issues tauri-apps/tauri#9253 and #11190 are open).
