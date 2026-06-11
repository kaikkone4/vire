# TASK-018 — Local Langfuse source supersession addendum

## Why

BA decision **DEC-020** (with technical-plan alignment **DEC-022**) makes **local Docker
self-hosted Langfuse** Vire's canonical/default AI trace source, with Langfuse Cloud demoted to an
explicit non-default override. The BA artifacts (`03_architecture_plan.md`, `04_technical_plan.md`,
`07_decision_log.md`) and the code docs (`README.md`, `docs/langfuse-local-setup.md`,
`docs/backup-restore.md`) were updated to that posture.

The already-merged/gated OpenSpec change `task-003-implementation-path-decision` was authored
2026-06-05 under the earlier **DEC-018 cloud-first** posture and still carries "cloud-first" /
"DEC-018" framing across its proposal/design/spec/arch-review/qa/sec/release artifacts. A TASK-007
implementer reading that package for binding importer constraints would be misled about the default
source.

This change is a **drift review + supersession addendum** before TASK-007 continues. It does **not**
reopen the TASK-003 implementation-path decision (DEC-019), which is unaffected by where Langfuse
runs.

## What Changes

- **Record the architecture addendum** (`arch-review.md`): what of TASK-003 / DEC-019 remains valid
  (the reuse + native-helper + Rust-core read-only REST importer path — all of it), what is
  superseded by DEC-020 (only the default-source posture: cloud-first → local-Docker-default), a
  cite-and-correct drift inventory of the task-003 package, and the **authoritative TASK-007
  direction** after DEC-020.
- **Add forward-pointing supersession banners** to the two task-003 architecture-reference docs a
  downstream implementer actually reads for constraints — `design.md` and `arch-review.md` — pointing
  to DEC-020/DEC-022 and this addendum. The gate-history files (qa/sec/review/release/spec/proposal/
  tasks) are deliberately left intact (no history rewrite).

## Impact

- **Affected specs:** none. No new requirement is added; DEC-020/DEC-022 already capture the posture
  at the BA layer and the code docs already implement the documentation side. No `specs/**/spec.md`
  delta; no `openspec validate` gate required for this review-only change.
- **Affected code:** none under product runtime. No file under `src/`, `src-tauri/src/`, or
  `observability/` is created, modified, or deleted; no build target is added. Deliverables are this
  addendum plus two additive banners on the task-003 architecture docs.
- **Downstream:** fixes-forward the source posture the **TASK-007** importer spike/MVP inherits
  (local Docker default; Cloud explicit override; environments remain primary mapping; multi-component
  local stack with MinIO/S3 risks visible; Docker-down ⇒ unavailable/stale/unknown, never zero cost;
  local prompt/session/metadata accepted for MVP with stricter redaction/retention as L2 follow-up).
- **Guardrails preserved:** local-only raw evidence, no SaaS/cloud sync, no raw-activity egress,
  locked webview CSP, Langfuse-only outbound allowlist, DEC-017 (no new pi/Claude adapter), DEC-019
  (reuse + native helper + Rust-core importer). DEC-018 cloud-first default is superseded by DEC-020.
- **Carried `feedback_to_ba[]`:** DEC-019 is still not recorded in `07_decision_log.md`; ratify it
  there and note its importer posture now reads against DEC-020, not DEC-018.
- **Branch:** done. This addendum, the task-003 banners, and the DEC-020 doc changes are committed on
  `feat/task-018-local-langfuse-source-addendum` and surfaced as draft PR #10. PR #10 is stacked on
  base `feat/task-003-implementation-path-decision`, so task-003 must merge to `main` first (or PR #10
  is retargeted to `main` once task-003 lands).
