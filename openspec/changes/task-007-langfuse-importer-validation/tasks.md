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

## 1. Local validation environment and safety scaffold
- [ ] 1.1 Bring up the local Langfuse stack (`scripts/setup-local-observability.sh`,
      `scripts/langfuse-up.sh`) and confirm health via `scripts/langfuse-smoke-test.sh`; use the
      loopback endpoint `http://localhost:3000` only.
- [ ] 1.2 Establish the isolated spike path `spikes/task-007-langfuse-importer/` and confirm it is
      **not** a member of any shipped build target (not added to the Tauri app, not referenced by
      `src-tauri/src/`, not under `observability/`).
- [ ] 1.3 Define the probe data-handling rule: probes load credentials only from local secure
      config, emit **redacted or synthetic** trace evidence (or ephemeral local logs with a
      documented cleanup step), and never persist/commit secrets, prompt/response text, command
      bodies, or environment dumps.
- [ ] 1.4 Confirm the `pi-observe` emitter and the legacy manual-tracker surface stay
      **reference-only**: not modified, not turned into a new pi/Claude adapter, not imported/reused.

## 2. Trace schema and time/usage/cost validation
- [ ] 2.1 Emit non-sensitive pi and Claude Code traces locally via `pi-observe` for `vire` (and a
      `default`/wrong-env case) and query them back through the Langfuse public API.
- [ ] 2.2 Record the **observed** trace/observation schema from `langfuse/langfuse:3.63.0`:
      identity, `environment`, start/end timestamps, session ID, name/metadata, usage, cost
      (field names, units, nullability) — do not assume field names.
- [ ] 2.3 Validate time, usage, and cost semantics as sufficient to serve as the primary AI
      time/usage/cost source where traces are valid; note any field that triggers `schema mismatch`.

## 3. Import flow: pagination, deduplication, cursors
- [ ] 3.1 Design and prove the REST query by `environment` + time window against the local stack.
- [ ] 3.2 Prove **pagination** to window completion and compute the per-environment cursor/checkpoint
      position (durable persistence deferred to TASK-007 MVP / TASK-004).
- [ ] 3.3 Prove **deduplication** by trace ID scoped to environment/project across pages, re-imports,
      and overlapping windows.

## 4. Source-health state model
- [ ] 4.1 Define the health-state model: `valid`, `missing`, `stale`, `wrong/default environment`,
      `delayed`, `duplicate`, `schema mismatch`, `auth/config failure`, `rate limit` — each with
      detection basis and user-visible consequence.
- [ ] 4.2 Validate detectable transitions against the local stack, including the **invariant that
      absence never equals zero usage/cost**.
- [ ] 4.3 Validate the workspace-specific failure modes: pi-langfuse traces landing in `default`
      (wrong-env), and Claude Code hook silent-fail (missing/stale) — surfaced, not silently trusted.

## 5. Project-mapping signal assessment
- [ ] 5.1 Assess `environment` (primary), session ID, and metadata (project key, tool/role, cwd
      basename, safe git branch/remote hash, command label) for trace→Vire-project mapping.
- [ ] 5.2 Record the `pi-observe` constraint that session IDs are **hashed before transmission**
      (opaque correlation only); defer classification rules to TASK-008 and summaries to TASK-010.

## 6. Schema-shape proposal for TASK-004 (no migration)
- [ ] 6.1 Specify the normalized AI-evidence shape (trace time, usage, cost, source, health) as a
      proposal to TASK-004; persist nothing durable.
- [ ] 6.2 Specify the `langfuse_import_runs` table shape (import cursor, environment, status,
      latest-trace timestamp, warnings; **no credentials**) as a proposal to TASK-004.

## 7. Security and boundary checks
- [ ] 7.1 Confirm the importer contacts only the configured Langfuse base URL / trace endpoints and
      egresses no raw macOS activity (SEC-002).
- [ ] 7.2 Confirm no credentials appear in any artifact, log, fixture, or PR output; documented
      config uses redacted placeholders only (SEC-003).

## 8. Deliverable and exit gate
- [ ] 8.1 Produce the validation report: observed schema, import-flow design, health-state model,
      mapping-signal assessment, the TASK-003 host-runtime friction signal, and the TASK-004
      shape proposals.
- [ ] 8.2 Confirm the exit gate: **Langfuse can be used as primary AI time/usage/cost source where
      valid; missing/stale/wrong-env/delayed/duplicate/schema/auth states are visible; credentials
      protected.** No durable importer shipped; no host-runtime or schema decision taken (TASK-003 /
      TASK-004 / TASK-007 MVP own those).
- [ ] 8.3 Verify all produced artifacts by re-reading them; confirm no credentials, prompt/response
      text, command bodies, secrets, or environment dumps appear.
- [ ] 8.4 Run `openspec validate task-007-langfuse-importer-validation --strict` → valid.
