# Code Review — TASK-021 `fix/task-021-langfuse-persist-failure-ipc-surfacing`

**Tier:** L2
**Reviewer:** SW-4 Code Reviewer
**Commit audited:** `99e1118`
**PR:** #13
**Date:** 2026-06-12

---

## Verdict: PASS

No blocking issues. Three minor suggestions (non-blocking) below.

---

## Scope verification

Three files changed in production code + one in tests, exactly matching the proposal:

| File | Change |
|---|---|
| `src-tauri/src/langfuse/importer.rs` | `const` → `pub const` for `PERSIST_FAILURE_MSG`; doc comment updated |
| `src-tauri/src/langfuse/mod.rs` | `run_blocking_import` captures summaries; new private `import_result` helper |
| `src-tauri/src/langfuse/tests.rs` | New regression test `persist_failure_surfaces_in_band_even_when_marker_write_also_fails` |
| `src-tauri/src/lib.rs` | **Unchanged** — verified |

No schema change, no new health state, no new dependency, no renderer change. Scope is tight.

---

## Correctness findings

### 1. Sentinel keying — correct

`import_result` (mod.rs:53–61) uses exact string equality `w == importer::PERSIST_FAILURE_MSG`, not a `health == Unknown` check. This correctly distinguishes a persist failure from a legitimately-persisted `ApiErrorKind::Indeterminate` run, which also produces `Unknown` but must not surface as `Err`. The existing `unknown_when_response_is_indeterminate` test (pre-TASK-021) confirms the non-failure `Unknown` path returns `Ok(())` — the two uses of `Unknown` are correctly separated.

### 2. IPC propagation path — correct

`lib.rs:212–214` (unchanged):
```rust
run_bounded(Duration::from_secs(IMPORT_TIMEOUT_SECS), move || {
    langfuse::run_blocking_import(&db_path)
})?;
let db = db_conn(&state)?;
langfuse::health_snapshot(&db)
```
The `?` operator after `run_bounded` causes `import_langfuse_now` to return immediately on `Err`, before `health_snapshot(&db)` is called. A persist failure can never fall through to the stale snapshot read.

### 3. Marker stays best-effort — correct

`importer.rs:445`: `let _ = store::insert_import_run(conn, &marker)` is unchanged. The marker is correctly demoted to defense-in-depth for the on-demand `get_langfuse_source_health` path. No load-bearing role, no load-bearing correctness assumption.

### 4. Secret-free surfacing — correct

`import_result` returns `Err(importer::PERSIST_FAILURE_MSG.to_string())` — a fixed constant string with no interpolation of driver errors, credentials, or config text. The `PERSIST_FAILURE_MSG` constant contains no credential material. All other error paths in `run_blocking_import` (`e.to_string()` for rusqlite, `e.message` for `ApiError`) are already documented as secret-free in `model.rs:74–75` ("Human-readable, secret-free description").

### 5. Regression test quality — strong

The new test (`tests.rs:608–672`) is thorough:

- Seeds a durable `healthy` run before the fault (exercises the false-healthy prevention claim directly)
- Installs a trigger on `langfuse_import_runs` — the exact table the marker write hits — forcing both `persist_import_run` and `insert_import_run` to fail under one fault, closing the gap the proposal identifies
- Embeds `sk-leak-canary token` in the RAISE message as a deliberate canary; all fragments (`sk-`, `token`, `canary`, `forced`, `RAISE`, `ABORT`) appear in the needle list, so the secret-echo check is not forgeable by reformulating the trigger message
- Asserts in-memory `Unknown` + exact sentinel (covers S-4 in-memory degrade)
- Asserts `import_result(&summaries).is_err()` (covers in-band IPC path)
- Asserts `after.health == "healthy"` (stale snapshot) while `import_result` returns `Err` — the two together prove the in-band channel is the authoritative one, not the DB read

The pre-existing `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` (trigger on `langfuse_ai_evidence`) is unmodified and still valid — it covers the marker-succeeds case.

---

## Suggestions (non-blocking)

### S1 — `import_result` double-`.any()` (mod.rs:54–56)

```rust
let persist_failed = summaries
    .iter()
    .any(|s| s.warnings.iter().any(|w| w == importer::PERSIST_FAILURE_MSG));
```

Readable as-is. An alternative with `.flat_map` would be marginally shorter but no clearer. No action needed unless a style pass standardizes warning iteration across the module.

### S2 — Multi-environment Err is always the same fixed string (mod.rs:58)

When two environments both fail to persist, `import_result` returns the same `PERSIST_FAILURE_MSG` regardless of which environments were affected. This is correct for the IPC contract (secret-free, fixed string), and the per-environment detail is already in the in-memory `summaries` available to the caller's log path. Worth a one-line note in the `import_result` doc comment if multi-environment use becomes more prominent, but not a problem now.

### S3 — Pre-existing clippy warnings (importer.rs:182, 305–316) — out of scope here

Four pre-existing warnings (confirmed in QA) in TASK-020 code; not introduced by this change. Tracked separately.

---

## Gate checklist

| Check | Result |
|---|---|
| Correctness of in-band persist-failure surfacing | PASS |
| Sentinel keys on exact `PERSIST_FAILURE_MSG`, not broad `Unknown` | PASS |
| `run_blocking_import` / `import_result` behavior | PASS |
| IPC propagation via `lib.rs` `?` without edit | PASS |
| Both-writes-fail regression test covers the gap | PASS |
| Pre-existing marker-succeeds test unmodified | PASS |
| Secret-free surfacing on all error paths | PASS |
| No schema change | PASS |
| No new health state | PASS |
| No TASK-006/runtime reconciliation scope creep | PASS |
| Code simplicity / no dead code | PASS |
| Conventions (naming, visibility, doc comments) | PASS |
| Zero new clippy warnings on changed code | PASS (confirmed by QA) |
