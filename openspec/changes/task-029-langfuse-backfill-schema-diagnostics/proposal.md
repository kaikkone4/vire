# TASK-029 — Langfuse backfill + forensic schema diagnostics + import correctness

## Why

Janne merged TASK-027, installed the build, and ran a manual import. The TASK-027 wins held —
**9 environments discovered, 640 traces seen** — but the import produced **no useful new data**:

> *No new traces imported across 9 environments (640 seen, 29 duplicates, 611 skipped)* — per-env state
> `schema_changed`, with the warning *"a trace did not match the expected shape"* repeated **hundreds of
> times**.

Three distinct gaps turn a working discovery pipeline into a useless import:

### A — The importer silently drops every trace it cannot fully deserialize

The import loop deserializes each list-payload trace with an **all-or-nothing**
`serde_json::from_value::<Trace>` (`src-tauri/src/langfuse/importer.rs:222`). On any failure it
increments `skipped_schema`, pushes the fixed string `"a trace did not match the expected shape"`
(`importer.rs:226`), and `continue`s — the trace is **never imported**. The strong leading hypothesis
(to be **confirmed by Workstream A diagnostics before any code is widened**) is the typed
`Trace.observations: Vec<Observation>` field (`model.rs:146`): Langfuse v3's
`GET /api/public/traces` **list** endpoint returns `observations` as an array of observation **ID
strings**, not embedded objects, so every trace that has observations fails to deserialize into
`Vec<Observation>`. That matches the numbers exactly — the 611 skipped are the traces *with*
observations; the 29 duplicates are observation-less traces that parse and were already seen.

Two design faults compound here:

1. **Trace identification is coupled to usage-shape parsing.** A trace we can clearly identify (it has
   an `id` and a `timestamp`) is dropped wholesale because a *peripheral* field (`observations`) has an
   unexpected shape — even though usage/cost are read from a **separate** `get_observations(trace_id)`
   fetch (`importer.rs:320`), not from the embedded array. Identification and usage-extraction must be
   decoupled.
2. **The diagnostic is a repeated opaque string, not a classification.** The same warning is pushed
   per-trace, so the report carries hundreds of identical lines and the user learns *nothing* about
   *why* traces are skipped.

### B — Import covers only a narrow "now − 7 days" window; there is no backfill or incremental cursor

`run_blocking_import` hard-codes `recent_window(7)` (`src-tauri/src/langfuse/mod.rs:50`): every import —
manual or auto — scans a fixed last-7-days window from `now`. There is **no historical backfill** and
**no cursor-driven incremental** import, even though the per-environment cursor (`cursor_ts`) is already
persisted (`store.rs:161`) and the architecture plan explicitly calls for currency tracking against
"last successful import, latest trace timestamp" (`03_architecture_plan.md:144`). Janne wants AI history
imported — *last month, or since the last synced timestamp* — not just a rolling 7-day slice. Until the
AI history is imported correctly and completely, the app stays in **Manual Mode**; this task is the
prerequisite for moving past manual-only.

### C — The import summary is noise, not signal

The summary repeats one opaque warning hundreds of times and gives no grouped, actionable breakdown of
*seen / new / duplicate / skipped* per environment, nor *why* the skips happened.

