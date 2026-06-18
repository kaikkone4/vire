# Spec delta — app-configuration

This delta adds a single non-secret import-range setting to the existing local Langfuse configuration. It
is additive (one key-value settings row), changes no existing setting, and stores no credential material.

## ADDED Requirements

### Requirement: The Langfuse import range is a persisted, validated, non-secret setting

The app SHALL persist a non-secret **import range** setting controlling how far back Langfuse imports
reach, with at least the choices last 7 days, last 30 days, last 90 days, all history, and a custom
"since" timestamp. The setting SHALL be stored locally alongside the other non-secret Langfuse settings
(base URL, source, environments, enabled), never in a credential store, and SHALL contain no secret
material. An absent or malformed value SHALL resolve to the default (last 30 days) rather than failing the
import, and a malformed custom value SHALL be reported in a secret-free way.

#### Scenario: The import range persists and is read back

- **WHEN** the user selects an import range in Settings and saves
- **THEN** the choice is persisted locally and applied to subsequent imports
- **AND** it is read back into the Settings panel on reopen.

#### Scenario: An absent or malformed range falls back to the default

- **WHEN** no import range is configured, or the stored value is malformed
- **THEN** imports use the default range (last 30 days)
- **AND** any malformed-value notice shown to the user contains no secret material.

#### Scenario: The import range carries no secrets

- **WHEN** the import-range setting is stored, read, or displayed
- **THEN** it is held with the other non-secret settings and contains no credential or secret value.
