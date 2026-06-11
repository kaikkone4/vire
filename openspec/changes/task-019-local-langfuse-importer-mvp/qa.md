# QA Gate Report — TASK-019 Local Docker Langfuse Importer MVP

**QA STATUS: pass**

- **Gate:** SW-3 QA
- **Branch:** `feat/task-019-local-langfuse-importer-mvp`
- **Commit under review:** `b30025e`
- **PR:** #11
- **Tier:** L2
- **QA run date:** 2026-06-11

---

## 1. OpenSpec Validation

```
openspec validate task-019-local-langfuse-importer-mvp --strict
→ Change 'task-019-local-langfuse-importer-mvp' is valid
```

**PASS.** All spec artifacts present and internally consistent.

---

## 2. Test Execution Summary

### 2a. Unit tests (`cargo test` — lib only, in-process)

```
test result: ok. 33 passed; 0 failed; 0 ignored  (finished in 0.02s)
```

- 25 `langfuse::tests::*` — new TASK-019 tests covering all 10 health states, pagination/dedup,
  absence-≠-zero, credential redaction, URL allowlist, observation-sourced cost.
- 8 pre-existing `tests::*` (lib.rs) — unchanged, all still pass.

### 2b. Adversarial integration tests (`cargo test --test adversarial`)

```
test archived_project_entries_remain_editable_for_historical_corrections ... ok
test report_operations_reject_inverted_date_ranges_instead_of_silently_returning_empty_data ... ok
test csv_export_neutralizes_formula_like_project_names_and_notes ... FAILED  ← pre-existing
```

**Classification: PRE-EXISTING FAILURE — not a TASK-019 regression.**

Evidence:
- `git diff b30025e^..b30025e -- src-tauri/tests/adversarial.rs` → **0 lines changed**
  (TASK-019 commit did not touch `adversarial.rs`).
- `git log --follow -- src-tauri/tests/adversarial.rs` → last modified at `b1a9c6f`
  ("test(export): fix CSV formula expectation", merged into main before TASK-018/TASK-019).
- The pre-TASK-019 branch head (`fb3e6ff`) also fails to compile the lib due to 5 pre-existing
  compile errors in `lib.rs` (borrow lifetime issues); TASK-019 fixed those compile errors while
  the CSV formula test remained unresolved from the earlier fix attempt.
- Test failure is in `csv_export_neutralizes_formula_like_project_names_and_notes` (CSV formula
  sanitization in the time-tracker core), which is outside TASK-019 scope. The actual failure:
  the note `" +SUM(1,2) with bare\rcarriage return"` is neutralized but the leading space and
  bare `\r` are not preserved through the `csv_escape` path.

**Non-blocker for TASK-019 gate.** Tracked as a pre-existing defect in the CSV export path
(outside `src-tauri/src/langfuse/`).

### 2c. Live Docker/Langfuse run

Not executed (Docker/Langfuse stack not confirmed up in CI context). All scenarios tested via the
in-memory `MockApi` (the `--mock` fixture style per spec §9 / feeder probe precedent). Live run
deferred; acceptable per spec — the mock strategy is explicitly specified in `design.md §9` and
`tasks.md §7`.

---

## 3. Scenario Coverage Matrix

All scenarios from `specs/langfuse-importer/spec.md` and `design.md`.

| # | Scenario | Test(s) | Result |
|---|---|---|---|
| S-1 | Local Docker Langfuse on loopback is the default source | `loopback_is_the_default_and_allowed_for_local` | PASS |
| S-2 | Cloud is an explicit override and the only off-host egress | `cloud_is_an_explicit_override_for_off_host_only`, `local_source_refuses_off_host_targets` | PASS |
| S-3 | Request URLs restricted to configured base + `/api/public/*` | `loopback_is_the_default_and_allowed_for_local` (path assertion), `non_http_scheme_is_refused`, `local_source_refuses_off_host_targets`, defense-in-depth check in `config.rs:255–264` | PASS |
| S-4 | All 10 health states represented and reachable | (see §4 below) | PASS |
| S-5 | Docker/stack down → `unavailable`, never zero cost | `unavailable_when_stack_is_down`, `absence_is_never_zero_cost_when_stack_down` | PASS |
| S-6 | Missing/null usage-cost → health flag, not zero | `time_only_trace_has_null_cost_not_zero`, `schema_changed_when_generation_lacks_usage_and_cost` | PASS |
| S-7 | pi/Claude traffic in `default` env → `wrong_env` (surfaced, not folded) | `wrong_env_when_traffic_lands_in_default` | PASS |
| S-8 | Multi-page window: paginate to completion + dedup by `(env, trace_id)` | `paginates_to_completion_and_dedups_cross_page` | PASS |
| S-9 | Delayed trace (pre-checkpoint, late-arriving) reconciled, cursor not regressed | `delayed_when_a_trace_predates_the_checkpoint` | PASS |
| S-10 | Usage/cost read from generation observations, not trace body | `cost_is_read_from_observations_not_trace_body` (trace body says 999.0; observation 1.5 wins), `observations_are_fetched_when_not_embedded` | PASS |
| S-11 | Credentials stay out of logs and evidence rows | `secret_and_credentials_never_render_their_values` (Debug impls redacted), `import_run_table_has_no_credential_columns` (PRAGMA check), `SourceHealthSnapshot` struct has no credential fields | PASS |
| S-12 | Raw local activity never reaches Langfuse | `importer_only_issues_read_calls` (only probe/get_traces/get_observations calls observed, no write path exists in `LangfuseApi` trait) | PASS |
| S-13 | MinIO/S3 docs remain visible and not contradicted | `docs/langfuse-local-setup.md` §MinIO: bucket `langfuse`, private access, volume `langfuse_minio_data`, three-store consistency. `docs/backup-restore.md`: MinIO in backup scope, divergence failure modes documented. Both docs updated as part of this PR and accurate. | PASS |
| S-14 | `unknown` before any import — absence ≠ zero | `snapshot_is_unknown_before_any_import` (health="unknown", last_import_at=None) | PASS |

