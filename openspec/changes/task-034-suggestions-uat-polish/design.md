# Design — TASK-034 Suggestions UAT polish

Technical design SW-2 implements from. Scope and architecture-fit are in `arch-review.md`. Do the
minimum — no refactors, no capture, no importer changes. All edits are additive and backward-compatible.

## 1. Workstream A — accept never stores zero (DEC-034)

### 1.1 Backend (authoritative) — `accept_suggestion_repo` (`src-tauri/src/lib.rs:485`)

After deriving `start_time`/`end_time` (the existing `edits … or hhmm_from_block_ts(…)` block,
`lib.rs:513-529`), before `parse_duration`:

```rust
// DEC-034: a timed suggestion whose start and end fall in the same clock minute must still be
// acceptable. Round end up to start + the engine's already-valid duration (>= 1 min) so we never
// store a zero/negative span. Only applies when end is NOT after start; an explicit user edit that
// supplies a valid span is left untouched.
let end_time = bump_end_if_not_after(&date, &start_time, &end_time, suggestion.duration_minutes)?;
```

`bump_end_if_not_after`: if `parse_duration(date, start, end)` would yield `<= 0`, compute
`end = start + minutes`, where `minutes = suggestion.duration_minutes.filter(|m| *m > 0).unwrap_or(1)`,
format back to `%H:%M`, and return it; otherwise return `end` unchanged. Implement with the existing
`chrono` `NaiveDateTime` + `Duration::minutes`. Guard the same-day assumption already baked into
`parse_duration` (start/end share `date`); a bump that would cross midnight is clamped to `23:59` (a
1-minute bump from 23:59 is the only realistic case and is acceptable for UAT polish — note it in code).

Leave `parse_duration` itself unchanged (manual entry keeps rejecting `start == end` — that path is a
human typo, not an engine artifact). The fix is local to the accept path.

### 1.2 Frontend echo — `suggestionRow` (`src/suggestions-ui.ts:105`)

When `timeOfDay(block_start_ts) === timeOfDay(block_end_ts)` and `duration_minutes != null`, set the
End input's default `value` to `start + duration_minutes` (pure helper, e.g. `addMinutesHHMM(start, n)`)
so the visible editable span equals what accept will store. Keep `timeOfDay` for all other cases.
Tests: `tests/suggestionsUi.test.mjs` — same-minute block renders an End default strictly after Start.

## 2. Workstream B — AI cost reaches Reports and CSV (DEC-003 completion)

### 2.1 Schema (additive) — `init_db` (`src-tauri/src/lib.rs`)

```rust
// after the time_entries CREATE TABLE / origin column, idempotent:
add_column_if_absent(conn, "time_entries", "cost_total", "REAL")?;
add_column_if_absent(conn, "time_entries", "cost_currency", "TEXT")?;
```

Nullable; manual entries leave both NULL (absence ≠ zero — NULL renders "—", never 0). `TimeEntry` gains
`cost_total: Option<f64>`, `cost_currency: Option<String>`; `row_to_entry` reads them. The manual form
and `create_time_entry`/`update_time_entry` write NULL (do not expose cost on the manual path).

### 2.2 Accept copies cost — `accept_suggestion_repo` (`src-tauri/src/lib.rs:540`)

Extend the INSERT to carry `suggestion.cost_total`, `suggestion.cost_currency` into the new columns.
A user cost edit is **out of scope** (no cost field in `SuggestionEdit`); accept copies the suggestion's
figures verbatim.

### 2.3 Summary separation — `summary_repo` / `SummaryRow` (`src-tauri/src/lib.rs:363,76`)

Add `ai_cost_total: Option<f64>` (+ `ai_cost_currency: Option<String>`) to `SummaryRow`. Extend the SQL:

```sql
SUM(CASE WHEN e.origin='ai_suggested' THEN e.cost_total ELSE 0 END)  -- ai_cost_total (NULL-safe)
```

Use `SUM` (returns NULL when no AI rows → render "—"), and pick currency via
`MAX(CASE WHEN e.origin='ai_suggested' THEN e.cost_currency END)` (currency is uniform-or-NULL in
practice — see §4). Do **not** add a human cost column: manual entries have no cost; keep existing
`duration_minutes`/`ai_minutes` meanings stable so prior numbers never shift (mirror the TASK-032 B
comment at `lib.rs:370`).

### 2.4 CSV — `export_csv_repo` (`src-tauri/src/lib.rs:421`)

Add `cost_total,cost_currency` to the header and each row (empty string when NULL). Keep `origin` so a
consumer can still separate AI rows. Reuse `csv_escape` for the currency cell.

### 2.5 Frontend — `summaryCards` (`src/main.ts:53`)

Where the AI-suggested `<small>` line is rendered, append cost when present:
`AI-suggested ${h(ai)} · ${costLabel(ai_cost_total, ai_cost_currency)}` (import/duplicate the
`suggestions-ui.ts` `costLabel` shape; "—" when null). `Summary` type gains the two fields.
Tests: extend existing summary-card / reports frontend tests for the cost line + "—" absence.

