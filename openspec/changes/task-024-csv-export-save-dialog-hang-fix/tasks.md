# Tasks — TASK-024 CSV export save-dialog hang fix

Single backend / Tauri-integration slice (the Rust IPC command that wires the save dialog to the CSV
writer). Sub-tasks are an implementation sequence within one OpenSpec change, **not** a component
split. All product-runtime edits land in `src-tauri/src/lib.rs`; an optional minimal touch in
`src/main.ts`. The acceptance gate is a **manual macOS smoke** (a GUI/threading deadlock is not caught
by a headless unit test). Recommended order:

## 1. Reproduce & localize (evidence before fix)

- [x] Build & run the app (`npm run tauri dev`, or the project run path), go **Reports → Export CSV…**,
      and confirm the freeze after the save dialog opens. *(Mechanism confirmed statically against the
      plugin source — see below; interactive reproduction is the SW-3/QA manual smoke, captured in
      `smoke.md`.)*
- [x] Confirm the mechanism: `export_report_csv` (`src-tauri/src/lib.rs:168`) is a **sync** command
      (main thread) calling `blocking_save_file()` (line 170). `blocking_save_file()` expands (plugin
      `blocking_fn!` macro) to a `sync_channel(0)` fed by a callback dispatched onto the **main run
      loop**; a main-thread caller parks on `recv()` and the callback can never be delivered → deadlock.
      No temporary instrumentation left in the tree.

## 2. Move the save dialog off the main thread

- [x] Make `export_report_csv` an **`async`** command (`state: State<'_, AppState>`).
- [x] Acquire the destination without blocking the main thread — used the **acceptable** off-main-thread
      path: `tauri::async_runtime::spawn_blocking(move || …blocking_save_file())` awaited from the async
      command (robust, no `blocking_send`-in-runtime panic risk). Kept `add_filter("CSV", &["csv"])` and
      `set_file_name("vire-report.csv")`.
- [x] Map a cancelled / empty selection to `Ok(None)`; kept the `into_path()` guard → `Err` on a
      non-local path; kept `validate_csv_destination(&path)` (`.csv` extension, not a directory).

## 3. Keep the write path correct and the lock safe

- [x] **After** awaiting the dialog, lock the DB (`db_conn`), call `export_csv_repo(&db, …, &path)`
      (unchanged), and return `Ok(Some(n))`. The `MutexGuard` is acquired after the `.await` and dropped
      synchronously — never held across it (clippy `await_holding_lock` clean).
- [x] A write failure (`fs::write`) surfaces as `Err(message)` to the renderer (already the case in
      `export_csv_repo`); confirmed not swallowed — new test `export_csv_repo_surfaces_write_failure_as_error_not_a_hang`.

## 4. Preserve TASK-023 security (guardrail, assert don't assume)

- [x] `export_csv_repo`, `csv_escape`, `csv_formula_neutralized` are **unchanged** (verified by diff and
      the green TASK-023 adversarial gate). Export columns remain exactly `date, project, start_time,
      end_time, duration_minutes, note, total_duration_hours`. No raw activity/window log, AI
      prompt/response, command body, or secret-shaped field added. No new network/egress, no new
      dependency, no capability change.

## 5. Frontend (optional, minimal — not required for correctness)

- [ ] (Not done — deliberately skipped) `Export canceled.` status on `Ok(None)`. The existing
      success/error path satisfies the required contract; left out to keep the blast radius minimal.
- [ ] (Not done — deliberately skipped) Disable the Export button while a dialog is in flight.
- [x] Confirmed errors still surface via `alertError` and success via the existing `Exported N entries.`
      alert (no persistent loading flag exists, so no spinner to clear). **No frontend change made.**

## 6. Verification

- [~] **Manual macOS smoke (acceptance gate):** matrix S1–S4 (success / cancel / write-failure /
      re-entry) **scripted in `smoke.md`**. Interactive GUI run is the **SW-3/QA gate** — this headless
      backend role cannot drive `NSSavePanel`; status captured honestly in `smoke.md` §4.
- [x] `cargo test` (full backend suite) green — **67 passed, 0 failed**, incl. the TASK-023 adversarial
      gate `summaries_and_csv_filtering_escape` and `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes`.
- [x] `cargo clippy --all-targets` — **no new warnings**, **no `await_holding_lock`** / unused-result on
      the async command (pre-existing langfuse/`db_path` warnings noted in `smoke.md` §5).
- [x] `openspec validate task-024-csv-export-save-dialog-hang-fix --strict` — **valid**.

## Out of scope (do not build here)

- Review/approval UI (TASK-009); summary/approval model and export records (TASK-010); macOS capture
  (TASK-005); classification (TASK-008); any new export column or detailed/raw export mode; any new CSV
  or serialization dependency; moving the picker to the JS dialog plugin; any change to
  `export_csv_repo`/`csv_escape`/`csv_formula_neutralized` behavior.
