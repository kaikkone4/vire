# TASK-001 — Repo/path assessment

## Why

The BA package (`final_package.md` §7, `05_project_plan_epics.md` §TASK-001) opens Phase A
delivery with a **spike** to determine what in the existing Tauri/Rust/TypeScript repo is
salvageable for the narrowed private, local-only, Janne-first Vire prototype. The technical
plan (`04_technical_plan.md` §3) recommends reusing the existing shell *if* its UI/store/export
pieces prove reusable, but explicitly forbids assuming either a wipe or a reuse before evidence
exists. This change establishes that evidence base so the downstream implementation-path decision
(TASK-003) is made from a documented inventory rather than assumption.

This is an **assessment/spike only**. It produces a salvage/reuse inventory and a gap/control
coverage map. It does **not** implement capture, the Langfuse importer, the evidence data model,
or any feature. No code is wiped, rewritten, or migrated under this change.

## What Changes

- Produce a **salvage/reuse inventory** of the current repo across the dimensions named in the
  BA scope: Tauri shell, Rust backend, SQLite layer, project/manual-entry domain, summary/export,
  tests, observability tooling (`pi-observe`, local Langfuse stack), and privacy posture.
- Classify each asset as **reuse-as-is**, **reuse-with-changes**, **reference-only**, or
  **retire/replace**, with the architectural rationale and the BA/EPIC/SEC reference it serves.
- Map existing coverage against the BA evidence data model (`04_technical_plan.md` §8) and the
  APP-005 controls (SEC-001, SEC-002, SEC-003, SEC-005, SEC-006, SEC-008) to surface gaps.
- Record open design tensions (notably DEC-017 vs. the existing `pi-observe` emitter) as inputs
  to TASK-003/TASK-006/TASK-007, **without deciding them here**.
- Output a documented assessment artifact that satisfies the exit gate: *clear salvage/reuse
  inventory; no assumption of wipe or reuse*.

## Impact

- **Affected specs:** adds `repo-path-assessment` capability (assessment deliverable + guardrails).
- **Affected code:** none. This change writes documentation/assessment artifacts only; no source,
  schema, or configuration under `src/`, `src-tauri/`, or `observability/` is modified.
- **Downstream:** unblocks TASK-002 (capture spike), TASK-006/TASK-007 (runtime/Langfuse spikes),
  and TASK-003 (implementation-path decision). Preserves all BA guardrails: local-only raw
  evidence, no SaaS/cloud sync, no raw activity egress, positive collection allowlist, no
  credentials in logs/SQLite/exports/tests, reviewed-summary export defaults.
- **Branch convention for implementation:** `feat/task-001-repo-path-assessment`.
