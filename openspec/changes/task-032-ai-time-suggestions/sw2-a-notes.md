# SW-2 Workstream A notes — TASK-032 AI time-entry suggestions

Engine + persistence + unit tests. Backend only. Stops at the A-checkpoint; B (accept/dismiss IPC,
`origin`, reporting separation) and C (UI) are **not** implemented.

## What shipped (tasks.md A1–A5)

- **A1** New module `src-tauri/src/suggestions/` (`mod.rs`, `engine.rs`, `store.rs`, `tests.rs`);
  `mod suggestions;` declared in `lib.rs`.
- **A2** `suggestions::store::migrate` creates `time_entry_suggestions` (design §1.1, verbatim schema
  + `idx_suggestions_status_date`). Called from `init_db` **after**
  `projects`/`time_entries`/`env_mapping`/`runtime_observer` so the `project_id` FK resolves.
- **A3** `suggestions::generate(conn)` — read-time LEFT JOIN of `langfuse_ai_evidence` →
  `langfuse_env_project_map` → `projects` (mirrors `env_mapping::list_evidence_projects_repo`),
  bucket by `(project_id, local date)`, cluster by the 30-min `GAP_MINUTES` constant, aggregate
  tokens/cost/counts/health, compute confidence/source/reason. Idempotent: `DELETE … WHERE
  status='pending'` then guarded insert that skips any natural key already `accepted`/`dismissed`.
- **A4** absence ≠ zero: untimed bucket → one suggestion with `duration_minutes NULL` /
  `block_start_ts NULL` and reason "needs manual time" (never 0); unmapped-env evidence returned in a
  separate `unmapped: [{environment, trace_count}]` summary (never dropped, never zeroed);
  absent tokens/cost stay `NULL` (never 0).
- **A5** 12 unit tests (`suggestions/tests.rs`). `cargo test` green, `cargo fmt --check` clean,
  `cargo clippy` clean (0 findings in the module).

## A-checkpoint evidence — engine output vs seeded fixtures

Each test seeds `langfuse_ai_evidence` rows + an env→project mapping in an in-memory DB and asserts the
`generate()` output. Timestamps use offset-less local wall-clock so dates/durations are TZ-independent;
the RFC-3339 (`Z`) import path is covered by `parse_local`.

| Test | Seeded fixture | Verified output |
|---|---|---|
| `timed_rows_within_one_window…` | 2 timed healthy rows, 2 sessions, 09:00–10:00 | 1 block, duration `Some(60)`, trace_count 2, session_count 2, tokens `Some(30)`, cost `Some(4.0)`, health healthy, confidence high, source `langfuse:veronavi` |
| `clustering_respects_the_gap_boundary` | second row at gap 30:00 / 30:01 / 29:00 | 1 / 2 / 1 block(s) — boundary compared in seconds |
| `untimed_rows_yield_unknown_duration_never_zero` | 2 rows, start present, end NULL | 1 block, duration `None` (not 0), start/end NULL, confidence low, reason "needs manual time", present token survives `Some(5)` |
| `absent_tokens_and_cost_stay_null_not_zero` | timed row, tokens/cost NULL | total_tokens `None`, cost_total `None`, cost_currency `None` |
| `unmapped_environment_evidence_is_excluded_and_reported` | mapped env + 2 traces in unmapped `ghost` | 1 block (mapped only); `unmapped=[{ghost,2}]` |
| `single_currency_sums_but_mixed_currency_nulls_the_cost` | USD + EUR rows in one block | cost_total `None`, cost_currency `None`, reason contains "mixed currencies" |
| `degraded_health_lowers_confidence_and_worst_health_wins` | healthy + schema_changed | health `schema_changed`, confidence medium |
| `regeneration_preserves_accepted_and_dismissed…` | accept 1 + dismiss 1, regenerate | decided rows survive, no pending duplicates, total rows 2, `accepted_entry_id` intact |
| `generation_never_writes_a_time_entry` | 1 timed row, generate | `time_entries` count 0 (DEC-006) |
| `surfaces_carry_no_secrets` | session_id `session-Bearer-sk-leak`, tokens 123, cost 9.99 | serialized output excludes session-id/secret/trace-id; aggregate numbers present (SEC-012) |
| `migrate_is_idempotent` / `empty_evidence…` | — | migrate twice no-op; empty in → empty out |

## Guarantees (design §5) held by A-scope

- **absence ≠ zero** (DEC-004) — verified (untimed/absent-tokens/absent-cost/unmapped tests).
- **no auto-posting** (DEC-006) — verified (`generation_never_writes_a_time_entry`); only the
  Workstream-B accept path will write `time_entries`.
- **no secrets** (SEC-012) — verified (`surfaces_carry_no_secrets`).
- **Vire-authoritative, read-only of Langfuse / no egress** (DEC-001/017) — engine is SQLite-only;
  no `reqwest`/network in the module; evidence joined at read time, never rewritten.
- **AI ≠ human time** (DEC-003) — deferred to B (`origin` column + reporting separation); A persists
  suggestions only, posts no entries.

## Interpretation decisions (non-blocking; flag to QA/BA)

1. **Mixed timed+untimed bucket.** design §2.4 specifies only the *all-untimed* bucket. Generalised
   safely: a `(project, date)` bucket that mixes usable and unusable rows yields its timed block(s)
   **plus** one untimed "needs manual time" block for the leftovers — so no evidence is dropped or
   zeroed. A "usable/timed" row requires **both** a parseable `ai_start_ts` and `ai_end_ts`.
2. **Fully date-less mapped rows** (no start *and* no end) carry no calendar day, so they are bucketed
   under the **generation day** and surfaced as "needs manual time" (duration stays NULL — never
   invented). Minor: a dismiss of such a block on day D would not suppress a regeneration on day D+1
   (different date ⇒ different natural key). Rare in practice (importer derives ts from generation
   observations). Acceptable for A; note for B/QA.
3. **`cost_currency`** is read from evidence but the importer currently always writes `NULL`
   (reserved). Mixed-currency logic is implemented + tested via direct fixture inserts so it is correct
   the moment the importer starts populating currency.
4. **Worst-health ordering** is a local severity rank in `engine.rs` (Healthy=best … Unknown=worst);
   the load-bearing contract is "all healthy ⇒ healthy, any degraded ⇒ confidence ≤ medium". The exact
   ranking among degraded states is display-only.

## Not in A-scope (next: Workstream B)

`time_entries.origin` column, `TimeEntry.origin`, the three IPC commands
(`list/accept/dismiss_time_entry_suggestion`), accept-writes-the-only-`ai_suggested`-entry, and the
`get_summary`/`export_report_csv` DEC-003 separation. The module's public `generate`/DTOs are marked
`#[allow(dead_code, unused_imports)]` until B/C consume them (documented in `mod.rs`).
