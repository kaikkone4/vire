# QA Report — TASK-032 Workstream A (SW-3)

**Gate scope**: Workstream A only — suggestion engine, `time_entry_suggestions` table, unit tests.  
**Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (draft)  
**Date**: 2026-06-20  
**Verdict**: **PASS**

---

## Scenario coverage matrix (A1–A5 + design §7-A)

| ID | Scenario | Test | Result |
|---|---|---|---|
| A1 | Module files exist; `mod suggestions;` in lib.rs | Code review | PASS |
| A2 | `time_entry_suggestions` schema matches design §1.1; migrate called after `projects`/`time_entries`/`env_mapping` | Code review + `migrate_is_idempotent` | PASS |
| A3 | Engine buckets by `(project, date)`, clusters by 30-min GAP, aggregates tokens/cost/counts/health | `timed_rows_within_one_window…`, `clustering_respects_the_gap_boundary` | PASS |
| A4a | absence≠zero — untimed bucket → `duration NULL`, never 0 | `untimed_rows_yield_unknown_duration_never_zero` | PASS |
| A4b | absence≠zero — absent tokens/cost → `NULL`, never 0 | `absent_tokens_and_cost_stay_null_not_zero` | PASS |
| A4c | absence≠zero — unmapped env evidence excluded from blocks, returned in `unmapped` summary | `unmapped_environment_evidence_is_excluded_and_reported` | PASS |
| A5 | 12 unit tests; `cargo test` / `fmt --check` / `clippy` clean in module | Full suite run | PASS |
| DEC-006 | No auto-post: `generate` writes zero `time_entries` rows | `generation_never_writes_a_time_entry` | PASS |
| DEC-004 | Idempotency: accepted/dismissed rows survive regeneration; no pending duplicates | `regeneration_preserves_accepted_and_dismissed_and_does_not_duplicate` | PASS |
| DEC-004 | Migrate idempotent (safe to call twice) | `migrate_is_idempotent` | PASS |
| SEC-012 | Serialized output contains no session-id, trace-id, Bearer, or `sk-*` substrings | `surfaces_carry_no_secrets` | PASS |
| DEC-001/017 | No network egress in suggestions module | `grep` for reqwest/http/network — 0 hits | PASS |
| mixed-currency | Mixed USD+EUR in one block → `cost_total NULL`, reason notes "mixed currencies" | `single_currency_sums_but_mixed_currency_nulls_the_cost` | PASS |
| health-worst | Worst health wins; degraded → confidence ≤ medium | `degraded_health_lowers_confidence_and_worst_health_wins` | PASS |
| empty-in | Empty evidence → empty suggestions + empty unmapped | `empty_evidence_yields_no_suggestions_and_no_unmapped` | PASS |

**15/15 scenarios covered. 0 failures.**

---

## Test run summary

```
cargo test (full suite): 154 passed / 0 failed
  suggestions module: 12 / 12
  pre-existing tests: 142 / 142 (no regression)
cargo fmt --check: clean
cargo clippy (suggestions module): 0 findings
```

**Pre-existing clippy warnings** (3 errors in `langfuse/importer.rs:1170-1181` and `lib.rs:863`): confirmed pre-date TASK-032 by `git diff main` — zero changes to those lines from this branch. Out of scope for this gate.

---

## Guarantees verified (design §5)

| Guarantee | Mechanism | Verified |
|---|---|---|
| absence≠zero (DEC-004) | `duration_minutes`/tokens/cost `Option`, untimed→`None`, unmapped→separate list | ✓ |
| no auto-posting (DEC-006) | `generate` writes only `time_entry_suggestions`; `time_entries` count=0 after generate | ✓ |
| secret-free surface (SEC-012) | `Suggestion` struct exposes only aggregates/labels/local-times; session-id/trace-id excluded | ✓ |
| no egress / Vire-authoritative (DEC-001/017) | SQLite-only; no network deps in `suggestions/` | ✓ |
| AI≠human (DEC-003) | Deferred to Workstream B per scope — no `time_entries.origin` yet; correct | — |

