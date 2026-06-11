# SW-4 Code Review — TASK-019 Local Docker Langfuse Importer MVP

**REVIEW STATUS: pass**

- **Reviewer:** Code Reviewer (SW-4)
- **Branch:** `feat/task-019-local-langfuse-importer-mvp`
- **Commits reviewed:** `b30025e` (feat slice) + `b2b28c2` (QA/docs)
- **PR:** #11
- **Tier:** L2
- **Upstream gates cleared:** SW-3 QA (PASS), SW-5 Security (PASS)
- **Review date:** 2026-06-11

---

## Verdict

**PASS** — no blocking findings. Eight non-blocking suggestions follow; none require a return to the developer before release.

---

## 1. Scope reviewed

| File | Lines |
|------|-------|
| `src-tauri/src/langfuse/config.rs` | 293 |
| `src-tauri/src/langfuse/api.rs` | 155 |
| `src-tauri/src/langfuse/importer.rs` | 421 |
| `src-tauri/src/langfuse/model.rs` | 252 |
| `src-tauri/src/langfuse/store.rs` | 279 |
| `src-tauri/src/langfuse/mod.rs` | 54 |
| `src-tauri/src/langfuse/tests.rs` | 560 |
| `src-tauri/src/lib.rs` (new commands only, lines 175–191) | 17 |
| `src/main.ts` (diff only) | 12 |

Pre-existing `lib.rs` density / dense one-liners are not evaluated here (pre-existing style, out of TASK-019 scope).

---

## 2. Architecture

### 2.1 Renderer off-network — PASS

The webview CSP (`connect-src ipc: http://ipc.localhost`) is unchanged. All Langfuse HTTP calls run through `reqwest::blocking` in the Rust core only. No `tauri-plugin-http` added, no capability file touched. Confirmed by `git diff main HEAD -- src-tauri/tauri.conf.json src-tauri/capabilities/` (0 lines changed).

### 2.2 URL allowlist invariant — PASS

`validate_target()` (`config.rs:184–204`) enforces:
- `Source::Local` → host must be `127.0.0.1`, `localhost`, or `::1` (loopback only).
- `Source::Cloud` → host must be non-loopback (deliberate off-host override).

`build_url()` (`config.rs:209–266`) constructs every request URL from the configured base plus a fixed `ApiPath` variant. Defense-in-depth post-checks at `config.rs:255–264` reject any URL whose path does not start with `/api/public/` or whose host/scheme differs from the base. No path or host from response data can ever be substituted — `ApiPath` has no arbitrary-URL variant.

### 2.3 reqwest client — PASS

`api.rs:46–55`: rustls-tls (no OpenSSL linkage), 15 s request timeout, 5 s connect timeout, `redirect::Policy::none()` (no auto-follow). The no-redirect policy is the critical invariant that prevents a redirect from bouncing requests off the allowlisted origin.

### 2.4 Credentials — PASS

`Secret` type (`config.rs:43–59`) redacts `Debug`/`Display`. `Credentials::Debug` redacts both keys. `public_from_env()` never reads credential env vars. Credentials applied at exactly one call site (`api.rs:65–67`). `SourceHealthSnapshot` carries no key material. DB schema has no credential columns.

---

## 3. Health taxonomy

### 3.1 All 10 states present and correctly wired — PASS

`HealthState` enum (`model.rs:11–22`) has all 10 BA §7 states. `classify_health()` (`importer.rs:332–366`) uses a fixed-precedence ordering: infrastructure/transport failures dominate, then shape problems, then content signals, finally `Healthy` only on a clean positive.

State routing:
| State | Classification path |
|-------|---------------------|
| `unavailable` | probe fails with `Unavailable` or `api_error = Unavailable` |
| `auth_or_network_error` | probe/list fails with `Auth`, `RateLimited`, or `Network` |
| `unknown` | probe/list fails with `Indeterminate`; or no import has run yet |
| `schema_changed` | `schema_issues > 0` (unparseable trace OR generation lacks usage+cost) |
| `wrong_env` | traces found in `default`/non-allowed env |
| `duplicate` | all seen traces suppressed (`traces_seen > 0 && unique == 0`) |
| `delayed` | a trace timestamp predates the prior checkpoint |
| `stale` | cursor older than `window.to − 24h` and no new unique traces |
| `missing` | `unique == 0` and not stale |
| `healthy` | none of the above |

