# TASK-032 — AI time-entry suggestions from imported Langfuse evidence (review-first, never auto-posted)

## Why

Janne ran a real-app test of the post-TASK-029 build. **Langfuse backfill now imports correctly** —
example result: *"Imported 7 new traces across 9 environments, 813 duplicates, 0 skipped"*, with
`veronavi` contributing 7 new traces. The import pipeline (TASK-019→029) works end to end and persists
normalized AI evidence in `langfuse_ai_evidence` (environment, trace id, session id, AI start/end
timestamps, tokens, cost, health — `src-tauri/src/langfuse/store.rs:38`).

**But nothing appears in Projects or time entries.** The imported evidence is invisible to the user:

- `langfuse_ai_evidence` rows are only *associable* to a project at read time via the env→project map
  (`list_evidence_projects_repo`, `src-tauri/src/env_mapping/mod.rs:212`). That function exists and is
  even registered as an IPC command (`lib.rs:920`), **but no frontend view renders it** (`grep
  list_evidence src/ → no hits`).
- Nothing ever aggregates evidence into **time blocks**, produces **suggestions**, or offers a
  **review/accept** path. The `time_entries` table is only ever written by the manual-entry form
  (`create_time_entry`, `lib.rs:270`). The Projects, Today, and Reports views all read `time_entries`,
  so imported AI work simply does not exist for them.

This is not a regression. It is the **two architecture components that were specified but never built**:

- **Suggestion Engine** — *"generate project/customer and billable-category suggestions with confidence,
  source, and reason; never finalizes billable totals"* (`03_architecture_plan.md` §3, §6; DEC-006).
- **Review & Approval UI** — *"prioritize uncertain/unclassified blocks; approve, split, merge, delete…
  human approval gate"* (`03_architecture_plan.md` §3).

The data-flow `Store → Classifier → Review → Store` (§3 component diagram) is drawn but the
`Classifier` and `Review` legs do not exist in code. TASK-029 explicitly deferred this
(*"Time-entry suggestion from imported AI evidence → TASK-030"* — `task-029/proposal.md:145`); TASK-030
became the create-project-button fix instead, so the suggestion leg was never picked up. **TASK-032 is
that work.**

No external Langfuse re-verification is required: the suggestion engine reads the **already-normalized
local `langfuse_ai_evidence` table**, not Langfuse. TASK-007/029 already verified the v3 trace, usage,
cost, session, and timestamp semantics that populate that table. **No research mandate.**

## What Changes

One OpenSpec change, **three sequenced workstreams (A → B → C)**, realizing one cohesive capability:
turn imported AI evidence into **user-reviewable time-entry suggestions**. A is the engine and gates B
and C. See `design.md` for the algorithm, data model, and IPC contract; `arch-review.md` §Scope for why
this stays one change rather than three.

### Workstream A — Suggestion engine + persistence  *(backend developer)*  *(gates B, C)*

- Add an importer-adjacent **Suggestion Engine** that reads mapped `langfuse_ai_evidence` (joined to a
  project via `langfuse_env_project_map`) and aggregates traces into **suggested time blocks**, grouped
  by `(project, local date)` and clustered into contiguous blocks by a configurable inter-trace **gap
  threshold** (default 30 min — DEC-033, tunable). Each block carries: span start/end, suggested
  duration, summed tokens, summed cost + currency, trace count, session count, worst health, a
  confidence bucket, a source string (`langfuse:<environment(s)>`), and a human-readable reason.
- Persist suggestions in a new additive table `time_entry_suggestions` keyed by
  `(project_id, date, block_start_ts)` with a lifecycle status `pending | accepted | dismissed`.
  **Generation is idempotent**: re-running replaces only `pending` rows; `accepted`/`dismissed`
  decisions are durable and never regenerated away.
- **absence ≠ zero (preserved):** a `(project, date)` group whose traces carry **no usable
  timestamps** yields a suggestion with **unknown duration flagged for manual entry**, never a 0-minute
  block. Evidence in an **unmapped** environment is surfaced as *"N traces in unmapped env X — map it to
  generate suggestions"*, never silently dropped or zeroed. Degraded source health is carried onto the
  suggestion, never converted to absence-as-zero.

### Workstream B — Review / accept / dismiss IPC + AI-origin time entry  *(backend developer)*

- New IPC: `list_time_entry_suggestions` (with optional regenerate), `accept_time_entry_suggestion`,
  `dismiss_time_entry_suggestion`. Accept MAY carry user edits (date / start / end / project / note)
  before commit (review-first; DEC-006).
- **Accept is the only path to a `time_entries` row.** On accept, create one time entry tagged
  `origin = 'ai_suggested'` with its AI provenance (environment, tokens, cost, trace/session counts),
  and mark the suggestion `accepted`. Dismiss marks it `dismissed` and writes nothing. **No
  auto-posting** — generation and import never create a time entry.
- **DEC-003 (AI evidence separate from human time) preserved.** Add `origin` to `time_entries`
  (default `'manual'`). AI-origin duration is reported **distinctly** from human (`'manual'`) duration —
  `get_summary` and CSV export separate the two totals (realizing APPROVED_SUMMARY's
  `approved_human_duration` vs `ai_runtime_duration`); AI cost is carried as its own figure. An accepted
  AI block is **not** silently summed into a billable human total.

