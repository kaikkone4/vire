# Design — TASK-032 AI time-entry suggestions

Technical design SW-2 implements from. Scope, rationale, and the architecture-fit/feedback_to_ba are in
`arch-review.md`. Do the minimum here — no extra refactors, no capture, no import changes.

## 1. Data model (additive only)

### 1.1 New table `time_entry_suggestions`

```sql
CREATE TABLE IF NOT EXISTS time_entry_suggestions (
  id              TEXT PRIMARY KEY,            -- uuid
  project_id      TEXT NOT NULL REFERENCES projects(id),
  date            TEXT NOT NULL,               -- local YYYY-MM-DD of block start
  block_start_ts  TEXT,                        -- local 'YYYY-MM-DD HH:MM:SS', NULL when no usable ts
  block_end_ts    TEXT,                        -- NULL when no usable ts
  duration_minutes INTEGER,                    -- NULL == unknown (absence ≠ zero); never 0 for "no ts"
  trace_count     INTEGER NOT NULL,
  session_count   INTEGER NOT NULL,
  total_tokens    INTEGER,                     -- NULL when absent, never 0-as-absence
  cost_total      REAL,                        -- NULL when absent
  cost_currency   TEXT,
  health          TEXT NOT NULL,               -- worst health across the block's evidence
  confidence      TEXT NOT NULL,               -- 'high' | 'medium' | 'low'
  source          TEXT NOT NULL,               -- e.g. 'langfuse:veronavi'
  reason          TEXT NOT NULL,               -- human-readable, secret-free
  status          TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'accepted' | 'dismissed'
  accepted_entry_id TEXT,                       -- set when accepted; FK-ish to time_entries.id
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  UNIQUE (project_id, date, block_start_ts)
);
CREATE INDEX IF NOT EXISTS idx_suggestions_status_date ON time_entry_suggestions(status, date);
```

Natural key `(project_id, date, block_start_ts)` makes regeneration idempotent. Migrate in a new
`suggestions::store::migrate(conn)` called from `init_db` **after** `projects`/`time_entries` exist
(FK ordering, mirror `env_mapping::migrate` at `lib.rs:110`).

### 1.2 `time_entries.origin`

```rust
// in init_db, after the time_entries CREATE TABLE, idempotent:
add_column_if_absent(conn, "time_entries", "origin", "TEXT NOT NULL DEFAULT 'manual'")
```

Reuse the existing `add_column_if_absent` helper pattern (`langfuse/store.rs:73`). Manual entries keep
`'manual'`; accepted suggestions write `'ai_suggested'`. `TimeEntry`/`TimeEntryInput` gain `origin`
(default `'manual'` on the manual path; the manual form does not expose it).

## 2. Suggestion engine (Workstream A)

Pure function over the store, no network. New module `src-tauri/src/suggestions/`
(`mod.rs`, `engine.rs`, `store.rs`, `tests.rs`).

**Inputs.** All `langfuse_ai_evidence` rows joined to a mapped project via `langfuse_env_project_map`
(reuse the join shape from `list_evidence_projects_repo`, `env_mapping/mod.rs:215`), plus their
`ai_start_ts`, `ai_end_ts`, `session_id`, `total_tokens`, `cost_total`, `cost_currency`, `health`.
Rows whose environment is **unmapped** are excluded from blocks and reported separately (see §2.4).

**Algorithm.**
1. Convert `ai_start_ts`/`ai_end_ts` to local time; bucket rows by `(project_id, local date of start)`.
2. Within a bucket, sort timed rows by start; **cluster** into contiguous blocks: open a new block when
   `next.start − current_block.end > GAP` (`GAP = 30 min`, the DEC-033 default, read from a single
   constant; flagged to BA F1 as tunable). Block `start = min(starts)`, `end = max(ends)`.
3. `duration_minutes = max(1, round((end − start) minutes))` for timed blocks.
4. Aggregate over the block: `trace_count`, `session_count = distinct non-null session_id`,
   `total_tokens = Σ` (NULL if every contributing row is NULL), `cost_total`/`cost_currency` (Σ within a
   single currency; mixed-currency → leave `cost_total` NULL and note it in `reason`), `health = worst`.
5. `confidence`: **high** = all rows timed + healthy; **medium** = some degraded health or mixed
   sources; **low** = any missing timestamps in the block.
6. `reason`: e.g. *"4 Langfuse traces, 2 sessions in env `veronavi`, 09:12–10:48"* — secret-free.

**absence ≠ zero (§2.4).**
- A `(project, date)` bucket whose rows are **all untimed** → one suggestion with
  `block_start_ts = NULL`, `duration_minutes = NULL`, `confidence = 'low'`,
  `reason = "N traces with no usable timestamps — needs manual time"`. **Never** a 0-minute block.
- Rows in **unmapped** environments are returned in a separate `unmapped` summary
  `[{environment, trace_count}]` for the UI to prompt mapping — never dropped, never zeroed.
