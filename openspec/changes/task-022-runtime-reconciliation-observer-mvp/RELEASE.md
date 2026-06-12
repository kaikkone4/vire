# Release — TASK-022 AI Runtime Reconciliation Observer (MVP)

- **Change:** `task-022-runtime-reconciliation-observer-mvp`
- **PR:** #14 · `feat/task-022-runtime-reconciliation-observer-mvp` → `main`
- **Tag:** `task-022/v0.1.0`
- **Tier:** L2
- **Release date:** 2026-06-12
- **BA parent task:** TASK-006 (runtime reconciliation health)
- **Branch head:** `15429e9`
- **Gate chain:** SW-3 PASS (`a8fc11e`) · SW-4 PASS (`b9e6cef`) · SW-5 PASS (`be8029b`) · SW-6 Docs PASS (`15429e9`)

---

## Summary

Introduces the AI Runtime Reconciliation Observer, a new Rust-core module that reads the local
`pi-observe` coarse session log and reconciles runtime sessions against existing Langfuse import
evidence. The observer surfaces a health posture — matched, unmatched, or unknown — on the
Settings panel without adding any network calls, process scanning, or cost/time authority.

**What changed:**

- New module `src-tauri/src/runtime_observer/` (5 submodules: `config`, `model`, `ingest`,
  `reconcile`, `store`; 23 unit tests).
- New table `ai_runtime_sessions` (11 columns; no cost/token/duration column by design).
- Additive nullable column `session_id` on `langfuse_ai_evidence` (idempotent migration via
  `add_column_if_absent`).
- New read-only Tauri IPC command `get_runtime_reconciliation` returning
  `RuntimeReconciliationSnapshot` (counts + posture only; no raw session content).
- New `RuntimeReconciliation` TS type and `runtimeLine()` status display in `src/main.ts`.

**What did not change:** Cargo.toml/Cargo.lock, `tauri.conf.json`, capability files, CSP, and
all existing modules. No cost or usage data paths are touched.

---

## 1. Deployment size

**MINOR**

No breaking changes to any existing API, IPC surface, DB schema, or renderer behaviour.
All additions are purely additive:

- The new table `ai_runtime_sessions` is created on first use; no existing table is altered.
- `langfuse_ai_evidence.session_id` is added via `add_column_if_absent`; the column is nullable
  and its absence in pre-TASK-022 installs is handled gracefully by the migration guard.
- The new IPC command is read-only; no existing commands are modified.
- The renderer change adds a single status line to the Settings panel; no existing UI element
  is altered.

---

## 2. Rollback strategy

**Partial-automated**

The desktop binary is the sole deployment artifact. Rollback = install the previous release binary.

Post-rollback DB state: `ai_runtime_sessions` and `langfuse_ai_evidence.session_id` remain in
the user's local SQLite database but are never queried by the previous binary. Both additions are
additive and nullable; no existing query path fails in their presence. Re-applying TASK-022
forward after a rollback is safe because the migration is idempotent.

**L2 requirement:** Rollback procedure must be tested in a staging environment before the
production release. Test checklist:

1. Install TASK-022 binary, confirm `ai_runtime_sessions` table and `session_id` column are
   created.
2. Roll back to previous binary; confirm app starts and all pre-TASK-022 features operate
   normally (Langfuse import, time tracking, settings panel).
3. Re-install TASK-022 binary; confirm `add_column_if_absent` is idempotent (no crash, no
   duplicate-column error surfaced to the user).

No automated rollback mechanism is available for a desktop app distributed as a binary installer;
"automated" applies only to steps 1–3 being scriptable in a staging environment.

---

## 3. Component compatibility matrix

### Rust core (pinned via `src-tauri/Cargo.lock`)

