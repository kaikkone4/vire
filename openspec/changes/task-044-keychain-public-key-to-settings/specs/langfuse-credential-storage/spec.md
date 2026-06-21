# Spec delta — langfuse-credential-storage (TASK-044 / F2a)

Relocates the **non-secret** Langfuse public key from the macOS Keychain into the plaintext
SQLite `settings` table, keeping the **secret** key in the Keychain. This removes one Keychain
item — and therefore one macOS "Keychain access" prompt per fresh launch (2 → 1) — while
preserving the DEC-026 credential-pair integrity and the SEC-009 secret non-echo guarantee.
Realizes F2a from TASK-041 under the SW-5 public-key non-secrecy sign-off and conditions
C1–C4 (TASK-041 `sec.md`). Backend-only; the renderer IPC contract is unchanged.

## MODIFIED Requirements

### Requirement: The Langfuse public key is stored as non-secret local settings, not in the Keychain

The app SHALL store the Langfuse **public** key (`pk-lf-…`, the HTTP Basic-Auth username) as a
single additive key/value row in the local SQLite `settings` table, in the same plaintext store
already used for `base_url`, `source`, `environments`, and `langfuse_enabled`. The app SHALL
store the Langfuse **secret** key (`sk-lf-…`) in the macOS Keychain and SHALL NOT write the
secret key to the `settings` table or any other plaintext store. After this change exactly one
Keychain item (the secret) SHALL exist for the credential pair.

#### Scenario: Public key persists outside the Keychain

- **WHEN** the user saves a Langfuse credential pair
- **THEN** the public key is written to the SQLite `settings` table
- **AND** the secret key is written to the macOS Keychain
- **AND** no public-key Keychain item is created.

#### Scenario: Secret never enters plaintext settings

- **WHEN** a credential pair is saved or read back
- **THEN** the secret key value is never written to, or read from, the `settings` table
- **AND** the secret key remains stored only in the macOS Keychain.

### Requirement: Credential read-back exposes the secret as a presence flag only

The renderer-facing settings view SHALL report the secret key as a **presence flag**
(`has_secret_key`) and SHALL NOT return the secret key value (SEC-009). The view SHALL report
public-key presence (`has_public_key`) derived from the `settings` row. Surfacing the public
key value is permitted (it is non-secret) but is not required by this change.

#### Scenario: Settings view never returns the secret

- **WHEN** the renderer reads the Langfuse settings view
- **THEN** the response contains `has_secret_key` and `has_public_key` flags
- **AND** the response contains no secret key value.

## ADDED Requirements

### Requirement: The credential pair is set and cleared atomically across both stores

The app SHALL maintain the credential pair as an atomic unit spanning the SQLite `settings`
public key and the Keychain secret key, such that after any set or clear that returns, the pair
is **either both present (the new pair or the prior pair) or both absent — never exactly one
store populated — whenever every local SQLite operation in the path succeeds**. The app SHALL
perform the fragile Keychain mutation last and compensate its failure with a reliable local
SQLite operation: on a partial failure during set it SHALL restore the previously-written store
to its prior state, and on clear it SHALL restore the `settings` public-key row if the Keychain
delete fails, so no half-written pair remains. Because the two stores have no shared transaction,
the app SHALL treat the residual window — a local SQLite operation failing immediately after
another succeeded, or a process crash between the two mutations — by (a) surfacing a distinct,
secret-free error when a compensation step fails, never silently discarding it, and (b) relying
on the resolver to render any one-store state inert: such a state SHALL resolve to no credentials,
never a usable or mixed-source pair, recoverable by a single re-save.

#### Scenario: Failed secret write rolls back the public key

- **WHEN** the public key is written to settings and the subsequent secret write to the
  Keychain fails
- **THEN** the settings public-key row is restored to its prior state (prior value reinstated,
  or removed when there was none)
- **AND** the operation returns a secret-free error
- **AND** the two stores are left consistent (both new, or both prior) — never a one-store pair.

#### Scenario: Clear removes both stores

- **WHEN** the user clears the stored credentials
- **THEN** both the Keychain secret and the settings public-key row are removed
- **AND** if the Keychain delete fails, the settings public-key row is restored to its prior value
  so the prior consistent pair is preserved (the net result is both-present — never one store
  populated)
- **AND** if the local settings delete fails first, the clear aborts before the Keychain delete so
  both stores remain.

### Requirement: The resolver never produces a mixed-source credential pair

When resolving credentials for an import or a Test connection, the app SHALL read the public key
from `settings` first and the secret key from the Keychain first. The marked developer env
fallback is a **pair-level** alternative: the app SHALL consult it **only when neither store holds
its key**, and SHALL then require **both** env keys to be present. When exactly one store holds its
key, the app SHALL resolve to no credentials rather than fill the missing key from env — so a
stored key is never combined with an env-sourced key. A genuine read failure of either store SHALL
surface as a coarse, secret-free error and SHALL NOT be downgraded to the env fallback, so a broken
store can never be mistaken for "no credential". In no case SHALL the resolver combine a value from
one store with a value from a different source (env, or the other store's stale counterpart) into a
mixed-source pair.

#### Scenario: A store read failure does not enable the env fallback

- **WHEN** the public-key settings read or the secret-key Keychain read fails (as opposed to
  reporting an absent key)
- **THEN** credential resolution returns a secret-free error
- **AND** the env fallback is not used to fill the failed key
- **AND** no mixed-source credential pair is produced.

#### Scenario: One store present and the other absent does not pair with env

- **WHEN** exactly one of the two stores holds its key (e.g. an existing install whose Keychain
  secret remains but whose `settings` public key has not yet been re-saved) and the developer env
  fallback keys are set
- **THEN** credential resolution returns no credentials
- **AND** the present stored key is not combined with an env-sourced key for the missing side.

### Requirement: A fresh launch issues at most one Keychain prompt for the credential pair

On a fresh launch, the credential-related code paths (settings view, Test connection, import)
SHALL trigger at most **one** macOS Keychain access prompt — for the secret key — because the
public key no longer resides in the Keychain. The public-key code paths SHALL perform no
Keychain access.

#### Scenario: Opening Settings and testing the connection prompts once

- **WHEN** the user opens Settings and runs Test connection on a fresh launch with credentials
  configured
- **THEN** macOS prompts for Keychain access at most once (for the secret key)
- **AND** reading or displaying public-key presence performs no Keychain access.
