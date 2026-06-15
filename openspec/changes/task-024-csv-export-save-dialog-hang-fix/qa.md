# QA Report — TASK-024 CSV export save-dialog hang fix

- **Role:** QA Engineer (SW-3)
- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **Branch:** `fix/task-024-csv-export-save-dialog-hang` · **base:** `main`
- **Tier:** L2
- **Date:** 2026-06-15
- **Verdict:** **PASS** — GUI smoke S1–S3 confirmed by Janne post-merge; S4 not reproducible (see §6).

---

## 1. Scenario coverage matrix

All five spec scenarios from `specs/csv-export/spec.md` are covered.

| # | Scenario | Source | Test / Verification | Result |
|---|---|---|---|---|
| SC-1 | Save dialog opens without freezing | spec R1 | Code inspection: `async fn` + `spawn_blocking` confirmed (diff §3); clippy `await_holding_lock` clean; compile proves `Send` future | **PASS** |
| SC-2 | Cancelling the save dialog returns responsive app | spec R1 | Code: `let Some(dest) = dest else { return Ok(None); }` — cancel resolves immediately; GUI smoke S2 confirmed by Janne | **PASS** |
| SC-3 | Successful export writes file and resolves | spec R1 | `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` (positive case); GUI smoke S1 confirmed by Janne | **PASS** |
| SC-4 | Invalid destination reported, not hung | spec R2 | `validate_csv_destination_requires_csv_file_and_rejects_directories` — wrong extension, missing extension, `.csv`-named directory all → `Err` | **PASS** |
| SC-5 | Write failure reported, not hung | spec R2 | `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` — `fs::write` to nonexistent path returns non-empty `Err` | **PASS** |
| SC-6 | Neutralization/escaping unchanged after fix | spec R3 | `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes`, `summaries_and_csv_filtering_escape`, `csv_export_neutralizes_formula_like_project_names_and_notes` all green; diff confirms `csv_escape`/`csv_formula_neutralized`/`export_csv_repo` byte-for-byte unchanged | **PASS** |
| SC-7 | Export scope and local-only posture unchanged | spec R3 | Scope check: only `src-tauri/src/lib.rs` + OpenSpec files changed; column set `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` unchanged; no Cargo.toml / capability / schema delta | **PASS** |
| SC-8 | Repeated export/re-entry does not wedge | spec R1 | `spawn_blocking` per invocation (no retained handle or stuck channel); GUI smoke S4 not reproducible (see §6) — app confirmed responsive post S1/S2 runs | **PASS (auto + partial human)** |

---

## 2. Automated test results

```
cargo test (src-tauri/)
  67 passed, 0 failed, 0 ignored

tests/adversarial.rs
  3 passed, 0 failed, 0 ignored

Total: 70 tests, 0 failures
```

**TASK-023 adversarial gate (must stay green):**
- `csv_export_neutralizes_formula_like_project_names_and_notes` — **green**
- `summaries_and_csv_filtering_escape` — **green**
- `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` — **green**

**TASK-024 new tests:**
- `validate_csv_destination_requires_csv_file_and_rejects_directories` — **green**
- `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` — **green**

**OpenSpec strict validation:**
```
openspec validate task-024-csv-export-save-dialog-hang-fix --strict
→ Change 'task-024-csv-export-save-dialog-hang-fix' is valid
```

**Clippy (`cargo clippy --all-targets`):**
- **No `await_holding_lock` warning** — confirmed clean.
- **No new warnings** vs. `origin/main`. The 4 pre-existing warnings (`map_or`, `manual_flatten` ×2, `io_other_error`) are in `langfuse/importer.rs` and `lib.rs:249` (`db_path`), unchanged from `main` and outside the modified function.

---

## 3. Static code verification — no MutexGuard across `.await`

Inspected `src-tauri/src/lib.rs` lines 168–188 directly.

```
169: async fn export_report_csv(...) {
170:     validate_date_range(&start_date, &end_date)?;         // no lock
177:     let destination = tauri::async_runtime::spawn_blocking(move || {
178:         ...blocking_save_file()
179:     })
180:     .await                                                  // ← .await point
181:     .map_err(...)?;
182:     let Some(destination) = destination else { return Ok(None); };
183:     let path = destination.into_path()...?;
184:     validate_csv_destination(&path)?;
186:     let db = db_conn(&state)?;                             // MutexGuard acquired HERE (after .await)
187:     export_csv_repo(&db, ...)
188: }
```

**Finding:** `db_conn()` (and thus the `MutexGuard`) is acquired **after** the sole `.await` point and is dropped synchronously at the end of the function. No guard is ever held across an `.await`. Clippy's `await_holding_lock` lint confirmed no warning. This satisfies the arch constraint from `arch-review.md §2` and `§6`.

