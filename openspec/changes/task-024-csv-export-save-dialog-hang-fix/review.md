# Code Review — TASK-024 CSV export save-dialog hang fix

- **Role:** Code Reviewer (SW-4)
- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **PR:** #16 — `fix/task-024-csv-export-save-dialog-hang` → `main`
- **Tier:** L2
- **Date:** 2026-06-15
- **Commit reviewed:** `df61a54`
- **Product code changed:** `src-tauri/src/lib.rs` only (+44/-2 net)

---

## Verdict: PASS

No blocking issues. Two non-blocking suggestions documented below. No architectural escalation required.

---

## 1. Correctness of the async fix (`lib.rs:169–188`)

### 1.1 Deadlock eliminated

`export_report_csv` is declared `async fn` (line 169). In Tauri v2, async commands run on the async runtime — off the main thread. `blocking_save_file()` is dispatched to `tauri::async_runtime::spawn_blocking` (line 177), so the blocking-channel wait runs on a pool thread while the main run loop remains free to pump the completion callback. The root-cause deadlock is structurally eliminated, not just papered over.

The explanatory comment at lines 171–175 correctly describes the threading invariant and the precise mechanism of the original hang. This is one of the few places in this file where a comment is genuinely necessary (hidden platform constraint), and it earns its keep.

### 1.2 No `MutexGuard` held across `.await`

Sequence verified:
```
line 170:  validate_date_range(...)     // no lock
line 177:  spawn_blocking(...)
line 180:  .await                       // ← sole await point
line 182:  let Some(destination) = ...
line 183:  into_path()
line 184:  validate_csv_destination()
line 186:  let db = db_conn(&state)?   // MutexGuard acquired HERE, after the await
line 187:  export_csv_repo(&db, ...)   // guard used and dropped synchronously
```

`db_conn` (and hence the `std::sync::MutexGuard<Connection>`) is acquired only at line 186, after the only `.await` point. The guard is never named across an await boundary. The comment at line 185 documents the ordering invariant. QA confirmed `cargo clippy --all-targets` emits no `await_holding_lock` warning. This satisfies `arch-review.md §2` and §6 verbatim.

### 1.3 Deterministic IPC contract — all five resolution paths

| Outcome | Code path | Status |
|---|---|---|
| Cancel / no selection | `let Some(dest) = dest else { return Ok(None); }` (line 182) | Correct |
| Worker panic inside `spawn_blocking` | `.map_err(\|_\| "CSV export save dialog could not be presented")` (line 181) | Correct — resolves as `Err`, not a hang |
| `into_path()` failure (non-local path) | `.map_err(\|_\| "CSV export destination must be a local file path")` (line 183) | Correct |
| Destination validation failure | `validate_csv_destination(&path)?` (line 184) | Correct |
| Write failure | `export_csv_repo(...).map(Some)` — propagates `fs::write` `Err` | Correct |
| Success | `export_csv_repo(...).map(Some)` → `Ok(Some(n))` | Correct |

Every path resolves. The command never hangs or panics silently.

### 1.4 `AppHandle` capture is safe

`dialog_app = app.clone()` (line 176) captures a `Clone + Send + Sync` `AppHandle` into the `spawn_blocking` closure. The closure is `Send + 'static`. No raw pointer, `Rc`, or `!Send` type crosses the thread boundary. Compilation with all 70 tests passing proves the future is `Send` (Tauri rejects async command futures that are `!Send` at compile time).

---

## 2. TASK-023 code preserved verbatim

Direct inspection of `lib.rs:142–156`:
- `csv_formula_neutralized` — byte-identical to `main`.
- `csv_escape` — byte-identical to `main`.
- `export_csv_repo` — byte-identical to `main`.
- Column set `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` — unchanged (line 153).

No adversarial hardening from TASK-023 was touched. The TASK-023 adversarial test gate remains green per QA §2.

---

## 3. New tests — quality assessment (`lib.rs:275–298`)

### `validate_csv_destination_requires_csv_file_and_rejects_directories` (lines 275–287)

- Tests wrong extension (`.txt` → error containing `.csv`) ✓
- Tests missing extension (no extension → `Err`) ✓
- Tests a `.csv`-named *directory* (created with `fs::create_dir`) → rejected as directory ✓
- Tests a valid `.csv` file path → `Ok(())` ✓

Full coverage of the `validate_csv_destination` function with the edge case (`.csv`-named directory) that is most likely to slip through naive implementations. Test quality is high.

### `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` (lines 289–298)

- Seeds a project and entry, then calls `export_csv_repo` with a path whose parent directory does not exist.
- Asserts the result is `Err` with a non-empty message.

This directly validates the write-failure deterministic-resolution contract. Correct and sufficient.

---

## 4. Minimality / scope check

Files changed on this branch vs. `main` (product code):
- `src-tauri/src/lib.rs` — `export_report_csv` async conversion + 2 new tests.

Not changed: `src/main.ts`, `Cargo.toml`, `Cargo.lock`, `capabilities/default.json`, `tauri.conf.json`, any schema, any migration, any other Rust source file. No new dependency, capability, command, export column, or IPC surface. Fully minimal.

---

## 5. Code style and conventions

- `async fn export_report_csv(app: tauri::AppHandle, state: State<'_, AppState>, ...)` — the explicit `'_` lifetime on `State` is required for async Tauri commands; synchronous commands omit it. Consistent with how Tauri v2 async commands must be declared.
- Early-return on cancel (`let Some(destination) = destination else { return Ok(None); }`) — idiomatic; consistent with project's early-return style.
- No `console.log`, no commented-out code, no dead code introduced.
- Two comments added; both explain non-obvious threading invariants and meet the project's comment standard (WHY, not WHAT).
- No new `unwrap()` or `expect()` in product code paths.
- Pre-existing 4 clippy warnings (`map_or`, `manual_flatten` ×2, `io_other_error`) are on `main` and outside the modified function — not introduced by this change.

---

## 6. Suggestions (non-blocking)

### S-1 — Error message for `spawn_blocking` join error is slightly imprecise

**Location:** `lib.rs:181`

```rust
.map_err(|_| "CSV export save dialog could not be presented".to_string())?;
```

A `JoinError` from `spawn_blocking` means the worker thread panicked, not necessarily that the dialog failed to appear (it may have appeared; the panic happened elsewhere in the closure). A marginally more accurate message would be `"CSV export save dialog failed unexpectedly"`. This is a very edge case (worker panic) and the current message is not harmful; the user would still see an error rather than a hang. Non-blocking.

### S-2 — `spawn_blocking` preferred over the callback+channel approach

The `arch-review.md §3` listed the non-blocking callback API as "Preferred" and `spawn_blocking` with the blocking API as "Acceptable alternative." The implementation used the acceptable alternative, which is simpler (no channel, one fewer moving part) and equally correct. This is a reasonable implementation choice and consistent with the existing `run_bounded` pattern in the same file. No action needed; noting for audit completeness only.

---

## 7. Blocking issues

None.

---

## 8. Escalations to SW Architect

None. The implementation follows `arch-review.md` constraints exactly: async command, `spawn_blocking` for the dialog, lock acquired after the await, TASK-023 code untouched. No component boundary is crossed; no design-level concern.

---

## 9. Gate verdict

**PASS** — proceed to SW-6 Release Manager (SW-5 Security already passed per `sec.md`).

GUI smoke matrix (S1–S4 from `qa.md §6`) still awaits Janne's local run before shipping; this is a QA gate condition, not a code-review blocker.
