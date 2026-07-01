# Spec delta — release-distribution

## MODIFIED Requirements

### Requirement: A published release is installable via an attached macOS DMG asset

Each published Vire release SHALL carry a downloadable macOS **DMG asset** for the supported
architecture, named per the Tauri v2 convention `{productName}_{version}_{arch}.dmg` (for v0.8.1:
`Vire_0.8.1_aarch64.dmg`). The asset SHALL be the **pristine** disk image produced by
`npm run tauri:build` for the tagged commit (not manually re-packed), and its embedded app version
SHALL match the release tag. An attached asset SHALL be **openable by a user who downloads it via a
web browser and follows the documented install steps** — i.e. the download-and-open path SHALL NOT
dead-end at Gatekeeper's "damaged" rejection.

To satisfy this the release SHALL take one of:

- **(A) signed + notarized:** the app is signed with a Developer ID Application identity, notarized by
  Apple, and stapled, so a browser-downloaded (quarantined) copy opens on double-click; **or**
- **(B) unsigned + accurate docs:** the app is unsigned/ad-hoc and the install docs give the *working*
  first-launch remedy for the "damaged" dialog (removing the quarantine attribute), with a security
  caveat — not the ineffective right-click → Open guidance.

#### Scenario: A browser-downloaded release opens by following the docs

- **WHEN** a user downloads `Vire_<version>_aarch64.dmg` from the release page via a web browser
  (the file therefore carries `com.apple.quarantine`) and follows the documented install steps
- **THEN** the app launches — either directly (notarized build) or after the documented
  quarantine-removal step (unsigned build)
- **AND** the user is never left at the "is damaged and can't be opened" dialog with no documented way
  forward.

#### Scenario: The attached asset matches the release version and is a clean build

- **WHEN** the DMG for a release tag `v<X.Y.Z>` is attached
- **THEN** the packaged `Vire.app` inside reports `CFBundleShortVersionString = <X.Y.Z>`
- **AND** the asset is the untouched `tauri:build` output (its signature/volume-icon layout intact)
- **AND** its `sha256` is recorded in `RELEASE.md`.

### Requirement: The download/install path is documented for prebuilt releases

The repo SHALL document a **"Download & install (prebuilt)"** path in `README.md` that directs users
to the GitHub Releases page, names the DMG asset, and describes mount → drag-only-`Vire.app` →
first launch. The first-launch guidance SHALL **match the actual Gatekeeper failure mode of the
shipped build**: for a signed+notarized build, double-click opens; for an unsigned build, the docs
SHALL give the quarantine-removal remedy that clears the "damaged" dialog, with a security caveat.
The docs SHALL NOT instruct users to use right-click → Open as the remedy for the "damaged" dialog
(that path does not clear it for unsigned/ad-hoc quarantined apps).

#### Scenario: A reader finds an install remedy that actually works

- **WHEN** a reader opens `README.md` and follows the prebuilt install steps for the shipped build
- **THEN** the first-launch instructions resolve the actual dialog that build produces
- **AND** for an unsigned build the quarantine-removal step is documented with its security caveat and
  a pointer to the signed/notarized fix (Plan A / TASK-028).
