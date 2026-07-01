# Spec delta — release-distribution

## ADDED Requirements

### Requirement: A published release is installable via an attached macOS DMG asset

Each published Vire release SHALL carry a downloadable macOS **DMG asset** for the supported
architecture, named per the Tauri v2 macOS bundle convention `{productName}_{version}_{arch}.dmg`
(for v0.8.1: `Vire_0.8.1_aarch64.dmg`). The asset SHALL be the disk image produced by
`npm run tauri:build` (`src-tauri/target/release/bundle/dmg/`) for the tagged commit, and its
embedded app version SHALL match the release tag. The release SHALL NOT rely on build-from-source as
the only install path. The DMG MAY be unsigned/un-notarized; when so, the first-launch Gatekeeper
step SHALL be documented rather than worked around.

#### Scenario: A user installs the published release without building from source

- **WHEN** a user opens the `v0.8.1` GitHub Release page
- **THEN** a downloadable `Vire_0.8.1_aarch64.dmg` asset is listed and downloadable
- **AND** mounting it and dragging **only `Vire.app`** to `/Applications` yields a launchable app
- **AND** on first launch the unsigned-app Gatekeeper prompt is resolved via right-click → Open
  (documented), after which the app runs with no dev server.

#### Scenario: The attached asset matches the release version

- **WHEN** the DMG for a release tag `v<X.Y.Z>` is attached
- **THEN** the packaged `Vire.app` inside reports `CFBundleShortVersionString = <X.Y.Z>`
- **AND** the asset file name encodes the same `<X.Y.Z>` and the `aarch64` architecture.

### Requirement: The download/install path is documented for prebuilt releases

The repo SHALL document a **"Download & install (prebuilt)"** path in `README.md` that directs users
to the GitHub Releases page, names the DMG asset, and describes mount → drag-only-`Vire.app` →
first-launch Gatekeeper right-click → Open. Build-from-source SHALL remain documented as the
alternative, not the primary, install path.

#### Scenario: A reader finds how to install without a toolchain

- **WHEN** a reader opens `README.md`
- **THEN** a "Download & install (prebuilt)" subsection precedes the build-from-source flow
- **AND** it links to the Releases page and names `Vire_<version>_aarch64.dmg`
- **AND** it states the app is unsigned and gives the Gatekeeper first-launch step.

### Requirement: A release is verified by a repeatable smoke checklist

`RELEASE.md` SHALL contain a **release smoke checklist** that verifies, for the release, that: the
DMG asset is present and downloadable; the app installs and launches from the DMG; the app runs
without a dev server; and the in-app **Check for updates** action returns *up to date* when the
installed build equals the latest release tag. The checklist SHALL record the shipped artifact's
sha256 for traceability.

#### Scenario: The release is verified rather than assumed

- **WHEN** the smoke checklist is run against `v0.8.1`
- **THEN** the DMG asset is confirmed present and downloadable on the release
- **AND** the installed app launches and runs without `npm run tauri:dev`
- **AND** **Settings → Check for updates** returns *up to date* (the check reads the release
  `tag_name`, independent of whether an asset is attached), confirming the version/update path
- **AND** the shipped DMG's sha256 is recorded.
