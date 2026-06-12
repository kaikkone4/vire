# QA Gate Report — TASK-020 Langfuse Importer Hardening

- **QA STATUS:** PASS
- **Branch:** `feat/task-020-langfuse-importer-hardening` (PR #12)
- **Tier:** L2
- **Test run:** `cargo test --locked` — 39/39 unit tests PASS; 1 adversarial pre-existing failure (out of scope, documented below)
- **Real Docker smoke:** skipped — no local Langfuse stack in session (acceptable per design.md §7)
- **TASK-006 reconciliation:** confirmed absent — no AI runtime observer code introduced

---

## Scenario Coverage Matrix

| Spec scenario | Test(s) | Result |
|---|---|---|
| **S-3a** A persistence failure mid-run leaves no partial state | `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` | PASS |
| **S-3b** A successful run commits as one unit | `successful_run_commits_as_one_consistent_unit` | PASS |
| **S-4a** A failed write is visible, not swallowed | `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` | PASS |
| **S-4b** Surfaced persistence errors carry no secrets | `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` | PASS |
| **S-5a** Run and import timestamps are UTC RFC3339 | `importer_emitted_timestamps_are_utc_rfc3339` | PASS |
| **S-5b** Legacy local-timestamp ordering invariant safe | `rfc3339_run_sorts_after_legacy_space_format_run_same_day` | PASS |
| **S-6a** A hung import does not block the UI forever | `run_bounded_times_out_promptly_with_a_secret_free_error` | PASS |
| **S-6b** A normal import returns its snapshot within the ceiling | `run_bounded_returns_the_works_result_within_the_ceiling` | PASS |

---

## Implementation Verification

### S-3 — Atomic persistence

`store::persist_import_run()` (`store.rs:102–118`) opens a transaction via `conn.unchecked_transaction()`, calls raw-trace upserts, evidence upserts, and the run-record insert on the `&tx`, then `tx.commit()`. On any error the `Transaction` is dropped without commit, rolling back all writes automatically. The atomicity unit is one `run_id` per environment, matching the per-env cursor model.

Pre-hardening: all three writes used `let _ = …` (independent statements, no transaction). Confirmed by inspecting the TASK-019 baseline commit `64d5f9f`.

### S-4 — Surface persistence failures, never swallow

`persist_run()` (`importer.rs:403–437`) now checks `store::persist_import_run(...).is_err()`. On failure: degrades `summary.health` to `HealthState::Unknown`, pushes the fixed `PERSIST_FAILURE_MSG` constant (no driver string interpolated, no config/credential material). A separate marker run record (`cursor_ts: None`, status `Unknown`) is inserted via `insert_import_run` so the failure is visible in the health snapshot.

The test uses a SQLite `BEFORE INSERT` trigger on `langfuse_ai_evidence` to force a mid-transaction abort, then verifies:
- 0 raw rows committed
- 0 evidence rows committed
- `health == Unknown` in the summary
- warning contains "persist" but not "sk-" or the raw driver string "forced test failure"
- snapshot health ≠ "healthy"

### S-5 — Uniform UTC RFC3339 timestamps

`now()` (`importer.rs:50–52`) changed from `Local::now().naive_local().format("%Y-%m-%d %H:%M:%S")` to `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)`. Feeds `started_at`, `finished_at` (run record), and `imported_at` (raw-trace row).

Ordering-key invariant: `'T'` (0x54) > `' '` (0x20), so every RFC3339 `…T…Z` row out-sorts any legacy space-format row under the `ORDER BY finished_at DESC` query used by `latest_run`/`latest_run_for_env`. Tested directly: a modern row stamped `2026-06-12T00:00:01Z` wins over a legacy row stamped `2026-06-12 23:59:59` in the same day.

### S-6 — Bounded `import_langfuse_now` IPC

`run_bounded()` (`lib.rs:192–204`) spawns a dedicated OS thread, sends its result over an `mpsc::channel`, and calls `rx.recv_timeout(Duration::from_secs(30))`. On `RecvTimeoutError` it returns `IMPORT_TIMEOUT_MSG` — a fixed, secret-free string that maps to a non-healthy IPC error. The orphaned worker is bounded by reqwest's own 15 s request / 5 s connect ceilings.

`IMPORT_TIMEOUT_SECS = 30` is comfortably above the reqwest ceilings; a normal slow import is not cut off. No new health-taxonomy state is introduced; the timeout surfaces as an IPC error, not a new `HealthState` variant.

Test with 50 ms ceiling and 1500 ms worker confirms return well under 1 s.

### A1 — Cargo.lock committed

Added by `f8fd591` (first-ever commit of this file; not previously in any `.gitignore`). No `cargo update` or version bumps — the lock pins exactly the TASK-019 dependency closure. File size: 5 159 lines, single-commit history.

---

## Regression Verification

| Area | Tests | Result |
|---|---|---|
| All 10 health states (unavailable, auth\_or\_network\_error, schema\_changed, wrong\_env, duplicate, delayed, stale, missing, unknown, healthy) | 12 existing tests | PASS |
| Absence-≠-zero invariant | `absence_is_never_zero_cost_when_stack_down`, `time_only_trace_has_null_cost_not_zero`, `schema_changed_when_generation_lacks_usage_and_cost` | PASS |
| Credential redaction | `secret_and_credentials_never_render_their_values`, `import_run_table_has_no_credential_columns` | PASS |
| Loopback default / Cloud explicit-override allowlist | `loopback_is_the_default_and_allowed_for_local`, `local_source_refuses_off_host_targets`, `cloud_is_an_explicit_override_for_off_host_only`, `non_http_scheme_is_refused` | PASS |
| Read-only / no raw-activity egress | `importer_only_issues_read_calls` | PASS |
| Pagination and cross-page dedup | `paginates_to_completion_and_dedups_cross_page` | PASS |
| Snapshot state (unknown before any import) | `snapshot_is_unknown_before_any_import` | PASS |
| Core time-tracker (project CRUD, entries, CSV) | 8 unit tests + 2 adversarial | PASS |

---

## Non-Blocking Finding

**Pre-existing adversarial failure: `csv_export_neutralizes_formula_like_project_names_and_notes`**

- **Scope:** `csv_escape`/`export_csv_repo` in `lib.rs` — time-tracker CSV path, entirely separate from the Langfuse importer.
- **Pre-existing:** The test exists unchanged on the TASK-019 baseline (`64d5f9f`); the same failure reproduces there. The design.md §8 explicitly flags it: *"The pre-existing `csv_export_neutralizes_formula_like_…` failure — time-tracker-core `csv_escape` defect, a separate follow-up, not importer scope."*
- **Impact on this PR:** None. The importer does not call `csv_escape` and this failure cannot be triggered by any importer path.
- **Action:** Defer to a dedicated follow-up task as noted in design.md. Do not block this gate.

---

## DEC-020 / TASK-019 Constraints Preserved

| Constraint | Evidence |
|---|---|
| Local Docker default | `DEFAULT_BASE_URL = "http://127.0.0.1:3000"`, `Source::Local`; `validate_target()` rejects off-host for `Local` |
| Cloud explicit override only | `Source::Cloud` required; `cloud_is_an_explicit_override_for_off_host_only` passes |
| Renderer off-network | No changes to Tauri capabilities, CSP, or webview config |
| No credential exposure | `PERSIST_FAILURE_MSG` is a fixed string; `import_run_table_has_no_credential_columns` asserts schema; `secret_and_credentials_never_render_their_values` passes |
| No raw-activity egress | `LangfuseApi` trait exposes only `probe`/`get_traces`/`get_observations`; `importer_only_issues_read_calls` passes |

---

## Gate Verdict

**QA STATUS: PASS**

All TASK-020 spec scenarios have observable test coverage and pass. The one failing test is a pre-existing, explicitly out-of-scope defect in an unrelated code path. Route to SW-4 (Code Reviewer) and SW-5 (Security Agent) in parallel.
