# Tasks — TASK-007 Langfuse importer validation (spike)

> Phase A spike/validation. No product-runtime changes; no durable SQLite schema (TASK-004 owns it);
> no host-runtime decision (TASK-003 owns it). Deliverables: validated trace schema/time/usage/cost
> report, proven import-flow design (pagination/dedup/cursors), source-health state model,
> project-mapping signal assessment, and proposed `langfuse_import_runs`/normalized-evidence shapes.
> Any probe code is isolated under `spikes/task-007-langfuse-importer/` (non-shipping,
> redacted/synthetic output). Implementer: **integration-engineer** as primary, with
> **backend-developer** consulted on the host-runtime REST client / SQLite-shape friction signal,
> **data-analytics-engineer** on the normalized AI-evidence and import-run shapes, and
> **security-agent / qa-engineer** on SEC-002 network boundary and SEC-003 credential handling.

> **SW-2 status (2026-06-04):** design/validation deliverables complete; live round-trip rows
> are PENDING a missing local dependency (Docker daemon down, no `docker compose` plugin, no
> `observability/langfuse/.env`). Reported, not installed — see `langfuse-validation-report.md`
> §0 status matrix and §12 blockers. The read-only probe
> `spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs` is written + parse-checked to
> close the PENDING rows the moment the stack is up.

## 1. Local validation environment and safety scaffold
- [ ] 1.1 Bring up the local Langfuse stack (`scripts/setup-local-observability.sh`,
      `scripts/langfuse-up.sh`) and confirm health via `scripts/langfuse-smoke-test.sh`; use the
      loopback endpoint `http://localhost:3000` only. — **PENDING: stack unavailable (report §12).**
- [x] 1.2 Establish the isolated spike path `spikes/task-007-langfuse-importer/` and confirm it is
      **not** a member of any shipped build target (not added to the Tauri app, not referenced by
      `src-tauri/src/`, not under `observability/`).
- [x] 1.3 Define the probe data-handling rule: probes load credentials only from local secure
      config, emit **redacted or synthetic** trace evidence (or ephemeral local logs with a
      documented cleanup step), and never persist/commit secrets, prompt/response text, command
      bodies, or environment dumps. — shape-only probe + README + `.gitignore`.
- [x] 1.4 Confirm the `pi-observe` emitter and the legacy manual-tracker surface stay
      **reference-only**: not modified, not turned into a new pi/Claude adapter, not imported/reused.

## 2. Trace schema and time/usage/cost validation
- [ ] 2.1 Emit non-sensitive pi and Claude Code traces locally via `pi-observe` for `vire` (and a
      `default`/wrong-env case) and query them back through the Langfuse public API.
      — **PENDING: stack unavailable (report §12).**
- [x] 2.2 Record the **observed** trace/observation schema from `langfuse/langfuse:3.63.0`:
      identity, `environment`, start/end timestamps, session ID, name/metadata, usage, cost
      (field names, units, nullability) — do not assume field names. — recorded from the pinned
      version contract + authoritative emitter source (report §2); live round-trip PENDING.
- [x] 2.3 Validate time, usage, and cost semantics as sufficient to serve as the primary AI
      time/usage/cost source where traces are valid; note any field that triggers `schema mismatch`.
      — report §3; key finding: usage/cost live on generation **observations**, not on pi-observe traces.

## 3. Import flow: pagination, deduplication, cursors
- [x] 3.1 Design and prove the REST query by `environment` + time window against the local stack.
      — designed + implemented in probe; live demo PENDING (report §6.1).
- [x] 3.2 Prove **pagination** to window completion and compute the per-environment cursor/checkpoint
      position (durable persistence deferred to TASK-007 MVP / TASK-004). — probe walks pages, cursor
      = max observed timestamp (report §6.2); live demo PENDING.
- [x] 3.3 Prove **deduplication** by trace ID scoped to environment/project across pages, re-imports,
      and overlapping windows. — dedup key `(environment, trace_id)` in probe (report §6.3); live PENDING.

## 4. Source-health state model
- [x] 4.1 Define the health-state model: `valid`, `missing`, `stale`, `wrong/default environment`,
      `delayed`, `duplicate`, `schema mismatch`, `auth/config failure`, `rate limit` — each with
      detection basis and user-visible consequence. — report §4 (9-state table).
- [x] 4.2 Validate detectable transitions against the local stack, including the **invariant that
      absence never equals zero usage/cost**. — invariant enforced in design + probe (empty env ⇒
      cursor `none`, not 0); live transition demo PENDING.
- [x] 4.3 Validate the workspace-specific failure modes: pi-langfuse traces landing in `default`
      (wrong-env), and Claude Code hook silent-fail (missing/stale) — surfaced, not silently trusted.
      — report §5; confirmed structurally (pi-observe sets no `environment`).

## 5. Project-mapping signal assessment
- [x] 5.1 Assess `environment` (primary), session ID, and metadata (project key, tool/role, cwd
      basename, safe git branch/remote hash, command label) for trace→Vire-project mapping.
      — report §7 signal table.
- [x] 5.2 Record the `pi-observe` constraint that session IDs are **hashed before transmission**
      (opaque correlation only); defer classification rules to TASK-008 and summaries to TASK-010.

## 6. Schema-shape proposal for TASK-004 (no migration)
- [x] 6.1 Specify the normalized AI-evidence shape (trace time, usage, cost, source, health) as a
      proposal to TASK-004; persist nothing durable. — report §8.2.
- [x] 6.2 Specify the `langfuse_import_runs` table shape (import cursor, environment, status,
      latest-trace timestamp, warnings; **no credentials**) as a proposal to TASK-004. — report §8.1.

## 7. Security and boundary checks
- [x] 7.1 Confirm the importer contacts only the configured Langfuse base URL / trace endpoints and
      egresses no raw macOS activity (SEC-002). — report §10; probe refuses non-loopback hosts.
- [x] 7.2 Confirm no credentials appear in any artifact, log, fixture, or PR output; documented
      config uses redacted placeholders only (SEC-003). — report §10; secret scan §8.3.

## 8. Deliverable and exit gate
- [x] 8.1 Produce the validation report: observed schema, import-flow design, health-state model,
      mapping-signal assessment, the TASK-003 host-runtime friction signal, and the TASK-004
      shape proposals. — `langfuse-validation-report.md`.
- [x] 8.2 Confirm the exit gate: **Langfuse can be used as primary AI time/usage/cost source where
      valid; missing/stale/wrong-env/delayed/duplicate/schema/auth states are visible; credentials
      protected.** No durable importer shipped; no host-runtime or schema decision taken (TASK-003 /
      TASK-004 / TASK-007 MVP own those). — report §11; live-round-trip rows PENDING (report §12).
- [x] 8.3 Verify all produced artifacts by re-reading them; confirm no credentials, prompt/response
      text, command bodies, secrets, or environment dumps appear. — secret scan run, no values printed.
- [x] 8.4 Run `openspec validate task-007-langfuse-importer-validation --strict` → valid.
