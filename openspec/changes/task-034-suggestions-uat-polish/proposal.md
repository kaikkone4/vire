# TASK-034 — Suggestions UAT polish (zero-duration accept, env-trackability clarity, cost → reports)

## Why

Janne ran a real-app UAT of the TASK-032 build (PR #27) and reported three issues. All three are
**small polish on the existing `ai-time-suggestions` capability** — not new capability, not active
capture. Each was traced to specific code; findings and rationale are in `arch-review.md`. A post-merge
code-to-spec check of TASK-032 then surfaced a fourth, spec-only item (Item 4 below) folded into this
change because it touches the same capability contract.

1. **Accepting a same-minute suggestion fails until the user nudges the end time.** When a suggested
   block's start and end fall in the **same clock minute** (e.g. a single short trace
   `09:12:05`–`09:12:50`), accept is rejected with *"End time must be after start time"* and the user
   must manually move the end forward by a minute. Root cause (`arch-review.md` §Item 1): the engine
   stores second-precision timestamps and computes `duration_minutes = max(1, round)` ≥ 1
   (`suggestions/engine.rs:251`), but `accept_suggestion_repo` re-derives start/end at **minute**
   granularity via `hhmm_from_block_ts` (`lib.rs:456`) and `parse_duration` rejects `mins <= 0`
   (`lib.rs:170`). The displayed span and the edit-panel pre-fill are both `HH:MM`, so the two read as
   equal and the engine's already-valid ≥1-minute duration is thrown away at accept.

2. **Some environments are never trackable/suggested and the UI does not explain why or what to do.**
   The unmapped-environment notice already exists (`suggestions-ui.ts:152` `unmappedNotice`), but it
   only covers *one* of several "no suggestion" causes. UAT shows the user could not tell, for a given
   environment, whether it was **not imported**, **imported but unmapped**, **mapped but untimed**
   ("needs manual time"), or **source disabled/down** — nor the action path for each.

3. **Langfuse cost reaches Suggestions but not Reports.** Confirmed (`arch-review.md` §Item 3): cost
   **amount** is extracted from Langfuse (`costDetails["total"]` / `calculatedTotalCost` / trace
   `totalCost` — `importer.rs:1127`), persisted to `langfuse_ai_evidence.cost_total`, aggregated by the
   engine, and **rendered in the Suggestions view** (`suggestions-ui.ts:76` `costLabel`). But it
   **stops there**: `time_entries` has no cost column, so an accepted AI block keeps its cost only as
   free text inside the entry note (`resolve_ai_note`, `lib.rs:465`); `SummaryRow` carries
   `ai_minutes` but **no cost** (`lib.rs:76`); Reports/Today cards show no cost (`main.ts:53`); CSV has
   no cost column (`lib.rs:432`). TASK-032's design intended *"AI cost carried as its own figure"* in
   summary/CSV (`task-032/design.md` §3) but that leg was descoped to minutes-only. **Currency** is a
   separate, genuine limitation: the Langfuse source model carries no per-call currency and `AiEvidence`
   has no currency field, so `store.rs:381` writes `cost_currency = NULL` always — currency is not
   source-derivable and is treated here as a documented limitation, not a code fix.

4. **Spec/code divergence on the inter-trace gap (post-merge check; spec-only).** The TASK-032 spec
   words the clustering gap as *"a configurable inter-trace gap (default 30 minutes)"*
   (`task-032/specs/ai-time-suggestions/spec.md` §"Imported AI evidence is aggregated…"), but the engine
   uses a **fixed** 30-minute gap: `const GAP_MINUTES: i64 = 30` (`suggestions/engine.rs:21`), with **no**
   settings field, persistence, IPC, or UI anywhere in `src-tauri/` or `src/`. Configurability was never
   built and no UAT item asks for it. The minimal fix is to **correct the contract to the fixed policy**,
   not to build a config surface — see "What Changes" below and `design.md` §7.

## What Changes

**One OpenSpec change**, three small workstreams on one capability (`arch-review.md` §Scope explains why
this is **not split**). All changes are additive and backward-compatible.

### Workstream A — Accept never stores zero; end rounds up to the next minute  *(backend-led; backend + frontend)*

- In `accept_suggestion_repo` (`lib.rs:485`): when the minute-granularity end is not after the start,
  **adjust end to `start + max(1, suggestion.duration_minutes)`** before `parse_duration`, so a valid
  timed suggestion is **always** acceptable and **never** stores a zero (or negative) duration —
  including via direct IPC, not only through the UI. The engine's existing ≥1-minute duration is the
  source of truth for the bump.
- In the edit panel (`suggestions-ui.ts` `suggestionRow`): when the pre-filled start and end render
  equal, **default the End field to start + duration** so the displayed, editable span matches what
  accept will store. (DEC-034 proposed — see ADR below.)

### Workstream B — AI cost reaches Reports and CSV  *(backend-led; backend + frontend)*

