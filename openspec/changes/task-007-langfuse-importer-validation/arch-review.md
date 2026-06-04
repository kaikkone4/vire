# Architecture Review — TASK-007 Langfuse importer validation (primary AI evidence)

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-007-langfuse-importer-validation`
- **Branch (implementation):** `feat/task-007-langfuse-importer-validation`
- **Tier:** L2 · **Gate context:** APP-005 (SEC-002 network boundary, SEC-003 credentials primary;
  SEC-004 only as a downstream consumer note — no totals are produced here)
- **Date:** 2026-06-04 · **Revised:** 2026-06-04 for **DEC-018** (cloud-first configured Langfuse
  source) — see §0 revision addendum
- **Verdict:** **SPLIT-REQUIRED (unchanged)** — WP-007 is a "Spike + MVP" work package whose two
  halves the BA's own dependency graph separates across the TASK-003/TASK-004 gate. This change is
  scoped to the **Phase A spike** (validation + design), which is fully doable now and unblocks the
  gate. The **durable importer MVP** remains a separate change gated on TASK-003 (host runtime) and
  TASK-004 (schema). **DEC-018 does not change the split**, but it **removes the local-Docker
  blocker** from Phase A: validation is now **cloud-first against the configured Langfuse API**, with
  local Docker as an optional dev/contract fixture (§0). No BA escalation: DEC-018 is the BA's own
  accepted addendum, so no `escalate-to-ba` is needed. No new TASK ID is required — both phases live
  under work package **WP-007 / TASK-007** (see §3).

## 0. DEC-018 revision addendum (2026-06-04) — cloud-first; Phase A no longer Docker-blocked

DEC-018 (accepted addendum to DEC-017; `07_decision_log.md`; `03` §4.4/§11.1/§11.2;
`04` §13/§14) makes Janne's **configured Langfuse API project the MVP canonical AI usage/cost
import source**, validated **cloud-first** where pi/Claude instrumentation already writes traces. It
explicitly states a **local Docker Langfuse stack is NOT a blocking validation dependency** — only
an optional developer fallback / contract-test fixture / self-host evaluation harness. This revises
the prior review on exactly one axis:

- **Reassessed: the earlier `blocked` status is withdrawn.** The previous SW-2 run returned
  `blocked` because the local Docker stack could not be brought up (daemon down, no compose plugin,
  no `.env`). Under DEC-018 **that is no longer a blocker**: the local stack was never the required
  path. The import-flow logic (pagination, dedup, cursor, health detection) is proven with **mocked
  HTTP fixtures** (and optionally local Docker) without a container; the **live round-trip** moves to
  the **configured cloud API** using project-scoped credentials from local secure config. If those
  credentials are not yet configured, the correct status is **`needs_input`** to request them via
  secure local `.env` — **not** `blocked` on Docker. SEC-003 still forbids probing/printing secrets.
- **Unchanged: SPLIT-REQUIRED.** DEC-018 changes the *source* (cloud vs local), not the
  *boundary ownership*. The durable importer MVP still needs the TASK-003 host runtime and the
  TASK-004 durable schema/cursors, so the MVP stays a follow-up change gated on both (§2, §3).
- **SEC-002 boundary re-affirmed, not widened.** DEC-018 confirms the allowed network path is
  **configured Langfuse trace import only** (cloud or optional local) — read-only import of
  *existing* AI traces. Vire still sends **no** raw macOS activity, window titles, prompts, command
  bodies, or env dumps to Langfuse. "Local-only Vire" governs *raw activity/evidence staying local*;
  it does **not** require the Langfuse service to be local. So the cloud base URL is an in-scope
  SEC-002 path, and the importer remaining a pull-only client is the control.
- **Configurability is now a first-class requirement** (carried into the spec): configurable base
  URL, project-scoped credentials, environment + date filtering, auth redaction, pagination/dedup,
  observed usage/cost schema validation, and trace-health states.

Sections 1–7 below remain valid as written; where they say "local stack," read "configured Langfuse
API (cloud-first), with local Docker as optional fixture." The targeted SEC-002/§5 updates are
inlined in those sections.

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
| A — spike (this change) | `task-007-langfuse-importer-validation` | depends on TASK-001 (done); cloud-first configured Langfuse API (DEC-018), local Docker optional fixture | **ready now (not Docker-blocked)** |
| B — MVP (follow-up) | `task-007-langfuse-importer-mvp` (proposed name) | **gated on TASK-003** (host runtime) **and TASK-004** (schema) | **deferred** |

The follow-up MVP change should be created **after** TASK-003 and TASK-004 land, reusing this
change's validated schema, import-flow design, and health-state model. No `escalate-to-ba` is
needed: this two-phase shape is the BA's own plan, not a divergence from it.

## 4. APP-005 / security posture (spike scope)

| Control | Spike-scope handling | Downstream (TASK-007 MVP / TASK-012) |
| --- | --- | --- |
| **SEC-002** network boundary | The importer's whole point is the one allowed network path. Per DEC-018 the spec pins requests to the **configured Langfuse base URL / trace endpoints** (Langfuse **Cloud** first; optional local stack) and forbids raw macOS activity / prompt / command-body / env-dump egress to Langfuse. The importer is read-only — it **pulls existing AI traces, never pushes activity**. "Local-only Vire" keeps raw activity/evidence local; it does not require the Langfuse service to be local. | Mocked-HTTP assertions + release network smoke test prove only configured-Langfuse import calls leave; no raw activity egress. |
| **SEC-003** credentials | Spec forbids credentials in SQLite rows, logs, exports, fixtures, PR output, screenshots; mandates redacted placeholders and local-secure-config loading only. Mirrors the repo's existing posture (`observability/langfuse/.env` is chmod 600 + gitignored; `pi-observe` loads keys via a data-only parser and never injects them into wrapped commands). | Secret-scan + redaction/log-format tests on the durable importer/config. |
| **SEC-004** approval invariant | **N/A here** — the spike produces no billable/profitability totals. Recorded so the MVP keeps AI cost/time **separate from approved human duration** and never auto-promotes to totals (TASK-009/010/013 own the invariant). | Summary-side concern (TASK-010/013), not the importer. |
| Probe data safety | Spec mandates **no real sensitive trace content persisted** — redacted/synthetic output or ephemeral local logs with documented cleanup; no secrets/prompt/response/command bodies/env dumps. | Carries into MVP importer test fixtures (synthetic/anonymized traces only). |

Gate D (APP-005 release gate) does **not** fire: the spike ships nothing durable and writes no
product evidence. SEC-002/SEC-003 coverage is recorded so the MVP importer inherits a control-aware
design.

## 5. Architectural findings surfaced by the spike scope (for downstream)

These are **design inputs**, not blockers — surfacing exactly this kind of detail is the spike's
purpose, so none warrant `escalate-to-ba`:

- **Cloud-first per DEC-018; local Docker is an optional fixture.** Validation targets the
  **configured Langfuse API** (cloud-first) with project-scoped credentials and environment/date
  filters. `observability/langfuse/` (`langfuse/langfuse:3.63.0`, loopback) + `pi-observe` +
  `scripts/langfuse-*.sh` remain a useful **offline/dev contract-test fixture and self-host
  evaluation harness**, but are **not** the blocking gate. The spike proceeds **without probing
  Janne's production secrets**: the live cloud round-trip uses credentials from local secure config,
  and if they are not configured the implementer returns **`needs_input`** to request them — the
  Docker daemon being unavailable is **no longer a blocker**.
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
  validation: validate **cloud-first against the configured Langfuse API** (configurable base URL,
  project-scoped credentials, environment/date filtering; local Docker optional fixture), validate
  trace schema/time/usage/cost, prove pagination/dedup/cursor logic (mocked fixtures + optional local
  Docker), define and validate the health-state model, and produce the mapping-signal assessment.
  Return `needs_input` for cloud credentials if not configured — never probe secrets.
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
  2. **Network-boundary check:** confirm validation traffic targets only the **configured Langfuse
     base URL / trace endpoints** (Langfuse Cloud or optional local), imports only existing AI
     traces, and that **no raw macOS activity, window titles, prompt/response text, command bodies,
     or env dumps** are sent to Langfuse (SEC-002, DEC-018).
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
