# Code/Docs Review — TASK-003 implementation-path decision

- **Role:** Code Reviewer (SW-4)
- **Change:** `task-003-implementation-path-decision`
- **Branch:** `feat/task-003-implementation-path-decision` · **PR:** #9
- **Tier:** L2 · **Gate:** SW-4 craft/consistency/traceability review (∥ SW-5 Security)
- **Date:** 2026-06-05
- **Verdict:** **PASS** (with one non-blocking merge-ordering condition for SW-6)

## 0. Review method

This is a **docs/OpenSpec/ADR decision package** — no product runtime. Craft review therefore
covers: Markdown/OpenSpec validity, internal consistency across the five artifacts, traceability of
cross-references, no-runtime-drift, data safety, and merge readiness. Security boundary correctness
is SW-5's gate and is not re-adjudicated here.

Verified mechanically against the diff and feeder branches; not taken from the QA report.

## 1. Craft & conventions — PASS

- **OpenSpec strict validation:** `openspec validate task-003-implementation-path-decision --strict`
  → `is valid` (exit 0), reproduced locally 2026-06-05.
- **Spec delta shape:** `specs/implementation-path-decision/spec.md` follows the ADDED-Requirement /
  `#### Scenario:` / WHEN-THEN-AND convention; 4 requirements, 6 scenarios, all well-formed.
- **Markdown hygiene:** all six artifacts end with a single trailing newline; tables are
  well-formed; heading hierarchy is consistent.
- **Dead code / commented-out code:** N/A (documentation-only change); none present.
- **Structure & readability:** proposal → design (options matrix + ADR + verification register) →
  arch-review → tasks → spec is a clean, navigable decision package. Naming (DEC-019, FB-003, option
  A/B/C) is consistent throughout.

## 2. Internal consistency — PASS

- **DEC-019 statement is reproduced verbatim-equivalent** across `proposal.md`, `design.md` (ADR),
  `arch-review.md` §2, and `qa.md`: reuse Tauri v2 shell + native Swift/AppKit capture helper
  (sidecar/`externalBin`) + read-only Rust-core Langfuse REST importer; ActivityWatch reference-only;
  Swift-first rejected. No drift between documents.
- **Decision driver is consistent:** "capture is non-differentiating (TASK-002 §10) → axis is reuse
  value vs. rebuild cost → DEC-009 switch condition not met" appears identically in design.md §"Options
  considered", arch-review §2, and spec Scenario 1. Coherent.
- **Comparison criteria align:** spec Scenario 1's criteria (reuse value, SEC-002 importer fit, added
  complexity, privacy boundary, EPIC-006 seams; capture/permission as non-differentiating gates)
  match the `design.md` matrix columns. The matrix carries two extra rows (pi/Claude+Langfuse
  reconciliation fit, capture/permission gates) — a superset of the spec's representative list, which
  is acceptable.
- **Minor (non-blocking) note:** the DEC trace list varies slightly by document — proposal.md and
  arch-review FB-003 cite `DEC-001/004/007/008/009/017/018`; the design.md ADR rationale names
  `DEC-001/007/008/009/017/018` (omits 004); spec Scenario 2 cites the load-bearing subset
  `DEC-008/009/017/018`. This is reasonable layering (spec lists only the load-bearing DECs), not a
  contradiction. Optional: align the full-trace lists in proposal vs. design for tidiness. Not
  required for merge.

## 3. Traceability — PASS (content correct), see §5 for merge ordering

- Every decision claim is sourced to a feeder spike section (TASK-001 §2/§4.3, TASK-002 §10,
  TASK-007 arch-review §2/§4/§6) and to BA DECs. The citation **paths and section anchors are
  accurate** — each referenced file resolves on its feeder branch:
  - `task-001-repo-path-assessment/salvage-reuse-inventory.md` → resolves on
    `origin/feat/task-001-repo-path-assessment`
  - `task-002-macos-capture-feasibility/capture-feasibility-report.md` → resolves on
    `origin/feat/task-002-macos-capture-feasibility`
  - `task-007-langfuse-importer-validation/arch-review.md` → resolves on
    `origin/feat/task-007-langfuse-importer-validation`
- The trace from spikes → DEC-019 → downstream owners (TASK-004/005/006/007-MVP) is complete and
  every downstream obligation is assigned. No orphaned or dangling obligation.

## 4. No runtime drift / data safety — PASS

- **No-build / no-drift (verified):** `git diff --name-only main...HEAD` = exactly 6 files, all under
  the change dir. `git diff --name-only main...HEAD -- src/ src-tauri/src/ observability/` → empty.
  No `tauri.conf.json`, `Cargo.toml`, `package.json`, or `capabilities/` touched. (The 6th file vs.
  QA's 5 is `qa.md` itself, added after the QA snapshot — still in-scope, docs-only.)
- **Data safety (independently re-scanned):** leak-pattern scan
  (`github_pat_|ghp_|gho_|ghu_|ghr_|sk-ant-|sk-proj-|AKIA…|xox[abprs]-|PRIVATE KEY`) over the change
  dir → clean. The sole textual hit is qa.md *describing* its own scan, not a secret. No real
  window/app titles, prompt/response text, command bodies, or env dumps.

## 5. Merge readiness — non-blocking condition for SW-6 (Release Manager)

**Finding (merge ordering):** TASK-003's cross-references to TASK-001/002/007 resolve **only on the
feeder branches**; those three branches are **not yet merged to `main`** (verified:
`git merge-base --is-ancestor origin/feat/task-00{1,2,7}-… main` → all NOT MERGED). On `main` /
this branch in isolation, the referenced spike files are absent, so the citations would not resolve
until the feeders land.

- **Why this is NOT a craft FAIL:** the references are *correct* — accurate paths and section
  anchors that resolve against the right artifacts. This is a repository merge-ordering dependency,
  not a defect in TASK-003's content or a broken link authored in error. TASK-003 is by design the
  decision gate that consumes the three completed spikes.
- **Condition for SW-6:** merge PR #9 **after (or in the same train as)** the three feeder PRs
  (TASK-001, TASK-002, TASK-007) so the cross-references resolve in `main`. If PR #9 were to merge
  first, the links would temporarily dangle on `main` until the feeders catch up.
- **No action required from the Developer (SW-2)** — this is a release-sequencing note, not a
  rework item.

## 6. Escalations

None. The decision unit, scope, and abstraction boundaries are correct for a Phase A decision gate
(consistent with SW-1 arch-review PASS). No design-level concern to route to SW Architect.

## 7. Verdict

**PASS.** Craft and conventions pass; OpenSpec strict-valid; artifacts are internally consistent and
fully traceable; no runtime drift; data-safe. The only open item is a **non-blocking merge-ordering
condition** (§5): PR #9 must merge after the TASK-001/002/007 feeder PRs for cross-references to
resolve in `main`. Hand to **SW-6 (Release Manager)** once **SW-5 (Security)** also passes, carrying
that ordering condition forward.
