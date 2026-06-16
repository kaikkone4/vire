# Spec delta — app-configuration

This delta extends the `app-configuration` capability (created in TASK-025, which documented the
env-var template). TASK-026 adds **in-app** Langfuse configuration with **secure secret storage** and
makes the in-app settings the primary source, demoting process env to a clearly-marked dev fallback.

## ADDED Requirements

### Requirement: Langfuse integration is configurable from inside the app

The app SHALL provide an in-app Settings panel to view and edit the Langfuse integration
configuration: base URL, source (`local` / `cloud`), environments (comma-separated), public key,
secret key, and an **enable/disable** switch for the integration. The user SHALL be able to change
these values and have them take effect **without** editing process environment variables or restarting
into a sourced shell. The DEC-020 defaults SHALL be unchanged: loopback `http://127.0.0.1:3000`,
`source=local`, `environments=vire`.

#### Scenario: User configures Langfuse without touching shell env

- **WHEN** the user opens Settings and edits the Langfuse base URL, source, environments, and keys
- **THEN** the app persists the configuration locally and uses it for subsequent imports and health
  checks
- **AND** no process environment variable or sourced `.env` is required for the change to take effect.

#### Scenario: Disabling the integration is not zero usage

- **WHEN** the user turns the Langfuse integration switch off
- **THEN** the app runs no import and fires no health probe
- **AND** the source panel shows an explicit **disabled** state — never zero AI usage or cost.

### Requirement: The Langfuse secret key is stored in secure OS storage, never in plaintext

The app SHALL store the Langfuse **secret key** in macOS secure storage (Keychain), and SHALL NOT
store it in the SQLite database, the `settings` table, application logs, evidence rows, or exports.
Non-secret settings (`base_url`, `source`, `environments`, `langfuse_enabled`) MAY be stored in the
local SQLite `settings` table. The secret key SHALL NOT be rendered back into the settings form or
returned by any read command; the app SHALL instead expose only a presence indicator (set / not set).

#### Scenario: Secret key is not retrievable in plaintext

- **WHEN** the settings are read back for display, or any command result or error string is produced
- **THEN** the secret key value does not appear in the SQLite database, logs, evidence, exports, the
  settings form, or any command output
- **AND** the form shows only whether a secret key is set, with actions to replace or clear it.

#### Scenario: Clearing the secret removes it from secure storage

- **WHEN** the user clears the stored secret key
- **THEN** the app removes the secret from the OS Keychain
- **AND** a subsequent import resolves to the marked dev-fallback env credential if present, otherwise
  reports `auth_or_network_error` — never zero AI usage or cost.

### Requirement: In-app settings take precedence over process env, which is a marked dev fallback

The importer SHALL resolve its configuration from the in-app settings store **first**; where a setting
is unset, it SHALL fall back to the corresponding process environment variable, and where that is also
unset, to the existing code default. Process environment configuration (the TASK-025 `VIRE_*`
template) SHALL be retained only as an explicitly documented **developer fallback**. The existing
loopback URL allowlist (`source=local` requires a loopback host) and credential-redaction invariants
SHALL apply unchanged to settings-sourced values.

#### Scenario: Stored settings win over env

- **WHEN** both an in-app setting and the matching environment variable are present
- **THEN** the importer uses the in-app setting value
- **AND** the environment variable is used only when the in-app setting is unset.

#### Scenario: Loopback boundary still holds for settings-sourced values

- **WHEN** the source is `local` and a non-loopback base URL is configured in-app
- **THEN** the importer refuses the target (loopback required for `local`), the same as for an
  env-sourced value
- **AND** an off-host base URL is permitted only when the source is explicitly set to `cloud`.

### Requirement: A Test connection action verifies reachability without exposing secrets

The Settings panel SHALL provide a **Test connection** action that checks whether the configured
Langfuse endpoint is reachable and the credentials authenticate, and reports a coarse verdict
(reachable / authentication-or-network error). The action SHALL be time-bounded so a hung probe cannot
freeze the UI, and SHALL NOT include any secret value, raw response body, or stack-internal detail in
its result.

#### Scenario: Test connection reports a coarse, secret-free verdict

- **WHEN** the user clicks Test connection with a configured endpoint and credentials
- **THEN** the app reports whether the endpoint is reachable and authenticates
- **AND** the result contains no secret value or raw response body, and the action returns within a
  bounded time even if the endpoint hangs.
