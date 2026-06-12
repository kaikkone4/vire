# TASK-021 — Surface a persistence failure to the import IPC, marker-independent

## Why

TASK-020 (PR #12) merged to `main` at `dd5d3b9` and added the S-3/S-4 hardening: an import run
persists atomically, and a persistence failure degrades the in-memory summary to a non-healthy
`Unknown` with a secret-free warning. Post-merge code-to-spec verification found that the **S-4
guarantee does not actually reach the IPC** when the database is broadly unwritable. The current code
surfaces a persist failure only through a best-effort durable marker write; when the same fault that
broke the run transaction also breaks the marker insert, the import command reports the **prior**
(possibly `healthy`) snapshot. That is a false-healthy — the exact outcome the absence-≠-zero invariant
exists to prevent.

Three coupled links produce the gap:

1. `src-tauri/src/langfuse/mod.rs` — `run_blocking_import` calls `importer::run_import` but **discards**
   the returned `Vec<ImportSummary>` and always returns `Ok(())`. The in-memory degrade to `Unknown`
   (importer.rs, on `persist_import_run` failure) never reaches the IPC caller.
2. `src-tauri/src/langfuse/importer.rs` — `persist_run` records the failure into the DB only via a
   marker run inserted with `let _ = store::insert_import_run(conn, &marker)`, whose error is
   **discarded**. The marker is the *only* durable channel to the snapshot.
3. `src-tauri/src/lib.rs` — `import_langfuse_now` returns `health_snapshot(&db)`, read from the most
   recent persisted run. With (1) and (2) both failing, that is the previously-persisted run, which can
   be `healthy`.

Net: when **both** the run transaction and the marker insert fail (read-only DB, disk full, lock
contention, a constraint/trigger covering `langfuse_import_runs`), the manual import returns
false-healthy. The existing S-3/S-4 test only forces a trigger on `langfuse_ai_evidence`, so the small
marker insert into `langfuse_import_runs` still succeeds and the gap is uncovered.

## What Changes

This is a **backend-only hotfix** to one component (`langfuse` importer + its single IPC command). It
adds no source, no schema change, no new health state, and no runtime reconciliation. It makes the
persist-failure signal **authoritative in-band** on the import command's return path, so correctness
no longer depends on a durable DB write that may itself be impossible.

- **Propagate the persist-failure signal out of `run_blocking_import` (mod.rs).** After `run_import`,
  if any returned summary carries the persist-failure sentinel, return a secret-free
  `Err(PERSIST_FAILURE_MSG)` instead of `Ok(())`. This converts the already-produced in-memory
  degradation into an in-band failure that the IPC cannot miss.
- **Expose the sentinel (importer.rs).** Make `PERSIST_FAILURE_MSG` `pub` so `run_blocking_import` can
  match the exact, unambiguous marker that `persist_run` already pushes into `summary.warnings`. The
  sentinel — not `health == Unknown` — is the signal, because `Unknown` is also reachable from a
  legitimately-persisted `ApiErrorKind::Indeterminate` classification and must not be conflated with a
  write failure.
- **IPC surfaces the failure (lib.rs).** No logic change is required: the existing `?` on
  `run_bounded(… run_blocking_import …)` in `import_langfuse_now` propagates the new `Err`, so the
  command returns a secret-free non-healthy result and never reads the stale snapshot — exactly the
  pattern already used for the bounded-timeout path (`IMPORT_TIMEOUT_MSG`).
- **Keep the marker best-effort (importer.rs).** The `let _ = insert_import_run(marker)` stays as
  defense-in-depth for the on-demand `get_langfuse_source_health` path when the small insert can still
  succeed. It is no longer load-bearing for correctness, so it remains `let _`.
- **Regression test (tests.rs).** Add a both-writes-fail case (trigger on `langfuse_import_runs` so the
  marker insert also aborts) asserting the run surfaces `Unknown` with a secret-free persist warning and
  that the import command's in-band path yields a non-healthy result — never the prior `healthy`
  snapshot. Cover a secret-free `Err` body.

## Impact

- **Affected specs:** `langfuse-importer` — MODIFIES the TASK-020 requirement "Persistence failures are
  surfaced, never read as healthy or zero" to make surfacing **IPC-reaching and marker-independent**.
  No requirement removed; the absence-≠-zero contract is tightened, not changed.
- **Affected code (Rust core only):** `src-tauri/src/langfuse/mod.rs` (`run_blocking_import` return),
  `src-tauri/src/langfuse/importer.rs` (`pub` sentinel; comment on best-effort marker),
  `src-tauri/src/langfuse/tests.rs` (both-writes-fail regression). `src-tauri/src/lib.rs` is in the
  affected chain but needs **no edit** — the `?` already propagates.
- **Unchanged surfaces (explicit):** schema/columns (no `ALTER`/`DROP`), the 10-state health taxonomy
  (no new state; `Unknown` reused), the REST contract, `tauri.conf.json` CSP / `capabilities/`, the
  renderer off-network posture, and `Cargo.toml`/`Cargo.lock` (no dependency change).
- **Out of scope (clean boundaries):** runtime reconciliation / retry / backoff (TASK-006 territory),
  any change to the on-demand `get_langfuse_source_health` semantics, a new health state, schema
  migration, and the renderer's handling of a rejected import (it already treats the timeout `Err` as
  non-healthy; this reuses that path). See `arch-review.md` §Boundary.
- **Guardrails preserved:** secret-free surfacing (the sentinel is a fixed string, never interpolates
  driver/credential text — SEC-003), absence-≠-zero, renderer off-network.
- **Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing`, base `main`
  (`dd5d3b9` — TASK-020 PR #12 merged; no merge-order dependency).
