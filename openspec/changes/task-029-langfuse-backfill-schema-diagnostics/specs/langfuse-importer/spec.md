# Spec delta — langfuse-importer

This delta makes the TASK-019/020/027 importer **import the traces it can identify** instead of silently
dropping them, **classify** why traces/observations are skipped without leaking content, import
**historical** evidence via a configurable range + incremental cursor + resumable backfill, and report a
**grouped, actionable** result. It strengthens — never relaxes — the absence-≠-zero and
no-secret-exposure contracts, adds **no** new state to the ten-state health taxonomy, and adds **no** new
network egress host.

## MODIFIED Requirements

### Requirement: The importer tolerates the current Langfuse usage/cost payload shape

The importer SHALL read token usage and cost from the field locations the current Langfuse public API
returns, in addition to the previously supported locations, so that present usage and cost are captured
rather than withheld. A field that is genuinely absent in every supported location SHALL remain absent
(`null`), never coerced to zero.

The importer SHALL decouple **trace identification** from **usage/cost extraction**: a list-payload entry
that carries a usable trace id SHALL be imported even when a peripheral field (for example the
`observations` array) has an unexpected shape. In particular the importer SHALL tolerate the current
Langfuse list payload in which `observations` is an array of observation **identifiers** rather than
embedded observation objects, reading usage/cost from the observations endpoint in that case. An entry
that cannot be identified (no usable trace id) SHALL be classified and counted, never recorded as a
silent zero.

A trace SHALL be degraded to the `schema_changed` health state **only** when its usage/cost are genuinely
unreadable in every supported location after the above tolerance is applied, or when it cannot be
identified — never merely because a peripheral field used the current (identifier-list) shape. A trace
degraded to `schema_changed` SHALL still be imported and surfaced for review with its affected count,
never silently dropped, and SHALL NOT contribute a numeric zero to any AI usage or cost total.

#### Scenario: Current-shape usage and cost are captured

- **WHEN** a generation observation reports token usage and cost in the current Langfuse field locations
- **THEN** the importer records the present usage and cost values
- **AND** does not degrade the trace to `schema_changed` solely because the legacy field names are absent.

#### Scenario: An identifiable trace with an identifier-list observations field is imported

- **WHEN** a list-payload trace has a usable id and an `observations` field that is an array of
  observation identifiers (not embedded objects)
- **THEN** the importer identifies and imports the trace, reading its usage/cost from the observations
  endpoint
- **AND** the trace is not skipped or dropped for the shape of its `observations` field.

#### Scenario: Absent usage stays absent, not zero

- **WHEN** a generation observation carries no token usage or cost in any supported location
- **THEN** the importer records absence (no value), never a numeric zero
- **AND** surfaces the affected trace as `schema_changed`, counted in the import result.

### Requirement: A manual import returns a secret-free, understandable result

The manual import command SHALL return a result that explains what the import did, including, per
environment and in total: the number of traces seen, the number newly imported (unique), the number
suppressed as duplicates, the number skipped or degraded for shape reasons, and the resulting health
state. An import that finds or imports nothing SHALL produce an explicit, human-readable result rather
than an empty or silent one.

The result SHALL report shape skips/degrades as **aggregated counts grouped by a fixed, secret-free
reason classification** — never the same warning string repeated once per affected trace. The result MAY
include a **bounded** number of structural diagnostic samples per reason to aid debugging.

The result SHALL NOT contain any credential, raw API response body, deserialization/parser error string,
field value, or trace prompt/session/metadata content; only counts, fixed reason labels, JSON key names,
JSON type names, health states, and the importer's existing fixed secret-free warning strings.

#### Scenario: Repeated skips are grouped, not repeated

- **WHEN** a manual import skips or degrades many traces for the same shape reason
- **THEN** the result reports that reason once with an aggregate count, per environment and in total
- **AND** does not repeat an identical warning string once per affected trace.

#### Scenario: A partial import reports counts and skips

- **WHEN** a manual import imports some traces and skips/degrades others for shape reasons
- **THEN** the result reports the imported, duplicate, and skipped/degraded counts per environment and in
  total, with the grouped reason breakdown.

#### Scenario: Diagnostics and samples carry no secrets

- **WHEN** the import result, its counts, its reason labels, or any structural sample is produced
- **THEN** no API key, local stack secret, raw response body, parser error string, field value, or trace
  prompt/session/metadata content appears in it
