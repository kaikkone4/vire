# Architecture Review — TASK-007 Langfuse importer validation (primary AI evidence)

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-007-langfuse-importer-validation`
- **Branch (implementation):** `feat/task-007-langfuse-importer-validation`
- **Tier:** L2 · **Gate context:** APP-005 (SEC-002 network boundary, SEC-003 credentials primary;
  SEC-004 only as a downstream consumer note — no totals are produced here)
- **Date:** 2026-06-04
- **Verdict:** **SPLIT-REQUIRED** — WP-007 is a "Spike + MVP" work package whose two halves the BA's
  own dependency graph separates across the TASK-003/TASK-004 gate. This change is scoped to the
  **Phase A spike** (validation + design), which is fully doable now and unblocks the gate. The
  **durable importer MVP** must be a separate change gated on TASK-003 (host runtime) and TASK-004
  (schema). No BA escalation: the BA plan already anticipates this two-phase split, so no
  `escalate-to-ba` is needed. No new TASK ID is required — both phases live under work package
  **WP-007 / TASK-007** (see §3).

## 1. Scope validation against BA architecture

TASK-007 is the **primary AI evidence task** under EPIC-003 (`05` §2; §3 row TASK-007),
`04` §7/§13 WP-007, `03` §11.1, and DEC-017/DEC-004. Its mandate: validate that Langfuse can serve
as the canonical pi/Claude Code AI time/usage/cost source, and establish the import design and
health-state model — with the standing invariant that **absence never equals zero usage/cost**.

- **Component-aligned, single boundary (for the spike).** All spike work targets exactly one BA
  component — the **Langfuse importer** (`04` §4: inputs Langfuse API traces by environment/date +
  import cursor; outputs `langfuse_import_run`, raw trace evidence, normalized AI time/usage/cost
  evidence, trace health). The spike crosses no other component boundary: it does **not** build the
  runtime observer (TASK-006), classification (TASK-008), summaries/export (TASK-010), capture
  (TASK-005), or schema (TASK-004). As a validation+design unit it is **not** splittable further —
  the single OpenSpec change is the right unit for Phase A.
- **DEC-017 preserved.** Langfuse is the primary AI cost/time source; local runtime observation is
  reconciliation/health only. This change treats the `pi-observe` emitter and local Langfuse stack
  as **reference-only inputs** and explicitly does **not** build a new pi/Claude adapter/emitter —
  exactly the DEC-017 boundary. The validated health-state taxonomy is the contract TASK-006 will
  reconcile against, but this change does **not depend on** the runtime observer implementation
  (correct per the task's stated constraint).
- **Sequencing is correct.** `05` §4 chain: TASK-001 → TASK-002 + **TASK-007 spike** + TASK-006
  spike → TASK-003 → TASK-004 → TASK-005 + **TASK-007 MVP** + TASK-006 MVP. We are in Phase A
  (only TASK-001, TASK-002 complete); the spike belongs here and must feed the importer
  host-runtime friction signal into TASK-003.

Conclusion: the **spike** task design is consistent with `03/04/05`, DEC-017, and APP-005. The MVP
half is correctly deferred (see §2). **SPLIT-REQUIRED.**

## 2. Why split — the MVP half crosses two boundaries this task does not own

The BA exit gate uses "implement/plan environment import, pagination, dedup, cursors, source health
states, and project mapping." Building the **durable importer MVP** in Phase A would force two
cross-boundary commitments TASK-007 does not own:

1. **Host runtime — owned by TASK-003.** The durable importer needs a host: a Tauri/Rust REST
   client in `src-tauri/src/`, or a Swift module if the path flips. TASK-003 (`05` §3; `04` §3
   decision table) owns the Tauri+helper-vs-Swift-first decision. Implementing the durable importer
   now would pre-decide it.
2. **Durable schema — owned by TASK-004.** The MVP needs persistent `langfuse_import_runs`
   (cursor, environment, status, latest-trace timestamp, warnings) and normalized AI-evidence
   tables. `04` §8 and TASK-004 (`05` §3) own these migrations and the raw/normalized/approved
   lifecycle boundaries. Writing durable schema here would collide with TASK-004's boundary.

Per the architect working rule — *"if a task's scope would cross component boundaries defined in
`03_architecture_plan.md`, stop and split"* — the durable import pipeline is split out. The spike
delivers everything safely doable in Phase A (validated schema, proven pagination/dedup/cursor
**logic**, the health-state model, mapping-signal assessment, and **proposed** table shapes as
inputs to TASK-004), so the MVP inherits a finished design and a validated schema rather than
re-discovering them.

## 3. Proposed task reuse / IDs (no new ID required)

`05` §2/§10 state all IDs TASK-001–TASK-016 are allocated and **no new task IDs are required**.
WP-007 is explicitly a **"Spike + MVP"** package, and the dependency chain lists TASK-007 in **both**
Phase A and Phase B. So the split is **two OpenSpec changes under the same WP-007 / TASK-007 work
package**, not a new ID:

| Phase | OpenSpec change | Gate / depends on | Status |
| --- | --- | --- | --- |
| A — spike (this change) | `task-007-langfuse-importer-validation` | depends on TASK-001 (done); uses local Langfuse stack | **ready now** |
| B — MVP (follow-up) | `task-007-langfuse-importer-mvp` (proposed name) | **gated on TASK-003** (host runtime) **and TASK-004** (schema) | **deferred** |

The follow-up MVP change should be created **after** TASK-003 and TASK-004 land, reusing this
change's validated schema, import-flow design, and health-state model. No `escalate-to-ba` is
needed: this two-phase shape is the BA's own plan, not a divergence from it.

## 4. APP-005 / security posture (spike scope)

| Control | Spike-scope handling | Downstream (TASK-007 MVP / TASK-012) |
| --- | --- | --- |
| **SEC-002** network boundary | The importer's whole point is the one allowed network path. Spec pins requests to the **configured Langfuse base URL / trace endpoints** (local stack `http://localhost:3000`) and forbids raw macOS activity egress. The importer is read-only against Langfuse — it pulls traces, never pushes activity. | Mocked-HTTP assertions + release network smoke test prove only Langfuse import fields leave; no raw activity egress. |
| **SEC-003** credentials | Spec forbids credentials in SQLite rows, logs, exports, fixtures, PR output, screenshots; mandates redacted placeholders and local-secure-config loading only. Mirrors the repo's existing posture (`observability/langfuse/.env` is chmod 600 + gitignored; `pi-observe` loads keys via a data-only parser and never injects them into wrapped commands). | Secret-scan + redaction/log-format tests on the durable importer/config. |
| **SEC-004** approval invariant | **N/A here** — the spike produces no billable/profitability totals. Recorded so the MVP keeps AI cost/time **separate from approved human duration** and never auto-promotes to totals (TASK-009/010/013 own the invariant). | Summary-side concern (TASK-010/013), not the importer. |
| Probe data safety | Spec mandates **no real sensitive trace content persisted** — redacted/synthetic output or ephemeral local logs with documented cleanup; no secrets/prompt/response/command bodies/env dumps. | Carries into MVP importer test fixtures (synthetic/anonymized traces only). |

