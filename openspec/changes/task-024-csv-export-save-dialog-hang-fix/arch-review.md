# Architecture Review (SW-1) — TASK-024 CSV export save-dialog hang fix

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **Branch (proposed):** `fix/task-024-csv-export-save-dialog-hang` · **base:** `main`
- **Tier:** L2 · **Gate context:** SW-1 task-design review before a developer implements the fix for
  the local Mac MVP smoke blocker (Reports → Export CSV… opens the save dialog, then the app freezes /
  endless loading). Follows TASK-023 (CSV writer hardening, merged, post-merge PASS).
- **Date:** 2026-06-15
- **Verdict:** **PASS** — one cohesive backend / Tauri-integration slice (the `export_report_csv` IPC
  command in `src-tauri/src/lib.rs`). No component boundary is crossed; **not split-required**. No BA
  escalation required (implements FR-016 + NFR-010 within the existing Exporter component and IPC).

---

## 1. Inputs read

- BA requirements: `01_requirements.md` FR-016 (local export of reviewed summaries, no cloud sync),
  NFR-010 (overhead low enough not to interfere with the workflow), NFR-001 (local-only).
- BA architecture: `03_architecture_plan.md` line 93 — **Exporter** component ("write reviewed
  summaries as local CSV; support formula-neutralized text and redacted details; boundary: no cloud
  sync or automatic invoicing").
- Prior change: `task-023-csv-adversarial-export-hardening` — `proposal.md`, `specs/csv-export/spec.md`
  (neutralization/escaping/no-raw-export requirements), `arch-review.md` (Exporter + input layer).
- Current code:
  - `src/main.ts:40` — the `#exportCsv` handler: `invoke('export_report_csv', …)`, `await`ed inside
    `run(action) = void action().catch(alertError)`; alerts `Exported N entries.` on a non-null count.
    There is **no separate frontend save dialog** and **no persistent loading flag**.
  - `src-tauri/src/lib.rs:168` — `export_report_csv`: a **synchronous** `#[tauri::command]` that calls
    `app.dialog().file()…blocking_save_file()` (line 170), then `into_path()` (172),
    `validate_csv_destination` (173/232), `db_conn` (174), `export_csv_repo` (175/152).
  - TASK-023 hardening (untouched by this change): `csv_formula_neutralized` (142), `csv_escape` (151),
    `export_csv_repo` (152).
  - `src-tauri/capabilities/default.json` — `dialog:default`, `dialog:allow-save` already granted.
  - `src-tauri/Cargo.toml` — `tauri = "2.2"`, `tauri-plugin-dialog = "2.2"`.

## 2. Defect diagnosis (evidence-based)

**Static evidence (code path):** The renderer has no dialog of its own — the picker is inside the Rust
command. `export_report_csv` is declared `fn` (synchronous). In Tauri v2 a synchronous command runs on
the **main thread**. On line 170 it calls `tauri_plugin_dialog`'s **`blocking_save_file()`**, whose
blocking variant parks the calling thread on a channel that is completed by a callback dispatched onto
the **main run loop**. Parking the main thread on that channel means the completion callback can never
be delivered → **deadlock**. The OS still presents `NSSavePanel` (hence the user *sees* the dialog),
but the result is never returned: the `invoke` promise the renderer `await`s never resolves, so the
webview stays "loading" and the app beachballs. This exactly matches the reported symptom — "dialog
opened asking where to save, then the app effectively froze / endless loading."

**Why it is the command wiring, not the writer:** `export_csv_repo` writes a tiny string with
`fs::write` and the `Mutex<Connection>` is uncontended, so neither can account for an *indefinite*
hang; and the freeze is observed *at the dialog*, before any write. The blocking-dialog-on-main-thread
anti-pattern is the only path consistent with "panel renders, then nothing returns."

**Confirmation owed by the developer (step 1 of `tasks.md`):** reproduce the freeze and verify via a
temporary log/DevTools that the renderer `invoke` never resolves (neither success nor error fires).
This mirrors TASK-023's evidence-first discipline; the fix should not land on a static diagnosis alone.

## 3. Architecture-consistency findings

1. **Fix the threading, not the writer.** The minimal correct change keeps the picker in the Rust
   command but moves it **off the main thread**: make `export_report_csv` `async` and acquire the
   destination via the non-blocking callback API (`…save_file(move |p| tx.send(p))` + awaited channel),
   or call `blocking_save_file()` from the async/worker context. The CSV writer and its TASK-023
   neutralization are correct and MUST stay byte-for-byte unchanged. No writer rewrite is warranted.
2. **Lock-after-await, never across an `.await`.** The app uses a `std::sync::Mutex<Connection>`
   (`db_conn`). The fixed async command MUST await the dialog *first*, then lock → validate →
   `export_csv_repo` → drop the guard → return. Holding the guard across the dialog await would trade
   one hang for another (and trip clippy's `await_holding_lock`). Sequencing the lock after the await
   keeps the existing single-mutex model intact — no new synchronization primitive is introduced.
3. **Deterministic resolution is the contract.** The command already encodes three of the four
   outcomes (None → `Ok(None)`; bad path → `Err`; success → `Ok(Some(n))`); the defect is only that the
   *dialog acquisition* never returns control. The fix preserves those mappings and adds the missing
   guarantee: **the command always resolves.** No new health state, status enum, or IPC surface is
   added — this is the same `CmdResult<Option<usize>>` contract, made reliable.
4. **No capability or permission change.** `dialog:default` + `dialog:allow-save` are already granted;
   the callback API uses the same plugin permission as the blocking API. No `tauri.conf.json` or
   capability edit is needed — a smaller blast radius than the alternatives in §4.

## 4. Split analysis — one slice, not split-required

Per the role rule, "split-required" means scope crosses a component boundary in
`03_architecture_plan.md`. It does not. The defect and the fix live entirely in the **Exporter / IPC
wiring** of the single Tauri Rust core (one command, one file). The only other potential touch — a
cosmetic "Export canceled." status in `src/main.ts` — is the same app's renderer talking to the same
command over the existing IPC, not a second component, and is **optional**.

| Listed piece | Component | Boundary crossing? |
| --- | --- | --- |
| `export_report_csv` → async, dialog off main thread, lock-after-await | Rust core — Exporter / IPC command | No |
| `export_csv_repo` / `csv_escape` / `csv_formula_neutralized` (unchanged) | Rust core — CSV writer | No (no change) |
| Optional `Export canceled.` status / button guard | Renderer (same app, existing IPC) | No (optional) |
| No export-field expansion, no new network (guardrail) | Cross-cutting invariant (SEC-006 / NFR-001) | No |

Considered and rejected as a split/redesign: moving the picker to the `@tauri-apps/plugin-dialog` **JS
API**. That would relocate path selection to the renderer and require passing a filesystem path across
IPC plus fs-write scoping — a larger blast radius for no benefit over the minimal async fix. **Verdict:
one cohesive backend/integration slice**, internal sequence in `tasks.md`, not a split.

## 5. Contract the developer must satisfy (the exact task ask)

| # | Contract clause | Where enforced |
| --- | --- | --- |
| 1 | Export must not leave the app in endless loading on **cancel** | Dialog off main thread; cancel → `Ok(None)`; command resolves (spec R1) |
| 2 | …on **path selection / validation failure** | `into_path()` guard + `validate_csv_destination` → `Err`; command resolves (spec R2) |
| 3 | …on **write failure** | `export_csv_repo`'s `fs::write` error → `Err`; command resolves (spec R2) |
| 4 | …on **success** | Write + `Ok(Some(n))`; app responsive (spec R1) |
| 5 | User-visible **error/status** for failures | Renderer `alertError` already surfaces `Err`; (optional) `Export canceled.` on `Ok(None)` (spec R2) |
| 6 | **No indefinite loading state** | Save dialog presented off the UI main thread; command always resolves (spec R1) |
| 7 | Preserve **TASK-023 CSV security** | `csv_escape` / `csv_formula_neutralized` / `export_csv_repo` unchanged; adversarial test stays green (spec R3) |
| 8 | **No new** export columns / schema / network / review-UI scope | Column set unchanged; no egress; out-of-scope list in `tasks.md` (spec R3) |

## 6. Empirical facts the developer must honor

- Synchronous Tauri v2 commands run on the **main thread**; `blocking_save_file()` must **not** be
  called there. The fix is to present the dialog off the main thread (async command + non-blocking
  callback, or off-main-thread blocking call) — not to add a timeout or retry around a main-thread
  block.
- Do **not** hold the `db_conn` `MutexGuard` across the dialog `.await` (clippy `await_holding_lock`;
  also a latent hang). Lock only after the destination is chosen.
- The renderer keeps **no loading flag**; the "endless loading" is the frozen webview, not a spinner
  the frontend forgot to clear. Once the command resolves promptly, existing `alertError` (failures)
  and the `Exported N entries.` alert (success) already satisfy the user-facing-status requirement; a
  cancel status is an optional nicety, not a required fix.
- Tests that must stay green: the TASK-023 adversarial gate
  `csv_export_neutralizes_formula_like_project_names_and_notes`, `summaries_and_csv_filtering_escape`,
  and the rest of the backend suite. The GUI deadlock itself is validated by **manual macOS smoke**,
  not a headless unit test.

## 7. ADR + open items

**Proposed ADR DEC-024 (record in BA decision log on ratification): the CSV save-dialog runs off the
UI main thread and the export IPC resolves deterministically.** The Export CSV command presents its
save dialog off the main thread (async command) and always resolves — cancel → no export, validation
or write failure → user-visible error, success → exported row count — so the renderer's awaited call
never hangs. Rationale: a synchronous command calling a blocking save dialog on the macOS main thread
deadlocks (FR-016 export unusable; NFR-010 violated by a frozen app). Scope: the export command wiring
only; the CSV writer and its TASK-023 neutralization are unchanged.

**`feedback_to_ba[]`: none required.** This implements existing FR-016 + NFR-010 within the existing
Exporter component; no requirement, decision, or component boundary changes. DEC-024 is recorded as an
SW-level integration ADR within this change and does **not** block developer start.

## 8. Recommendation — next role and branch

- **Change name:** `task-024-csv-export-save-dialog-hang-fix` (this dir).
- **Branch:** `fix/task-024-csv-export-save-dialog-hang`, base `main`.
- **Next role (primary):** **backend-developer (Rust / Tauri integration)** — convert
  `export_report_csv` to an `async` command, move the save dialog off the main thread (non-blocking
  callback + awaited channel preferred), keep the lock-after-await sequence and the existing
  validation/`export_csv_repo` write, and run the manual macOS smoke + full `cargo test` / `clippy`.
- **Frontend role: optional / not required.** The renderer already surfaces failures (`alertError`) and
  success (`Exported N entries.`) and has no loading flag to clear. A small frontend touch (an
  `Export canceled.` status and a double-click guard) is a nice-to-have a frontend developer can add in
  the same change if desired, but it is not on the required-fix path.
- **Then:** SW-4 (code review — async-command threading, no lock-across-await, no swallowed errors),
  SW-5 (security — TASK-023 neutralization/escaping and no-raw-export/no-network preserved), SW-6
  (release). SW-3/QA centers on the manual macOS smoke matrix (success / cancel / failed destination).

## 9. Verdict

**PASS.** TASK-024 is one cohesive backend / Tauri-integration bugfix slice inside the Tauri Rust core
— not split-required, no BA escalation. The local Mac smoke blocker is a main-thread deadlock:
`export_report_csv` is a synchronous command (main thread) calling `blocking_save_file()`, so the IPC
promise never resolves and the app freezes after the dialog opens. The fix moves the dialog off the
main thread (async command, non-blocking picker) and makes the command resolve deterministically on
cancel/validation-failure/write-failure/success, while leaving the TASK-023 CSV writer and its
security neutralization untouched and adding no export column, schema, network, or review-UI scope.
Deliverables (`proposal.md`, `tasks.md`, `specs/csv-export/spec.md`, this review) are in place; ADR
DEC-024 recorded; no `feedback_to_ba[]`. Route to backend-developer (Rust / Tauri) on
`fix/task-024-csv-export-save-dialog-hang`.