- **AND** a structural sample contains only fixed reason labels, JSON key names, and JSON type names.

## ADDED Requirements

### Requirement: The importer classifies why traces or observations are skipped or degraded

The importer SHALL classify each skipped or degraded trace/observation into a fixed, secret-free reason
taxonomy (for example: missing trace id; observations not embedded; identification field type mismatch;
generation lacks usage and cost in every supported location; observations fetch failed) and SHALL
aggregate the counts per reason, per environment and in total. The classification SHALL be derived from a
structural inspection of the payload, NOT by passing through a deserialization/parser error message.

#### Scenario: Skip reasons are classified and counted

- **WHEN** the importer cannot fully process some traces or observations
- **THEN** each is attributed to a fixed reason category and the per-reason counts are reported
- **AND** the reason categories are a closed, secret-free set rather than free-form error text.

#### Scenario: A bounded sample aids diagnosis without leaking content

- **WHEN** the importer records a structural sample for a skip/degrade reason
- **THEN** at most a small fixed number of samples per reason are kept
- **AND** each sample names only the JSON keys present and the JSON type of the offending field, never a
  value or any payload content.

### Requirement: The import range is configurable and imports are incremental by cursor

The app SHALL let the user configure how far back imports reach, with at least the options last 7 days,
last 30 days, last 90 days, all history, and a custom "since" timestamp. Absent configuration SHALL
resolve to a sane default (last 30 days).

A normal (non-backfill) import SHALL resume per environment from the last successfully synced timestamp
(the persisted cursor), importing from that point — less a small reconciliation overlap so late-arriving
traces are re-seen — through now. The first import of an environment with no prior cursor SHALL start at
the configured range floor. The import cursor SHALL NOT regress when a late/older trace is reconciled.
Imports SHALL remain idempotent: a trace already stored SHALL be de-duplicated by its environment and
trace id, not re-imported.

#### Scenario: Incremental import resumes from the last synced timestamp

- **WHEN** an environment has a prior successful import cursor and a normal import runs
- **THEN** the import covers from that cursor (less a small overlap) through now
- **AND** traces already stored are de-duplicated rather than re-imported.

#### Scenario: First import uses the configured range

- **WHEN** an environment has no prior cursor and a normal import runs
- **THEN** the import covers from the configured range floor through now.

#### Scenario: The default range applies when unconfigured

- **WHEN** no import range has been configured
- **THEN** imports use the default range (last 30 days) rather than a narrow current-day window.

### Requirement: Historical evidence can be backfilled durably and resumably

The app SHALL provide an explicit "Backfill now" action that re-scans from the configured range floor
through now, independent of the incremental cursor. A backfill SHALL persist its progress in bounded,
atomically-committed units so that an interruption or timeout loses at most the in-flight unit, and a
re-run SHALL continue from where it stopped (converging via environment+trace-id de-duplication and a
non-regressing cursor) rather than restarting from scratch. A backfill SHALL NOT hold the entire history
in a single all-or-nothing transaction.

Backfill SHALL honour the same boundaries as a normal import: the loopback/cloud network boundary, the
read-only (GET-only) contract, the integration enable/disable switch, off-user-interface-thread
execution, and serialization so no two imports run concurrently against the local store. Backfill SHALL
add no new network egress host. When a backfill reaches an internal pagination/window backstop, it SHALL
report that it was bounded rather than silently truncating, so the user can continue it by re-running.

#### Scenario: Backfill imports history beyond the incremental window

- **WHEN** the user triggers "Backfill now" with a wider configured range
- **THEN** the importer re-scans from the range floor through now and imports older traces not previously
  seen.

#### Scenario: An interrupted backfill resumes without duplication

- **WHEN** a backfill is interrupted and later re-run
- **THEN** already-imported traces are de-duplicated, the cursor advances monotonically, and the backfill
  converges
- **AND** no trace is imported twice.

#### Scenario: A bounded backfill says so rather than truncating silently

- **WHEN** a backfill window reaches an internal pagination/window backstop
- **THEN** the result indicates the run was bounded and can be continued by re-running
- **AND** the importer does not silently drop the remainder.

#### Scenario: Backfill preserves the network and disabled boundaries

- **WHEN** a backfill runs
- **THEN** every request stays under the public API root on the configured host, a `local` source still
  requires a loopback host, and a disabled integration runs nothing and probes nothing
- **AND** no new network egress host is introduced.