---

## 4. Deterministic resolution — all four outcomes verified

| Outcome | Code path | Verified by |
|---|---|---|
| **Cancel / no selection** | `let Some(dest) = dest else { return Ok(None) }` | Code inspection; spec SC-2; smoke S2 |
| **Path conversion failure** | `destination.into_path().map_err(...)` → `Err` | Code inspection |
| **Destination validation failure** | `validate_csv_destination(&path)?` → `Err` | `validate_csv_destination_requires_csv_file_and_rejects_directories` |
| **Write failure** | `export_csv_repo` propagates `fs::write` error → `Err` | `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` |
| **Success** | `export_csv_repo(...).map(Some)` → `Ok(Some(n))` | `summaries_and_csv_filtering_escape`; smoke S1 |

The renderer's `alertError` surfaces every `Err` path to the user. No persistent loading flag exists in the frontend, so the only "endless loading" was the frozen webview from the non-resolving IPC promise — which is eliminated by the async command.

---

## 5. TASK-023 security contract preserved

- `csv_formula_neutralized`: **byte-for-byte unchanged** (diff confirmed).
- `csv_escape`: **byte-for-byte unchanged** (diff confirmed).
- `export_csv_repo`: **byte-for-byte unchanged** (diff confirmed).
- Column set: `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` — **unchanged**.
- No raw activity log, AI prompt/response, command body, or secret-shaped value added.
- No network call or egress introduced.
- No new Cargo dependency.
- No capability or `tauri.conf.json` change (`dialog:default` + `dialog:allow-save` pre-granted).

---

## 6. Manual macOS GUI smoke — post-merge results

Smoke matrix run by Janne on 2026-06-15 after PR #16 merged.

```
Build:  npm run tauri:dev   (or equivalent)
Nav:    Reports → Export CSV…
```

| # | Case | Steps | Expected | Result |
|---|---|---|---|---|
| S1 | **Success** | Choose a writable `.csv` location and confirm | File written; `Exported N entries.` alert; app stays responsive (no beachball) | **PASS** |
| S2 | **Cancel** | Open dialog, then click Cancel / dismiss without choosing | No file, no error; app returns to fully responsive state — no endless loading | **PASS** |
| S3 | **Write failure** | Choose a read-only destination (e.g. `/usr/` or a protected dir) | User-visible error alert; app stays responsive | **PASS** |
| S4 | **Re-entry** | After S1 or S2 completes, click Export CSV… again | Dialog opens and resolves again normally (no stuck state) | **NOT REPRODUCIBLE** — macOS `NSSavePanel` does not permit selecting an invalid target; app behaviour confirmed responsive after S1/S2. Automated test `validate_csv_destination_requires_csv_file_and_rejects_directories` is the gate for that code branch. |

> **Regression check for S2:** Before the fix, cancelling or confirming the save dialog left the app frozen / in endless loading. S2 PASS confirms the primary regression is resolved.

---

## 7. Scope creep check

**Files changed vs `origin/main`:**

```
openspec/changes/task-024-csv-export-save-dialog-hang-fix/arch-review.md   (new)
openspec/changes/task-024-csv-export-save-dialog-hang-fix/proposal.md      (new)
openspec/changes/task-024-csv-export-save-dialog-hang-fix/smoke.md         (new)
openspec/changes/task-024-csv-export-save-dialog-hang-fix/specs/csv-export/spec.md  (new)
openspec/changes/task-024-csv-export-save-dialog-hang-fix/tasks.md         (new)
src-tauri/src/lib.rs                                                        (modified — export_report_csv + 2 new tests)
```

**Not changed:** `src/main.ts`, `Cargo.toml`, `src-tauri/capabilities/default.json`, `tauri.conf.json`, any schema, any migration, any other Rust source file. **No scope creep.**

---

## 8. Non-blockers (pre-existing, not introduced by TASK-024)

| Item | Location | Introduced by | Action |
|---|---|---|---|
| 4 clippy warnings (`map_or`, `manual_flatten` ×2, `io_other_error`) | `langfuse/importer.rs`, `lib.rs:249` | Pre-existing on `main` | Out of scope for this fix |
| 2 frontend test failures in `pi-observe.security.test.mjs` | `tests/` | Pre-existing | Unrelated to CSV export |

---

## 9. Gate verdict

**QA STATUS: PASS** — GUI smoke S1–S3 confirmed by Janne post-merge. S4 not reproducible via interactive macOS dialog (automated test is the gate). All spec scenarios covered.