---

## Scope boundary confirmed

B/C items absent as expected: no `accept`/`dismiss`/`list_time_entry_suggestion` IPC commands, no `time_entries.origin` column, no `get_summary`/`export_report_csv` DEC-003 separation, no frontend changes. `#[allow(dead_code, unused_imports)]` in `mod.rs` is intentional until B/C wire the public surface.

---

## Changed paths (Workstream A)

- `src-tauri/src/suggestions/mod.rs` — module root + DTOs (`Suggestion`, `UnmappedEnv`, `SuggestionList`)
- `src-tauri/src/suggestions/engine.rs` — `generate`, clustering, aggregation
- `src-tauri/src/suggestions/store.rs` — `migrate`, `load_evidence`, `delete_pending`, `insert_if_not_decided`, `list_pending`
- `src-tauri/src/suggestions/tests.rs` — 12 unit tests
- `src-tauri/src/lib.rs` — `mod suggestions;` (line 5) + `suggestions::store::migrate(conn)` in `init_db` (lines 113–115)

---

## Blockers

None.

## Routing

**A-checkpoint PASS → SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel.** (Routing deferred pending B-checkpoint below.)

---

---

# QA Report — TASK-032 Workstream B (SW-3)

**Gate scope**: Workstream B — accept/dismiss IPC, `time_entries.origin`, DEC-003 reporting separation.  
**Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27  
**Date**: 2026-06-20  
**Verdict**: **PASS**

---

## Scenario coverage matrix (B1–B5 + design §7-B)

| B-test | Design requirement | Result |
|---|---|---|
| `accept_creates_exactly_one_ai_entry_marks_accepted_and_is_decided_once` | Accept writes exactly one `origin='ai_suggested'` entry; suggestion → `accepted` + `accepted_entry_id`; re-accept and dismiss both rejected after deciding | PASS |
| `accept_of_untimed_block_requires_edits_and_never_invents_a_duration` | Untimed block, no edits → error "start and end time", 0 entries, status stays `pending`; with edits → entry created, origin=`ai_suggested` | PASS |
| `dismiss_writes_no_entry_is_idempotent_and_cannot_undo_an_accept` | Dismiss writes 0 entries; re-dismiss = Ok (idempotent); accept-after-dismiss rejected; dismiss-of-accepted rejected with "accepted" | PASS |
| `summary_and_csv_report_human_and_ai_time_separately` | `summary_repo`: `duration_minutes=60` (human), `ai_minutes=30` (AI); CSV header has `origin`; rows carry `,manual,` and `,ai_suggested,` | PASS |

**`cargo test`: 158 passed / 0 failed** (+4 B tests vs A-checkpoint; 0 regressions).  
`cargo fmt --check`: clean. `cargo clippy --all-targets`: no new findings.

---

## B1 — `time_entries.origin` column

- `add_column_if_absent(conn, "time_entries", "origin", "TEXT NOT NULL DEFAULT 'manual'")` in `init_db` (lib.rs:120–125). Idempotent; existing rows backfill to `'manual'`.
- `add_column_if_absent` is `pub(crate)` (langfuse/store.rs:75) — not widened.
- `TimeEntry` struct gains `origin: String` (lib.rs:58). `TimeEntryInput` does NOT include `origin`.
- `SummaryRow` gains `ai_minutes: i64` as a distinct field (lib.rs:84); `duration_minutes` meaning is unchanged.

## B2 — Three IPC commands

Registered in `generate_handler!` (lib.rs:1123–1125):
- `list_time_entry_suggestions(state, regenerate: bool)` — `false` → `suggestions::current`, `true` → `suggestions::generate`.
- `accept_time_entry_suggestion(state, id: String, edits: Option<SuggestionEdit>)`.
- `dismiss_time_entry_suggestion(state, id: String)`.
- `SuggestionEdit { project_id?, date?, start_time?, end_time?, note? }` (suggestions/mod.rs:81–88).

