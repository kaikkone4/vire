# Code/Documentation Review — TASK-001 Repo/Path Assessment

- **Gate:** SW-4 (Code Reviewer) · **Tier:** L2
- **Change:** `task-001-repo-path-assessment` · **Branch:** `feat/task-001-repo-path-assessment` · **PR:** kaikkone4/vire#6
- **Type:** Read-only spike/assessment — deliverable is documentation/artifacts only.
- **Date:** 2026-06-04
- **Verdict: PASS** (security review is SW-5's mandate, not assessed here)

## Scope of this review

Craft, conventions, traceability, OpenSpec quality, and the read-only guardrail. The deliverable
ships no product code, so "craft" here means: artifact correctness, evidentiary accuracy of the
cited claims, internal/cross-document consistency, and Markdown hygiene.

## What was reviewed

`proposal.md`, `tasks.md`, `design.md`, `salvage-reuse-inventory.md` (primary deliverable),
`specs/repo-path-assessment/spec.md`, `arch-review.md`, `qa.md`. Diff vs `main`: 7 files, +642
lines, **all** under `openspec/changes/task-001-repo-path-assessment/`.

## Verification performed (independent, read-only)

1. **No product source/schema/config touched.** `git diff --name-only main..HEAD` filtered for
   anything outside the change dir → **none**. Working tree clean. Satisfies the spec's "No source
   or schema is modified" requirement (spec.md:38-42) and proposal Impact (proposal.md:34-35).
2. **OpenSpec strict validation:** `openspec validate task-001-repo-path-assessment --strict` →
   *valid*. Spec delta is well-formed (one ADDED capability, 3 requirements, each with ≥1 scenario).
3. **Cited file:line anchors all match exactly** against the working tree:
   - `lib.rs`: `init_db:36`, `summary_repo:124`, `csv_formula_neutralized:133`, `csv_escape:142`,
     `export_csv_repo:143`, `validate_csv_destination:169` — all confirmed (inventory §2.3-2.5).
   - `pi-observe.mjs`: `redact:32`, dotenv parser `:51`, `isLoopbackLangfuseHost:205` — confirmed
     (inventory §2.7).
   - CSP string `default-src 'self'; … connect-src ipc: http://ipc.localhost` — byte-exact match
     (`tauri.conf.json:14`, inventory §2.1).
   - Capabilities `core:default`, `dialog:default`, `dialog:allow-save` — match (inventory §2.1).
   - Schema: `projects`, `time_entries`, `settings` tables, `idx_entries_date_project`,
     `capture_status=manual_mode_deferred` — all match `lib.rs:38-49` (inventory §2.4, §3).
   - `.gitignore` exclusions (`*.sqlite`, `*.db`, `observability/langfuse/.env`, `.env.*` +
     `!.env.example`) — match (inventory §2.8, §6).
   - Langfuse pin `langfuse/langfuse:3.63.0` — match `docker-compose.yml:143` (inventory §2.7).
4. **Markdown hygiene:** every produced file ends with a single trailing newline; no double-blank
   EOF; tables are column-consistent and render. Non-empty, valid CommonMark.

## Craft assessment

- **Traceability is excellent.** Every inventory row carries exactly one classification from the
  spec-defined vocabulary plus a BA/EPIC/SEC reference, satisfying the "Every inventoried area is
  classified" scenario (spec.md:13-19). The classification vocabulary in the deliverable
  (inventory.md:32-33) matches the spec delta verbatim.
- **BA/APP-005 traceability is concrete, not hand-waved.** §3 maps all 13 BA evidence entities
  (1 partial, 12 absent); §5 maps the full L2 control set (SEC-001/002/003/005/006/008) with a
  coverage+gap column; §7 records the two mandated known gaps (no outbound client, no
  SBOM/signing/notarization). Matches the spec's data-model and APP-005 scenarios.
- **Exit gate honored.** The no-wipe/no-reuse posture is stated up front (inventory.md:10-14) and
  re-confirmed in §9; both paths are explicitly deferred to TASK-003. The DEC-017 tension is
  recorded and routed to TASK-003/006/007 without resolution (§4), exactly as the spec requires.
- **Naming/structure/idiom:** section numbering is consistent, the method/evidence base (§1) is
  auditable, and the artifact-safety self-check (§10) is appropriate for the privacy posture.
- **No dead content / no scope creep:** the docs describe only the inventory; no stray TODOs,
  commented-out blocks, or speculative implementation leaked in.

## Blocking issues

None.

## Suggestions (non-blocking)

1. **Downstream task-number references diverge across documents.** The forward-pointers for the same
   control don't agree between `arch-review.md` and `salvage-reuse-inventory.md`:
   - SEC-005: arch-review.md:48 → `TASK-004/013`; inventory.md:162 → `TASK-004/010`.
   - SEC-006: arch-review.md:49 → `TASK-011/013`; inventory.md:163,196 → `TASK-010/011`.
   - SEC-008: arch-review.md:50 → `TASK-015`; inventory.md:164,197 → `TASK-012`.

   Downstream task numbering isn't finalized by this spike, so these are directional pointers only
   and don't affect the inventory's correctness — hence non-blocking. Worth reconciling (or noting
   that numbering is indicative) so a later reader doesn't treat either set as authoritative.

2. **Minor entity-count phrasing.** `design.md:35` says "11+ BA entities absent" while the
   finalized deliverable (inventory.md §3) precisely states 12 absent / 1 partial. `design.md` is
   the architect's preliminary read so the looser figure is expected, but aligning it to the final
   count would avoid a reader cross-referencing the two and seeing a discrepancy.

## Escalations to SW Architect

None. The single design-level item (DEC-017 vs. `pi-observe` emitter role) is already captured as
`FB-001` / `feedback_to_ba[]` in `arch-review.md` §4 and correctly routed downstream; it is by
design the output of this spike, not a defect to escalate.

## Gate decision

**PASS.** Craft and conventions pass; complexity is appropriate for a documentation deliverable;
no dead code/content; cited evidence verified exact; OpenSpec valid; read-only guardrail intact.
Per the gate protocol, hold release until SW-5 (Security Agent) also passes before SW-6.
