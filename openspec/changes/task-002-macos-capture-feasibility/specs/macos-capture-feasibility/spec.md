# Spec delta — macos-capture-feasibility

## ADDED Requirements

### Requirement: Capture feasibility is validated across the required signals

The spike SHALL validate macOS capture feasibility across NSWorkspace/NSRunningApplication
active-app capture, AXUIElement focused-window/title capture where permitted, an optional Quartz
Window Services fallback, and an idle/away signal, and SHALL document the evidence quality and
degraded states for each.

#### Scenario: Active-app capture is validated

- **WHEN** the active-app signal is evaluated
- **THEN** frontmost app bundle id/name and timestamp capture is validated for app switch,
  launch/terminate, sleep/wake, multiple Spaces, and full-screen apps
- **AND** whether the signal requires any TCC permission is recorded explicitly.

#### Scenario: Window-title capture and its degraded states are validated

- **WHEN** the AXUIElement focused-window/title signal is evaluated
- **THEN** title capture is validated for Janne's core apps where Accessibility is granted
- **AND** unavailable, permission-denied, and redacted states are recorded explicitly as
  capture-health-shaped data rather than silently dropped.

#### Scenario: Quartz fallback is evaluated against its permission burden

- **WHEN** the Quartz Window Services fallback is evaluated
- **THEN** whether the selected calls require Screen Recording permission on the target macOS
  version is recorded
- **AND** Quartz is recommended only if its title-matching benefit exceeds the added permission
  burden.

#### Scenario: Idle/away signal is validated

- **WHEN** the idle signal is evaluated
- **THEN** a last-event-age (or equivalent local) signal is validated
- **AND** its conversion to `active` / `idle_candidate` / `away` states after configurable
  thresholds is documented.

### Requirement: Permission burden is documented

The spike SHALL document the macOS permission burden for the capture path: which permissions are
required, which are optional, and the degraded behaviour when each is missing or revoked, so the
TASK-003 implementation-path decision can weigh permission cost.

#### Scenario: Permission burden and degraded behaviour are recorded

- **WHEN** the feasibility report is produced
- **THEN** required vs optional permissions (Accessibility, optional Screen Recording for Quartz)
  are listed
- **AND** the degraded capture behaviour for each missing/revoked permission is documented.

### Requirement: Field allowlist and APP-005 implications are identified

The spike SHALL produce a positive field allowlist and an explicit non-collection list, and map
their APP-005 / SEC-001 implications for the downstream capture adapter (TASK-005).

#### Scenario: Positive allowlist and non-collection list are produced

- **WHEN** the field allowlist is produced
- **THEN** the positive allowlist contains only app bundle/name, focused window title where
  permitted, coarse timestamps, permission/degraded state, and idle/away state
- **AND** the explicit non-collection list excludes screenshots, keystrokes, screen pixels, full
  browser contents, full URLs, terminal command bodies, shell history, prompt/response text,
  environment dumps, and secrets.

#### Scenario: APP-005 field-allowlist implications are recorded

- **WHEN** the feasibility report reaches its conclusion
- **THEN** the APP-005 / SEC-001 field-allowlist implications for the TASK-005 capture adapter are
  identified
- **AND** the allowlist is mapped to the six-field UX evidence record (day, time_range, app_name,
  window_title, source, review_state).

### Requirement: Manual validation matrix is provided

The spike SHALL provide a manual validation matrix enumerating the cases to exercise and the
expected observable behaviour for each.

#### Scenario: Matrix covers the required cases

- **WHEN** the manual validation matrix is produced
- **THEN** it covers permission grant/revoke, degraded states, sleep/wake, Spaces/full-screen,
  core-app title availability, idle thresholds, and Quartz permission burden
- **AND** each case records the expected observable behaviour.

### Requirement: Spike outputs are isolated from product and legacy code

The spike SHALL isolate any exploratory probe code so it does not entangle with product runtime or
legacy/manual-tracker code. Probe code SHALL live under a clearly-named, non-shipping spike path,
SHALL NOT be woven into `src/`, `src-tauri/src/`, or `observability/`, and SHALL NOT import,
migrate, or reuse the legacy manual-tracker surface.

#### Scenario: Probe code is confined to the isolated spike path

- **WHEN** exploratory probe code is created
- **THEN** it resides under `spikes/task-002-macos-capture/`
- **AND** it is not a member of any shipped build target and is not referenced by product runtime
  under `src/`, `src-tauri/src/`, or `observability/`.

#### Scenario: Legacy/manual-tracker code stays reference-only

- **WHEN** the spike is performed
- **THEN** the legacy manual-tracker surface (`time_entries` table, manual-entry view, stopwatch
  CRUD) is treated as reference-only
- **AND** it is not imported, migrated, reused, or wiped by this change; that decision is deferred
  to TASK-003.

### Requirement: Probe data handling preserves privacy

The spike SHALL NOT persist real private window/app titles. Probe output SHALL be redacted or
synthetic, or written to ephemeral local logs with a documented cleanup step, and SHALL contain no
secrets, prompt/response text, terminal command bodies, or environment dumps.

#### Scenario: No real private titles are persisted or committed

- **WHEN** a probe records captured evidence
- **THEN** the recorded output is redacted or synthetic, or is an ephemeral local log with a
  documented cleanup step
- **AND** no real private window/app titles, secrets, prompt/response text, command bodies, or
  environment dumps are persisted to the repository or to durable storage.

### Requirement: No capture MVP and no implementation-path decision

The spike SHALL remain a feasibility assessment. It SHALL NOT ship the capture adapter, write to
the product datastore, or select the implementation path; the Tauri+helper-vs-Swift-first decision
is handed to TASK-003.

#### Scenario: Spike defers MVP and path decisions

- **WHEN** the spike concludes
- **THEN** it produces feasibility evidence and a Tauri+helper-vs-Swift-first signal
- **AND** it does not ship a capture adapter, does not write product evidence rows, and explicitly
  defers the implementation-path decision to TASK-003.
