# Spec delta — langfuse-importer

## ADDED Requirements

### Requirement: The importer is a read-only Rust-core REST client defaulting to local Docker Langfuse

The TASK-019 Langfuse importer SHALL be a read-only REST client implemented in the Rust core (never
the webview), SHALL default to local Docker self-hosted Langfuse at a loopback base URL
(`http://127.0.0.1:3000`), and SHALL treat Langfuse Cloud as an explicit non-default override that
is the only path producing off-host egress. It SHALL never write to Langfuse and SHALL restrict
every request URL to the configured base URL plus a fixed set of `/api/public/*` paths.

#### Scenario: Local Docker Langfuse on loopback is the default source

- **WHEN** the importer resolves its trace source with no override configured
- **THEN** it selects local Docker self-hosted Langfuse at `http://127.0.0.1:3000` (loopback) as the
  default source
- **AND** the webview CSP (`connect-src ipc:`) is unchanged and the renderer makes no outbound HTTP.

#### Scenario: Cloud is an explicit override and the only off-host egress

- **WHEN** the operator explicitly sets the source to Cloud
- **THEN** the importer targets the Cloud endpoint only because of that explicit override
- **AND** with no such override no request leaves the local host.

#### Scenario: Request URLs are restricted to the configured base URL

- **WHEN** the importer builds any request
- **THEN** the URL is derived from the configured base URL plus a fixed `/api/public/*` path
- **AND** any other host or scheme is refused.

### Requirement: A down or unreachable local stack is never read as zero usage or cost

The importer SHALL check local Docker/Langfuse availability before each run and SHALL resolve a
health state from the taxonomy `healthy / missing / stale / wrong_env / delayed / duplicate /
schema_changed / auth_or_network_error / unavailable / unknown`. Any absence of data (down stack,
missing/stale traces, null usage/cost, auth/network failure, indeterminate currency) SHALL resolve
to a health state and SHALL NOT contribute a numeric zero to any AI usage or cost total.

#### Scenario: Docker or the local stack is down

- **WHEN** Docker or the local Langfuse stack is down or unreachable at import time
- **THEN** the importer records `unavailable`
- **AND** it does not interpret the absence of traces as zero AI usage or zero cost.

#### Scenario: Missing or null usage/cost degrades visibly

- **WHEN** an expected trace is absent, or a usage/cost field is null or has an unexpected shape
- **THEN** the importer records the appropriate health state (`missing` / `schema_changed`)
- **AND** the affected total is not folded to zero.

### Requirement: Traces in the wrong environment are surfaced, not silently trusted

The importer SHALL use Langfuse environments as the primary Vire→project mapping (per project,
starting with `vire`) and SHALL treat traces appearing in `default` or an unexpected environment as
a first-class `wrong_env` signal rather than a silent pass, accommodating that some emitters set no
environment.

#### Scenario: pi/Claude traffic lands in the default environment

- **WHEN** traces for a project mapped to `vire` appear in the `default` environment
- **THEN** the importer records `wrong_env` and surfaces those traces for review
- **AND** it does not silently accept them as the project's authoritative total.

### Requirement: Imports paginate, dedup, and read usage/cost from observations

The importer SHALL paginate the public traces API to window completion, persist a per-environment
cursor/checkpoint, re-import with an overlap window to capture delayed traces, deduplicate by
`(environment, trace_id)`, and read token usage and cost from generation observations aggregated to
the trace rather than assuming usage on the trace body. It SHALL tolerate sparse traces (nullable
session id, empty name, variable metadata).

#### Scenario: A multi-page window is imported once

- **WHEN** a time window spans multiple pages and overlaps a prior import checkpoint
- **THEN** the importer paginates to completion and deduplicates by `(environment, trace_id)` so each
  trace is counted once
- **AND** a late-arriving trace before the checkpoint is reconciled as `delayed`, not dropped.

#### Scenario: Usage and cost come from observations

- **WHEN** the importer computes a trace's AI usage and cost
- **THEN** it reads token usage and cost from the trace's generation observations and aggregates them
- **AND** it does not assume usage or per-call cost lives on the trace body.

### Requirement: Credentials and raw activity never leak through the importer

The importer SHALL source API credentials from local secure configuration, use them only for the
authorization header, and never print, log, persist to evidence rows, export, or place them in
diagnostics. Raw macOS activity and window titles SHALL never be sent to Langfuse or mixed into
Langfuse traces. Local self-hosted trace payloads MAY include prompt/session/metadata for the MVP,
with stricter redaction/retention deferred to an L2 follow-up.

#### Scenario: Credentials stay out of logs and evidence

- **WHEN** the importer authenticates or records an import run, warning, or error
- **THEN** no API key or local stack secret appears in logs, evidence rows, exports, or diagnostics
- **AND** an `auth_or_network_error` is reported without exposing secret material.

#### Scenario: Raw local activity never reaches Langfuse

- **WHEN** the importer runs against the local stack
- **THEN** no raw macOS activity or window title is sent to Langfuse or written into a Langfuse trace
- **AND** off-host egress occurs only when the explicit Cloud override is set.
