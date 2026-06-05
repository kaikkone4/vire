# TASK-003 — Implementation-path decision

## Why

Phase A (`05_project_plan_epics.md` §3 row TASK-003; §4 dependency chain; `04_technical_plan.md`
§3 path-decision table) requires a **recorded implementation-path decision before the TASK-004
schema/feature build starts**. The three Phase A spikes that feed this decision are complete:

- **TASK-001 — repo/path assessment** (`task-001-repo-path-assessment/salvage-reuse-inventory.md`):
  the existing Tauri v2 / Rust / TypeScript shell is a high-value salvage candidate — a locked
  webview CSP (`connect-src ipc:`, **no outbound HTTP**), narrow capabilities, a parameterised
  SQLite layer, and **`reuse-as-is` CSV escaping + formula-neutralization** primitives tested in
  unit + adversarial suites. 12 of 13 BA evidence entities are absent (net-new → TASK-004), and no
  migration framework exists. Both **reuse** and **replace** were left open for this task.
- **TASK-002 — macOS capture feasibility** (`task-002-macos-capture-feasibility/capture-feasibility-report.md`):
  capture is feasible at the needed quality and a low permission burden (one Accessibility grant;
  NSWorkspace frontmost-app and CGEventSource idle need **no** grant; Quartz/Screen-Recording is
  off the default path). Critically, §10 found the capture APIs are **equally reachable** from a
  native helper or a Swift-first app — **capture feasibility does not favour either path**. The
  real trade is *helper packaging / IPC / TCC-trust placement* (Tauri+helper) **vs.** *rebuild cost
  of review/store/export* (Swift-first). The AX grant must attach to whichever binary actually
  calls AX.
- **TASK-007 — Langfuse importer validation** (`task-007-langfuse-importer-validation/arch-review.md`):
  the durable Langfuse importer needs a **host runtime** whose choice is explicitly owned by
  TASK-003. Under DEC-018 the importer is a **read-only, cloud-first REST pull** behind the
  SEC-002 network boundary; TASK-001 §4.3 recommends it run **server-side in the Rust backend** so
  the locked webview CSP stays intact and the outbound allowlist is enforced off-renderer.

This change **makes and records** the decision the BA flow anticipated in DEC-008/DEC-009 ("reuse
the current Tauri shell unless the capture spike proves replacement is simpler"), now that spike
evidence exists. It serves **EPIC-001** (capture path) and **EPIC-006** (extension seams / scope
control without MVP expansion).

This is a **decision/spike task**. It produces a decision record (ADR **DEC-019**), an
options-comparison matrix grounded in the three spikes, a verified technology register, and the
architecture constraints handed to TASK-004/005/007-MVP. It **does not** build the schema
(TASK-004), the capture adapter (TASK-005), the durable Langfuse importer (TASK-007 MVP), or the
runtime observer (TASK-006); it does **not** migrate or wipe the legacy manual-tracker surface
(that migrate-vs-retire call is TASK-004's, on the boundary this decision sets).

## What Changes

- **Compare the three BA-mandated options** against weighted criteria drawn from the spikes:
  capture API access & permission burden, reuse value of validated assets, SEC-002 network-boundary
  fit for the Langfuse importer, added complexity (IPC / helper packaging / TCC-trust placement vs.
  rebuild cost), privacy-boundary strength, and EPIC-006 extension seams.
  1. **Tauri v2 shell (reuse) + native Swift/AppKit capture helper (sidecar) + Rust-backend Langfuse
     REST importer** — the candidate path.
  2. **Swift/AppKit-first single-binary app** — simplest TCC-trust model, but discards the validated
     review/store/export/CSV-safety reuse with no capture benefit.
  3. **ActivityWatch import/reference** — reference/benchmark only; brings its own server + watcher
     processes and schema with weak fit for pi/Claude session boundaries and Langfuse reconciliation.
- **Record the decision (ADR DEC-019)** with rationale traced to TASK-001/002/007 evidence and to
  DEC-001/004/007/008/009/017/018, and route it to the BA decision log via `feedback_to_ba[]` for
  ratification (it confirms/operationalizes DEC-009 rather than diverging from it).
- **Verify every named technology** the decision depends on against official docs / API reference
  (Tauri v2 sidecar/`externalBin`, Tauri v2 IPC, Tauri v2 HTTP client, macOS TCC/codesigning,
  Swift/AppKit, ActivityWatch, SQLite), marking each **verified (source URL)** or **assumption +
  spike/packaging follow-up**.
- **Hand downstream the architecture constraints** the decision fixes: capture stays in the native
  helper (never the webview); the helper holds the AX grant; the Langfuse importer is a read-only
  Rust-backend REST client behind the locked CSP and a URL allowlist; raw evidence stays local;
  reuse is evidence-driven and must **not** re-admit the legacy generic-tracker CRUD surface as a
  capture path.

## Impact

- **Affected specs:** adds the `implementation-path-decision` capability (the decision, its evidence
  basis, the verified-technology requirement, and the downstream architecture constraints).
- **Affected code:** **none** under product runtime. No source, schema, or configuration under
  `src/`, `src-tauri/src/`, or `observability/` is created, modified, or deleted. No build target is
  added. This change is a decision record only.
- **Downstream:** unblocks **TASK-004** (schema/lifecycle migrations build on the chosen host and
  evidence boundaries), and sets the host runtime for **TASK-005** (capture adapter = native helper)
  and the **TASK-007 MVP** (durable importer = Rust-backend REST client). Hands **TASK-006** the
  helper/runtime boundary and **TASK-004** the legacy `time_entries` migrate-vs-retire decision
  input.
- **Guardrails preserved:** local-only raw evidence, no SaaS/cloud sync, no raw activity egress to
  Langfuse, locked webview CSP, Langfuse-only outbound allowlist, DEC-017 (no new pi/Claude
  adapter), DEC-018 (cloud-first import), and the legacy manual-tracker **reference-only** boundary
  (no wipe and no reuse-as-capture decided by momentum).
- **Branch convention for implementation:** `feat/task-003-implementation-path-decision`.
