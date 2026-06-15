# Verification & manual macOS smoke — TASK-024 CSV export save-dialog hang fix

- **Role:** Backend Developer (SW-2)
- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **Branch:** `fix/task-024-csv-export-save-dialog-hang` · **base:** `origin/main`
- **Date:** 2026-06-15

## 1. What changed (product runtime)

`src-tauri/src/lib.rs` — **`export_report_csv` only**:

- Signature `fn` → **`async fn`**; `state: State<AppState>` → `state: State<'_, AppState>` (required for
  an async Tauri command holding a borrowed `State`).
- The save dialog is now acquired **off the UI main thread** via
  `tauri::async_runtime::spawn_blocking(move || …blocking_save_file())` awaited from the async command.
  Root cause (confirmed against the plugin source): `tauri_plugin_dialog`'s `blocking_save_file()`
  expands (via the `blocking_fn!` macro) to a `sync_channel(0)` whose completion callback is dispatched
  onto the **main run loop**; called from a synchronous command (which runs on the main thread), the
  parked main thread can never deliver that callback → deadlock (panel renders, IPC promise never
  resolves, app beachballs). Running the blocking picker on the blocking pool keeps the main run loop
  free, so the dialog resolves and the command **always returns**.
- **Lock-after-await preserved:** the `db_conn` `MutexGuard` is acquired only *after* the `.await`, then
  used and dropped synchronously — never held across an `.await` (clippy `await_holding_lock` clean).
- Deterministic outcomes unchanged and now reliably reached: cancel/empty → `Ok(None)`; non-local path
  → `Err`; `validate_csv_destination` failure (non-`.csv` / directory) → `Err`; `fs::write` failure →
  `Err`; success → `Ok(Some(n))`.

**Untouched (TASK-023 security contract, asserted not assumed):** `export_csv_repo`, `csv_escape`,
`csv_formula_neutralized`, and the 7-column set
`date, project, start_time, end_time, duration_minutes, note, total_duration_hours`. No new
column / schema / migration / dependency / capability / network. No frontend change (the renderer
already surfaces `Err` via `alertError` and success via `Exported N entries.`, and keeps no loading
flag, so a promptly-resolving command removes the perceived freeze).

## 2. Automated verification (run by the developer)

| Check | Command | Result |
| --- | --- | --- |
| Backend unit tests | `cargo test --lib` (in `src-tauri/`) | **67 passed, 0 failed** |
| TASK-023 adversarial gate | (within above) `summaries_and_csv_filtering_escape`, `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` | **green** |
| New TASK-024 tests | (within above) `validate_csv_destination_requires_csv_file_and_rejects_directories`, `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` | **green** |
| Lints | `cargo clippy --all-targets` | **no new warnings**; **no `await_holding_lock`**; the async command compiles (proves `State<'_>` + `Send` future) |
| Spec | `openspec validate task-024-csv-export-save-dialog-hang-fix --strict` | **valid** |
| All targets compile | `cargo clippy --all-targets` compiled lib + bin + tests | **clean compile** |

New unit tests added for the non-dialog logic the command relies on to always resolve (destination
validation, write-failure surfacing). The GUI deadlock itself is **not** unit-testable headlessly — it
is validated by the manual macOS smoke matrix below (the acceptance gate, per `arch-review.md` §6 and
SW-3/QA).

## 3. Manual macOS smoke matrix (acceptance gate — to run interactively)

Build & run: `npm run tauri:dev` (or the project run path). Navigate **Reports → Export CSV…**.

| # | Case | Steps | Expected (with fix) |
| --- | --- | --- | --- |
| S1 | **Success** | Pick a writable `.csv` location and confirm | File written; `Exported N entries.` alert; **app stays responsive** (no beachball) |
| S2 | **Cancel** | Open the save dialog, then Cancel / dismiss | No file, no error; **app returns to a responsive state**, no endless loading |
| S3 | **Write failure** | Choose a read-only / unwritable target (e.g. a protected dir), or otherwise cause `fs::write` to fail | **User-visible error** via the existing error alert; app stays responsive |
| S4 | **Re-entry** | After S1/S2, immediately export again | Dialog opens and resolves again normally (no stuck state from a prior run) |

> Regression focus vs. the pre-fix build: previously S1/S2 froze the app immediately after the panel
> opened (IPC promise never resolved). With the fix the command resolves in every case.

## 4. Manual smoke status (honest)

- **Automated checks above were executed and pass.** All targets compile (the `.dmg`/desktop binary
  builds via the same all-targets compile path used by clippy).
- **The interactive GUI smoke (S1–S4) was NOT executed by this agent.** This backend role runs
  non-interactively (headless) and cannot drive the macOS `NSSavePanel` (click Save / Cancel / pick a
  read-only target). Per `arch-review.md` §6 ("the GUI deadlock itself is validated by manual macOS
  smoke, not a headless unit test") and §8 (SW-3/QA centers on the manual macOS smoke matrix), the
  interactive run is the **SW-3 / QA acceptance gate**. The matrix above is the reproducible script for
  that gate (and for Janne's local smoke that originally surfaced the blocker).

## 5. Pre-existing non-blockers (NOT introduced by TASK-024)

These exist on `origin/main` (the branch has **zero** diff vs. `origin/main` in `src/**`, `tests/**`,
and frontend config) and are **out of scope** for this hang fix:

- `cargo clippy` warnings in `src/langfuse/importer.rs` (`map_or`, two `manual_flatten`) and
  `src/lib.rs:249` (`db_path` `io_other_error`). None are in the changed `export_report_csv` or the new
  tests.
- Frontend `npm run test:frontend`: **36 pass / 2 fail**, both failures in
  `tests/pi-observe.security.test.mjs` (pi-observe telemetry host-blocking assertions) — unrelated to
  CSV export and untouched by this change.