### Workstream C — Review/Accept UI  *(frontend developer)*

- A new **"Suggestions"** view in the nav (`src/main.ts` `views`) listing suggested blocks grouped by
  project and date, each showing duration, time span, tokens, cost, trace/session counts, health, and
  the reason; with **Accept**, **Edit-then-accept**, and **Dismiss** controls.
- Surfaces **absence ≠ zero**: unmapped-environment evidence prompts mapping (links to Settings);
  unknown-duration blocks are flagged for manual completion; degraded source health shows the existing
  banner. Empty state explains *why* (no evidence imported / no environments mapped) rather than showing
  a bare empty list.
- Secret-free (SEC-012): the view renders only environment names, project references, time bounds,
  aggregate token/cost numbers, counts, and health — never prompt/session/metadata content, raw payload,
  credentials, or raw session-id values.

## Impact

- **Affected code (A):** new `src-tauri/src/suggestions/` (engine + store + tests); reads
  `langfuse_ai_evidence` + `langfuse_env_project_map`; `lib.rs` `init_db` migration call.
- **Affected code (B):** `src-tauri/src/suggestions/` (accept/dismiss repo), `src-tauri/src/lib.rs`
  (new IPC commands; `time_entries.origin`; origin-aware `get_summary` + `export_report_csv`).
- **Affected code (C):** `src/main.ts` (new view + render + bind), `src/forms.ts`/`src/html.ts`
  (suggestion row + edit affordance), `src/style.css` (suggestion list styling).
- **Data model:** additive only — one new table `time_entry_suggestions`; one new nullable-defaulted
  column `time_entries.origin` (`ALTER TABLE … ADD COLUMN`, idempotent per the existing
  `add_column_if_absent` pattern, `langfuse/store.rs:73`). No existing table/column altered destructively;
  forward/backward compatible (reverting leaves the table unused and `origin` defaulting to `'manual'`).
- **Affected specs:** **ADD `ai-time-suggestions`** (new capability: generate, review, accept/dismiss,
  absence≠zero, secret-free, AI-vs-human separation). No existing capability spec is modified; the
  reporting-separation requirement is stated inside the new capability spec to keep the change
  self-contained.
- **Security (SEC-012):** suggestions, the review surface, and the AI-origin provenance carried onto an
  accepted entry are secret-free — no credentials, raw bodies, `serde` strings, raw session ids, or
  prompt/session/metadata content. Extends SEC-010 / SEC-011. **No new network egress** — the engine is
  pure-local over the SQLite store; Langfuse is not contacted.
- **Out of scope (clean boundaries):**
  - **Automatic capture / macOS activity evidence** (DEC-002) — unrelated; not touched.
  - **Re-importing or changing import behaviour** — TASK-029 owns the importer; this change only *reads*
    what it persisted.
  - **Billable-category / customer classification beyond project** — the engine suggests
    `project + time block`; billable-category suggestion (DEC-006 full scope) is a later increment.
  - **Split / merge / multi-trace manual block editing** beyond accept-with-simple-edits — the review UI
    ships accept / edit-fields / dismiss; richer block surgery is deferred.
  - **Raw-trace retention enforcement** (DEC-007 gap flagged by TASK-029 F2) — unchanged here.

## ADR — DEC-033 (proposed): AI evidence → suggested time entries (review-first, AI time kept separate)

**Decision.** Realize the planned Suggestion Engine + Review/Approval components by aggregating mapped
`langfuse_ai_evidence` into suggested time blocks (grouped by `(project, date)`, clustered by a default
30-minute inter-trace gap), persisted as `time_entry_suggestions` with a durable `pending/accepted/
dismissed` lifecycle. A `time_entries` row is created **only** by an explicit human accept; import and
generation never post time. Accepted entries are tagged `origin='ai_suggested'` and their duration/cost
are reported **separately** from manually-entered human time (DEC-003). Absence is never rendered as
zero (DEC-004 posture extended to suggestions). Vire remains the cost/time authority; Langfuse is read
only (DEC-001/DEC-017).

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log via
`feedback_to_ba[]` (see `arch-review.md`).

**Alternatives considered.** (1) *Auto-create time entries on import* — rejected: violates DEC-006
(suggestion-first) and DEC-003 (would inject AI time into human billable totals). (2) *Write accepted AI
blocks straight into `time_entries` with no origin tag* — rejected: conflates AI and human time in
`get_summary` (`lib.rs:348`), breaking DEC-003. (3) *Keep AI blocks in a fully separate table, never in
`time_entries`* — rejected: the reported expectation is that AI work becomes reviewable **entries**; an
`origin`-tagged row with separated reporting satisfies both visibility and DEC-003 with less surface.
(4) *Split into three OpenSpec changes (engine / IPC / UI)* — rejected: one capability, one spec; see
`arch-review.md` §Scope. Internal A→B→C sequencing with a checkpoint after A gives the same staging
without fragmenting the contract.
</content>
</invoke>
