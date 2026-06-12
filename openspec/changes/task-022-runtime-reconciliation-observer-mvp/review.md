# Code Review (SW-4) — TASK-022 runtime-reconciliation-observer-mvp

- **Reviewer:** SW-4 Code Reviewer
- **Branch:** `feat/task-022-runtime-reconciliation-observer-mvp`
- **Tier:** L2
- **Gate context:** SW-3 QA PASS (64/64), SW-5 Security PASS. This is the rerun after the prior
  Codex attempt was blocked by a read-only sandbox; no code changed between attempts.
- **Date:** 2026-06-12
- **Verdict:** **PASS**

---

## Files reviewed

| File | Role |
|---|---|
| `src-tauri/src/runtime_observer/mod.rs` | Public entry point, orchestration, snapshot builder |
| `src-tauri/src/runtime_observer/config.rs` | Path resolution, env-map parsing, environment resolution |
| `src-tauri/src/runtime_observer/model.rs` | Domain types, `RawEvent` allowlist, `ReconciliationState` |
| `src-tauri/src/runtime_observer/ingest.rs` | File safety, lifecycle pairing, coarse-status mapping |
| `src-tauri/src/runtime_observer/reconcile.rs` | Session matching, health gating, `count_unmatched_traces` |
| `src-tauri/src/runtime_observer/store.rs` | `ai_runtime_sessions` DDL, upsert, read-only evidence/run projections |
| `src-tauri/src/runtime_observer/tests.rs` | 23 observer unit tests |
| `src-tauri/src/langfuse/model.rs` | `AiEvidence.session_id` additive field |
| `src-tauri/src/langfuse/store.rs` | `session_id` column in DDL + `add_column_if_absent` migration |
| `src-tauri/src/lib.rs` | `get_runtime_reconciliation` IPC command registration |
| `src/main.ts` | `RuntimeReconciliation` TS type, `runtimeLine()`, `renderSettings()` |

---

## Architecture boundary check

Every deliverable stays inside the AI Runtime Observer component:

| Concern | Spec requirement | Implementation | Result |
|---|---|---|---|
| No network calls | R1 | No `reqwest`/socket in `runtime_observer/`; `source` hardcoded to `"local"` | ✓ |
| No process scanning | R1 | Reads only the configured log file path; no `sysinfo`/process API | ✓ |
| No cost/time authority | R3 | `ai_runtime_sessions` has no `cost`/`token`/`duration` column (verified by structural test) | ✓ |
| Read-only IPC surface | R6 | `get_runtime_reconciliation` returns `RuntimeReconciliationSnapshot` — counts + posture only | ✓ |
| No classifier / review UI / export / new adapter | Arch boundary | None present | ✓ |
| CSP unchanged | R1 | No new capabilities or `connect-src` entries | ✓ |

No scope creep observed.

---

## Ingest allowlist

`model.rs:56-92` — `RawEvent` is the allowlist: serde silently drops every field not declared on the
struct. This is the correct pattern — a runtime filter could be bypassed; a type definition cannot.
Verified fields kept: `event`, `project_key`/`project`, `tool`, `run_id`, `session_id`, `ts`,
`status`, `exit_code`, `billable`, `duration_ms`. `billable` and `duration_ms` are parsed but
intentionally not persisted (`#[allow(dead_code)]` with a design comment) — consistent with
DEC-003/DEC-017.

The adversarial test (`tests.rs:124-150`) injects `prompt`, `command_body`, `shell_history`, `env`,
`metadata.summary/command_label/git_branch/git_remote_hash/cwd_basename`, `api_key` and asserts
none reaches any column of `ai_runtime_sessions`. ✓

---

## Schema correctness

`store.rs:16-32` — `ai_runtime_sessions` DDL: 11 columns — `session_key`, `project_key`, `tool`,
`session_id`, `environment`, `started_at`, `ended_at`, `status`, `reconciliation`,
`matched_trace_id`, `observed_at`. No `cost`, `token`, `duration`, `price`, `prompt`, `command`,
`secret` column. Enforced by a PRAGMA structural test (`tests.rs:408-425`). ✓