429/rate-limited folds into `auth_or_network_error` per design.md §6 — correct.

### 3.2 Absence-never-zero — PASS

`sum_opt_i64` / `sum_opt_f64` (`importer.rs:298–316`) preserve `None` when every contributor is `None`. Token/cost columns in `langfuse_ai_evidence` are nullable with no `DEFAULT`. Probe gate (`importer.rs:60–70`) prevents any evidence rows from being written when the stack is down. Tested by `absence_is_never_zero_cost_when_stack_down`, `time_only_trace_has_null_cost_not_zero`, `schema_changed_when_generation_lacks_usage_and_cost`.

### 3.3 Down-stack behavior — PASS

`unavailable_summary()` (`importer.rs:107–123`) returns a health-tagged summary with zero evidence, using the last known cursor from the DB for continuity. Summaries are persisted without evidence rows. State is surfaced in the banner and settings panel.

---

## 4. Persistence schema

### 4.1 No secrets in DB/evidence rows — PASS

All three importer tables (`langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence`) have no credential-bearing columns. Enforced by PRAGMA test `import_run_table_has_no_credential_columns`.

### 4.2 Dedup correctness — PASS

`seen_trace_ids()` (`store.rs:150–157`) seeds the in-memory `seen` set from the DB at the start of each environment run. New trace IDs are inserted via `HashSet::insert` (returns `false` on collision → `duplicates += 1`). Upserts on `(environment, trace_id)` primary key are idempotent across re-imports and overlap windows.

### 4.3 Cursor never regresses — PASS

`importer.rs:211–215`: cursor takes `max(max_ts, prior_cursor)` where the comparison is lexicographic over RFC3339 strings. Tested by `delayed_when_a_trace_predates_the_checkpoint` — confirms the cursor stays at `06-06` even when a `06-02` trace arrives late.

### 4.4 Migrations safe — PASS

All three tables use `CREATE TABLE IF NOT EXISTS` (`store.rs:13–53`). No `DROP`, `ALTER`, or destructive DDL. Called from `init_db` (`lib.rs:54`) on every open, so fresh installs and reopens are both handled.

---

## 5. IPC / frontend

### 5.1 No secrets exposed — PASS

`SourceHealthSnapshot` (`store.rs:222–231`) fields: `base_url`, `source`, `environments`, `last_import_at`, `latest_trace_ts`, `health`, `message`. No key/token/credential fields. `import_langfuse_now` returns the same snapshot after the run.

### 5.2 Status banner coherent — PASS

`main.ts:16`: `degradedHealth = ['unavailable','stale','unknown','auth_or_network_error']` — the states where AI totals are currently unknown or out-of-date. `sourceBanner()` renders in both Today and Settings views. All interpolated snapshot fields are escaped via `esc()` (= `escapeHtml`) before `innerHTML`.

### 5.3 No raw activity egress — PASS

`LangfuseApi` trait (`api.rs:14–32`): `probe`, `get_traces`, `get_observations` — no write/push path exists. Enforced by `importer_only_issues_read_calls`.

---

## 6. Test coverage

25 new tests in `langfuse::tests` cover all 10 health states, pagination/dedup (cross-page and cross-import), absence-≠-zero at DB and application level, credential redaction, URL allowlist (loopback/cloud/scheme), observation-sourced cost, read-only posture. All pass.

Pre-existing CSV formula test failure (`csv_export_neutralizes_formula_like_project_names_and_notes`) is correctly classified as a pre-existing defect: `adversarial.rs` was last modified at `b1a9c6f` (before TASK-018/TASK-019), TASK-019 diff shows 0 lines changed in that file. The failure is in `csv_escape`'s bare-CR / leading-whitespace handling in the time-tracker core, outside TASK-019 scope. Non-blocking.

---

## 7. OpenSpec / spec alignment

All 10 BA §7 health states are implemented with their exact string names (`as_str()` in `model.rs`). `wrong_env` is surfaced for the `default` environment via the synthetic probe in `run_import` (`importer.rs:78–80`). `missing` vs `stale` disambiguation uses the `STALE_AFTER_HOURS = 24` threshold per design.md §6. Usage/cost sourced from observations, not trace body, per design.md §4. Nullable token/cost schema per design.md §5.

