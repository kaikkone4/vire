# TASK-024 — CSV export save-dialog hang (local Mac MVP smoke blocker)

## Why

Local Mac MVP smoke (manual flow) reached **Reports → Export CSV…**, the macOS save dialog opened
asking where to save, and after that the app **froze / showed endless loading** and never proceeded.
This is an **MVP smoke blocker** for FR-016 (local CSV export of reviewed summaries) and contradicts
NFR-010 (Vire's overhead must stay low enough not to interfere with the workflow — a frozen app is the
opposite). It immediately follows TASK-023, which hardened the CSV writer and passed post-merge.

Root cause (diagnosed from the code; to be confirmed by reproduction, see `tasks.md` step 1):

- The renderer's Export handler (`src/main.ts:40`) calls `invoke('export_report_csv', …)` and `await`s
  it; there is **no separate frontend save dialog** — the picker lives inside the Rust command.
- `export_report_csv` (`src-tauri/src/lib.rs:168`) is a **synchronous** `#[tauri::command]`. In Tauri
  v2, a synchronous command runs on the **main thread**.
- At line 170 it calls `tauri_plugin_dialog`'s **`blocking_save_file()`**. The `blocking_*` dialog
  variants block the calling thread on a channel that is fed by a completion callback dispatched onto
  the **main run loop**. Called *from* the main thread, that callback can never be delivered while the
  main thread is parked on the channel → **deadlock**. The native panel can still render (the OS
  presents `NSSavePanel`), which is exactly why the user *sees* the dialog before everything freezes;
  the selected/cancelled result is never returned. The `invoke` promise the renderer `await`s never
  resolves, so the webview stays "loading" forever and the app beachballs.

This is an **integration/IPC threading defect in the export command wiring**, not a defect in the CSV
writer. `export_csv_repo`, `csv_escape`, and `csv_formula_neutralized` (TASK-023 hardening) are correct
and are **not** touched by this change. No BA decision is reopened; no export field is added.

## What Changes

- **Take the save dialog off the main thread so the command resolves deterministically.** Convert
  `export_report_csv` to an `async` command (async commands run on the async runtime, off the main
  thread) and acquire the destination **without blocking the main thread**:
  - **Preferred:** the non-blocking callback API — `app.dialog().file()…save_file(move |maybe_path| { let _ = tx.send(maybe_path); })`
    with the result awaited via a oneshot/`mpsc` channel (`rx.await` / `rx.recv()` on the worker). The
    callback fires on the main run loop, which is now free to pump.
  - **Acceptable alternative:** keep `blocking_save_file()` but call it from the async/worker context
    (off the main thread) so the blocking wait does not park the main run loop.
  - **Not acceptable:** leaving the command synchronous (main-thread) with a `blocking_*` dialog call.
- **Deterministic IPC contract for all four outcomes** (the exact task ask): the command MUST always
  resolve —
  - **Cancel / no selection →** `Ok(None)` (no error, no hang).
  - **Path conversion or destination validation fails →** `Err(message)` (existing
    `validate_csv_destination` and the `into_path` guard are retained).
  - **Write fails →** `Err(message)` (the `fs::write` error from `export_csv_repo`).
  - **Success →** `Ok(Some(count))`.
- **Do not hold the DB lock across an `.await`.** Sequence: await the dialog first; *then* lock the DB
  (`db_conn`), validate, call `export_csv_repo`, drop the guard, return. Locking only after the await
  keeps the existing `Mutex<Connection>` model safe and avoids a new class of hang.
- **Preserve TASK-023 CSV security and export scope verbatim.** `export_csv_repo` / `csv_escape` /
  `csv_formula_neutralized` are unchanged; the column set
  (`date, project, start_time, end_time, duration_minutes, note, total_duration_hours`) is unchanged;
  no raw activity/window log, AI prompt/response, command body, or secret-shaped field is added. **No
  new network/egress** — the dialog and write are local; the renderer stays off-network.
- **Frontend (optional, minimal — not required for correctness).** The renderer already alerts on
  success (`Exported N entries.`) and surfaces errors via `alertError`, and it keeps **no persistent
  loading flag**, so once the backend resolves promptly the perceived freeze is gone. Optionally: show
  an explicit `Export canceled.` status on `Ok(None)`, and guard the Export button against
  double-clicks while a dialog is open. These are nice-to-haves, deliberately kept out of the required
  contract to avoid scope creep.

## Impact

- **Affected code (product runtime):** `src-tauri/src/lib.rs` — **`export_report_csv` only** (signature
  → `async`, dialog acquisition moved off the main thread, lock-after-await). Optionally a few lines in
  `src/main.ts`'s `#exportCsv` handler for the cancel-status nicety. No schema, no migration, no
  `tauri.conf.json` change, no new capability/permission (`dialog:default` + `dialog:allow-save` are
  already granted in `capabilities/default.json`).
- **Affected specs:** `csv-export` — **ADDED** requirements for export-flow responsiveness /
  deterministic IPC resolution and failure surfacing. TASK-023's neutralization/escaping/no-raw-export
  requirements are **unchanged** (not modified, not removed).
- **NFR / FR alignment:** FR-016 (local CSV export) and NFR-010 (low overhead, must not interfere with
  the workflow). Local-only posture (NFR-001) preserved — no network introduced.
- **Out of scope (clean boundaries):** review/approval UI (TASK-009); summary/approval model and export
  records (TASK-010); macOS capture (TASK-005); classification (TASK-008); any new export column,
  detailed/raw export mode, or CSV library; **moving the picker to the `@tauri-apps/plugin-dialog` JS
  API** (a larger-blast-radius redesign — it would change where the path is chosen and require passing
  a filesystem path across IPC plus fs-write scoping; not warranted for a minimal hang fix).
- **ADR:** proposed **DEC-024** — the CSV save-dialog runs **off the UI main thread** and the export
  IPC command **resolves deterministically** (cancel → none, validation/write failure → error, success
  → count). This is an SW-level integration decision implementing FR-016/NFR-010; **no BA escalation**
  is required (no requirement or component-boundary change).
- **Branch:** `fix/task-024-csv-export-save-dialog-hang`, base `main`.