These are correctness and usability gaps in the **existing importer component**. They operationalize
behaviours the architecture already specified (currency cursor, environment health, "absence is a
warning not zero") but never realized. No new component, no change to the AI-evidence taxonomy, the
data boundary, or the local-only posture.

## What Changes

One OpenSpec change, **four sequenced workstreams (A → B → C → D)**. **A is diagnose-first** and gates
B: the parser is widened **only** for shapes the diagnostics prove are real. **Time-entry suggestion
(the future TASK-030) is explicitly NOT built here** — see *Out of scope*.

### Workstream A — Forensic, secret-free schema diagnostics  *(backend developer)*  *(gates B)*

- Replace the per-trace repeated warning (`importer.rs:226`) with a bounded **skip-reason classifier**.
  Each skipped trace/observation is classified into a fixed, secret-free `SkipReason` enum (e.g.
  *missing trace id*, *observations not embedded (ID-only list shape)*, *field type mismatch*,
  *generation lacks usage and cost in every supported location*, *observations fetch failed*). The
  report carries **aggregated counts per reason** per environment + total — never the same string N
  times.
- Capture a **bounded structural sample** (≤ a small fixed N per reason) that names only the **JSON key
  names present** and the **JSON type** of the offending field (e.g. `observations` element type =
  `string`). It MUST NOT include any field **value**, raw payload bytes, prompt/session/metadata
  content, or a passed-through `serde_json` error string (those can embed data values) (**SEC-011**).

### Workstream B — Widen the v3 parser + decouple identification from usage  *(backend developer)*

- Make **trace identification total**: a list-payload entry with a usable `id` (and `timestamp`) SHALL
  be imported even when a peripheral field shape is unexpected. Identification (id/ts/environment/
  session) is separated from usage/cost extraction.
- Widen `Trace.observations` parsing (`model.rs:146`) to tolerate the v3 **list** shape (array of
  observation **ID strings**) as well as embedded observation **objects** — keeping object elements as
  the fast path and falling through to the existing `get_observations(trace_id)` fetch
  (`importer.rs:320`) when the list carried only IDs. Driven by, and only for, the shapes Workstream A
  proves.
- `schema_changed` is **reserved for genuinely unsupported shapes after widening** (a generation with
  usage/cost in *no* supported location, or a trace with no usable id) — never for a shape the
  diagnostics showed is the normal v3 list payload. Absence stays absence (`None`, never `0`); an
  identifiable-but-usage-unreadable trace is **imported as `schema_changed`** (counted, surfaced for
  review), not silently dropped.

### Workstream C — Configurable import range + incremental cursor + resumable backfill  *(backend developer)*

- Add an `app-configuration` setting **import range**: `last_7d | last_30d | last_90d | all |
  since:<ISO-8601>`, plus an explicit **"Backfill now"** action. Default range **last 30 days**
  (was a fixed 7-day window).
- **Normal (incremental) import** resolves a **per-environment** window from the persisted cursor:
  `from = cursor_ts(env) − overlap` (a small reconciliation lookback so delayed traces are re-seen and
  deduped; the cursor never regresses — `importer.rs:277`), `to = now`. The first import of an
  environment with no cursor uses the configured range floor.
- **Backfill** re-scans from the configured range floor to `now`, **chunked into bounded sub-windows
  each persisted atomically** so progress is **durable and resumable** (preserving the S-3 atomic-per-run
  invariant — `store.rs:135`) rather than one giant transaction that loses all progress on
  interruption/timeout. Dedupe by `(environment, trace_id)` is already durable (`store.rs:213`), so a
  re-run continues where it stopped and re-seen traces are cheap no-ops.
- Backfill honours the **same** loopback/cloud boundary (SEC-002), `langfuse_enabled` short-circuit,
  import serialization (`import_lock`), and bounded off-UI-thread execution as normal import. A backfill
  that reaches its page/window backstop **says so** (never silent truncation) and is continued by
  re-running.

### Workstream D — Grouped, actionable import summary  *(frontend developer)*

- Extend `ImportReport`/`EnvImportLine` (`importer.rs:48,59`) with the **grouped skip-reason breakdown**
  and bounded structural samples from Workstream A, replacing the repeated-string `warnings`.
- `importReportLine()` (`src/main.ts:55`) renders **seen / new / duplicate / skipped per environment**
  plus a grouped *"611 skipped: 611 observations-not-embedded"* breakdown and an incremental-vs-backfill
  headline — never the same line hundreds of times. Secret-free (SEC-011).

## Impact

- **Affected code (A, B):** `src-tauri/src/langfuse/{model.rs,importer.rs}` (tolerant trace
  identification, widened observation parsing, skip-reason classifier + bounded structural samples).
- **Affected code (C):** `src-tauri/src/langfuse/{mod.rs,importer.rs,config.rs,store.rs}` (per-env
  cursor window, range floor, chunked resumable backfill window builder), `src-tauri/src/settings/`
  (new `langfuse_import_range` setting), `src-tauri/src/lib.rs` (new IPC: get/set import range,
  `backfill_langfuse_now`; longer bound for backfill).
- **Affected code (D):** `src-tauri/src/langfuse/importer.rs` (report struct extension),
  `src/main.ts` (grouped report rendering), `src/style.css` (report list styling), `src/langfuse-settings.ts`
  (range-control helpers).
- **Data model:** additive only — one new key-value `settings` row (`langfuse_import_range`); existing
  `langfuse_*` tables unchanged (the durable cursor + dedupe already exist). Idempotent `init_db` /
  `CREATE TABLE IF NOT EXISTS` posture preserved; forward/backward compatible.
- **Affected specs:** **MODIFY `langfuse-importer`** (tolerant identification + reserved-`schema_changed`,
  forensic diagnostics, configurable range + incremental cursor + resumable backfill, grouped report);
  **ADD `app-configuration`** (import-range setting). `desktop-ui`, `project-env-mapping`, `csv-export`,
  `runtime-reconciliation` are **not** modified.
- **Security (SEC-011):** the diagnostics, structural samples, grouped report, and range/backfill
  surfaces are secret-free — no credentials, raw bodies, `serde` error strings, or prompt/session/
  metadata content; extends SEC-003 / SEC-010. Backfill preserves the SEC-002 loopback boundary, the
  disabled short-circuit, and the read-only (GET-only) contract. **No new network egress host.**
- **Out of scope (clean boundaries):**
  - **Time-entry suggestion from imported AI evidence → TASK-030.** TASK-029 makes the AI evidence
    (`langfuse_ai_evidence`: session id, AI start/end, tokens, cost, health) **correct and complete**;
    turning it into suggested time entries is a separate task and is **not** implemented here unless the
    architect rules it necessary (it is **not** — see `arch-review.md` §Split). Respects DEC-006
    (suggestion-first, human-approved) and DEC-003 (Vire is the cost/time authority).
  - Any change to the **ten-state health taxonomy** (`model.rs:11`), capture, the runtime observer, the
    classifier/suggestion engine, the review UI, env→project mapping, or CSV export.
  - **OTEL trace emission** — Vire stays a read-only trace consumer (SEC-002).
  - **Raw-trace retention enforcement** — backfill amplifies an existing unrealized DEC-007 gap; flagged
    to BA (`arch-review.md` `feedback_to_ba[]` F2), not built here.

## ADR — DEC-030 (proposed): configurable import range + incremental cursor + resumable chunked backfill

**Decision.** Replace the fixed `now − 7 days` window with: (1) a configurable **import range**
(`last_7d/30d/90d/all/since:<ISO>`, default `last_30d`); (2) **per-environment incremental** imports
that resume from the persisted `cursor_ts` minus a small overlap; and (3) an explicit **"Backfill now"**
that re-scans the range floor → now **in bounded, atomically-persisted, resumable chunks**. Dedupe by
`(environment, trace_id)` stays the idempotency key. Backfill reuses the single `run_blocking_import`
engine and obeys the SEC-002 boundary, the disabled switch, import serialization, and bounded off-UI
execution identically to incremental import. The importer remains the sole AI cost/time authority
(DEC-003 / DEC-017 unchanged).

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log via
`feedback_to_ba[]`.

**Alternatives considered.** (1) *Keep the fixed 7-day window* — rejected: cannot import history Janne
needs; the panel is permanently shallow. (2) *One giant backfill transaction* — rejected: loses all
progress on interruption/timeout, holds the whole result in memory, and a single huge atomic commit is
fragile; chunked-resumable keeps the S-3 atomic-per-run guarantee while making progress durable. (3)
*Walk oldest→newest with a stored "backfilled-through" floor* — folded into the cursor model: durable
dedupe + a non-regressing cursor already make re-runs idempotent and resumable.

## ADR — DEC-031 (proposed): forensic secret-free schema diagnostics + decoupled trace identification

**Decision.** Trace **identification** (id + timestamp) is decoupled from **usage/cost extraction**: an
identifiable trace is always imported (as `schema_changed` when usage is unreadable), never silently
dropped. Skip/degrade reasons are classified into a **fixed, secret-free** `SkipReason` taxonomy with
**aggregated counts** and a **bounded structural sample** (JSON key names + JSON type names only). The
parser is widened **only** for shapes the diagnostics prove are real, in that order. `schema_changed`
is reserved for genuinely unsupported shapes after widening.

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log.

**Alternatives considered.** (1) *Pass through the raw `serde` error* — rejected: `serde_json` error
strings can embed field values (prompt/session content) → violates SEC-003/010. (2) *Keep all-or-nothing
trace deserialization and just widen `observations`* — rejected: still brittle; any future peripheral
field drift would again drop whole traces. Decoupling identification is the durable fix.
