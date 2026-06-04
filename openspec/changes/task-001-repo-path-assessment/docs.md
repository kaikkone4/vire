# Documentation Review (SW-6) — TASK-001 Repo/Path Assessment

- **Gate:** SW-6 (Documentation Engineer) · **Tier:** L2
- **Change:** `task-001-repo-path-assessment` · **Branch:** `feat/task-001-repo-path-assessment` · **PR:** kaikkone4/vire#6
- **Type:** Read-only spike/assessment — deliverable is OpenSpec documentation only.
- **Upstream gates:** SW-3 QA **PASS**, SW-4 Code Review **PASS**, SW-5 Security **PASS**.
- **Date:** 2026-06-04
- **Verdict: complete** (no docs drift; one minor handoff-clarity note applied)

## What SW-6 assessed

Whether the L2 documentation set matches the released/handed-off state and whether the handoff
documentation is sufficient for the downstream spikes this change unblocks (TASK-002/003/006/007).
This change ships **no product code**, so "released state" is unchanged from `main`.

## 1. README / runbook drift — none required

- `README.md` describes Vire v0.1: local-only Tauri v2 manual time tracker (projects, manual
  entries, summaries, CSV export) with **automatic capture intentionally deferred** and a no-network
  privacy posture.
- This branch's diff is **entirely** under `openspec/changes/task-001-repo-path-assessment/`
  (`git diff --name-only main...HEAD` → 7 OpenSpec files; `review.md`/`sec.md` untracked gate
  reports). No `src/`, `src-tauri/`, `observability/`, schema, or config change.
- The inventory §2.8 independently confirms the current code is a manual tracker with capture
  deferred ("Manual Mode") and the documented non-collection posture — i.e. **the README still
  matches the shipped app exactly**.
- **Conclusion:** no README, installation guide, user guide, API/OpenAPI, or runbook update is
  required by this change. APP-005 Gate D (release integrity) does not fire — the spike releases and
  distributes nothing — so no release-notes or version-bump documentation is due either.

## 2. L2 doc-set applicability for this deliverable

| L2 doc artifact | Required now? | Why |
| --- | --- | --- |
| README accuracy | Already accurate | No product behavior changed; verified against current tree. |
| Installation / maintenance guide | No | No operational/deploy change; nothing new for an operator to run. |
| User guide | No | No end-user-facing feature added or changed. |
| API docs (OpenAPI) | No | No API surface exists or changed; app is local IPC-only. |
| Onboarding docs | No new need | The salvage inventory itself is high-value onboarding context for the repo's current shape. |
| Release notes | No | Nothing released/distributed; APP-005 Gate D not triggered. |
| ADR / decision log | Deferred (correct) | DEC-017 tension is **recorded** and routed to TASK-003/006/007, not decided here — matches the spike's exit gate. |

## 3. Handoff-doc sufficiency for downstream tasks — sufficient

The primary deliverable (`salvage-reuse-inventory.md`) carries the downstream handoff context:

- **§3** — full BA evidence data-model coverage (13 entities: 1 partial, 12 absent) → TASK-004 input.
- **§4** — open design tensions (DEC-017 emitter-vs-import, `time_entries` migrate-vs-retire,
  network-boundary placement, export-default policy) routed to TASK-003/004/006/007/010/011 as
  decision inputs, explicitly unresolved.
- **§5** — APP-005 L2 control coverage + per-control gap, routed downstream.
- **§7** — consolidated gaps register with downstream pointers.

This is concrete, traceable, and adequate to unblock TASK-002 (capture spike), TASK-003
(implementation-path decision), and TASK-006/007 (runtime/Langfuse spikes).

## 4. Documentation change applied (minimal, in-scope)

One documentation-clarity issue from the SW-4 review (non-blocking suggestion 1): downstream
TASK-NNN pointers diverge between `arch-review.md` and `salvage-reuse-inventory.md` for SEC-005/006/008.

- **Action:** added a single clarifying note at the head of `salvage-reuse-inventory.md` §7 stating
  the TASK-NNN pointers are **indicative** (TASK-003 owns sequencing) and that the §5 control mapping,
  not the task number, is authoritative — so a later reader treats neither set as canonical.
- This is additive and non-substantive: it changes **no** asset classification, evidence, control
  mapping, or exit-gate statement, so it does not disturb the SW-3/4/5 correctness or security
  findings. It implements exactly the reviewer's recommended "note that numbering is indicative".
- **Not changed:** SW-4 suggestion 2 (`design.md:35` "11+ entities absent" vs the inventory's exact
  "12 absent / 1 partial"). `design.md` is the Architect's **preliminary** read; the finalized count
  lives in the deliverable (`salvage-reuse-inventory.md` §3), which is authoritative. Rewriting the
  Architect's preliminary artifact is out of SW-6 scope and would misrepresent that record. Flagged
  here as accepted, non-blocking.

## 5. Doc hygiene check

- All produced/edited Markdown files render as valid CommonMark and end with a single trailing
  newline; tables are column-consistent. (`salvage-reuse-inventory.md`, this `docs.md`.)
- No credentials, raw window/app titles, prompt/response text, terminal-command bodies, or
  environment dumps in the produced/edited documentation.

## Verdict

**DOCS complete.** No README/runbook drift; no L2 customer-facing or operational doc update is due
for this read-only spike. Handoff documentation for TASK-002/003/006/007 is sufficient. One minimal,
additive handoff-clarity note was applied to the inventory's §7; the gated correctness/security
findings are undisturbed. Ready for Release Manager (SW-6 release-doc gate) to mark the change ready.
