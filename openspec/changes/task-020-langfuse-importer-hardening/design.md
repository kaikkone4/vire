# Design — TASK-020 Langfuse importer hardening + L2 release hygiene

- **Author:** SW Architect (SW-1) scaffold for the developer role.
- **Change:** `task-020-langfuse-importer-hardening`
- **Decisions honored:** DEC-017 (Langfuse-first, no duplicate ledger), DEC-019 (Rust-core importer),
  DEC-020 / DEC-022 (local Docker default; Cloud explicit override), SEC-002 (network boundary),
  SEC-003 (credentials).
- **Source of truth:** TASK-019 `review.md` §9 (S-3…S-6) + carryover A1; TASK-019 `design.md` §5–§7
  (persistence + privacy posture); current `main` runtime
  (`src-tauri/src/langfuse/{importer,store,mod}.rs`, `src-tauri/src/lib.rs`).

## 1. Component boundary — no boundary crossed

Every change lives inside the existing **Langfuse importer** component (`03_architecture_plan.md` §4,
`04_technical_plan.md` §4) plus one repo-level build-hygiene file (`Cargo.lock`). It touches no other
component — not the macOS capture adapter, AI runtime observer, classifier, review UI, or CSV
exporter. The SQLite store is the same importer-owned substrate from TASK-019; no new table, no
column change. → single cohesive change, **not** a split (see `arch-review.md` §3).

```
 unchanged                Rust core (server-side) — hardened here          unchanged
 ─────────                ──────────────────────────────────────          ─────────
 webview CSP              import_langfuse_now (IPC)                         REST contract
 capabilities/      ──▶   └─ bounded join (S-6) ──┐                         10-state taxonomy
 renderer off-net                                 ▼                         schema columns
                          run_import → persist_run (S-3 txn, S-4 errors)
                          now() → UTC RFC3339 (S-5)
                                          ▼
                          SQLite importer tables (same schema; values normalized)
 repo root: Cargo.lock now tracked (A1) ── reproducible/SBOM-auditable build
```

## 2. A1 — commit `Cargo.lock` (build reproducibility / SBOM)

- `src-tauri/Cargo.lock` exists (~125 KB) but is **untracked** and is **not** in any `.gitignore`
  (verified: root `.gitignore` lists `src-tauri/target/`, not the lock). Vire ships an application
  binary (a Tauri app), and the L2 evidence path now links a TLS/HTTP stack (`reqwest` + `rustls`);
  the canonical Rust guidance is to commit the lock for binaries so the dependency closure is pinned
  and auditable.
- **Action:** `git add src-tauri/Cargo.lock` and commit. **No** `cargo update`, no version bumps —
  pin exactly the closure TASK-019 built and validated. The lock becomes the SBOM input the RELEASE
  hygiene path (SW-6) and any future `cargo audit`/`cargo deny` consume.
- This is a repo-hygiene fact, **not** a `langfuse-importer` capability requirement — it gets a
  tasks.md entry and a release note, **no** spec delta.

## 3. S-3 — atomic persistence (one transaction per run)

- **Now:** `persist_run` (`importer.rs:384–420`) runs N raw-trace upserts + M evidence upserts + 1
  run-record insert as independent statements. A failure midway leaves a partial state (e.g. evidence
  rows written, run record missing — so `latest_run` won't see them, or raw rows without their
  evidence).
- **Design:** wrap the whole run's writes in a single SQLite transaction. Preferred shape — a
  `store`-level entry point that owns the transaction so SQL stays in `store.rs`:
  `store::persist_import_run(conn, &summary, raw, window, &run_id, &stamp) -> rusqlite::Result<()>`
  using `let tx = conn.unchecked_transaction()?; … tx.commit()?;` (rusqlite drops→rollback on the
  error path automatically). The importer calls it and handles the `Result` (see §4). Keep the three
  existing `upsert_*`/`insert_*` helpers; just call them on the `&tx`.
- **Atomicity unit = one import run** (one `run_id`). Cross-run/cross-environment atomicity is **not**
  a goal — each environment's run remains its own unit, matching the per-env cursor model.
- WAL is already the effective mode for the bundled SQLite; one short transaction per manual import is
  cheap and removes the partial-write window.

## 4. S-4 — surface persistence failures (never swallow, never read healthy)

- **Now:** all three writes use `let _ = …`, discarding `rusqlite::Result`. The probe gate already
  prevents evidence rows when the **stack** is down, but a **DB** failure (locked, disk-full,
  constraint) is silently lost; the snapshot can then show a stale/clean state while the current run
  actually failed to persist — a latent absence-≠-zero violation (a failed write must not look like a
  healthy zero).
- **Design (two acceptable strengths; pick the stronger that fits cleanly):**
  1. **Propagate** — `persist_run`/`persist_import_run` returns `rusqlite::Result<()>`; `run_import`
     bubbles it into the `run_blocking_import` error string, which `import_langfuse_now` already maps
     to a secret-free IPC error. The transaction (§3) guarantees no partial commit on that path.
  2. **Record + degrade (minimum)** — on a write error, push a secret-free message into
     `summary.warnings` and resolve the persisted run `status` to a non-healthy state (`unknown` is
     the correct "could not establish currency/completeness" signal) so the failure is visible in the
     run record and the health snapshot.
- **Constraint:** the error text must stay **secret-free** (SEC-003) — `rusqlite` errors don't carry
  credentials, but the message that reaches logs/IPC must not interpolate config/credential material.
  Prefer a fixed message + error kind over echoing arbitrary driver strings.
