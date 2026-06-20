# Architecture review — TASK-032 ai-time-suggestions

**Verdict: PASS.** One OpenSpec change, internally sequenced A→B→C. No split into separate changes. No
research mandate. Design accepted; ready for SW-2 implementation starting Workstream A.

## 1. Gap diagnosis (grounded in code, not just the report)

Janne's real-app test: backfill imports correctly (*"7 new traces across 9 environments, 813 duplicates,
0 skipped"*, `veronavi` +7) but **nothing appears in Projects/time entries**. Confirmed in code:

- Imported evidence lands in `langfuse_ai_evidence` (`src-tauri/src/langfuse/store.rs:38`) and is
  *associable* to a project only at read time via `list_evidence_projects_repo`
  (`src-tauri/src/env_mapping/mod.rs:212`). That IPC is registered (`lib.rs:920`) but **no frontend view
  consumes it** (`grep list_evidence src/` → nothing).
- `time_entries` is written **only** by the manual form (`create_time_entry`, `lib.rs:270`). Today /
  Projects / Reports all read `time_entries`. There is **no aggregation, no suggestion record, no
  review/accept path** between evidence and entries.

So imported AI work is structurally invisible. This is exactly the two architecture components that were
drawn but never built:

| Architecture component (`03_architecture_plan.md`) | Code status today |
| --- | --- |
| **Suggestion Engine** — suggestions w/ confidence/source/reason; never finalizes totals (§3, §6; DEC-006) | **absent** |
| **Review & Approval UI** — human approval gate; approve/dismiss blocks (§3) | **absent** |
| `Store → Classifier → Review → Store` data flow (§3 diagram) | only `Store` exists |

TASK-029 deferred this leg (`task-029/proposal.md:145`, "→ TASK-030"); TASK-030 became the
create-project fix, so it was never picked up. **This change realizes the planned architecture — it is
not a redesign.**

## 2. Scope decision — one change, not three (the "split" question)

The task asked whether this must split backend/frontend/data. **It need not.** It is one cohesive
capability (`ai-time-suggestions`) with one spec contract. Splitting into three OpenSpec changes would
fragment a single capability's spec across three change dirs and three review cycles for no boundary
benefit. Precedent: **TASK-029 shipped backend + frontend + data as one change with four sequenced
workstreams** — the house pattern. I apply the same here:

- **A** (suggestion engine + `time_entry_suggestions` table) — backend; **gates B, C**.
- **B** (accept/dismiss IPC + `origin` tag + DEC-003 reporting separation) — backend.
- **C** (Suggestions review/accept view) — frontend.

with a **checkpoint after A** (engine output verified against a seeded fixture) before B/C. Each
workstream can be its own commit/PR inside the change. This gives the staging of a split without the
fragmentation. *If* implementation reveals A alone is larger than expected, A may be PR-split — that is a
PR-boundary call for the dev, not an OpenSpec-change split.

**No component boundary is crossed improperly.** The change touches the planned Suggestion Engine + Review
UI + Store legs that the architecture explicitly couples (`Store→Classifier→Review→Store`). It does
**not** touch capture, the importer's import behaviour, or the runtime observer.

## 3. Architecture-critical design constraints (enforced in `design.md`)

1. **DEC-003 — AI evidence separate from human time.** `get_summary` (`lib.rs:348`) currently sums all
   `duration_minutes` indiscriminately. Writing accepted AI blocks into `time_entries` with no marker
   would conflate AI and human billable time → DEC-003 violation. Mitigation: `time_entries.origin`
   (`'manual'` default / `'ai_suggested'`) + origin-separated reporting in summary and CSV, realizing
   APPROVED_SUMMARY's `approved_human_duration` vs `ai_runtime_duration` split. **This is the single most
   important constraint; SW-3/SW-5 must verify it.**
2. **DEC-006 — suggestion-first, human-approved.** Accept is the only writer of a time entry; import and
   generation never post. Mirrors the env-mapping create-project contract (TASK-027/030).
3. **DEC-004 posture — absence ≠ zero.** Untimed evidence → unknown-duration suggestion (never 0-min);
   unmapped evidence → map-prompt (never dropped/zeroed); unknown tokens/cost → "—".
4. **SEC-012 — secret-free** (extends SEC-010/011). Suggestions/UI/provenance carry environment names,
   project refs, time bounds, aggregate numbers, counts, health only. Note `session_id` is already an
   opaque hashed id; it is used only as a **count/grouping key**, never rendered as a value.
5. **DEC-001/017 — Vire authoritative, Langfuse read-only.** Engine is pure-local over SQLite; **no new
   egress**.

## 4. Research mandate — none

The engine reads the **already-normalized local `langfuse_ai_evidence`** table, not Langfuse. The v3
trace/usage/cost/session/timestamp semantics were verified in TASK-007 and operationalized in TASK-029.
No external Langfuse re-verification is needed for this change.

## 5. ADR

**DEC-033 (proposed)** — AI evidence → suggested time entries (review-first; AI time kept separate). See
`proposal.md` for the full decision, status, and alternatives. Routed to BA via `feedback_to_ba[]` F3.

## 6. feedback_to_ba[]

- **F1 (decision input).** Default inter-trace clustering gap = **30 min** and the AI-vs-human reporting
  split are design defaults chosen to honor DEC-003/004. Confirm the 30-min default and whether AI cost
  should appear in CSV export or summaries only. *(non-blocking; sensible defaults shipped.)*
- **F2 (boundary confirmation).** Should an accepted AI-origin entry be *reclassifiable* by the user into
  billable human time (which would re-cross the DEC-003 separation deliberately), or remain permanently
  AI-origin? This change keeps it permanently AI-origin. *(non-blocking; conservative default shipped.)*
- **F3 (decision-log).** Record **DEC-033** in the canonical BA decision log (`07_decision_log.md`),
  alongside the still-open DEC-030/031/032 proposed by TASK-029.

None of these block SW-2: the design ships conservative, guarantee-preserving defaults.

## 7. NFR implications

- **Security/privacy:** SEC-012 keeps the new surface secret-free; no egress; local SQLite only. ✔
- **Auditability:** every time entry's `origin` is durable; suggestions retain status + provenance; AI
  totals are auditable separately from human totals. ✔
- **Maintainability:** new `suggestions/` module is a separate adapter (matches §9 "keep adapters
  separate"); additive schema only. ✔
- **Performance:** Janne-local scale; one pass over `langfuse_ai_evidence` joined to a small map. ✔

## 8. Exact next action for SW-2

Implement **Workstream A** (suggestion engine + `time_entry_suggestions` table + unit tests) per
`tasks.md` A1–A5 and `design.md` §1–§2, on branch `feat/task-032-ai-time-suggestions`. Stop at the
**A-checkpoint** (verify engine output against a seeded fixture) before starting B/C. Do not implement
capture, import changes, or any UI before A passes.
</content>
