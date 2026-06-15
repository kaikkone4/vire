# SW-5 Security Review — TASK-024 (CSV export save-dialog hang fix)

- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **PR:** #16 — `fix/task-024-csv-export-save-dialog-hang` → `main`
- **Tier:** L2 (secrets + CVE≥7 + Trivy + semgrep ERROR)
- **Verdict: PASS**
- **Reviewed at commit:** `df61a54`
- **Code surface of the change:** `src-tauri/src/lib.rs` only (the other 5 PR files are OpenSpec docs). +44/-2 net.

## Scope of change

`export_report_csv` converted from a synchronous `#[tauri::command]` to `async`, with the
`blocking_save_file()` call moved off the UI main thread via
`tauri::async_runtime::spawn_blocking`. State signature widened to `State<'_, AppState>` for the
async command. Two unit tests added. No other product code changed.

Confirmed unchanged by this commit (verified via `git show df61a54`):
- `csv_formula_neutralized` / `csv_escape` (TASK-023 hardening) — byte-identical.
- `export_csv_repo` and its 7-column header `date,project,start_time,end_time,duration_minutes,note,total_duration_hours`.
- `invoke_handler` / `generate_handler!` registration list — no command added/removed.
- `Cargo.toml` / `Cargo.lock` — not touched (no dependency change).
- No `tauri.conf.json` / capability / schema / migration change.

## Tier 1 scanner results

| Scanner | Target | Result | Gate (auto-fail) | Status |
|---------|--------|--------|------------------|--------|
| **semgrep** 1.165.0 | `src-tauri/src/lib.rs` (`--config=auto`) | 0 findings, 0 ERROR | any ERROR-severity | **PASS** |
| **gitleaks** 8.30.1 | full repo (108 commits) + branch range `709a34c..HEAD` (65 commits) | no leaks found | any secret | **PASS** |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (487 pkgs) | 0 Critical, 0 High, 1 Medium (6.9), 16 no-CVSS | CVSS ≥ 7.0 | **PASS** |
| **Trivy** 0.71.1 | `fs` scan — vuln + secret + misconfig, `--severity HIGH,CRITICAL` | 0 vuln / 0 secret / 0 misconfig | HIGH or CRITICAL | **PASS** |

No container image exists for this macOS desktop app; the Trivy filesystem scan is the applicable
L2 substitute and covers `Cargo.lock` + `package-lock.json`.

## Manual secure code review — TASK-024 focus checklist

1. **Async/`spawn_blocking` introduces no new thread-safety issue — PASS.**
   `spawn_blocking` captures `dialog_app = app.clone()` (`AppHandle` is `Send + Sync + Clone`); the
   closure is `Send + 'static`. The DB lock is taken **only after** the `.await`
   (`lib.rs:186`), so no `std::sync::MutexGuard` is ever held across an await point. This is proven,
   not just asserted: a guard held across `.await` would make the command future `!Send`, which Tauri
   rejects at compile time — and the crate compiles and all 67 lib tests pass. DB concurrency model
   (single `Mutex<Connection>`) is unchanged; the write remains serialized.

2. **No new TOCTOU / path / permission surface — PASS.**
   Path still originates from the native macOS `NSSavePanel` (user-chosen), `into_path()` still
   rejects non-local paths, and the `.csv`/not-a-directory checks still run before the write. The
   validate→write window is identical to TASK-023 (pre-existing, benign for a single-user local app
   writing to a path the user just picked); moving the dialog to the blocking pool does not widen it.
   No new capability/permission — `dialog:default` + `dialog:allow-save` were already granted and the
   IPC handler list is unchanged. `spawn_blocking` is thread-pool dispatch, not a privilege boundary.

3. **CSV destination validation still enforced after selection — PASS.**
   Order preserved: dialog → `into_path()` local-path guard (`lib.rs:183`) →
   `validate_csv_destination` `.csv`-extension + not-a-directory checks (`lib.rs:184,244-248`) → DB
   lock → write. Covered by new test `validate_csv_destination_requires_csv_file_and_rejects_directories`
   (incl. a `.csv`-named directory rejected as a directory).

4. **TASK-023 formula neutralization/escaping + 7-column set unchanged — PASS.**
   `csv_formula_neutralized`/`csv_escape`/`export_csv_repo` untouched; adversarial test
   `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` still passes.

5. **No raw activity/prompt/secret export expansion — PASS.**
   `export_csv_repo` emits only manual time-entry fields (date, project name, start/end, duration,
   note, hours). No Langfuse, runtime-observer, activity-log, prompt/response, or command-body data is
   reachable from the export path.

6. **No new network/egress, dependency, schema, IPC command/capability/auth/rate-limit surface — PASS.**
   `spawn_blocking` + `fs::write` are local; no HTTP client involved. `Cargo.*` untouched; no schema
   or migration; no new `#[tauri::command]`; no capability/auth/rate-limit change.

7. **Error messages do not leak secrets or paths — PASS.**
   The three new/affected error strings — `"CSV export save dialog could not be presented"` (join
   error), `"CSV export destination must be a local file path"`, and the
   `validate_csv_destination` messages — contain no path or secret. The write-failure error is
   `std::io::Error::to_string()`, whose Display omits the path (e.g. `"No such file or directory (os
   error 2)"`); the only path in play is the user's own selected local destination. Surfaced by new
   test `export_csv_repo_surfaces_write_failure_as_error_not_a_hang`.

Additional positive: `spawn_blocking` join errors (incl. a panic inside the dialog) map to `Err`, so
even a worker panic resolves deterministically rather than hanging — reinforcing the IPC contract.

## Advisory findings (non-blocking — documented per rubric)

- **OSV `RUSTSEC-2024-0429` / `GHSA-wrw7-89jp-8q8g` — `glib` 0.18.5, CVSS 6.9 (Medium).** Below the
  ≥7.0 auto-fail threshold. Transitive dependency of the Tauri/GTK stack; **not introduced by
  TASK-024** (`Cargo.lock` unchanged). Fixed in glib 0.20.0 — track as a framework-level dependency
  bump outside this PR.
- **16 no-CVSS RustSec advisories** on `atk`, `gdk*`, `gtk*`, `proc-macro-error`, `unic-*` — mostly
  "unmaintained" notices on the GTK/Linux binding crates. The macOS target does not link the GTK
  Linux backend; all are pre-existing transitive deps, none added by TASK-024. Advisory only.

These predate this change and are out of TASK-024's scope; they do not gate this PR. Recommend a
dedicated dependency-hygiene task to refresh the Tauri/GTK chain.

## Escalations

None. No design-level (trust-boundary / missing-auth) issue. No `feedback_to_ba[]`.

## Verdict

**PASS.** No L2 auto-fail condition met (no secrets, no CVSS≥7, no Trivy HIGH/CRITICAL, no semgrep
ERROR). All seven TASK-024 security-focus items verified. Proceed — hold for SW-4 (Code Review)
before SW-6 release.
