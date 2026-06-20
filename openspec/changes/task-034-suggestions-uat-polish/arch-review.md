# Architecture review — TASK-034 Suggestions UAT polish

**Role:** SW Architect (SW-1 design review). **Verdict:** PASS — proceed to SW-2.
**Scope class:** small UAT polish on the existing `ai-time-suggestions` capability (TASK-032). No new
component, no active capture, no importer change. All edits additive and backward-compatible.

## Triage of the three UAT items (each traced to code)

### Item 1 — accepting a same-minute suggestion fails until the user nudges the end  — REAL BUG

- The engine works at **second** precision and already guarantees a non-zero duration:
  `duration = max(1, round((end-start)/60))` (`src-tauri/src/suggestions/engine.rs:251-252`);
  `block_start_ts`/`block_end_ts` are stored as `'YYYY-MM-DD HH:MM:SS'`.
- Accept throws that away: `accept_suggestion_repo` re-derives start/end at **minute** granularity via
  `hhmm_from_block_ts` (`lib.rs:456`, takes `HH:MM`), then `parse_duration` rejects `mins <= 0`
  (`lib.rs:170-172`). A block like `09:12:05`–`09:12:50` → both `09:12` → rejected. The edit-panel
  pre-fill (`suggestions-ui.ts:105-106`, `timeOfDay`) is also `HH:MM`, so the user sees `09:12`–`09:12`
  and must bump the end by hand — exactly the report.
- **Decision:** authoritative fix at the **write path** (accept): when end is not after start, store
  `end = start + max(1, suggestion.duration_minutes)`; never store zero/negative. Mirror it as an
  edit-panel default for UX agreement. `parse_duration` and the manual entry path stay unchanged (a
  human typing `start == end` is a real error, not an engine artifact). New ADR **DEC-034** (proposed).
- **Owner:** backend developer (accept) + frontend developer (edit default). Backend-led.

### Item 2 — some environments are not trackable/suggested; UI lacks clear explanation + action  — UX GAP (partial today)

- The unmapped notice exists and links to Settings (`suggestions-ui.ts:152-165`), and the untimed
  "needs manual time" block exists with a hint (`suggestions-ui.ts:107-109`). But the surface does not
  *name and differentiate* the distinct causes a user actually hits: (a) env **not imported**, (b)
  imported **but unmapped**, (c) mapped **but untimed** (not auto-trackable), (d) source **disabled/down**.
- **Decision:** frontend-only copy/logic polish over **data already available** — make each cause
  explicit with its action path; no new backend signal required for the four cases above. The single
  case that *would* need backend help — distinguishing "mapped-but-no-evidence" from "not-imported" —
  is **deliberately deferred** (not worth a backend round-trip for UAT polish); if SW-2 finds it
  necessary, that is a scope flag back to SW-Architect, not an invented change. Extends DEC-004 posture
  to UX; no new DEC.
- **Owner:** frontend developer.

### Item 3 — confirm Langfuse cost reaches Suggestions/Reports  — PARTIAL: reaches Suggestions, NOT Reports/CSV

- **Cost amount IS plumbed to Suggestions:** extracted from Langfuse (`Generation::cost()` reads
  `calculatedTotalCost` / `costDetails["total"]`, trace `totalCost` fallback — `importer.rs:1127-1130`,
  `model.rs:444-461`), persisted to `langfuse_ai_evidence.cost_total` (`langfuse/store.rs`), aggregated
  by the engine (`engine.rs:286`, `Aggregate` `cost_total`), and rendered in the Suggestions view
  (`suggestions-ui.ts:76-79`, `costLabel`). **This leg works end-to-end.**
- **Cost STOPS before Reports:** `time_entries` has no cost column → an accepted AI block keeps cost
  only as free text in the note (`resolve_ai_note`, `lib.rs:465-478`); `SummaryRow` has `ai_minutes`
  but **no cost** (`lib.rs:76-85`); Reports/Today cards show no cost (`main.ts:53`); CSV has no cost
  column (`lib.rs:432`). TASK-032 `design.md` §3 intended *"AI cost carried as its own figure"* in
  summary/CSV but that was descoped to minutes-only.
- **Currency is a genuine limitation, not a bug:** the Langfuse source model carries **no per-call
  currency**; `AiEvidence` (`langfuse/model.rs`) has no currency field; `store.rs:381` writes
  `cost_currency = NULL` by design. Currency is **not source-derivable**. Documented, not "fixed."
- **Decision:** include the **minimal fix** — additive `time_entries.cost_total`(+`cost_currency`)
  columns, populated on accept, summed separately in `summary_repo` (`ai_cost_total`) and added to CSV;
  surfaced on the Reports/Today AI line. This **completes the existing DEC-003** reporting-separation for
  cost (it is not a new architecture decision). Currency rendered "—" when NULL.
- **Owner:** backend developer (schema/accept/summary/CSV) + frontend developer (cards). Backend-led.

### Item 4 — inter-trace gap: spec says "configurable", code is fixed (post-merge check)  — SPEC/CODE DIVERGENCE (spec-only fix)