`langfuse/store.rs:38-59` — `langfuse_ai_evidence` DDL includes `session_id TEXT` on fresh installs;
`add_column_if_absent` handles upgrades from pre-TASK-022 installs by catching the SQLite "duplicate
column name" error. The pattern is correct for SQLite (no `ADD COLUMN IF NOT EXISTS` syntax). ✓

`upsert_ai_evidence` (`langfuse/store.rs:232-269`) includes `session_id` in both the `INSERT` and
`ON CONFLICT DO UPDATE SET` clauses, so it is populated and refreshed correctly. ✓

---

## Reconciliation correctness

`reconcile.rs:29-87` — `resolve_one` implements the spec's precedence exactly:

1. **`session_id` exact** — cross-environment; strongest signal (`reconcile.rs:41-49`).
2. **`environment` + time-window overlap** — only when the session maps to a known environment
   (`reconcile.rs:52-58`).
3. **No environment mapping** → `unmatched_runtime` (`reconcile.rs:61-68`).
4. **Import-health gate** — `observed_no_trace` asserted only under `HealthState::Healthy`;
   `Unavailable | Unknown | AuthOrNetworkError` → `reconciliation_unknown`; all other states
   (`Stale`, `Missing`, `WrongEnv`, `Delayed`, `Duplicate`, `SchemaChanged`) → `reconciliation_unknown`
   via the wildcard arm (`reconcile.rs:70-86`). ✓ Matches spec §4.1 and NFR-008.

`import_health_for` (`reconcile.rs:114-131`) selects the latest-finished run (by `finished_at`
string comparison — safe because RFC3339 lexicographic order equals chronological order) covering the
session window. When no run exists for the env or none covers the window, returns `HealthState::Unknown`
— never silently `Healthy`. ✓

`count_unmatched_traces` (`reconcile.rs:100-109`) — correct set-difference: evidence rows not
referenced by any `matched_trace_id`. ✓

---

## Path safety

`ingest.rs:42-56` — `read_safely`:
- Calls `fs::symlink_metadata` (not `metadata`) to avoid following the link before the type check.
- Rejects symlinks, non-files, and files over `MAX_LOG_BYTES` (8 MiB).
- Returns `None` on any `io::Error` — absence and unreadable states are equivalent.
- Uses lossy UTF-8 decode; non-UTF-8 bytes degrade to replacement chars in fields never persisted.

All four safety properties are covered by dedicated tests: `absent_log_is_no_evidence_not_zero`,
`empty_log_is_no_evidence`, `symlinked_log_is_refused`, `oversize_log_is_refused`. ✓

---

## IPC surface / frontend

`lib.rs:184-192` — `get_runtime_reconciliation` is read-only: acquires the mutex, calls
`observe_and_reconcile`, returns `RuntimeReconciliationSnapshot`. No write path visible in the
command body; any persist error becomes the stable `PERSIST_FAILURE_MSG` constant (no rusqlite
detail leaks).

`RuntimeReconciliationSnapshot` (`model.rs:151-163`) contains only `bool`, `u32`, and `String`
fields — no session content, no raw log lines. The `source` field is always the literal `"local"`
set in code. The `message` field is a server-side formatted string using only count variables.

`main.ts:42` — `runtimeLine()` interpolates only the numeric fields (`r.observed_runs`,
`r.observed_no_trace`, `r.reconciliation_unknown`) — safe against XSS. The server-side `message`
field is intentionally not rendered (the renderer builds its own message from counts). ✓

`main.ts:12` — `RuntimeReconciliation` TS type matches the Rust serialization shape exactly
(snake_case, matching field names and types). ✓

---

## Idempotency

`store.rs:37-73` — `upsert_session` uses `INSERT … ON CONFLICT(session_key) DO UPDATE SET` covering
all columns, so re-ingesting the same log updates the reconciliation state in place without
duplicating rows. Test `idempotent_reingest_does_not_duplicate_rows` asserts `session_count == 1`
after two passes. ✓

---

## Test quality

