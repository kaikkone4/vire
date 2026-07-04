# Spec delta — in-app-update-check

## ADDED Requirements

### Requirement: The v0.8.2 release is detectable by an older build

The project SHALL publish a **full** (non-draft, non-prerelease) GitHub Release tagged `v0.8.2`
against the `main` commit that carries `0.8.2` metadata **and** the TASK-056 Settings-UI merge, and
the app's compiled version SHALL read `0.8.2` (`CARGO_PKG_VERSION`, mirrored in
`src-tauri/tauri.conf.json` and the `vire` entry of `src-tauri/Cargo.lock`). This operationalizes the
already-specified check for the TASK-056 build — it adds **no** update-check logic, network host,
capability, or auto-download/install feature; the check stays U-lite (detect → open Releases →
manual install).

#### Scenario: The previously distributed 0.8.1 build sees the update

- **WHEN** a build whose `CARGO_PKG_VERSION` is `0.8.1` runs "Check for updates" against the published
  `v0.8.2` release
- **THEN** the result is `UpdateAvailable` with `current = "0.8.1"`, `latest = "0.8.2"`, and the
  release's real `html_url` as `release_url`
- **AND** the app does not download or install anything automatically; the user reaches the release
  via "Open GitHub Releases" and installs manually.

#### Scenario: The rebuilt 0.8.2 build is up to date

- **WHEN** a build whose `CARGO_PKG_VERSION` is `0.8.2` runs "Check for updates" against the published
  `v0.8.2` release
- **THEN** the result is `UpToDate` with `current = "0.8.2"`
- **AND** the equal-version case is never reported as a downgrade or as `UpdateAvailable`.

#### Scenario: A draft or prerelease does not satisfy the requirement

- **WHEN** the only `v0.8.2` Release is marked draft or prerelease
- **THEN** `/releases/latest` still returns `v0.8.1` (the endpoint skips drafts/prereleases)
- **AND** the requirement is NOT met until a full `v0.8.2` release is published.

### Requirement: Each macOS Release attaches an installable aarch64 DMG with unsigned-launch guidance

For each shipped macOS version the published Release SHALL attach the built
`Vire_<version>_aarch64.dmg` disk image, and — because the build is unsigned and unnotarized — its
notes SHALL document the user-run Gatekeeper remedy so the app can be launched. The quarantine remedy
SHALL be user-run and per-app; no quarantine-bypass is embedded in the shipped artifact, and no
global-Gatekeeper-disable guidance is given. The only mechanism that removes the user step is a valid
Apple Developer ID signature plus notarization (tracked separately, TASK-028).

#### Scenario: The DMG asset is present and downloadable

- **WHEN** the `v0.8.2` Release is published
- **THEN** it has an attached, downloadable asset named `Vire_0.8.2_aarch64.dmg`
- **AND** "Open GitHub Releases" reaches a page that offers that install-ready binary.

#### Scenario: Notes carry the unsigned first-launch remedy, not a bypass

- **WHEN** a user reads the `v0.8.2` Release notes after macOS reports the app as "damaged"
- **THEN** the notes state the build is unsigned/unnotarized and give the exact command
  `xattr -dr com.apple.quarantine /Applications/Vire.app`, an official-source trust caveat, and a
  pointer to Developer ID signing + notarization (TASK-028) as the long-term fix
- **AND** the notes do NOT instruct `sudo`, `spctl --master-disable`, or any global Gatekeeper
  disable, and the DMG itself embeds no quarantine-clearing script.
