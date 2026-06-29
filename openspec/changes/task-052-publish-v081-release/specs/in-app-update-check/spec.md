# Spec delta — in-app-update-check

## ADDED Requirements

### Requirement: A published GitHub Release backs the latest-release check

For each shipped version the project SHALL publish a corresponding **non-draft, non-prerelease**
GitHub Release whose tag is the version prefixed with `v` (e.g. `v0.8.1`), so that
`https://api.github.com/repos/kaikkone4/vire/releases/latest` returns `200` and the in-app check
resolves to a real state instead of the fail-soft `Unknown` caused by an empty release list. The
GitHub `/releases/latest` endpoint excludes drafts and prereleases, so the Release MUST be a full
release for the check to observe it. Publishing the Release adds **no** update-check logic, network
host, capability, or feature; it operationalizes the already-specified check by giving it real data.

#### Scenario: Latest-release endpoint resolves after the Release is published

- **WHEN** a full (non-draft, non-prerelease) GitHub Release tagged `v0.8.1` is published on
  `kaikkone4/vire`
- **THEN** `https://api.github.com/repos/kaikkone4/vire/releases/latest` returns HTTP `200`
- **AND** its `tag_name` is `v0.8.1` and its `html_url` is the real release page
- **AND** the in-app "Check for updates" no longer fail-softs to `Unknown` for a "no release"
  reason.

#### Scenario: A draft or prerelease does not satisfy the requirement

- **WHEN** the only `v0.8.1` Release is marked draft or prerelease
- **THEN** `/releases/latest` still returns `404` (the endpoint skips it)
- **AND** the requirement is NOT met until a full release is published.

### Requirement: Compiled app version metadata matches the published release

The app's compiled version SHALL equal the published release version, so the semver comparison in
`compare_release` yields the correct `UpToDate` / `UpdateAvailable` verdict and never reports a
downgrade. That version is sourced from `CARGO_PKG_VERSION` (`src-tauri/Cargo.toml`) and mirrored in
`src-tauri/tauri.conf.json` and the `vire` entry of `src-tauri/Cargo.lock`. A build older than the
latest release MUST resolve to `UpdateAvailable`; a build at the latest release version MUST resolve
to `UpToDate`.

#### Scenario: Older build sees the update

- **WHEN** a build whose `CARGO_PKG_VERSION` is `0.8.0` runs "Check for updates" against a published
  `v0.8.1` release
- **THEN** the result is `UpdateAvailable` with `current = "0.8.0"`, `latest = "0.8.1"`, and the
  release's real `html_url` as `release_url`.

#### Scenario: Current build is up to date

- **WHEN** a build whose `CARGO_PKG_VERSION` is `0.8.1` runs "Check for updates" against the
  published `v0.8.1` release
- **THEN** the result is `UpToDate` with `current = "0.8.1"`
- **AND** the equal-version case is never reported as a downgrade or as `UpdateAvailable`.