---

## 8. Blocking findings

None.

---

## 9. Non-blocking suggestions

| ID | File:line | Finding | Suggestion |
|----|-----------|---------|------------|
| S-1 | `importer.rs:129` | `_config: &ImporterConfig` parameter is accepted but unused (underscore prefix confirms this). | Remove the parameter and its call-site argument, or promote it to use when per-environment config customisation is added. Dead parameters in public functions widen the mental model unnecessarily. |
| S-2 | `importer.rs:172` | `ts.as_str() < cur.as_str()` is a lexicographic string comparison of RFC3339 timestamps. Works correctly while Langfuse consistently emits UTC/Z-format strings, but would silently misclassify `+00:00`-offset timestamps (which compare lexicographically different). | Parse both sides with `parse_ts()` (already available at `importer.rs:368`) and compare as `DateTime<Utc>`. Same approach as `is_stale()` at `importer.rs:374–381`. |
| S-3 | `importer.rs:384–420` | `persist_run` issues multiple independent statements (upsert raw traces, upsert evidence rows, insert run record) without a surrounding transaction. A mid-run failure leaves a partially-written state that is invisible to callers. | Wrap the body of `persist_run` in `conn.execute("BEGIN")?` / `COMMIT` / rollback on error. One SQLite WAL transaction per import run is cheap and makes the write atomic. |
| S-4 | `importer.rs:395,401,419` | All three DB calls in `persist_run` use `let _ = ...` — failures are silently swallowed. | Propagate errors up (or at minimum push the error string into `summary.warnings`) so partial failures are surfaced in the run record and health snapshot. |
| S-5 | `importer.rs:46` | `now()` returns a local-time string (`"%Y-%m-%d %H:%M:%S"`) while trace timestamps and window boundaries are UTC RFC3339. `SourceHealthSnapshot.last_import_at` is a local-time string; `latest_trace_ts` is RFC3339. Two different time formats in the same DTO will surprise future consumers. | Use `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` for `started_at`/`finished_at`/`imported_at` to make all timestamps uniformly UTC RFC3339. |
| S-6 | `lib.rs:188` | `std::thread::spawn(...)`.`join()` has no timeout. If the import thread hangs beyond the `reqwest` client-level timeouts (e.g. due to a deadlock or a bug in a dependency), the `import_langfuse_now` IPC command blocks the UI indefinitely. | Add a channel-based timeout (e.g. `mpsc::channel` + `recv_timeout(Duration::from_secs(30))`) or document explicitly that the 15 s + 5 s reqwest timeouts are the effective ceiling. |
| S-7 | `main.ts:16` | `degradedHealth` omits `schema_changed`, `missing`, and `wrong_env`. `schema_changed` means totals are withheld for review — arguably warrants a banner. `missing` and `wrong_env` are data-quality signals that may also deserve surfacing to the user on the Today view, not only in Settings. | Evaluate adding `schema_changed` to `degradedHealth` before TASK-009 (review/approval UI). `missing` and `wrong_env` are lower priority. Current MVP behaviour is deliberate and documented in design.md §1 — flag for TASK-009 consideration only. |
| S-8 | `store.rs:237` | `source_health_snapshot` calls `latest_run(conn)` (across all environments) not `latest_run_for_env`. If a `wrong_env` run for `"default"` finishes most recently, the banner shows `wrong_env` rather than the last `"vire"` state. This is not wrong, but may surprise the operator. | Low-priority; acceptable for MVP. Consider filtering to `allowed_environments` in a follow-up once multi-environment UX is defined. |

---

## 10. Pre-existing defect (carry-over)

`csv_export_neutralizes_formula_like_project_names_and_notes` adversarial test failure is pre-existing (last changed `b1a9c6f`), unrelated to the Langfuse importer, and classified as non-blocking here consistent with QA NB-1. Needs a dedicated fix in a follow-up change targeting `csv_escape` in `lib.rs`.

---

## 11. Handoff

**PASS** — route to SW-6 Release Manager. Both SW-4 and SW-5 have cleared TASK-019.

Suggestions S-1 through S-8 are candidates for a post-MVP hygiene pass; none require rework before release.
