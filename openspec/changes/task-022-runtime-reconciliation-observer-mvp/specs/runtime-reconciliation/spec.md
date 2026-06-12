# Spec delta — runtime-reconciliation

## ADDED Requirements

### Requirement: The runtime observer ingests a coarse local session log read-only and never scans processes

The TASK-022 runtime observer SHALL obtain pi/Claude Code runtime evidence by reading a local,
metadata-only coarse session log (default: the `pi-observe` event log at
`$PI_OBSERVE_STATE_DIR/events.jsonl`, overridable via `VIRE_RUNTIME_LOG_PATH`). It SHALL be
implemented in the Rust core, SHALL make no network calls, SHALL NOT scan processes or read
process command-lines, and SHALL NOT build or act as a pi/Claude emitter or adapter. The webview
CSP SHALL be unchanged and the renderer SHALL make no outbound HTTP.

#### Scenario: Runtime evidence comes from the local session log, not process scanning

- **WHEN** the observer collects runtime sessions
- **THEN** it reads them from the configured local coarse session log
- **AND** it does not enumerate processes, read command-lines, or open any network connection.

#### Scenario: The runtime log is absent

- **WHEN** the configured runtime log file is absent or empty
- **THEN** the observer records that it has no runtime evidence
- **AND** it does not interpret the absence as zero AI usage or zero cost, and it does not require
  `pi-observe` to be running.

### Requirement: Ingest keeps only a fixed allowlist and drops all prohibited fields

The observer SHALL filter each session-log record to a fixed allowlist of coarse metadata
(`event`, `project`/`project_key`, `tool`, `run_id`, `session_id`, `ts`, `status`/`exit_code`,
`billable`, `duration_ms`) and SHALL drop every other field before anything is persisted or logged.
No prompt or response text, terminal command body, shell history, environment dump, secret, free-text
summary, or repository/path identifier beyond the safe project token SHALL be persisted to
`ai_runtime_sessions` or written to any log, even when a malformed or hostile log injects such fields.

#### Scenario: Injected prohibited fields are dropped

- **WHEN** a session-log record carries a prompt, command body, shell-history line, env dump, or
  secret-shaped value in any field
- **THEN** the observer drops those fields and persists only allowlisted coarse metadata
- **AND** no prohibited value appears in `ai_runtime_sessions`, logs, or diagnostics.

#### Scenario: Malformed and unsafe input is tolerated, not fatal

- **WHEN** the log contains malformed JSON lines, is a symlink, or exceeds the size cap
- **THEN** the observer skips malformed lines and refuses unsafe files without crashing
- **AND** it records no partial or prohibited data.

### Requirement: Runtime evidence is reconciliation/health only and never a cost or time authority

The observer SHALL store only coarse session boundaries and a reconciliation state in
`ai_runtime_sessions` and SHALL NOT store any token, cost, or duration value as an AI cost/time
total. AI time, usage, and cost SHALL continue to be sourced from the Langfuse importer's evidence
(DEC-003 / DEC-017). The observer SHALL NOT duplicate or override valid Langfuse time/cost when a
matching trace exists.

#### Scenario: Runtime sessions carry no cost or time authority

- **WHEN** the observer persists a runtime session
- **THEN** the stored row contains coarse boundaries and a reconciliation state but no token or cost
  total
- **AND** project/day cost and time totals still derive from Langfuse evidence, not from runtime
  sessions.

### Requirement: Sessions reconcile against imported traces by session id then environment and time

The observer SHALL reconcile each runtime session against the importer's persisted AI evidence,
matching first by exact `session_id` and otherwise by `environment` plus time-window overlap. It
SHALL resolve a reconciliation state from `matched / observed_no_trace / unmatched_runtime /
reconciliation_unknown`, and SHALL mark an imported trace with no runtime session as
`unmatched_trace`. It SHALL reference the importer's existing trace-side health states
(`stale / missing / wrong_env / delayed / duplicate / unavailable / unknown`) rather than
re-deriving them.

#### Scenario: A wrapped session with a usable trace is matched

- **WHEN** a runtime session and an imported trace share a session id, or fall in the same
  environment and overlapping time window
- **THEN** the observer records `matched` and the matched trace id
- **AND** it does not recompute or duplicate the trace's cost/time.

#### Scenario: A trace with no runtime session is surfaced

- **WHEN** an imported trace has no corresponding runtime session
- **THEN** the observer records `unmatched_trace` for review
- **AND** it does not treat the absent runtime evidence as zero AI usage.

### Requirement: observed_no_trace is asserted only under a healthy import (absence is never zero)

The observer SHALL gate the `observed_no_trace` state on the importer's recorded health for the
session's window and environment. When that import was `unavailable`, `unknown`, or
`auth_or_network_error`, an observed session SHALL resolve to `reconciliation_unknown`. Only when the
import for the session's window and environment was `healthy`/complete and no matching trace exists
SHALL the observer record `observed_no_trace`. A missing trace under a down or uncertain import
SHALL NOT be reported as a real gap and SHALL NOT contribute a numeric zero to any AI usage or cost
total.

#### Scenario: Observed agent run with no trace under a down stack

- **WHEN** a runtime session exists but the import covering its window and environment was
  `unavailable` or `unknown`
- **THEN** the observer records `reconciliation_unknown`
- **AND** it does not record `observed_no_trace` and does not imply zero AI usage or cost.

#### Scenario: Observed agent run with no trace under a healthy import

- **WHEN** a runtime session exists, the import covering its window and environment was `healthy`,
  and no matching trace was found
- **THEN** the observer records `observed_no_trace` as a real trace-health gap
- **AND** it surfaces the gap without inventing a cost or time value.

### Requirement: The reconciliation surface exposes coarse state only, with no secrets or content

The observer SHALL expose reconciliation results through a read-only IPC command returning coarse
per-state counts, the runtime-log presence flag, and the local-only source posture. The surface
SHALL NOT include any secret, session content, prompt, command body, or raw runtime-log line. The
renderer SHALL present this as a thin status line only; the full review/approval UI is out of scope.

#### Scenario: The reconciliation surface carries no sensitive material

- **WHEN** the renderer requests the runtime reconciliation status
- **THEN** it receives coarse counts and states with no secret, session content, or command text
- **AND** the renderer makes no network call to obtain it.
