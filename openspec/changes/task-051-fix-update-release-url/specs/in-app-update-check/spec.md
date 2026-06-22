# Spec delta — in-app-update-check

## ADDED Requirements

### Requirement: Update-check targets resolve to the canonical repository owner

The update-check targets SHALL all reference the canonical repository owner/name `kaikkone4/vire`, and a guard MUST fail the build/tests if any target contains the incorrect owner token `kaikkonen4`.

The targets covered are the GitHub *latest release* API endpoint, the Releases page URL opened in
the browser, and the `opener:allow-open-url` capability allowlist entry. `kaikkone4/vire` matches
`remote.origin.url` (`https://github.com/kaikkone4/vire.git`) and the TASK-050 spec, so the
post-merge divergence recorded in TASK-050 cannot recur. This requirement changes **only** the
owner/name segment of already-specified URLs; it adds no new network host, capability type, or
feature, and does not change the running-version source or the release/version contract.

#### Scenario: Releases page resolves to a real page

- **WHEN** the user activates "Open GitHub Releases"
- **THEN** the OS default browser opens `https://github.com/kaikkone4/vire/releases`
- **AND** the page is the real repository Releases index, not a `kaikkonen4/vire` 404.

#### Scenario: Latest-release check hits the real repository

- **WHEN** the user activates "Check for updates"
- **THEN** the Rust core GETs `https://api.github.com/repos/kaikkone4/vire/releases/latest`
- **AND** a published release resolves to `UpToDate` or `UpdateAvailable` rather than a fail-soft
  `Unknown` caused by a nonexistent repository.

#### Scenario: Owner-typo regression guard fails the build/tests

- **WHEN** any compiled update-check target (`RELEASES_URL`, the latest-release API URL, or the
  `opener:allow-open-url` allowlist entry) contains the token `kaikkonen4`
- **THEN** the regression guard (static assertion or unit test) fails
- **AND** the scoped opener allowlist URL equals the compile-time `RELEASES_URL` constant.
