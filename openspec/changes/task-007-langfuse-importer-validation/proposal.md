# TASK-007 — Langfuse importer validation (primary AI evidence)

## Why

TASK-007 is the **primary AI evidence task** (`05_project_plan_epics.md` §2 EPIC-003, §3 row
TASK-007; `04_technical_plan.md` §7, §13 WP-007; `03_architecture_plan.md` §11.1; DEC-017,
**DEC-018**). Per DEC-017/DEC-004, Langfuse is the canonical pi/Claude Code AI time/usage/cost
source where valid traces exist; local runtime observation (TASK-006) is reconciliation/health
only and must never become a duplicate cost/time ledger. Per **DEC-018** (accepted addendum to
DEC-017), the MVP canonical import source is **Janne's configured Langfuse API project, validated
cloud-first** where pi/Claude instrumentation already sends traces — a **local Docker Langfuse
stack is explicitly NOT a blocking validation dependency**, only an optional developer
fallback / contract-test fixture. Before any AI total can be trusted, the Langfuse public-API
trace **schema, time, usage, and cost fields must be validated against real non-sensitive data**,
and the import design — configurable base URL, project-scoped credentials, environment/date
filtering, pagination, deduplication, per-environment cursors, source-health states, and
project-mapping signals — must be established.

This change is scoped to the **Phase A spike** half of WP-007 (`05` §1 Phase A; §4 dependency
chain: TASK-001 → TASK-002 + **TASK-007 spike** + TASK-006 → TASK-003 → TASK-004 → **TASK-007
MVP**). Per DEC-018 it validates the importer **cloud-first against the configured Langfuse API**
(configurable base URL such as Langfuse Cloud, project-scoped Basic-auth credentials loaded from
local secure config), with the **local Docker stack in the repo** (`observability/langfuse/`,
pinned `langfuse/langfuse:3.63.0`, `127.0.0.1:3000`) and the `pi-observe` emitter retained as an
**optional offline/dev contract-test fixture**. It produces a validated schema, a concrete
importer design, and a health-state model. It does **not** ship the durable product importer: the
host runtime (Tauri+helper vs Swift-first) is owned by TASK-003 and the durable SQLite
import/health tables are owned by TASK-004, so the MVP import pipeline is split to a follow-up
gated on both (see `arch-review.md`).

## What Changes

- Validate the **public-API trace schema** against the **configured Langfuse API (cloud-first per
  DEC-018)** using project-scoped credentials and a **configurable base URL**: trace/observation
  identity, `environment`, start/end timestamps, session ID, name/metadata, and the **usage and
  cost** field shapes. Record the observed schema; do not assume field names. The local Docker
  stack (`scripts/langfuse-up.sh`, `langfuse/langfuse:3.63.0`) and `pi-observe` emitter remain an
  **optional offline/dev fallback / contract-test fixture**, not the blocking validation path.
- Validate **time, usage, and cost** semantics against non-sensitive real traces in the configured
  Langfuse project (pi + Claude Code), confirming Langfuse can serve as the primary AI
  time/usage/cost source where valid. Real-data confirmation uses project-scoped credentials from
  local secure config; if those credentials are not yet configured, the implementer returns
  `needs_input` to request them via secure local `.env` rather than probing or embedding secrets.
- Design and document the **REST import flow**: query by `environment` + **date/time window**,
  **pagination** to window completion, **deduplication** by trace ID scoped to environment/project,
  and **per-environment import cursors/checkpoints** (`04` §7). Prove the pagination/dedup/cursor
  logic against the **configured Langfuse API**, using mocked HTTP fixtures and the optional local
  Docker stack as offline contract tests; durable persistence of cursors is deferred to the
  TASK-007 MVP.
- Define the **source-health state model** and validate the detectable transitions: `valid`,
  `missing`, `stale`, `wrong/default environment`, `delayed`, `duplicate`, `schema mismatch`,
  `auth/config failure`, and `rate limit` — with the invariant that **absence never equals zero
  usage/cost** (`04` §7 health states; DEC-004).
- Validate the **workspace-specific risks**: pi-langfuse environment propagation landing traces in
  `default` (wrong-env detection) and Claude Code hook silent-fail (missing/stale detection), so
  the importer flags rather than silently trusts defaults (`04` §7 workspace risks; DEC-017).
- Produce a **project-mapping signal assessment**: usefulness of `environment` (primary), session
  ID, and metadata for mapping traces to Vire projects, including the `pi-observe` constraint that
  session IDs are hashed before transmission. **Full classification is deferred to TASK-008** and
  summary shaping to TASK-010 — this change only assesses signal availability/quality.
- Specify the **normalized AI-evidence shape** (trace time, usage, cost, source, health) and the
  **`langfuse_import_runs` table shape** the importer needs, as an input proposal to TASK-004.
  This change does **not** create or migrate durable product schema.
- Preserve the **network and credential boundary** (DEC-018 / APP-005): the importer talks only to
  the **configured Langfuse API base URL** (Langfuse Cloud, or an optional local stack) and only to
  import existing AI traces — **no raw macOS activity, prompts, command bodies, or env dumps egress
  to Langfuse**; **no credentials in SQLite rows, logs, exports, fixtures, PR output, or
  screenshots** — credentials stored/protected in local secure config, auth redacted, redacted
  placeholders only in documentation (SEC-002, SEC-003).
- If exploratory probe code is needed, isolate it under a clearly-named non-shipping spike path
  (`spikes/task-007-langfuse-importer/`) that is not a member of any shipped build target, is never
  woven into `src/`, `src-tauri/src/`, or `observability/`, and persists no real trace
  content/secrets (redacted/synthetic or ephemeral local logs with documented cleanup).

## Impact

- **Affected specs:** adds `langfuse-importer-validation` capability (spike deliverables: validated
  schema, import-flow design, health-state model, mapping-signal assessment, isolation/credential
  guardrails).
- **Affected code:** none under product runtime. No source/schema/config under `src/`,
  `src-tauri/src/`, or `observability/` is modified. Validation is **cloud-first against the
  configured Langfuse API** (DEC-018); the existing `observability/` Langfuse stack and `pi-observe`
  emitter are used **as-is** only as an **optional offline/dev contract-test fixture** and are
  **reference-only** (this change does not modify the emitter or build a new pi/Claude adapter —
  that is explicitly out of MVP per DEC-017). Any probe code is confined to the isolated,
  non-shipping spike path.
- **Downstream:** feeds **TASK-003** (importer host-runtime friction signal for the path decision),
  **TASK-004** (proposed `langfuse_import_runs` / normalized AI-evidence table shapes), **TASK-006**
  (the validated health-state taxonomy the runtime observer reconciles against), and the **TASK-007
  MVP** follow-up (validated schema + import design to implement durably). Identifies APP-005
  SEC-002/SEC-003 implications for the durable importer.
- **Guardrails preserved:** configured-Langfuse-only network boundary (cloud or optional local; no
  other endpoint), no raw activity/prompt/command-body/env-dump egress, credentials stored/protected
  locally and never logged/exported, absence ≠ zero usage/cost, legacy/manual-tracker and
  `pi-observe` emitter stay reference-only (no new adapter/emitter work, DEC-017), and no durable
  product schema/runtime committed here (TASK-003/TASK-004 own those).
- **Branch convention for implementation:** `feat/task-007-langfuse-importer-validation`.
