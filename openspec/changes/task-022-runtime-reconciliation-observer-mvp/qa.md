# QA Report — TASK-022 runtime-reconciliation-observer-mvp

**Gate tier:** L2  
**Branch:** `feat/task-022-runtime-reconciliation-observer-mvp`  
**Verdict:** PASS — route to SW-4 (Code Reviewer) and SW-5 (Security Agent) in parallel.

---

## Scenario coverage matrix

| Scenario | Spec section | Test | Result |
|---|---|---|---|
| Evidence from local log, not process scanning | Req 1 | no reqwest/sysinfo/process in runtime_observer/ | PASS |
| Runtime log absent → no runtime evidence, not zero | Req 1 | `absent_log_is_no_evidence_not_zero` | PASS |
| Empty log → no runtime evidence | Req 1 | `empty_log_is_no_evidence` | PASS |
| Injected prohibited fields dropped, never persisted | Req 2 | `allowlist_drops_injected_prohibited_fields_and_never_persists_them` | PASS |
| Malformed JSON lines skipped, not fatal | Req 2 | `malformed_lines_are_skipped_not_fatal` | PASS |
| Symlink path refused | Req 2 | `symlinked_log_is_refused` | PASS |
| Oversize log refused (byte cap) | Req 2 | `oversize_log_is_refused` | PASS |
| ai_runtime_sessions has no token/cost/duration column | Req 3 | `runtime_sessions_table_has_no_cost_token_or_duration_column` | PASS |
| Matched session carries no recomputed cost/time | Req 3 | schema structural check (PRAGMA) | PASS |
| session_id exact match (cross-environment) | Req 4 | `matches_by_session_id_exact_even_cross_environment` | PASS |
| env+time-window fallback when no session_id | Req 4 | `matches_by_env_and_time_overlap_when_no_session_id` | PASS |
| session_id precedence over time overlap | Req 4 | `session_id_takes_precedence_over_time_overlap` | PASS |
| Unmatched runtime when no env mapping | Req 4 | `unmappable_project_is_unmatched_runtime` | PASS |
| unmatched_trace for evidence with no session | Req 4 | `trace_with_no_runtime_session_is_unmatched_trace` | PASS |
| observed_no_trace only under healthy import | Req 5 | `observed_no_trace_only_under_a_healthy_import` | PASS |
| unavailable import → reconciliation_unknown, never gap | Req 5 | `down_or_uncertain_import_is_reconciliation_unknown_never_a_gap` | PASS |
| unknown import → reconciliation_unknown, never gap | Req 5 | `down_or_uncertain_import_is_reconciliation_unknown_never_a_gap` | PASS |
| auth_or_network_error → reconciliation_unknown, never gap | Req 5 | `down_or_uncertain_import_is_reconciliation_unknown_never_a_gap` | PASS |
| stale/missing/duplicate → reconciliation_unknown (not a gap) | Req 5 | `non_healthy_import_does_not_license_a_gap` | PASS |
| No covering import run → reconciliation_unknown | Req 5 | `no_import_run_for_env_is_unknown_not_a_gap` | PASS |
| Absent log + evidence → unknown message, never zero | Req 5 | `absent_log_with_evidence_yields_unknown_message_never_zero` | PASS |
| IPC surface: counts only, no secret/content/command | Req 6 | `snapshot_serializes_to_counts_only_with_no_secret_or_content` | PASS |
| Idempotent re-ingest (no duplicate rows) | tasks §2 | `idempotent_reingest_does_not_duplicate_rows` | PASS |
| Reconciliation persisted and re-readable | tasks §2 | `reconciliation_is_persisted_and_reread` | PASS |
| Environment resolution: explicit map > allowed-env > None | config | `environment_resolution_map_then_allowed_then_none` | PASS |
| paired start+finish into one session | ingest | `pairs_start_and_finish_into_one_session` | PASS |
| non-zero exit → failed; orphan → open | ingest | `nonzero_exit_pairs_to_failed_and_orphan_to_open` | PASS |

---

## Test run summary

```
cargo test --lib     →  64 passed / 0 failed / 0 ignored
runtime_observer::   →  23 passed / 0 failed / 0 ignored
```

All tests pass. No pre-existing failures detected.

---

## OpenSpec validation

```
openspec validate task-022-runtime-reconciliation-observer-mvp --strict
→ Change 'task-022-runtime-reconciliation-observer-mvp' is valid
```

---

## Detailed findings

### Focus 1 — OpenSpec strict: PASS

All spec requirements have observable test coverage. The reconciliation-state taxonomy, import-health gate, allowlist, and IPC surface match the spec exactly.

### Focus 2 — Rust lib + runtime_observer tests: PASS

64 tests total (23 runtime_observer, 27 langfuse, 14 lib). Zero failures. Tests are reproducible: all use `tempfile::tempdir()` and in-memory SQLite; no flakiness vectors.

### Focus 3 — Schema/migration: PASS