| Crate | Declared (Cargo.toml) | Pinned (Cargo.lock) | Notes |
|---|---|---|---|
| `tauri` | `^2.2` | `2.11.2` | Tauri desktop runtime |
| `tauri-plugin-dialog` | `^2.2` | — | File-picker dialog |
| `tauri-build` | `^2.0` (build dep) | — | Build helper |
| `rusqlite` | `^0.32` (bundled) | `0.32.1` | Embedded SQLite; bundled feature |
| `serde` | `^1` | `1.0.228` | Serialization |
| `serde_json` | `^1` | `1.0.150` | JSON parse / serialize |
| `chrono` | `^0.4` | `0.4.45` | RFC3339 timestamps |
| `uuid` | `^1` | `1.23.3` | Session key generation |
| `thiserror` | `^2` | `2.0.18` | Error types |
| `reqwest` | `^0.12` (rustls-tls) | `0.12.28` | Langfuse HTTP importer (pre-existing) |
| `url` | `^2` | `2.5.8` | URL validation (pre-existing) |
| `tempfile` | `^3` (dev dep) | `3.27.0` | Test temp dirs |

### Frontend (npm, locked via `package-lock.json`)

| Package | Declared | Notes |
|---|---|---|
| `@tauri-apps/api` | `^2.2.0` | IPC bridge for `invoke()` calls |
| `@tauri-apps/plugin-dialog` | `^2.2.0` | File-picker (pre-existing) |
| `typescript` | `^5.7.2` | Strict mode; `RuntimeReconciliation` type |
| `vite` | `^6.0.7` | Frontend bundler |

### External runtime inputs

| Component | Min supported version | Notes |
|---|---|---|
| `pi-observe` event log | Any version emitting NDJSON with `event` field | Format: one JSON object per line; unknown keys silently dropped by `RawEvent` allowlist. Absent or malformed log is tolerated (no crash, `log_present=false`). |
| Langfuse importer (pre-existing) | Existing `langfuse_ai_evidence` table | `session_id` column added additively; existing evidence rows get `session_id = NULL` until re-imported. No existing query is broken. |

### Platform

| OS | Minimum version |
|---|---|
| macOS | 10.15 (Tauri 2.x requirement) |
| Windows | 10 (Tauri 2.x requirement) |
| Linux | Ubuntu 18.04 / glibc 2.27 (Tauri 2.x requirement) |

---

## Advisories rollup (non-blocking — no action required before release)

All gate findings below are non-blocking. None require developer action before this release.
Logged here for tracking.

### From SW-3 QA

| ID | Finding | Disposition |
|---|---|---|
| NB-1 | `WrongEnv`, `SchemaChanged`, `Delayed` health states not individually parametrized in `non_healthy_import_does_not_license_a_gap`; covered correctly by the `_` catch-all | Add explicit parametrization to test loop in a future follow-up |

### From SW-4 Code Review

| ID | Finding | Disposition |
|---|---|---|
| CR-1 | `nonempty` helper duplicated in `model.rs`, `ingest.rs`, `reconcile.rs` | Extract to private `util` submodule when the module grows further |
| CR-2 | `add_column_if_absent` matches on SQLite error message string; brittle across versions in theory | Replace with `PRAGMA table_info` pre-check in a future refactor; acceptable for MVP |
| CR-3 | `coarse_status` uses `s.contains("ok")` — could incidentally match other values | Prefer exact `s == "ok"` match long-term; risk negligible (controlled emitter) |
| CR-4 | `window_covers` returns `true` for sessions with unparseable timestamps → conservative `observed_no_trace` under healthy import | Add per-session `reconciliation_unknown` short-circuit if noise emerges in practice |

### From SW-5 Security

| ID | Finding | Disposition |
|---|---|---|
| A1 | `add_column_if_absent` builds `ALTER TABLE` via string interpolation | Not exploitable (literal callsite only); keep function private; do not allow external identifiers |
| A2 | `runtimeLine()` interpolates counts without `esc()` | Safe today (only `u32` fields rendered); apply `esc()` before surfacing any string field from the snapshot |
| A3 | Pre-existing GTK3/Tauri dependency advisories (max CVSS 6.9); not introduced by TASK-022 | Track a Tauri / gtk-rs bump in a dedicated dependency-hardening task |