- **Found by:** post-merge code-to-spec check of TASK-032 (amendment, 2026-06-20).
- **Divergence:** TASK-032 spec — *"clustered into contiguous blocks by a configurable inter-trace gap
  (default 30 minutes)"* (`task-032/specs/ai-time-suggestions/spec.md`). Code — a hardcoded
  `const GAP_MINUTES: i64 = 30` (`suggestions/engine.rs:21`), consumed by the pure `cluster_by_gap`
  (`engine.rs:214-221`). A repo-wide check (`grep` over `src-tauri/` + `src/`) finds **no** settings
  field, persistence column, IPC command, or UI control for the gap — configurability was never built.
  No UAT item, and no other task, asks for it.
- **Decision — correct the contract to the fixed policy (minimal path):** MODIFY the requirement to say a
  **fixed 30-minute** gap, matching shipped behavior. **No code change** — the code is already correct;
  the spec overstated a config surface. Adding real configurability is rejected here: it is net-new
  feature scope (settings field + persistence + threading the value through the pure engine + a UI
  control), has no UAT driver, and contradicts this change's "additive UAT polish" class. 30 minutes is a
  sensible, already-tested heuristic (`suggestions/tests.rs:145` pins the at/over/under boundary).
- **Architecture fit:** the engine stays a pure local function with a fixed policy constant — consistent
  with DEC-001/017 (Vire authoritative, engine pure-local, no network). Crosses **no** component boundary
  in `03_architecture_plan.md`. No new DEC; logged as a contract correction + feedback_to_ba F4.
- **Owner:** none (spec-only). SW-2 verifies the const and existing test still hold (`tasks.md` D1–D2);
  SW-2 MUST NOT edit `engine.rs`.

## Scope decision — one change, NOT split

All three items polish the **same capability** and overlap the **same files** (`lib.rs`
`accept_suggestion_repo`; `src/suggestions-ui.ts`; `src/main.ts` cards). They are cohesive, additive, and
cross **no component boundary** in `03_architecture_plan.md` (Suggestion Engine / Review UI stay as-is;
only their accept + reporting + copy are refined). Splitting into three changes would fragment one spec
and create false ordering between edits to the same functions. → **One OpenSpec change,
`task-034-suggestions-uat-polish`**, three internal workstreams (A → B → C), each its own commit.

This is consistent with TASK-032's own §Scope reasoning (one capability → one change).

## Sequencing constraint (openspec)

This change **MODIFIES** requirements introduced by TASK-032, whose `ai-time-suggestions` capability is
**not yet archived** to `openspec/specs/` (PR #27 still open; `openspec list` shows it as a live change).
TASK-032 must be **merged and archived first**; only then does the MODIFIED delta have a base to apply
against and `openspec validate --strict` pass cleanly. SW-2 may begin implementation in parallel on the
branch, but should branch **from / rebase onto** merged TASK-032. Recorded so the pipeline does not
sequence TASK-034 ahead of the TASK-032 merge.

## Architecture-fit & invariants (all preserved)

- **DEC-006 no auto-posting** — accept remains the sole writer of an `ai_suggested` entry.
- **DEC-003 AI ≠ human** — extended: AI cost now reported separately (`ai_cost_total`), never folded in.
- **DEC-004 absence ≠ zero** — preserved: NULL cost/tokens → "—"; manual entries keep NULL cost;
  every no-suggestion cause surfaced, never zeroed.
- **SEC-012 secret-free** — unchanged: only aggregate numbers added; no payloads/raw session ids/prompts.
- **DEC-001/017 Vire authoritative, Langfuse read-only** — engine still pure-local; no new egress.

## feedback_to_ba[] (route to BA-flow Architect)

- **F1 — DEC-034 (propose & log).** "Accept rounds a same-minute suggestion up to a ≥1-minute span;
  Vire never stores a zero/negative-duration entry." Proposed in this change; needs a canonical
  `DEC-###` in the BA decision log. (Implementation policy realizing the suggestion-first contract.)
- **F2 — Cost currency is not source-derivable.** Langfuse provides cost amounts with no currency code.
  DEC-003's "cost with currency" should acknowledge currency as a future **display-config** setting, not
  a source-derived field. No code action this change beyond rendering "—"; flagged for the decision log.
- **F3 — TASK-032 implementation gap (informational).** TASK-032 delivered DEC-003 minute-separation but
  descoped cost-separation in summary/CSV (`task-032/design.md` §3 vs shipped `SummaryRow`). TASK-034
  completes it. Noted so the BA architecture record matches shipped behavior.
- **F4 — Inter-trace gap is a fixed 30-minute policy, not configurable (contract correction).** TASK-032
  spec said *"configurable … (default 30 minutes)"* but the engine ships a fixed `GAP_MINUTES = 30` and
  no config surface exists. TASK-034 corrects the contract to the fixed policy (spec-only, no code). The
  BA architecture record / decision log should reflect "fixed 30-minute clustering gap" as the intended
  contract; if a configurable gap is ever wanted, it needs a new BA decision + its own task (settings +
  persistence + engine plumbing + UI), not a polish amendment.

## Output for SW-2

- **Design to implement from:** `design.md` (this change dir). **Spec contract:** `specs/ai-time-suggestions/spec.md`.
- **Start with Workstream A** (`tasks.md` A1–A4): the zero-duration accept fix is the highest-signal UAT
  item and is self-contained in `accept_suggestion_repo` + the edit-panel default.
- Then B (cost plumbing) then C (trackability copy). Gates: `cargo test`/`fmt`/`clippy`,
  `npm run test:frontend`/`build`.