- **`ai_runtime_sessions`** (`runtime_observer/store.rs:14-32`): `session_key`, `project_key`, `tool`, `session_id`, `environment`, `started_at`, `ended_at`, `status`, `reconciliation`, `matched_trace_id`, `observed_at`. No token, cost, duration, prompt, command, secret column. Enforced structurally (PRAGMA test bans any column name containing `cost`, `token`, `duration`, `price`, `prompt`, `command`, `secret`).
- **`langfuse_ai_evidence.session_id`** (`langfuse/store.rs:58`): Additive nullable column added via `add_column_if_absent`. Idempotent: duplicate-column error on fresh installs treated as success. `upsert_ai_evidence` already populates and upserts the column. Observer reads only normalized evidence rows; never reads `langfuse_raw_traces.payload`.

### Focus 4 — Ingest allowlist: PASS

`RawEvent` struct (`runtime_observer/model.rs:57-92`) **is** the allowlist — serde silently drops all undeclared keys. Prohibited fields (`prompt`, `command_body`, `shell_history`, `env`, `metadata.*`, `api_key`, `response`, etc.) have no struct declaration and cannot be deserialized or reach the store, even in hostile input.

Path safety (`ingest.rs:42-56`):
- Symlink rejection: `symlink_metadata` + `is_symlink()` check before any read.
- Byte cap: `MAX_LOG_BYTES = 8 MiB`; file length checked before `fs::read`.
- Missing file: `symlink_metadata` returns `None` → `IngestResult::default()` (no evidence, no crash).
- Malformed lines: `serde_json::from_str` error branch → `continue` (skip, not panic).

Adversarial test confirms 10 prohibited needles (`SECRET`, `AKIA`, `sk-ant`, `rm -rf`, `export K`, `git_branch`, `summary`, `command_body`, `AWS_SECRET`, `cwd`) are absent from every column of every persisted row.

### Focus 5 — Matcher precedence: PASS

`reconcile.rs` resolution order:
1. **`session_id` exact** (line 42–49): checked across all evidence regardless of environment.
2. **env + time-window overlap** (line 52–58): only when session maps to an environment; bounded slop (default 300s).
3. **No env mapping** (line 61–68): → `unmatched_runtime`.
4. **Import-health gate** (line 71–87): `Unavailable | Unknown | AuthOrNetworkError` → `ReconciliationUnknown`; `Healthy` → `ObservedNoTrace`; all other states → `ReconciliationUnknown` via `_` catch-all.

All three match paths have dedicated tests. Precedence test (`session_id_takes_precedence_over_time_overlap`) seeds two evidence rows — one overlapping in time, one matching by session_id — and verifies session_id wins.

### Focus 6 — Health gating: PASS

The import-health gate is correctly placed after both match paths: a session that matches by session_id or time-overlap never reaches the gate. Only unmapped sessions that have an environment but no matching evidence consult import health.

Explicit health states tested:
- `Unavailable`, `Unknown`, `AuthOrNetworkError` → `reconciliation_unknown` ✓
- `Stale`, `Missing`, `Duplicate` → `reconciliation_unknown` ✓
- No covering import run → `reconciliation_unknown` (Unknown default) ✓
- `Healthy` + no match → `observed_no_trace` ✓

**Non-blocking observation NB-1:** `WrongEnv`, `SchemaChanged`, and `Delayed` are handled correctly by the `_` catch-all (`reconcile.rs:79`) but are not individually parameterized in the test loop at `non_healthy_import_does_not_license_a_gap`. The implementation is correct; coverage is complete but the three states share the tested `_` branch without an explicit assertion. No gate risk.

### Focus 7 — Read-only IPC / no content leak: PASS

`get_runtime_reconciliation` (`lib.rs:188-192`) returns only `RuntimeReconciliationSnapshot`: boolean `runtime_log_present`, string `source` (always `"local"`), and five `u32` counts plus `message`. No session key, no project content, no run log line, no command text. CSP is unchanged (`connect-src ipc: http://ipc.localhost`). The renderer calls `invoke('get_runtime_reconciliation')` through Tauri IPC; no outbound HTTP.

### Focus 8 — No out-of-scope scope: PASS

Checked `runtime_observer/` and `src/main.ts` for:
- Process scanner (`sysinfo`, `process::Command`): none.
- Network (`reqwest`, `TcpStream`, `std::net`, `tokio::net`): none in runtime_observer (reqwest patterns are all in the pre-existing langfuse module).
- Classifier (TASK-008): none.
- Review/approval UI (TASK-009): UI is a thin read-only status line only (`runtimeLine()` in `main.ts`).
- Export (TASK-010): none.
- New pi/Claude adapter (DEC-017): none.
- New Tauri capability or CSP expansion: none.

### Focus 9 — Pre-existing failures: PASS

Zero test failures. No known pre-existing failures in scope.

---

## Blockers

None.

## Non-blockers

| ID | Severity | Finding |
|---|---|---|
| NB-1 | Low | `wrong_env`, `schema_changed`, `delayed` health states not individually parametrized in `non_healthy_import_does_not_license_a_gap`; covered by `_` catch-all which is correct. Suggest adding them to the loop in a follow-up. |

---

## Gate verdict

**PASS** — route to SW-4 (Code Reviewer) and SW-5 (Security Agent) in parallel.
