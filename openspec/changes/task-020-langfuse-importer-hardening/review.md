# Code Review — TASK-020 Langfuse Importer Hardening

- **Verdict:** PASS
- **Branch:** `feat/task-020-langfuse-importer-hardening` (PR #12)
- **Tier:** L2
- **Reviewer:** SW-4 Code Reviewer
- **Scope reviewed:** A1 (`Cargo.lock`), S-3 (atomicity), S-4 (error surfacing), S-5 (UTC RFC3339), S-6 (bounded IPC), scope-creep check

---

## Blocking Issues

None.

---

## Per-area Assessment

### A1 — Cargo.lock committed (commit `f8fd591`)

Isolated commit, first-ever addition of the file. `Cargo.toml` is unchanged — no version bumps, no `cargo update`. Correct hygiene for a binary/app crate. The lock pins exactly the TASK-019 validated dependency closure and becomes the SBOM input for `cargo audit` / `cargo deny`. No issues.

### S-3 — Atomic persistence (`store.rs:102–118`, `importer.rs:403–437`)

`store::persist_import_run` opens `conn.unchecked_transaction()`, threads all three write groups (`upsert_raw_trace`, `upsert_ai_evidence`, `insert_import_run`) through the `&tx` borrow, then calls `tx.commit()`. A `Transaction` dropped without commit rolls back automatically — the no-commit path is the entire error path. `unchecked_transaction` is appropriate here: the production connection is always held through a `Mutex<Connection>` so no concurrent writer can nest into it, and the tests are single-threaded. Atomicity unit is one `run_id` per environment, matching the per-env cursor model.

`persist_run` in `importer.rs` calls `store::persist_import_run(...).is_err()` and branches correctly on failure; the transaction wrapping is entirely in `store.rs` as the design specified.

### S-4 — Surface persistence failures (`importer.rs:388–436`)

On `persist_import_run` failure:
- `summary.health` is degraded to `HealthState::Unknown` — correct, never "healthy".
- `PERSIST_FAILURE_MSG` is pushed into `summary.warnings`. It is a `const &str` — no driver string interpolated, no config or credential material reachable (SEC-003 honored).
- A separate marker `ImportRunRecord` (fresh `Uuid`, `cursor_ts: None`, `status: Unknown`) is inserted via `store::insert_import_run` so the failed run is visible in the health snapshot.
- The marker insert error is silently discarded (`let _ = store::insert_import_run(conn, &marker)`). This is pragmatically correct — we are already on an error path and there is nothing further to degrade — but it is the second silent discard on the failure path. See suggestion S1.

### S-5 — UTC RFC3339 timestamps (`importer.rs:50–52`)

`now()` changed to `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)`. Feeds `started_at`, `finished_at` (run record), and `imported_at` (raw-trace row). Ordering invariant is sound: `'T'` (0x54) > `' '` (0x20), so every new `…T…Z` row out-sorts any legacy space-format row under the `ORDER BY finished_at DESC` queries in `latest_run` / `latest_run_for_env`. No backfill needed and none attempted.

One naming curiosity: `started_at` and `finished_at` both receive the same `stamp` value captured at the start of `persist_run`, i.e. after the import data-fetch is complete. `started_at` therefore records "when persistence began" rather than "when the import began". This is pre-existing behavior from TASK-019 and is a deferred S-1 item per `design.md §8`. Not a TASK-020 finding — noted as suggestion S2.

### S-6 — Bounded `import_langfuse_now` (`lib.rs:184–216`)

`IMPORT_TIMEOUT_SECS = 30` is above the reqwest 15 s request / 5 s connect ceilings, so a normal slow import is not cut off. `run_bounded` spawns an OS thread (correct: blocking reqwest client must stay off the Tauri async runtime), sends the result over `mpsc::channel`, and calls `rx.recv_timeout`. On `RecvTimeoutError`, it returns `IMPORT_TIMEOUT_MSG` — a fixed, secret-free const. No new health-taxonomy state is introduced; the timeout surfaces as an IPC error. An orphaned worker is bounded by the reqwest ceilings and would persist atomically (S-3) if it later completes.

`run_bounded` is well-decomposed as a standalone testable function; both tests in `lib.rs` (prompt return on timeout, verbatim passthrough of normal result/error) are direct and correct.

### Scope-creep check

Files touched in the code change commit (`b4f0f3d`): `src-tauri/src/langfuse/importer.rs`, `src-tauri/src/langfuse/store.rs`, `src-tauri/src/lib.rs`. No TASK-006 reconciliation code, no classifier, no review UI, no CSV exporter changes, no Tauri capabilities or CSP changes. Scope is exactly A1 + S-3 + S-4 + S-5 + S-6. Clean.

### Test quality

All eight spec scenarios have direct, observable test coverage. The S-3/S-4 test uses a SQLite `BEFORE INSERT` trigger to force a genuine mid-transaction abort — a stronger signal than a mock. The S-5 ordering test directly inserts a legacy-format row and a modern RFC3339 row on the same day and asserts the correct winner. S-6 tests use real timing with `std::time::Instant`. No spec scenario is covered by assertion alone without observable state.

---

## Suggestions (non-blocking)

**S1 — Document the double-discard on marker insert failure (`importer.rs:435`)**

`let _ = store::insert_import_run(conn, &marker)` is two levels of error handling deep. The silent discard is intentional but a future reader may wonder why it has no `map_err` branch. A one-line comment would clarify:

```rust
// If even the marker cannot be written there is nothing further to degrade.
let _ = store::insert_import_run(conn, &marker);
```

**S2 — `started_at` == `finished_at` naming drift (`importer.rs:420–421`)**

Both fields receive the same `stamp` (captured at persistence time). `started_at` is semantically misleading — it records when persistence began, not when the import began. Capturing a start stamp at the top of `run_import` (or at the top of `persist_run`'s caller) would fix the semantics. This is a deferred S-1 item per `design.md §8`; flagging for tracking, not blocking.

**S3 — `run_blocking_import` discards `run_import` return value without annotation (`mod.rs:42`)**

```rust
importer::run_import(&api, &conn, &config, &window);
```

The `Vec<ImportSummary>` return is intentionally discarded (the caller reads state from DB afterward), but the discard is unmarked. Adding `let _ =` makes the intent explicit and consistent with the other intentional discard on line 435.

---

## Gate Verdict

**PASS** — All four hardening scenarios (S-3, S-4, S-5, S-6) and the A1 lock hygiene are correctly implemented, testable, and within the stated scope. No scope creep. No dead code introduced. Conventions followed throughout.

Route to SW-5 (Security) in parallel. SW-6 (Release Manager) pending both SW-4 and SW-5 PASS.
