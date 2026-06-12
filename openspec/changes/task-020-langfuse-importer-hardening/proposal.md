# TASK-020 — Langfuse importer hardening + L2 release hygiene

## Why

TASK-019 (PR #11) shipped the first AI-evidence runtime slice — the read-only local Docker Langfuse
importer in the Rust core — and merged to `main` (`64d5f9f`) with a clean post-merge code-to-spec
verification. SW-4 review passed with **eight non-blocking suggestions** (S-1…S-8) and one
release-hygiene carryover (A1). TASK-020 closes the subset of those carryovers that affect
**correctness durability, observability of failure, and reproducible/auditable builds** — the
hardening a freshly-landed L2 evidence path needs before it accrues consumers (TASK-006 runtime
reconciliation, TASK-009 review UI).

This is a **hardening + release-hygiene** change, not new capability. It adds no new source, no
classifier, no runtime observer, and no review UI. It does not reopen DEC-019 (implementation path)
or DEC-020/DEC-022 (local Docker default, Cloud explicit override). It strengthens the guarantees the
importer already makes — most importantly, it removes two silent-failure paths that could let a
partial or failed persistence masquerade as a healthy zero, which would violate the
**absence-≠-zero** invariant the importer is built to protect.

## What Changes

- **Commit `src-tauri/Cargo.lock` (A1).** The lock file is currently untracked and not gitignored.
  An L2 evidence binary must build reproducibly and be SBOM-auditable; an uncommitted lock leaves the
  exact dependency closure (incl. the new `reqwest`/`rustls` TLS stack) unpinned. Track it; keep it
  out of `.gitignore` (it already is). No dependency version changes — pin the closure that TASK-019
  already validated.
- **Wrap importer persistence in one transaction (S-3).** `persist_run` issues three independent
  write groups (raw-trace upserts, evidence upserts, run-record insert) with no surrounding
  transaction. A mid-run failure leaves a partial state. Wrap the run's writes in a single SQLite
  transaction so an import run persists atomically (all-or-nothing) per run.
- **Surface persistence errors instead of swallowing them (S-4).** All three DB calls in
  `persist_run` use `let _ = …`, discarding `rusqlite::Result`. A failed write is invisible to the
  caller, the run record, and the health snapshot — which can present a misleadingly clean state.
  Propagate the error, or at minimum record it into the run's `warnings` and resolve a non-healthy
  health signal, so a persistence failure is observable and never reads as healthy/zero.
- **Normalize importer-emitted timestamps to UTC RFC3339 (S-5).** `now()` returns a **local-time**
  string (`"%Y-%m-%d %H:%M:%S"`) used for `started_at`/`finished_at`/`imported_at`, while trace
  timestamps, window bounds, and `cursor_ts` are UTC RFC3339. The mixed format in one DTO surprises
  consumers and is non-comparable across zones. Emit all importer-generated timestamps as
  `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` so every importer timestamp is uniformly
  UTC RFC3339.
- **Bound the `import_langfuse_now` IPC (S-6).** The command does `thread::spawn(…).join()` with no
  timeout; if the worker hangs past the `reqwest` 15 s/5 s ceilings (dependency bug, deadlock), the
  IPC blocks the UI indefinitely. Add a channel + `recv_timeout` bound so the command always returns
  within a defined ceiling and reports a timeout as a non-healthy state, never an indefinite hang.
- **(Optional) Mocked/local importer smoke test.** Extend the existing mock-based suite with an
  end-to-end persist→snapshot smoke (atomic-write + error-surface + timeout path). A real local
  Docker Langfuse smoke is included **only if** a stack is readily available in-session; absence of
  one is not a blocker and must not gate the change.

## Impact

- **Affected specs:** `langfuse-importer` (MODIFIED via ADDED hardening requirements — atomic
  persistence, surfaced persistence failure, uniform UTC RFC3339 timestamps, bounded import IPC). No
  requirement is removed; the absence-≠-zero and no-secret-exposure contracts are strengthened, not
  changed.
- **Affected code (Rust core only):** `src-tauri/src/langfuse/importer.rs` (`persist_run`, `now()`),
  `src-tauri/src/langfuse/store.rs` (transaction-capable persistence entry point + error-returning
  signatures), `src-tauri/src/lib.rs` (`import_langfuse_now` bounded join), `src-tauri/src/langfuse/
  tests.rs` (smoke + regression). Repo root: `src-tauri/Cargo.lock` (newly tracked).
- **Unchanged surfaces (explicit):** `tauri.conf.json` CSP, `capabilities/`, the REST contract, the
  10-state health taxonomy semantics, the schema columns (no `ALTER`/`DROP`; timestamp format is a
  string-value change, not a column change), and the renderer's off-network posture. No new
  dependency is added; `Cargo.toml` versions are unchanged.
- **Out of scope (clean boundaries, not this task):** TASK-006 AI runtime observer / reconciliation
  (DEC-017 — no duplicate ledger, no new pi/Claude adapter), TASK-008 classifier, TASK-009 review/
  approval UI, TASK-010 AI-evidence CSV export, TASK-005 macOS capture. S-1 (dead `_config` param),
  S-7 (`degradedHealth` banner set — deferred to TASK-009), and S-8 (per-env snapshot selection) are
  **not** in scope; S-2 (lexicographic compare of Langfuse-emitted trace timestamps) is adjacent to
  S-5 but distinct (it concerns externally-supplied values) and is noted, not required (see
  `design.md` §6). The pre-existing `csv_export_*` adversarial failure is a separate time-tracker-core
  defect, not importer scope.
- **Guardrails preserved (DEC-020 / TASK-019 §5):** local Docker default + loopback `127.0.0.1`,
  Cloud explicit override as the sole off-host egress, environment-first mapping, renderer
  off-network, no credential/secret exposure in logs/evidence/exports, no raw macOS-activity egress,
  and **absence is never zero** — the latter materially reinforced by S-3/S-4.
- **Branch:** `feat/task-020-langfuse-importer-hardening`, base `main` (TASK-019 PR #11 already
  merged at `64d5f9f`; no merge-order dependency).