Gate D (APP-005 release gate) does **not** fire: the spike ships nothing durable and writes no
product evidence. SEC-002/SEC-003 coverage is recorded so the MVP importer inherits a control-aware
design.

## 5. Architectural findings surfaced by the spike scope (for downstream)

These are **design inputs**, not blockers — surfacing exactly this kind of detail is the spike's
purpose, so none warrant `escalate-to-ba`:

- **Local validation environment is already present.** `observability/langfuse/`
  (`langfuse/langfuse:3.63.0`, loopback) + `pi-observe` + `scripts/langfuse-*.sh` give a
  non-sensitive validation harness. The spike can proceed **without probing Janne's production
  secrets** (BA: "validate API shape using local/dev or non-sensitive real data where possible").
  If real-environment **cost** confirmation later requires Janne's keys, the implementer returns
  `needs_input` rather than probing — no secrets in this spike.
- **Schema must be observed, not assumed.** `04` §14 flags "Langfuse cost schema varies." The spike
  records the **observed** `3.63.0` usage/cost field shapes and makes `schema mismatch` a
  first-class health state, so the importer degrades visibly instead of producing wrong cost totals.
- **Mapping leans on `environment`, not session ID.** `pi-observe` **hashes session IDs before
  transmission**, so session-based mapping yields opaque correlation handles only. Mapping must use
  `environment` first (DEC-004/DEC-017), then metadata; `wrong/default environment` (pi-langfuse
  v1.4.3 propagation risk) is a first-class health state, not a silent default. Full classification
  is TASK-008; this spike only assesses signal quality.