## 3. Workstream C — environment trackability explanation (DEC-004 posture)

Frontend-only copy/logic in `src/suggestions-ui.ts`. Make the "why no suggestion" causes explicit, each
with an action — using data already available, **no new backend signal required**:

- **Unmapped env** (`unmappedNotice`, already present): keep; tighten copy to *"not trackable until
  mapped — Map in Settings"*.
- **Mapped but untimed** (the existing `duration_minutes == null` "needs manual time" row): relabel so
  it reads as *"this evidence has no usable time — not auto-trackable; add a start/end to log it"*
  (the hint at `suggestions-ui.ts:108` already says this; surface it on the row too, e.g. a small
  badge, not only inside the edit panel).
- **Source disabled/down**: reuse the existing source banner (`sourceBanner()` in `main.ts`) on the
  Suggestions view; ensure the empty state names this cause.
- **Not imported yet**: `emptyState` (`suggestions-ui.ts:169`) already names this — keep, point to
  Settings → Import.

If, during implementation, distinguishing "mapped-but-no-evidence" from "not-imported" needs a backend
signal, that is a **scope flag back to SW-Architect**, not an invented backend change (arch-review §Item 2
records this as the one place richer backend data *could* help but is deliberately deferred).

## 4. Currency — documented limitation (no code fix)

Langfuse v3 `costDetails` / legacy `calculatedTotalCost` are **numeric with no currency code**;
`AiEvidence` (`langfuse/model.rs`) has no currency field; `store.rs:381` writes `cost_currency = NULL` by
design. Currency is therefore **not source-derivable**. This change does not invent one. `cost_currency`
columns/fields are added for forward-compatibility and render "—"/blank when NULL. A future config-level
"display currency" is a separate decision — flagged to BA (`arch-review.md` feedback_to_ba F2), not done
here.

## 5. Guarantees checklist (must all hold)

- **never zero duration** — accept stores a span ≥ 1 minute; manual path unchanged. (DEC-034.)
- **absence ≠ zero** — NULL cost/tokens render "—", manual entries keep NULL cost. (DEC-004.)
- **AI ≠ human** — AI cost summed separately (`ai_cost_total`), never folded into human totals. (DEC-003.)
- **no auto-posting** — accept remains the sole writer of an `ai_suggested` entry. (DEC-006.)
- **no secrets / no egress** — only aggregate numbers added; engine still pure-local. (SEC-012, DEC-001/017.)
- **additive & reversible** — idempotent `ADD COLUMN`; reverting leaves columns unused.

## 6. Sequencing & tests

A and C are independent; B shares `lib.rs`/`suggestions-ui.ts` with A. Recommended order **A → B → C**,
each its own commit within the one change. Per workstream:

- **A:** Rust unit test — accept of a same-minute block stores duration ≥ 1 (no edits needed) and
  `end > start`; accept of a normal block unchanged; manual `create_time_entry` still rejects
  `start==end`. Frontend test — same-minute edit End default > Start.
- **B:** Rust — accept copies cost onto the entry; `summary_repo` returns `ai_cost_total` separate from
  human; manual entry yields NULL AI cost; CSV has the cost columns. Frontend — Reports card shows AI
  cost and "—" when null.
- **C:** Frontend — each cause (unmapped / untimed / disabled / not-imported) renders its named copy +
  action; no bare empty table; no "0".
- **Gates:** `cargo test` + `cargo fmt`/`clippy` clean; `npm run test:frontend` + `npm run build` green.

## 7. Inter-trace gap — contract correction, fixed policy (no code change)

Post-merge code-to-spec check (TASK-032) found a divergence: the TASK-032 spec said the clustering gap is
*"configurable … (default 30 minutes)"*, but the engine hardcodes it:

```rust
// src-tauri/src/suggestions/engine.rs:21
const GAP_MINUTES: i64 = 30;   // a new block opens when the next row starts > 30 min after block end
```

There is **no settings field, no persistence, no IPC, and no UI** for this value anywhere in
`src-tauri/` or `src/` — configurability was never built. No UAT item asks for it.

**Decision: keep the gap fixed at 30 minutes and correct the OpenSpec contract to match** (done in
`specs/ai-time-suggestions/spec.md`, MODIFIED "Imported AI evidence is aggregated into suggested time
blocks"). Making it configurable is rejected for this change: it would add a settings field +
persistence + engine plumbing (threading the value through the pure `cluster_by_gap`) + a UI control —
net-new feature scope that contradicts "small UAT polish, additive only" and has no UAT driver. 30 minutes
is a sensible, well-tested heuristic (`suggestions/tests.rs:145` `clustering_respects_the_gap_boundary`
already pins the at/over/under boundary).

**Implementation:** none. `GAP_MINUTES` stays as-is; do **not** touch `engine.rs`. The only verification
SW-2 owes here is that the const still reads `30` and the spec wording matches (`tasks.md` D1). If a
future task genuinely needs a configurable gap, that is a new BA decision (`arch-review.md`
feedback_to_ba F4), not this change.
