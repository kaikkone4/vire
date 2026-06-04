# Design — TASK-007 Langfuse importer validation (spike)

## Context

EPIC-003's central risk is that Langfuse — the canonical AI time/usage/cost source under DEC-017 —
has **not been validated against Janne's real non-sensitive data** for schema, time, usage, and
cost, nor for the failure modes that make AI totals misleading (missing/stale/wrong-env/
delayed/duplicate traces). `04_technical_plan.md` §14 and `05_project_plan_epics.md` §6 list
"Langfuse validation fails for pi/Claude time/usage/cost" and "Langfuse cost schema varies" as the
primary AI-evidence risks to retire **before** any AI summary is relied upon. This spike retires
them.

The repo already contains a **local-only Langfuse stack** (`observability/langfuse/`, pinned
`langfuse/langfuse:3.63.0`, loopback-bound) and the **`pi-observe`** metadata-only emitter, with
`scripts/langfuse-up.sh`, `scripts/langfuse-smoke-test.sh`, and `scripts/langfuse-down.sh`. This is
exactly the non-sensitive validation environment the BA asks for ("validate API shape using
local/dev or non-sensitive real data where possible") — so the spike can proceed **without probing
Janne's production secrets**.

## Goals / Non-goals

- **Goals:** validate the public-API trace schema/time/usage/cost actually served by the pinned
  Langfuse version; design and prove pagination, deduplication, and per-environment cursors;
  define and validate the source-health state model; assess project-mapping signal quality;
  specify the normalized AI-evidence and `langfuse_import_runs` shapes as inputs to TASK-004;
  produce the importer host-runtime friction signal for TASK-003.
- **Non-goals:** shipping the durable importer; choosing the host runtime (TASK-003); creating or
  migrating durable SQLite schema (TASK-004); building a new pi/Claude Code extension/adapter or
  modifying the `pi-observe` emitter (out of MVP per DEC-017); runtime reconciliation
  implementation (TASK-006); classification (TASK-008) or summaries/export (TASK-010).

## Key decisions

### Decision: scope this change to the Phase A spike; split the durable MVP behind TASK-003 + TASK-004

WP-007 is a "Spike + MVP" work package, and the BA's own dependency chain (`05` §4) places the
**spike in Phase A** (before TASK-003/TASK-004) and the **MVP in Phase B** (after TASK-004
durable boundaries). Doing the MVP now would force two cross-boundary commitments this task does
not own: (a) selecting the importer **host runtime** (Tauri/Rust REST client vs Swift) — TASK-003's
decision; and (b) creating the durable **`langfuse_import_runs` / normalized-evidence** tables and
persistent cursors — TASK-004's boundary. Per the architect working rule "if a task's scope would
cross component boundaries, stop and split", the durable import pipeline is split to a **TASK-007
MVP** follow-up. This change delivers everything that is safely doable in Phase A and unblocks the
gate, while the MVP inherits a validated schema and a finished import design. See `arch-review.md`
§verdict.

### Decision: validate against the local Langfuse stack, not production secrets

The spike runs against `observability/langfuse/` (`http://localhost:3000`) with traces emitted
locally by `pi-observe` for pi + Claude Code. This keeps validation inside the L2 privacy posture,
exercises the real pinned server schema, and avoids probing Janne's production keys. If validating
against Janne's **actual** environments/keys later proves necessary (e.g. to confirm production
cost fields), the implementer returns `needs_input` to request keys via secure local `.env` rather
than embedding or printing any secret.

### Decision: record the observed schema; never hard-code assumed field names

Langfuse cost/usage field shapes vary by version and SDK (`04` §14 "Langfuse cost schema varies").
The spike's deliverable is the **observed** schema from `3.63.0` (trace/observation usage and cost
field names, units, nullability), plus a `schema mismatch` health state that fires when an expected
field is absent/incompatible — so the importer degrades visibly instead of producing wrong totals.

### Decision: map by environment first; treat session IDs as opaque

DEC-004/DEC-017 make `environment` the primary trace→project mapping signal. The spike records that
`pi-observe` **hashes session IDs before transmission**, so session-based mapping can only correlate
opaque handles, not recover project identity — mapping must lean on `environment` then metadata
(project key, tool/role, cwd basename, safe git branch/remote hash, command label). Wrong/`default`
environment (pi-langfuse v1.4.3 propagation risk) is therefore a first-class **health state**, not
a silent default. Full classification logic is TASK-008; this spike only assesses signal quality.

## Health-state model (validated, not implemented durably)

| State | Detection basis | Invariant |
| --- | --- | --- |
| `valid` | recent import; traces align with expected env/session | Langfuse usable as AI time/usage/cost source |
| `missing` | local runtime/expected activity but no matching trace | **absence ≠ zero** usage/cost |
| `stale` | latest trace/import older than expected threshold | flagged, not treated as current |
| `wrong_env` | traces in `default`/unexpected env vs project mapping | never silently trusted |
| `delayed` | trace arrives after runtime window / prior checkpoint | reconciled, re-imported safely |
| `duplicate` | repeated trace IDs / overlapping imports | deduped by trace ID + env/project scope |
| `schema_mismatch` | expected usage/cost/timestamp field absent/incompatible | importer degrades visibly |
| `auth_or_config_error` | import auth/config failure | reported without exposing secret material |
| `rate_limited` | API rate-limit response | backoff/retry; surfaced as health, not zero |

## Open questions routed downstream

- **Importer host runtime.** Whether the durable importer is a Tauri/Rust REST client or a Swift
  module depends on TASK-003. The spike produces the integration-friction signal; the decision is
  TASK-003's.
- **Durable cursor/health persistence.** The `langfuse_import_runs` and normalized AI-evidence table
  shapes are proposed here as inputs to TASK-004, which owns the migrations.
- **Real-environment cost confirmation.** If local-stack cost fields differ from Janne's production
  configuration, the TASK-007 MVP re-confirms against real keys via `needs_input`, never by probing
  secrets in this spike.