- `total_tokens`/`cost_total` NULL means *unknown*, rendered as "—", never "0".

**Idempotent generation.** `generate(conn)`:
- `DELETE FROM time_entry_suggestions WHERE status = 'pending'`,
- recompute and `INSERT OR IGNORE` blocks (the UNIQUE key skips any that match an `accepted`/`dismissed`
  row, so decided blocks are never resurrected as new pending duplicates),
- return the fresh `pending` set plus the `unmapped` summary.

## 3. IPC contract (Workstream B)

```rust
#[tauri::command] fn list_time_entry_suggestions(state, regenerate: bool) -> CmdResult<SuggestionList>
#[tauri::command] fn accept_time_entry_suggestion(state, id: String, edits: Option<SuggestionEdit>) -> CmdResult<TimeEntry>
#[tauri::command] fn dismiss_time_entry_suggestion(state, id: String) -> CmdResult<()>
```

- `SuggestionList { suggestions: Vec<Suggestion>, unmapped: Vec<UnmappedEnv> }`.
- `SuggestionEdit { project_id?, date?, start_time?, end_time?, note? }` — optional overrides applied
  before commit; same validation as `create_time_entry` (`parse_duration`, length caps).
- **accept**: validate (an unknown-duration block REQUIRES edits supplying start/end, else error — never
  invents a duration); create a `time_entries` row with `origin='ai_suggested'` and a note carrying the
  secret-free provenance summary; set the suggestion `status='accepted'`, `accepted_entry_id`. Single
  transaction. Re-accepting an already-accepted/dismissed suggestion is rejected.
- **dismiss**: set `status='dismissed'`; write no time entry. Idempotent.
- Register all three in `generate_handler!` (`lib.rs:895`).

**Reporting separation (DEC-003).** `get_summary` (`lib.rs:348`) and `export_report_csv` split totals by
`origin`: return `human_minutes` (origin='manual') and `ai_minutes` (origin='ai_suggested') as distinct
figures rather than one `duration_minutes` sum. `SummaryRow` gains `ai_minutes` (or a parallel field);
keep the existing field meaning human-only to avoid silently changing prior numbers. CSV gains an
`origin` (or separate AI column) so AI time is never tallied into a billable human column.

## 4. Frontend (Workstream C)

- Add `'Suggestions'` to `views` and `View` (`src/main.ts:12,21`); `renderSuggestions()` + bind, same
  shape as the other render fns; route in `render()` (`main.ts:46`).
- Load via `list_time_entry_suggestions` (regenerate=false on view open; a **"Refresh suggestions"**
  button calls regenerate=true).
- Render groups by project → date; each suggestion row shows duration (or "needs manual time"), span,
  tokens/cost ("—" when unknown), trace/session counts, health chip, confidence, reason, and
  **Accept / Edit / Dismiss**. Edit reveals inline date/start/end/note fields (reuse `forms.ts`
  helpers), then Accept submits with `edits`.
- Unmapped summary → a notice linking to Settings ("Map env `X` (N traces) to get suggestions").
- Empty/absence states explain the cause (no evidence imported / nothing mapped / all dismissed), never
  a bare empty table. Source-health banner reused from `sourceBanner()`.
- Scroll preservation (TASK-031) already applies via `shell()`.

## 5. Guarantees checklist (must all hold)

- **absence ≠ zero** — unknown duration/tokens/cost are NULL→"—"/"needs manual time", never 0;
  unmapped + degraded surfaced, never dropped. (DEC-004 posture.)
- **no auto-posting** — only `accept_time_entry_suggestion` writes `time_entries`. (DEC-006.)
- **AI ≠ human time** — `origin` tag + separated reporting. (DEC-003.)
- **no secrets** — environment/project/time/aggregate-number/count/health only; no payload, no raw
  session id, no prompt/metadata, no credentials. (SEC-012, extends SEC-010/011.)
- **Vire authoritative, read-only of Langfuse** — engine reads SQLite only; no egress. (DEC-001/017.)

## 6. Sequencing & checkpoint

A (engine + table + tests) → **checkpoint: engine output verified against a seeded fixture** → B (IPC +
origin + reporting) → C (UI). A gates B and C. Each workstream can be its own commit/PR within the one
change.

## 7. Tests (per workstream)

- **A:** engine unit tests (`suggestions/tests.rs`) — clustering at/over/under the gap; multi-session
  block; all-untimed → unknown not zero; unmapped excluded+reported; mixed currency → cost NULL;
  idempotent regenerate preserves accepted/dismissed.
- **B:** accept creates exactly one `origin='ai_suggested'` entry + marks accepted; accept of
  unknown-duration without edits errors; dismiss writes nothing; `get_summary`/CSV separate human vs AI.
- **C:** frontend pure-builder tests (`tests/*.test.mjs` style) for the suggestion row + empty/absence
  copy + "needs manual time" rendering; `npm run test:frontend` + `npm run build` green.
- **Rust:** `cargo test` green; `cargo fmt`/`clippy` clean.
</content>
