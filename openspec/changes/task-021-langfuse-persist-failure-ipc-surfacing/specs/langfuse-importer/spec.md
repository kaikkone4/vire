# Spec delta — langfuse-importer

This delta tightens the TASK-020 surfacing requirement so that a persistence failure reaches the
manual-import IPC even when the durable marker write also fails. It strengthens the absence-≠-zero and
no-secret-exposure contracts; it relaxes nothing.

## MODIFIED Requirements

### Requirement: Persistence failures are surfaced, never read as healthy or zero

The importer SHALL NOT silently discard a persistence error. A failed write SHALL be propagated to the
caller or recorded in the run's warnings with a non-healthy health state, so the failure is observable
in the run record and the health snapshot. A persistence failure SHALL NOT be recorded as `healthy`
and SHALL NOT contribute a numeric zero to any AI usage or cost total. Any surfaced error text SHALL be
free of credential or secret material.

Surfacing SHALL NOT depend solely on a successful database write. When a run cannot be persisted, the
manual import command SHALL return a non-healthy, secret-free result **in-band** — independent of
whether any durable failure-marker write also succeeds. A persistence failure SHALL NOT cause the
manual import command to return a previously-persisted `healthy` (or otherwise stale) snapshot. The
surfacing path SHALL NOT introduce a new health-taxonomy state beyond the existing ten, SHALL NOT add a
schema change, and SHALL NOT perform runtime reconciliation or retry.

#### Scenario: A failed write is visible, not swallowed

- **WHEN** persisting an import run fails
- **THEN** the failure is propagated to the caller, or recorded in the run's warnings with a
  non-healthy health state
- **AND** the run is not reported as `healthy` and no zero usage/cost total is produced for it.

#### Scenario: A persistence failure reaches the import command even when the failure-marker write also fails

- **WHEN** an import run cannot be persisted **and** the durable failure-marker write also fails
- **THEN** the manual import command returns a non-healthy, secret-free result rather than a
  previously-persisted `healthy` or stale snapshot
- **AND** the result is produced from the in-band run outcome, not solely from the database snapshot.

#### Scenario: Surfaced persistence errors carry no secrets

- **WHEN** a persistence failure is recorded in a warning, log line, or IPC error
- **THEN** no API key, local stack secret, or credential material appears in that message.
