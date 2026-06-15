# Spec delta — csv-export

## ADDED Requirements

### Requirement: CSV export resolves deterministically and never leaves the app in indefinite loading

The CSV export command SHALL acquire its save destination **without blocking the UI main thread**, and
SHALL always resolve to a definite outcome so the renderer's awaited call never hangs. Specifically:
the save dialog SHALL be presented off the main thread (e.g. from an `async` command via a non-blocking
file-picker callback or an off-main-thread blocking call); a cancelled or empty selection SHALL resolve
as "no export" (no error, no file); a successful selection SHALL write the file and resolve with the
exported row count; and the application SHALL remain responsive while the dialog is open and after it
closes. A synchronous command invoking a blocking save dialog on the main thread (which deadlocks on
macOS) SHALL NOT be used.

#### Scenario: Save dialog opens without freezing the app

- **WHEN** the user triggers Export CSV and the macOS save dialog is presented
- **THEN** the application remains responsive while the dialog is open
- **AND** the export command does not block the UI main thread waiting for the dialog result.

#### Scenario: Cancelling the save dialog returns to a responsive app

- **WHEN** the user cancels or dismisses the save dialog without choosing a location
- **THEN** the export resolves as "no export" — no file is written and no error is raised
- **AND** the application returns to a fully responsive state with no indefinite loading.

#### Scenario: Successful export writes the file and resolves

- **WHEN** the user chooses a valid `.csv` destination
- **THEN** the reviewed summary rows are written to that file
- **AND** the command resolves with the number of exported entries and the application stays responsive.

### Requirement: CSV export failures are surfaced to the user and never silently hang

When destination validation, path conversion, or the file write fails, the export command SHALL resolve
with a user-visible error rather than leaving the application in a loading or frozen state. The existing
destination checks (a `.csv` extension is required; the destination must be a file, not a directory)
and the local-path conversion guard SHALL be retained, and any write failure SHALL be reported.

#### Scenario: Invalid destination is reported, not hung

- **WHEN** the chosen destination fails validation (for example, it is a directory or lacks a `.csv`
  extension) or cannot be converted to a local file path
- **THEN** the export resolves with a user-visible error message
- **AND** the application remains responsive (no indefinite loading).

#### Scenario: Write failure is reported, not hung

- **WHEN** the file cannot be written (for example, the destination is not writable)
- **THEN** the export resolves with a user-visible error message describing the failure
- **AND** the application remains responsive (no indefinite loading).

### Requirement: The hang fix preserves the TASK-023 CSV security contract and export scope

Fixing the save-dialog hang SHALL NOT change the CSV writer's behavior. Formula-injection
neutralization and delimiter/quote/newline escaping of user-controlled text cells (project name, note)
SHALL remain exactly as specified for TASK-023, and the exported column set SHALL remain
`date, project, start_time, end_time, duration_minutes, note, total_duration_hours` with no raw
captured activity, app/window log, AI prompt/response text, command body, or secret-shaped value added.
The export SHALL remain local-only and SHALL NOT introduce any network call or egress.

#### Scenario: Neutralization and escaping are unchanged after the fix

- **WHEN** a report containing a formula-like project name or note is exported through the fixed command
- **THEN** the cell is neutralized and escaped exactly as before (the `'` guard is prepended and
  delimiters/quotes/newlines are escaped)
- **AND** the existing adversarial export test continues to pass.

#### Scenario: Export scope and local-only posture are unchanged after the fix

- **WHEN** a report is exported through the fixed command
- **THEN** the file contains only the reviewed summary columns with no raw activity, prompt/response,
  command body, or credential value
- **AND** no network request or egress occurs as part of the export.