---

## 4. Ten-State Health Coverage

| State | Trigger in tests | Test | Result |
|---|---|---|---|
| `healthy` | generations with usage+cost present | `healthy_when_generations_have_usage_and_cost` | PASS |
| `missing` | environment returns no traces, no prior cursor | `missing_when_environment_is_empty_with_no_prior_cursor` | PASS |
| `stale` | prior cursor exists but older than window.to − 24h; nothing new | `stale_when_only_an_old_cursor_remains` | PASS |
| `wrong_env` | traces in `default` (not in `allowed_environments`) | `wrong_env_when_traffic_lands_in_default` | PASS |
| `delayed` | trace timestamp precedes existing checkpoint | `delayed_when_a_trace_predates_the_checkpoint` | PASS |
| `duplicate` | re-import suppresses every trace seen (all already in `seen`) | `duplicate_when_a_reimport_suppresses_everything` | PASS |
| `schema_changed` | generation observation lacks usage/cost; trace not parseable | `schema_changed_when_generation_lacks_usage_and_cost`, `schema_changed_when_a_trace_is_unparseable` | PASS |
| `auth_or_network_error` | probe returns `Auth` error; listing returns `RateLimited` error | `auth_or_network_error_on_probe_auth_failure`, `auth_or_network_error_on_rate_limit_during_listing` | PASS |
| `unavailable` | probe returns `Unavailable` (connection refused) | `unavailable_when_stack_is_down` | PASS |
| `unknown` | listing returns `Indeterminate` error; snapshot before first import | `unknown_when_response_is_indeterminate`, `snapshot_is_unknown_before_any_import` | PASS |

---

## 5. Absence-≠-Zero Invariant Verification

| Scenario | Assertion | Result |
|---|---|---|
| Stack unavailable → 0 rows in `langfuse_ai_evidence` | `SELECT COUNT(*) = 0` after `unavailable` probe | PASS |
| Time-only trace (no generation) → cost is `NULL` not `0` | `evidence_cost(conn, "vire", "A") == None` | PASS |
| Generation lacks usage/cost → evidence cost `NULL` | `evidence_cost(conn, "vire", "A") == None` | PASS |
| Snapshot before first import → health `unknown` | `snap.health == "unknown"`, no numeric total | PASS |
| Snapshot message for `unavailable` | Contains "unknown, not zero" | PASS |

SQL schema: `langfuse_ai_evidence` columns `prompt_tokens`, `completion_tokens`, `total_tokens`,
`cost_total` are all nullable with no DEFAULT — absence cannot be coerced to zero at the DB level.

---

## 6. Credential & Security Checks

| Check | Evidence | Result |
|---|---|---|
| `Secret::fmt` returns `***redacted***` | `secret_and_credentials_never_render_their_values` | PASS |
| `Credentials::fmt` redacts both public and secret key | same test | PASS |
| `ImporterConfig::fmt` redacts credentials | same test | PASS |
| `langfuse_import_runs` schema has no credential columns | `import_run_table_has_no_credential_columns` (PRAGMA check for secret/token/password/key) | PASS |
| `SourceHealthSnapshot` struct (IPC payload) has no credential fields | Code inspection: `base_url`, `source`, `environments`, `last_import_at`, `latest_trace_ts`, `health`, `message` — no key/secret | PASS |
| Transport errors contain no URL/header material | `map_transport_error` in `api.rs:125–136` — messages are generic strings only | PASS |
| `public_from_env()` never reads credential env vars | Code inspection: reads only `VIRE_LANGFUSE_BASE_URL`, `VIRE_LANGFUSE_SOURCE`, `VIRE_LANGFUSE_ENVIRONMENTS` | PASS |
| `LangfuseApi` trait is GET-only (no write path) | Trait definition: `probe`, `get_traces`, `get_observations` only | PASS |

