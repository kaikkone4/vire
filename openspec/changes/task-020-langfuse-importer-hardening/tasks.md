# Tasks — TASK-020 Langfuse importer hardening + L2 release hygiene

Single component (Langfuse importer in the Rust core) plus one repo-level lock file. Sub-tasks are an
implementation sequence within one OpenSpec change, **not** a component split. Recommended order:

## 1. Build reproducibility (A1)

- [x] `git add src-tauri/Cargo.lock` and commit it (it is untracked and not gitignored). **No**
      `cargo update`, no version bumps — pin exactly the closure TASK-019 validated.
- [x] Confirm `.gitignore` does **not** ignore `Cargo.lock` (it currently does not; keep it that way).

## 2. Atomic persistence (S-3)

- [x] Add a `store`-level transactional entry point (`store::persist_import_run(...)`) that wraps
      the run's writes — raw-trace upserts, evidence upserts, run-record insert — in one
      `conn.unchecked_transaction()` (commit on success; rusqlite rolls back on the error path).
- [x] Reuse the existing `upsert_raw_trace` / `upsert_ai_evidence` / `insert_import_run` helpers on the
      transaction; atomicity unit = one import run (one `run_id`), per environment.

## 3. Surface persistence errors (S-4)

- [x] Stop discarding DB results (`let _ = …`). Persistence now returns `rusqlite::Result<()>`; on a
      write error the in-memory summary degrades to `unknown` (non-healthy) with a secret-free
      warning, and a separate marker run records that non-healthy state into the snapshot.
- [x] Keep all surfaced text secret-free (SEC-003): fixed `PERSIST_FAILURE_MSG`; no config/credential
      material and no raw driver string interpolated.
- [x] A persistence failure is never recorded as `healthy` and never contributes a zero total.

## 4. Uniform UTC RFC3339 timestamps (S-5)

- [x] Change `now()` to `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` so
      `started_at`/`finished_at`/`imported_at` match the existing RFC3339 `cursor_ts`/window/observation
      timestamps. No schema change (columns are already `TEXT`).
- [x] Added a code-comment note that the `finished_at` sort key tolerates the legacy→RFC3339
      transition: `'T'` > `' '`, so new rows always sort latest; **no** data migration.

## 5. Bounded import IPC (S-6)

- [x] In `import_langfuse_now`, replaced the unbounded `spawn(...).join()` with an `mpsc::channel` +
      `recv_timeout(Duration::from_secs(IMPORT_TIMEOUT_SECS))` (30 s — above the reqwest 15 s/5 s
      ceilings). The blocking import stays on its dedicated OS thread (off the Tauri runtime).
- [x] On timeout, return a secret-free non-healthy error. **No** 11th health state — surfaced as an
      IPC error string.

## 6. Tests (extend the mock-based suite; no network/credentials)

- [x] Atomicity: a forced mid-run write failure leaves no partial run (raw + evidence rows absent).
- [x] Error surfacing: a persistence failure appears as a non-healthy `unknown` warning; never
      `healthy`, never a zero total.
- [x] Timestamps: `started_at`/`finished_at`/`imported_at` parse as RFC3339 UTC; legacy-vs-new ordering
      holds (a `…T…Z` row sorts latest over a space-format row).
- [x] Bounded IPC: an over-timeout worker yields a bounded secret-free error; a normal run returns its
      result within the ceiling.
- [x] Regression: all existing importer tests stay green (absence-≠-zero, credential redaction).
- [ ] (Optional) Real local Docker Langfuse smoke — **skipped**: no local stack readily up in-session;
      per the task this is not a blocker and does not gate the change.

## 7. Verification

- [x] `cargo test` green (39 passed; 0 failed).
- [x] `cargo build --locked` succeeds against the now-committed `Cargo.lock` (reproducible closure).
- [x] `openspec validate task-020-langfuse-importer-hardening --strict`.

## Out of scope (do not build here)

- TASK-006 AI runtime observer / reconciliation (DEC-017: no duplicate ledger, no new pi/Claude
  adapter), TASK-008 classifier, TASK-009 review/approval UI (incl. S-7 `degradedHealth` tuning),
  TASK-010 AI-evidence CSV export, TASK-005 macOS capture adapter.
- S-1 (dead `_config` param) and S-8 (per-env snapshot selection) — deferred polish.
- S-2 (lexicographic compare of Langfuse-emitted trace timestamps) — adjacent to S-5 but distinct;
  optional only if near-zero-cost while already in `importer.rs` (reuse `parse_ts()`); must not expand
  the change. **Not** touched — left as a distinct follow-up.
- The pre-existing `csv_export_*` adversarial failure — separate time-tracker-core defect.
