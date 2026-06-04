# Spec delta — langfuse-importer-validation

## ADDED Requirements

### Requirement: Langfuse trace schema, time, usage, and cost fields are validated

The spike SHALL validate the Langfuse public-API trace schema actually served by the pinned local
stack (`langfuse/langfuse:3.63.0`) against real non-sensitive data, and SHALL record the observed
fields for trace/observation identity, `environment`, start/end timestamps, session ID,
name/metadata, usage, and cost rather than assuming field names.

#### Scenario: Observed schema is recorded from the running stack

- **WHEN** the importer queries the local Langfuse public API for traces emitted by `pi-observe`
- **THEN** the observed trace/observation schema is recorded, including `environment`, start/end
  timestamps, session ID, name/metadata, and the usage and cost field shapes (names, units,
  nullability)
- **AND** the record notes any field whose presence or shape differs from the technical-plan
  assumptions so the importer can detect it rather than produce wrong totals.

#### Scenario: Time, usage, and cost semantics are validated as the primary AI source

- **WHEN** non-sensitive pi and Claude Code traces are imported from the local stack
- **THEN** trace time, usage, and cost values are validated as sufficient to serve as the primary
  AI time/usage/cost source where traces are valid
- **AND** the absence of a trace is never interpreted as zero usage or zero cost.

### Requirement: REST import flow with pagination, deduplication, and per-environment cursors

The spike SHALL design and prove the REST import flow against the local stack: query by
`environment` and time window, paginate to window completion, deduplicate by trace ID scoped to
environment/project, and define per-environment import cursors/checkpoints. Durable persistence of
cursors is deferred to the TASK-007 MVP.

#### Scenario: Pagination completes a time window

- **WHEN** a time window contains more traces than a single API page
- **THEN** the import flow paginates until the window is complete
- **AND** the per-environment cursor/checkpoint position is computed for resuming the next import.

#### Scenario: Duplicate traces are detected

- **WHEN** the same trace ID appears across pages, re-imports, or overlapping windows
- **THEN** it is deduplicated by trace ID scoped to environment/project
- **AND** a `duplicate` health signal is available rather than double-counting usage/cost.

### Requirement: Source-health state model is defined and validated

The spike SHALL define the importer source-health state model and validate the detectable
transitions, with the invariant that absence of traces never equals zero usage or cost.

#### Scenario: Required health states are covered

- **WHEN** the health-state model is produced
- **THEN** it covers `valid`, `missing`, `stale`, `wrong/default environment`, `delayed`,
  `duplicate`, `schema mismatch`, `auth/config failure`, and `rate limit`
- **AND** each state records its detection basis and the user-visible consequence.

#### Scenario: Workspace-specific failure modes are detected

- **WHEN** traces land in `default`/an unexpected environment (pi-langfuse propagation risk) or no
  trace arrives for an observed agent (Claude Code hook silent-fail)
- **THEN** the importer surfaces `wrong/default environment` or `missing`/`stale` respectively
- **AND** it does not silently trust default-environment traces or treat the gap as zero cost.

### Requirement: Project-mapping signal usefulness is assessed

The spike SHALL assess the usefulness of `environment`, session ID, and metadata for mapping traces
to Vire projects, including the constraint that `pi-observe` hashes session IDs before transmission.
Full classification is deferred to TASK-008 and summary shaping to TASK-010.

#### Scenario: Mapping signals are assessed without building classification

- **WHEN** the mapping-signal assessment is produced
- **THEN** it records `environment` as the primary mapping signal, evaluates metadata fields
  (project key, tool/role, cwd basename, safe git branch/remote hash, command label), and notes
  that hashed session IDs are opaque correlation handles only
- **AND** it defers the classification rules to TASK-008 and the summary shaping to TASK-010.

### Requirement: Normalized evidence and import-run shapes are proposed to TASK-004

The spike SHALL specify the normalized AI-evidence shape (trace time, usage, cost, source, health)
and the `langfuse_import_runs` table shape the importer needs, as an input proposal to TASK-004.
This change SHALL NOT create or migrate durable product schema.