---

## 7. Network Boundary Checks

| Check | Evidence | Result |
|---|---|---|
| Default base URL is `http://127.0.0.1:3000` | `DEFAULT_BASE_URL` constant; `loopback_is_the_default_and_allowed_for_local` | PASS |
| Local source refuses off-host targets | `local_source_refuses_off_host_targets` (`example.com` → `Err`) | PASS |
| Cloud source is rejected for loopback | `cloud_is_an_explicit_override_for_off_host_only` (Cloud + loopback → `Err`) | PASS |
| Non-http/https scheme refused | `non_http_scheme_is_refused` (`ftp://` → `Err`) | PASS |
| All request URLs go through `build_url` (path locked to `/api/public/`) | `api.rs:63` — `self.config.build_url(&path)` on every `get_json` call; `config.rs:255` — path must start with `/api/public/` | PASS |
| `reqwest` client: no redirect policy | `api.rs:49` — `redirect::Policy::none()` | PASS |
| Tauri `tauri.conf.json` and `capabilities/` unchanged | `git diff main HEAD -- src-tauri/tauri.conf.json src-tauri/capabilities/` → 0 lines | PASS |
| No new Tauri capability added | `reqwest` added to `Cargo.toml`; no capability files in diff | PASS |

---

## 8. Persistence Schema Checks

| Check | Result |
|---|---|
| `langfuse_import_runs`: no credential-bearing columns | PASS (`import_run_table_has_no_credential_columns`) |
| `langfuse_raw_traces`: primary key `(environment, trace_id)`; no credential columns | PASS (schema review) |
| `langfuse_ai_evidence`: token/cost columns nullable (no DEFAULT 0) | PASS (schema review: `INTEGER` and `REAL` with no default) |
| No modification to `projects` or `time_entries` tables | PASS (schema review: `init_db` only calls `langfuse::store::migrate`) |
| Migration is additive only (CREATE TABLE IF NOT EXISTS) | PASS |

---

## 9. Frontend / IPC Banner

| Check | Result |
|---|---|
| `get_langfuse_source_health` IPC command registered | PASS (`lib.rs:201` — in `generate_handler!`) |
| `import_langfuse_now` IPC command registered | PASS (`lib.rs:201`) |
| Blocking import runs on a dedicated OS thread (off Tauri runtime) | PASS (`lib.rs:186` — `std::thread::spawn`) |
| `sourceBanner()` renders for `unavailable`, `stale`, `unknown`, `auth_or_network_error` | PASS (`main.ts:16` — `degradedHealth` array; `main.ts:28` — `sourceBanner()`) |
| Settings view `sourcePanel()` shows base URL and source, no secrets | PASS (`main.ts:41` — shows `s.health`, `s.message`, `s.source`, `s.base_url`, `s.environments`; no key field) |
| Banner emitted in Today view | PASS (`main.ts:34` — `sourceBanner()` in `renderToday`) |
| Banner emitted in Settings view | PASS (`main.ts:40` — `sourceBanner()` in `renderSettings`) |

---

## 10. Blockers and Non-Blockers

### Blockers (TASK-019 gate blockers)
None.

### Non-Blockers

| ID | Finding | Severity | Classification |
|---|---|---|---|
| NB-1 | `csv_export_neutralizes_formula_like_project_names_and_notes` adversarial integration test FAILS | Low | **Pre-existing defect** — adversarial.rs last changed at `b1a9c6f` (before TASK-018); 0 diff lines from TASK-019; affects `csv_escape` in time-tracker core, outside TASK-019 scope. The leading whitespace and bare-CR path in `csv_escape` are not fully met. Needs a follow-up fix in a separate change. |

---

## 11. Gate Verdict

**QA STATUS: pass**

All TASK-019 spec scenarios have observable, passing test coverage. The 10-state health taxonomy
is fully covered. The absence-≠-zero invariant is asserted at both the application and DB schema
levels. Credentials are redacted at Debug boundary and absent from all persisted/IPC surfaces. The
network boundary invariants (loopback default, Cloud-explicit-only off-host, path-locked URLs) are
enforced in Rust and tested. OpenSpec validation passes. MinIO/S3 documentation is present and
accurate. The pre-existing adversarial CSV test failure is classified as non-blocking with evidence.

**Route:** → SW-4 (Code Reviewer) ∥ SW-5 (Security Agent)
