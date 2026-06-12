# SW-1 Architecture Review — TASK-021

**Change:** `task-021-langfuse-persist-failure-ipc-surfacing`
**Tier:** L2 · **Type:** post-merge backend hotfix to TASK-020 (PR #12, merged `main@dd5d3b9`)
**Verdict:** PASS — single task, backend-only, no split, no BA escalation.

## 1. Problem under review

Post-merge code-to-spec verification of TASK-020 found that its S-4 requirement
("Persistence failures are surfaced, never read as healthy or zero") is satisfied for the in-memory
summary but **not delivered to the IPC** when the database is broadly unwritable. The manual import
can return a previously-persisted `healthy` snapshot — a false-healthy that violates absence-≠-zero.

### Failure chain (verified against merged code)

| Link | Location | Behavior |
|------|----------|----------|
| 1 | `langfuse/importer.rs` `persist_run` | On `persist_import_run` failure: degrades in-memory summary to `Unknown` + pushes `PERSIST_FAILURE_MSG`, then records it durably **only** via `let _ = store::insert_import_run(conn, &marker)` — marker error **discarded**. |
| 2 | `langfuse/mod.rs` `run_blocking_import` | Calls `run_import` but **discards** the `Vec<ImportSummary>`; always returns `Ok(())`. The in-memory `Unknown` never reaches the caller. |
| 3 | `lib.rs` `import_langfuse_now` | Returns `health_snapshot(&db)` = `store::latest_run` = the most recent **persisted** run. |

When the run transaction fails **and** the marker insert also fails (read-only DB, disk full, lock
contention, or a constraint/trigger on `langfuse_import_runs`), nothing for the current run is written.
`latest_run` returns the prior run, which may be `healthy`. Links 2 and 3 then surface that stale
`healthy` to the renderer. The only channel S-4 relied on (the durable marker) is precisely the thing
that fails under the same fault — a single-point-of-failure design defect, not a logic typo.

**Test blind spot:** `persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced` (tests.rs)
forces a trigger on `langfuse_ai_evidence` only. The marker insert targets `langfuse_import_runs`, so it
still succeeds — the both-writes-fail path is never exercised, which is why the gap survived review.

## 2. Architectural decision

**The persist-failure signal must be authoritative in-band on the import command's return path,
independent of any durable DB write.** Rationale: when both DB writes fail, no durable "unknown" can be
recorded *by definition* — the store is unwritable. Correctness therefore cannot depend on a database
write succeeding. The only fault-independent channel is the function return value of the import call.

### Contract (the fix must satisfy all)

1. A run that cannot be persisted SHALL surface a **non-healthy, secret-free** result to the
   `import_langfuse_now` IPC, **even when the durable failure-marker write also fails**.
2. A persistence failure SHALL NEVER cause the IPC to return a prior `healthy`/stale snapshot, and SHALL
   NEVER contribute a zero usage/cost total.
3. Surfacing text SHALL carry no credential/secret material (reuse the fixed `PERSIST_FAILURE_MSG`;
   never interpolate driver/config strings — SEC-003).
4. NO new health-taxonomy state (reuse `Unknown`); NO schema change; NO runtime reconciliation/retry.

### Chosen mechanism

- Expose `PERSIST_FAILURE_MSG` as `pub` (sentinel `persist_run` already emits into `summary.warnings`).
- `run_blocking_import` inspects the summaries; if the sentinel is present, return
  `Err(PERSIST_FAILURE_MSG)` instead of `Ok(())`.
- `import_langfuse_now` needs no edit — the existing `run_bounded(…)?` propagates the `Err`, so the
  command returns a secret-free non-healthy result and never reads the stale snapshot.

**Signal choice — sentinel, not `health == Unknown`.** `Unknown` is also produced by a
legitimately-*persisted* `ApiErrorKind::Indeterminate` classification (classify_health). Keying on
health would convert a healthy-persisted indeterminate run into a false `Err`. The fixed warning string
is pushed *only* on persist failure, so it is the unambiguous signal.

### DEC-021 (proposed, implementation-level)

> A persistence failure is surfaced to the import IPC **in-band** (a secret-free `Err` from
> `run_blocking_import`), not via a durable failure-marker. The marker remains best-effort
> defense-in-depth for the on-demand snapshot. This reaffirms S-4; it does not diverge from the BA
> architecture, so it is recorded here rather than routed as `feedback_to_ba[]`. BA may ratify into the
> global decision log if desired.

### Err vs Ok(Unknown-snapshot) — decision

Chosen: **`Err(secret-free message)`**. It is consistent with the established TASK-020 timeout path
(`IMPORT_TIMEOUT_MSG` already returns via `?`), is the smallest surface, and is impossible to mistake
for healthy. The renderer already handles a rejected `import_langfuse_now` (timeout) as non-healthy, so
no renderer change is needed. *Alternative considered:* return `Ok(snapshot{ health:"unknown" })` — a
structured non-healthy DTO. Rejected: larger surface, requires fabricating snapshot fields, and diverges
from the timeout precedent for no benefit. (If a future renderer contract ever needs a structured DTO
on import failure, that is a renderer-led change, not this hotfix.)

## 3. Boundary — explicitly out of scope

- **On-demand `get_langfuse_source_health` between imports.** It reads `latest_run` and, after a
  both-writes-fail import, will reflect the last *durably persisted* run. This is acceptable and
  **inherently bounded**: if the DB is unwritable, no durable "unknown" can exist. The path reports the
  last-good persisted state with its own `last_import_at` label — it is not a false-healthy of the
  current action, which now surfaces honestly in-band. Reconciling the persistent snapshot under an
  unwritable DB is impossible without retry/reconciliation, which is TASK-006 territory and explicitly
  excluded.
- No runtime reconciliation, retry, or backoff. No new health state. No schema migration. No renderer
  change.

## 4. Component-boundary & scope check (split test)

- **Single component.** All edits live in the `langfuse` importer module + its one IPC command. No
  cross-component boundary from `03_architecture_plan.md` is crossed.
- **Backend-only.** Rust core; renderer, CSP, capabilities, REST contract untouched.
- **No schema/contract change.** Reuses `Unknown`; no `ALTER`/`DROP`; no DTO field change.
- **Atomic, one concern.** "Make persist-failure surface to the IPC marker-independently." Touches 3
  files (importer.rs, mod.rs, tests.rs; lib.rs verify-only) but one cohesive change unit.

→ **No split required.** A single backend developer task. No design-level concern needs BA escalation —
this reaffirms an existing BA/spec guarantee (S-4) rather than altering architecture.

## 5. NFR / invariants

- **Absence-≠-zero:** strengthened — the false-healthy hole is closed at the IPC.
- **SEC-003 (no secret exposure):** preserved — fixed sentinel string; regression asserts secret-free.
- **Determinism of health taxonomy:** preserved — no new state; `Unknown` reused.
- **Atomicity (S-3):** unchanged — the run transaction and best-effort marker are untouched in behavior.

## 6. Handoff

- **Next role:** Backend Developer (Rust core).
- **Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing`, base `main` (`dd5d3b9`).
- **Deliverables for the developer:** `tasks.md` (this change) — 4 steps + gate. Spec contract in
  `specs/langfuse-importer/spec.md` (MODIFIED requirement). Verify-only on `lib.rs`.
- **Gate after implementation:** SW-4 code review (regression must exercise the both-writes-fail path),
  then SW-5/SW-6 as per the TASK-020 pattern. `openspec validate --strict` must pass.
