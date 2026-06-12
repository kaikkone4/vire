# TASK-021 — Implementation tasks

Single backend task. Rust core only. No schema change, no new health state, no renderer change.

## 1. Expose the persist-failure sentinel

- [x] `src-tauri/src/langfuse/importer.rs`: change `const PERSIST_FAILURE_MSG` to `pub const`.
- [x] Update the `persist_run` doc comment: the marker insert is **best-effort** (defense-in-depth for
      the on-demand snapshot); correctness is now guaranteed by the in-band return in `run_blocking_import`.
      Keep `let _ = store::insert_import_run(conn, &marker);` as-is — it stays best-effort.

## 2. Propagate the failure in-band out of the importer entry point

- [x] `src-tauri/src/langfuse/mod.rs`: in `run_blocking_import`, capture the summaries:
      `let summaries = importer::run_import(&api, &conn, &config, &window);`
- [x] If any summary carries the sentinel, return a secret-free `Err`.
- [x] Match on the exact sentinel string — **not** on `health == Unknown` (Unknown is also reachable from
      a legitimately-persisted `ApiErrorKind::Indeterminate` and must not be treated as a write failure).

> Implementation note (equivalent-but-simpler per arch-review §"Use equivalent only if simpler"):
> the sentinel check is extracted into a tiny private helper `import_result(&[ImportSummary]) ->
> Result<(), String>` that `run_blocking_import` calls (`import_result(&summaries)`). Behaviour is
> identical to the inline snippet, but the helper makes the in-band surfacing path directly
> unit-testable (the regression test exercises the exact production function rather than a duplicated
> check). No extra public surface — the helper is private to the `langfuse` module.

## 3. IPC surfacing (verify only — no edit expected)

- [x] `src-tauri/src/lib.rs`: confirmed `import_langfuse_now` propagates the new `Err` via the existing
      `run_bounded(Duration::from_secs(IMPORT_TIMEOUT_SECS), … run_blocking_import …)?` (lib.rs:212-214)
      and therefore never reaches `health_snapshot(&db)` on a persist failure. No code change required.

## 4. Regression test — both writes fail

- [x] `src-tauri/src/langfuse/tests.rs`: added `persist_failure_surfaces_in_band_even_when_marker_write_also_fails`,
      which installs a `BEFORE INSERT` trigger on `langfuse_import_runs` (so both `persist_import_run`
      **and** the marker `insert_import_run` abort).
- [x] Asserts the surfaced run is `HealthState::Unknown` with a warning equal to the persist sentinel.
- [x] Seeds a prior **durably-persisted** `healthy` run, then forces the failing import, and asserts the
      in-band `import_result(&summaries)` yields `Err` (non-healthy) while the DB snapshot is *still*
      stale-`healthy` — proving the in-band `Err`, not `source_health_snapshot`, is the surfacing channel.
- [x] Asserts the surfaced message is secret-free (no `sk-`, `Bearer`, `Authorization`, `password`,
      `token`; the trigger's `RAISE` message embeds a `sk-leak-canary token` to prove the driver string
      is never echoed).
- [x] Existing `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` test
      (marker-succeeds case) left unchanged — both paths hold.

## 5. Gate

- [x] `cargo test --lib` green (40 passed), including the new regression. NOTE: the pre-existing
      `tests/adversarial.rs::csv_export_neutralizes_formula_like_project_names_and_notes` integration
      test fails identically on the base commit `dd5d3b9` (CSV export, unrelated to TASK-021) — not
      introduced by this change.
- [x] `cargo clippy --lib` clean on the touched code (`import_result`, the `pub const`, doc comments,
      and the new test introduce zero warnings). Pre-existing warnings at importer.rs:182 / 305-316 are
      in untouched TASK-020 code and out of scope.
- [x] `openspec validate task-021-langfuse-persist-failure-ipc-surfacing --strict` passes.