23 observer-owned tests covering:
- Adversarial allowlist (injected prohibited fields)
- Malformed lines (skipped not fatal)
- Path safety (absent, empty, symlink, oversize)
- Session lifecycle pairing (start+finish, non-zero exit, orphan)
- Matcher precedence (session_id-exact, env+time, session_id beats time, unmappable project)
- Import-health gating (healthy→gap, unavailable/unknown/auth_error→unknown, stale/missing/duplicate→unknown, no covering run→unknown)
- Absence ≠ zero (absent log + evidence, unmatched trace)
- Structural schema check (PRAGMA table_info — no cost/token/duration column)
- IPC surface (snapshot serializes counts only, no secret/content)
- Environment resolution (map > allowed-env > None)
- Idempotent re-ingest and persistence round-trip

All 64 tests pass (QA confirmed). Test naming is descriptive and consistent with the importer's
`tests.rs` style. ✓

---

## Naming and readability

- Module names (`ingest`, `reconcile`, `store`, `config`, `model`) are idiomatic and unambiguous.
- `ReconciliationState` variants match the spec's taxonomy exactly.
- `RawEvent`, `RuntimeSession`, `ReconciledSession`, `EvidenceRef`, `ImportRunHealth` are clear.
- `observe_with_config` is correctly `pub(crate)` for test injection; `observe_and_reconcile` is
  the public surface.
- Design comments (the load-bearing `PERSIST_FAILURE_MSG`, `absence ≠ zero` inline annotations,
  `DEC-003/DEC-017` references) explain non-obvious invariants, not what the code does.

---

## Dead code

- `RawEvent.billable` and `RawEvent.duration_ms` are intentionally parsed and `#[allow(dead_code)]`
  because they are on the allowlist but not persisted (DEC-003/DEC-017). The design comment on the
  field explains the intent. Not removable without narrowing the allowlist.
- `Trace.name`, `Trace.metadata`, `Observation.model`, `PageMeta.page/limit/total_items`,
  `Usage.unit` carry `#[allow(dead_code)]` in `langfuse/model.rs` — pre-existing, not introduced by
  this change.

No gratuitous dead code introduced by TASK-022.

---

## Suggestions (non-blocking)

1. **`nonempty` helper is duplicated** (`model.rs:111`, `ingest.rs:186`, `reconcile.rs:194`).
   The three definitions are identical. If the `runtime_observer` module grows further, consider
   extracting this to a private `util` submodule or an inline `use super::model::nonempty`. No
   functional impact today.

2. **`add_column_if_absent` matches on SQLite error message string** (`langfuse/store.rs:74`).
   `msg.contains("duplicate column name")` is stable across SQLite versions in practice, but it is
   a message-string dependency. An alternative using `PRAGMA table_info` to pre-check column
   existence would be more explicit. Acceptable as-is for an MVP.

3. **`coarse_status` substring matching on `status` field** (`ingest.rs:169`). `s.contains("ok")`
   would incidentally match values like "invoking" or "clocking". Since `status` is an allowlisted
   field sourced from pi-observe (a controlled emitter), the false-positive risk is negligible.
   Exact-value matching (`s == "ok"`) would be more precise long-term.

4. **`window_covers` fallback for timestampless sessions** (`reconcile.rs:143`). A session with no
   parseable timestamps returns `true` from `window_covers`, meaning it is considered covered by any
   import run for the env. The comment explains this, but the implication is that a timestampless
   session with an env mapping can reach `observed_no_trace` under a healthy import. This is
   conservative; if it produces noise in practice, adding a per-session `reconciliation_unknown`
   short-circuit for missing timestamps is the clean fix.

---

## Gate verdict

**PASS** — no blocking findings. The implementation faithfully delivers the spec across all six
review focus areas: architecture boundary, ingest allowlist, schema, reconciliation correctness,
IPC/frontend, and scope. Naming is clear, test coverage is comprehensive, and dead code is minimal
and annotated. Four non-blocking suggestions logged above; all are refinements, none require
developer action before release.

**Next:** wait for SW-5 security gate (already committed at `be8029b` — PASS), then route to SW-6
Release Manager.