- **Invariant reinforced:** a persistence failure resolves to a health signal (or a surfaced error),
  **never** a silent success and never a numeric zero.

## 5. S-5 — uniform UTC RFC3339 for importer-emitted timestamps

- **Now:** `now()` → `Local::now().naive_local().format("%Y-%m-%d %H:%M:%S")`. It feeds
  `started_at`, `finished_at`, and `imported_at`. Meanwhile `cursor_ts`/`latest_trace_ts`, window
  bounds, and `ai_start_ts`/`ai_end_ts` are UTC RFC3339. One DTO (`SourceHealthSnapshot`,
  `ImportRunRecord`) therefore mixes `2026-06-12 14:30:00` (local) with `…T…Z` (UTC).
- **Design:** `now()` → `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` (same call the module
  already uses in `recent_window`, `mod.rs:26–27`). Every importer-generated timestamp becomes
  uniform UTC RFC3339. No schema change — these columns are already `TEXT`.
- **Ordering-key migration note (must be honored):** `finished_at` is the sort key for
  `latest_run`/`latest_run_for_env` (`ORDER BY finished_at DESC`). Legacy rows hold space-separated
  local strings; new rows hold `…T…Z`. Lexicographically `'T'` (0x54) > `' '` (0x20), so **every new
  RFC3339 row sorts after every legacy row** — "latest run" stays correct across the format
  transition with **no data migration**. (A legacy row can never out-sort a new row, which is the
  behavior we want.) Document this; do not write a backfill.
- **S-2 is adjacent but out of scope** (see §6).

## 6. S-6 — bounded `import_langfuse_now` IPC

- **Now:** `lib.rs:182–191` spawns the blocking import on an OS thread and `.join()`s with no
  timeout. The `reqwest` client has 15 s request / 5 s connect ceilings, but a hang **outside** an
  HTTP call (dependency deadlock, SQLite lock contention) is unbounded → UI blocks forever.
- **Design:** keep the dedicated-thread model (the blocking client must stay off the Tauri async
  runtime), but bound the wait: worker sends its `Result` over an `mpsc::channel`; the command does
  `rx.recv_timeout(Duration::from_secs(IMPORT_TIMEOUT_SECS))`. On timeout, return a secret-free
  non-healthy error (the UI surfaces it like any other failed import); the orphaned worker is bounded
  by the reqwest ceilings and writes atomically (§3) if it later finishes. Choose
  `IMPORT_TIMEOUT_SECS` comfortably above the reqwest ceilings (≈30 s) so a normal slow import is not
  cut off, but an indefinite hang is.
- **No new health-taxonomy state.** A timeout maps to an existing non-healthy signal
  (`auth_or_network_error` for a transport-class stall, or a surfaced IPC error). Do **not** add an
  11th state — the BA §7 taxonomy is fixed.

### Adjacent, NOT in this task (noted for honesty, do not implement)

- **S-2** — `import_environment` compares Langfuse-emitted trace timestamps lexicographically
  (`ts.as_str() < cur.as_str()` for `delayed`, and the `max_ts`/cursor `max`). That is correct while
  Langfuse emits `…Z` UTC (empirically validated, `design.md` §4) but would misorder `+00:00`-offset
  strings. It concerns **externally-supplied** values (independent of S-5's importer-emitted ones) and
  is **not** in TASK-020's scope. If the developer judges it a near-zero-cost win while in this file,
  the fix is to reuse the existing `parse_ts()` helper (`importer.rs:368`) — but it is optional and
  must not expand the change. Flag, don't force.

## 7. Test direction (handoff to SW-3)

Extend `langfuse::tests` (mock-based, no network/credentials) with:
- **Atomicity (S-3):** a forced mid-run write failure leaves **no** partial run persisted (run record
  absent ⇒ raw/evidence rows for that `run_id` absent too).
- **Error surfacing (S-4):** a persistence failure is observable — it propagates as an error **or**
  appears in the run `warnings` with a non-healthy `status`; it is **never** recorded as `healthy`
  and never contributes a zero total.
- **UTC RFC3339 (S-5):** `started_at`/`finished_at`/`imported_at` parse as RFC3339 UTC; assert the
  legacy-vs-new ordering note (a `…T…Z` row sorts as latest over a space-format row).
- **Bounded IPC (S-6):** a worker that exceeds the timeout yields a bounded, secret-free error rather
  than blocking; a normal run returns its snapshot well within the ceiling.
- **Regression:** the existing 25 importer tests stay green; absence-≠-zero and credential-redaction
  assertions unchanged.
- **Optional real smoke:** only if a local Docker Langfuse is readily up in-session — a single
  read-only `run_blocking_import` against loopback, asserting a health state resolves and **no**
  secret/zero leaks. Absence of a stack is not a failure; record it as skipped.

## 8. Out-of-scope boundaries (do not build here)

- TASK-006 AI runtime observer / reconciliation (DEC-017: no duplicate cost/time ledger, no new
  pi/Claude adapter) — consumes this importer; built later.
- TASK-008 classifier, TASK-009 review/approval UI (incl. S-7 `degradedHealth` banner tuning),
  TASK-010 AI-evidence CSV export, TASK-005 macOS capture adapter.
- S-1 (dead `_config` param) and S-8 (per-env snapshot selection) — non-blocking polish, deferred.
- The pre-existing `csv_export_neutralizes_formula_like_…` failure — time-tracker-core `csv_escape`
  defect, a separate follow-up, not importer scope.