#### Scenario: Table shapes are proposed, not migrated

- **WHEN** the import-run and normalized-evidence shapes are produced
- **THEN** they are recorded as a proposal for the TASK-004 schema work, including import cursor,
  environment, status, latest-trace timestamp, and warning fields, with no credentials persisted
- **AND** no durable SQLite schema or migration is created under product runtime by this change.

### Requirement: Network and credential boundary is preserved

The importer SHALL communicate only with the configured Langfuse API base URL / trace endpoints
(the local stack `http://localhost:3000` for the spike), SHALL NOT egress raw macOS activity, and
SHALL NOT place credentials in SQLite rows, logs, exports, test fixtures, PR output, or
screenshots. Configuration examples SHALL use redacted placeholders only.

#### Scenario: Only the configured Langfuse endpoint is contacted

- **WHEN** the importer makes network requests
- **THEN** requests target only the configured Langfuse API base URL and trace endpoints
- **AND** no raw macOS activity, window titles, prompt/response text, or command bodies are sent.

#### Scenario: Credentials never appear in evidence, logs, or output

- **WHEN** the importer is configured and run, and when artifacts are produced
- **THEN** credentials are loaded only from local secure configuration and never written to SQLite
  rows, logs, exports, fixtures, PR output, or screenshots
- **AND** documented configuration uses redacted placeholders (e.g. `LANGFUSE_PUBLIC_KEY=...`).

### Requirement: Spike outputs are isolated; emitter and legacy code stay reference-only

The spike SHALL isolate any exploratory probe code under a clearly-named, non-shipping spike path,
SHALL NOT weave it into `src/`, `src-tauri/src/`, or `observability/`, and SHALL NOT modify the
`pi-observe` emitter, build a new pi/Claude Code adapter/emitter, or import/migrate/reuse the legacy
manual-tracker surface.

#### Scenario: Probe code is confined and disposable

- **WHEN** exploratory probe code is created
- **THEN** it resides under `spikes/task-007-langfuse-importer/`
- **AND** it is not a member of any shipped build target and is not referenced by product runtime
  under `src/`, `src-tauri/src/`, or `observability/`.

#### Scenario: Emitter and legacy code are not modified

- **WHEN** the spike is performed
- **THEN** the `pi-observe` emitter and the local Langfuse stack are used as-is for validation only
- **AND** no new pi/Claude Code extension/adapter is built and the legacy manual-tracker surface is
  not imported, migrated, reused, or wiped.

### Requirement: Probe data handling preserves privacy

The spike SHALL NOT persist real sensitive trace content. Probe output SHALL be redacted or
synthetic, or written to ephemeral local logs with a documented cleanup step, and SHALL contain no
secrets, prompt/response text, terminal command bodies, or environment dumps.

#### Scenario: No secrets or raw content are persisted or committed

- **WHEN** a probe records imported trace evidence
- **THEN** the recorded output is redacted or synthetic, or is an ephemeral local log with a
  documented cleanup step
- **AND** no credentials, prompt/response text, command bodies, or environment dumps are persisted
  to the repository or durable storage.

### Requirement: No durable importer MVP and no host-runtime or schema decision

The spike SHALL remain a validation/design assessment. It SHALL NOT ship the durable importer,
select the importer host runtime, or create durable SQLite import/health tables; those are handed to
TASK-003 (host runtime), TASK-004 (schema), and the TASK-007 MVP follow-up.

#### Scenario: Spike defers MVP, host-runtime, and schema decisions

- **WHEN** the spike concludes
- **THEN** it produces a validated schema, a proven import-flow design, the health-state model, the
  mapping-signal assessment, and the TASK-003 host-runtime friction signal
- **AND** it does not ship a durable importer, does not pick the host runtime, and does not create
  durable product schema — those are deferred to TASK-003, TASK-004, and the TASK-007 MVP.
