# Spec delta — langfuse-importer

These requirements harden the TASK-019 importer. They strengthen the existing absence-≠-zero and
no-secret-exposure contracts; they do not relax or remove any TASK-019 requirement.

## ADDED Requirements

### Requirement: An import run persists atomically

The importer SHALL persist each import run's writes (raw trace evidence, normalized AI evidence, and
the run record) within a single database transaction scoped to that run, so that an import run is
recorded all-or-nothing. A failure during persistence SHALL NOT leave a partially-written run.

#### Scenario: A persistence failure mid-run leaves no partial state

- **WHEN** a database write fails partway through persisting an import run
- **THEN** none of that run's raw-trace rows, AI-evidence rows, or run record are committed
- **AND** the store contains no partially-written run for that run id.

#### Scenario: A successful run commits as one unit

- **WHEN** an import run completes its writes without error
- **THEN** its raw-trace rows, AI-evidence rows, and run record are committed together
- **AND** the run becomes visible to the read-only health snapshot as a single consistent record.

### Requirement: Persistence failures are surfaced, never read as healthy or zero

The importer SHALL NOT silently discard a persistence error. A failed write SHALL be propagated to the
caller or recorded in the run's warnings with a non-healthy health state, so the failure is observable
in the run record and the health snapshot. A persistence failure SHALL NOT be recorded as `healthy`
and SHALL NOT contribute a numeric zero to any AI usage or cost total. Any surfaced error text SHALL
be free of credential or secret material.

#### Scenario: A failed write is visible, not swallowed

- **WHEN** persisting an import run fails
- **THEN** the failure is propagated to the caller, or recorded in the run's warnings with a
  non-healthy health state
- **AND** the run is not reported as `healthy` and no zero usage/cost total is produced for it.

#### Scenario: Surfaced persistence errors carry no secrets

- **WHEN** a persistence failure is recorded in a warning, log line, or IPC error
- **THEN** no API key, local stack secret, or credential material appears in that message.

### Requirement: Importer-emitted timestamps are uniform UTC RFC3339

The importer SHALL emit every timestamp it generates (import start, import finish, and raw-trace
import time) as UTC RFC3339, consistent with the cursor, window-boundary, and observation timestamps it
already records. The importer SHALL NOT emit a local-time or zone-ambiguous timestamp in any persisted
row or read-only snapshot.

#### Scenario: Run and import timestamps are UTC RFC3339

- **WHEN** the importer records an import run or a raw-trace import time
- **THEN** the start, finish, and import timestamps are formatted as UTC RFC3339
- **AND** they are directly comparable with the cursor and trace timestamps in the same record.

### Requirement: The manual import command is bounded and cannot hang the UI

The read-only manual import command SHALL return within a defined time ceiling. If the underlying
import does not complete within that ceiling, the command SHALL return a secret-free, non-healthy
result rather than blocking the user interface indefinitely. Bounding the command SHALL NOT add a new
health-taxonomy state beyond the existing ten.

#### Scenario: A hung import does not block the UI forever

- **WHEN** a manual import does not complete within the command's time ceiling
- **THEN** the command returns a bounded, secret-free non-healthy result
- **AND** the user interface is not blocked waiting on the import indefinitely.

#### Scenario: A normal import returns its snapshot within the ceiling

- **WHEN** a manual import completes normally within the time ceiling
- **THEN** the command returns the resulting read-only health snapshot
- **AND** no new health state outside the existing ten-state taxonomy is introduced.