## B3 — Accept is the sole writer of `origin='ai_suggested'`

Exactly two `INSERT INTO time_entries` paths confirmed in lib.rs:
- **lib.rs:291** (`create_entry_repo`) — no `origin` column listed → DB default `'manual'`.
- **lib.rs:541** (`accept_suggestion_repo`) — hardcodes `'ai_suggested'`.

Single transaction: INSERT entry + guarded `UPDATE … WHERE id=? AND status='pending'`; 0 rows matched → rollback (zero writes). Status-gate before any write: `accepted` or `dismissed` → early error. Failed accept leaves suggestion `pending`, 0 entries (verified by test).

`dismiss_suggestion_repo`: no INSERT. Already-dismissed → Ok. Already-accepted → Err("…accepted…").

## B4 — DEC-003 reporting separation

`summary_repo` SQL (lib.rs:373–375) — CASE WHEN split:
- Human = `COALESCE(SUM(CASE WHEN origin='ai_suggested' THEN 0 ELSE duration_minutes END),0)`
- AI = `COALESCE(SUM(CASE WHEN origin='ai_suggested' THEN duration_minutes ELSE 0 END),0)`

Conservative: any non-`'ai_suggested'` origin counts as human. AI can never silently land in the billable column.

CSV: header `"…,origin,…"` (lib.rs:432); each row writes `csv_escape(&e.origin)` (lib.rs:443).

## B5 — Tests

4 unit tests at lib.rs:1664, 1716, 1750, 1798. All seed fixtures directly into `time_entry_suggestions` (deterministic, importer-independent). No flaky behaviour observed.

---

## §5 Guarantees check (B-scope)

| Guarantee | Status |
|---|---|
| **absence ≠ zero** (DEC-004) — untimed accept without edits → error, 0 entries, status stays `pending`; no fabricated duration | PASS |
| **no auto-posting** (DEC-006) — generate/current write only `time_entry_suggestions`; accept is sole writer of `time_entries` | PASS |
| **AI ≠ human time** (DEC-003) — `origin` tag; `summary_repo` + CSV separate the two; `SummaryRow.ai_minutes` is a distinct field | PASS |
| **no secrets** (SEC-012) — `session_id` used only to compute `session_count`, never in `Suggestion` struct or IPC output; provenance note uses only `source`+`reason`; `SuggestionEdit` carries only times/refs/notes | PASS |
| **no egress** (DEC-001/017) — no network imports in suggestions module; `git diff main` shows no reqwest/ureq/hyper | PASS |

---

## Interpretation decisions (non-blocking)

1. **Dismiss idempotency** — re-dismiss = Ok; dismiss-of-accepted = Err. Correct per design §3. ✓
2. **Provenance sticky on edit** — `update_entry_repo` does not touch `origin`; a later manual edit of an accepted AI entry keeps `origin='ai_suggested'`. Acceptable; flag for C UX if warranted.
3. **Human-minutes conservative** — `duration_minutes` = anything not `'ai_suggested'`. Correct per DEC-003 intent.

---

## Changed paths (Workstream B, additions to A)

- `src-tauri/src/lib.rs` — `TimeEntry.origin`, `SummaryRow.ai_minutes`, `add_column_if_absent` call in `init_db`, `accept_suggestion_repo`, `dismiss_suggestion_repo`, 3 IPC commands, 4 B unit tests
- `src-tauri/src/suggestions/mod.rs` — `SuggestionEdit`, `#[allow]` removed, `pub use engine::current`
- `src-tauri/src/langfuse/store.rs` — `add_column_if_absent` visibility `pub` → `pub(crate)`

No frontend files changed in B.

---

## Blockers

None.

## Routing

**B-checkpoint PASS → SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel.**