- Additive schema: `time_entries.cost_total REAL` (+ `cost_currency TEXT`) via the existing
  `add_column_if_absent` idempotent pattern (`langfuse/store.rs:73`). Nullable; manual entries leave it
  NULL (absence ≠ zero).
- On accept, copy the suggestion's `cost_total`/`cost_currency` onto the created `ai_suggested` entry.
- `summary_repo`/`SummaryRow`: add a separate **`ai_cost_total`** (and `ai_cost_currency`) summed over
  `origin='ai_suggested'`, mirroring the existing `ai_minutes` separation (DEC-003). Human cost stays
  unreported (manual entries have no cost) — AI cost is never folded into a human figure.
- `export_csv_repo`: add a `cost_total` (and `cost_currency`) column so AI cost is a structured CSV
  field, not buried in the note.
- Frontend: surface AI cost on the Reports/Today AI-suggested line (`main.ts` `summaryCards`), rendered
  "—" when unknown (absence ≠ zero).

### Workstream C — Environment trackability is explained with an action path  *(frontend)*

- Improve the Suggestions copy so each "no suggestion" cause is named with its action (`suggestions-ui.ts`):
  **unmapped** → map it in Settings (existing notice, keep); **mapped-but-untimed** → the "needs manual
  time" block already exists, label it clearly as *not auto-trackable, add time manually*; **source
  disabled/down** → reuse the existing source banner with a pointer to Settings; **not imported** →
  empty-state copy points to import. No silent drops, no zeros (DEC-004 posture preserved).

### Spec correction — inter-trace gap is a fixed 30-minute policy  *(spec-only, NO code)*

- MODIFY the `ai-time-suggestions` requirement *"Imported AI evidence is aggregated into suggested time
  blocks"* to state a **fixed 30-minute** inter-trace gap instead of *"configurable … (default 30
  minutes)"*, matching the shipped engine (`GAP_MINUTES = 30`). **No code change** — `engine.rs` already
  implements the fixed gap; SW-2 only verifies the const and the existing boundary test still hold
  (`tasks.md` D1–D2). Making the gap configurable is **rejected for this change** (net-new settings +
  persistence + engine plumbing + UI; no UAT driver; contradicts "additive polish"). If a future task
  needs it, that is a fresh BA decision (`arch-review.md` feedback_to_ba F4). Rationale in `design.md` §7.

## Impact

- **Affected code (A):** `src-tauri/src/lib.rs` (`accept_suggestion_repo`), `src/suggestions-ui.ts`
  (edit-panel End default).
- **Affected code (B):** `src-tauri/src/lib.rs` (`init_db` migration, `accept_suggestion_repo`,
  `summary_repo`, `SummaryRow`, `export_csv_repo`), `src/main.ts` (`summaryCards`).
- **Affected code (C):** `src/suggestions-ui.ts` (notice/empty-state/untimed copy), `src/style.css` if
  needed.
- **Data model:** additive only — two nullable columns on `time_entries` (`cost_total`, `cost_currency`),
  idempotent `ADD COLUMN`. No table altered destructively; reverting leaves the columns unused.
- **Affected specs:** **MODIFY `ai-time-suggestions`** (refine accept-never-zero, cost-in-reporting, and
  absence-explanation requirements; plus the spec-only contract correction making the inter-trace gap a
  fixed 30-minute policy). Depends on TASK-032 being merged and its capability archived first — see
  `arch-review.md` §Sequencing.
- **Security (SEC-012):** unchanged posture — cost amount and currency are aggregate numbers, already
  deemed secret-free in TASK-032; no new fields expose payloads, raw session ids, prompts, or
  credentials. No new network egress (engine still pure-local).
- **Out of scope:** active/macOS capture (DEC-002); re-import or importer changes (TASK-029 owns it);
  deriving a real currency from Langfuse (not source-available — documented limitation); split/merge
  block surgery (deferred in TASK-032); **building a configurable inter-trace gap** (Item 4 corrects the
  contract to the existing fixed policy only — no config surface, no `engine.rs` change).

## ADR — DEC-034 (proposed): accept rounds a sub-minute block up; never stores zero duration

**Decision.** A timed AI suggestion is always acceptable: when its start and end resolve to the same
clock minute, accept stores `end = start + max(1, suggested duration_minutes)` rather than rejecting it.
Vire never stores a zero- or negative-duration time entry. The engine's `max(1, round)` duration remains
authoritative for the bump.

**Status.** Proposed (this change). Routed to BA-flow Architect via `feedback_to_ba[]` (see
`arch-review.md`).

**Alternatives considered.** (1) *Keep rejecting and force a manual edit* — rejected: that is the UAT
complaint. (2) *Store second precision in `time_entries`* — rejected: the whole app is minute-granular
(`%H:%M`); changing the time-entry grain is far beyond UAT polish. (3) *Round at the engine instead of
accept* — rejected as insufficient alone: direct IPC accept must also be safe; the authoritative guard
belongs at the write path, with the edit-panel default as a UX echo.