- **Health taxonomy is the TASK-006 contract.** The nine-state model (`valid`, `missing`, `stale`,
  `wrong/default environment`, `delayed`, `duplicate`, `schema mismatch`, `auth/config failure`,
  `rate limit`) is the interface TASK-006's runtime observer reconciles against — defined here,
  implemented for reconciliation there, **without this change depending on the observer**.

## 6. Other architectural notes for downstream

- **TASK-003:** consume the importer host-runtime friction signal (Rust/Tauri REST client vs Swift
  module) alongside the capture signal from TASK-002; decide the path before the TASK-007 MVP or
  TASK-004 schema is built.
- **TASK-004:** own the durable `langfuse_import_runs` and normalized AI-evidence migrations; take
  this change's proposed shapes as input. Keep raw trace payloads on short configurable retention
  and ensure import-run rows carry **no credentials**.
- **TASK-006:** reconcile runtime observations against this change's validated health states; do
  **not** turn runtime into a duplicate cost/time ledger when valid Langfuse traces exist (DEC-017).
- **TASK-007 MVP:** implement the durable importer behind the TASK-003 host boundary using the
  validated schema and import design; re-confirm cost fields against Janne's real environments via
  `needs_input` if the local-stack shape differs.
- **Spike disposal.** `spikes/task-007-langfuse-importer/` should be deleted or archived once the
  MVP consumes its findings, so no probe code lingers near product runtime.

## 7. Handoff

- **SW-2 implementer (primary):** **integration-engineer** — owns the Langfuse public-API import
  validation: bring up the local stack, validate trace schema/time/usage/cost, prove
  pagination/dedup/cursor logic, define and validate the health-state model, and produce the
  mapping-signal assessment.
- **Consulted:**
  - **backend-developer** (Rust/Tauri) — the host-runtime REST-client / SQLite-shape friction
    signal that feeds TASK-003, and the proposed `langfuse_import_runs` shape for TASK-004.
  - **data-analytics-engineer** — the normalized AI time/usage/cost evidence shape and import-run
    table proposal for TASK-004.
  - **security-agent / qa-engineer** — SEC-002 network-boundary and SEC-003 credential-handling
    review of the validation harness and any probe code.
- **Mandatory QA / security checks for this spike's deliverables:**
  1. **Credential safety:** confirm no credentials (or secret-shaped strings) appear in any
     artifact, probe output, log, fixture, or PR text; documented config uses redacted placeholders;
     secret-scan committed spike artifacts (filenames/counts only, no values printed) (SEC-003).
  2. **Network-boundary check:** confirm validation traffic targets only the configured Langfuse
     base URL / trace endpoints (local `http://localhost:3000`) and that **no raw macOS activity,
     window titles, prompt/response text, or command bodies** are sent (SEC-002).
  3. **Isolation check:** confirm `spikes/task-007-langfuse-importer/` is **not** referenced by any
     shipped build target and that no file under `src/`, `src-tauri/src/`, or `observability/` was
     modified; confirm the `pi-observe` emitter and legacy manual-tracker surface were not modified,
     reused, or turned into a new adapter (DEC-017).
  4. **Probe data-safety review:** confirm trace evidence is redacted/synthetic or
     ephemeral-with-cleanup; no real prompt/response text, command bodies, secrets, or environment
     dumps persisted.
  5. **Schema/health completeness:** confirm the **observed** `3.63.0` trace schema is recorded
     (usage/cost field names/units/nullability), the nine health states are defined with detection
     basis, and the **absence ≠ zero usage/cost** invariant is validated.
  6. **Pagination/dedup/cursor proof:** confirm pagination completes a multi-page window, dedup by
     trace ID + env/project scope is proven, and the per-environment cursor position is computed.
  7. **Mapping-signal assessment present:** confirm `environment`-first mapping, metadata
     evaluation, and the hashed-session-ID constraint are recorded, with classification deferred to
     TASK-008 and summaries to TASK-010.
  8. **Exit-gate text:** Langfuse usable as primary AI time/usage/cost source where valid; the
     missing/stale/wrong-env/delayed/duplicate/schema/auth states are visible; credentials
     protected; no durable importer shipped and no host-runtime/schema decision taken
     (TASK-003 / TASK-004 / TASK-007 MVP own those).
- **OpenSpec status:** `openspec validate task-007-langfuse-importer-validation --strict` → valid.
