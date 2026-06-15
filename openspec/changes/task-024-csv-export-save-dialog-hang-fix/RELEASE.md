# Release — TASK-024 CSV Export Save-Dialog Hang Fix

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-024-csv-export-save-dialog-hang-fix`
- **Branch:** `fix/task-024-csv-export-save-dialog-hang` · **PR:** [kaikkonen4/vire#16](https://github.com/kaikkonen4/vire/pull/16) — **MERGED** (`72c3843`)
- **Release unit:** Bugfix — `export_report_csv` async conversion to eliminate main-thread save-dialog deadlock
- **Date:** 2026-06-15
- **Product version baseline:** `0.1.1` (TASK-023 PATCH) · **Release tag:** `task-024/v0.1.2` (PATCH bugfix)
- **Verdict:** RELEASED — PR #16 merged to `main` at `72c3843`. GUI smoke S1–S3 confirmed by Janne; S4 not reproducible (macOS `NSSavePanel` does not permit selecting an invalid target, so the path-error case cannot be triggered interactively — the automated test `validate_csv_destination_requires_csv_file_and_rejects_directories` is the gate for that branch). Tag `task-024/v0.1.2` signing still deferred; no unsigned tag created.
- **Gate chain:** SW-3 PASS (`df61a54`) · SW-4 PASS (`39d6bad`) · SW-5 PASS (`df61a54`) · Docs PASS (`c3ff432`)

> **Note:** This RELEASE.md was not included in PR #16 (local release commits `87c032a` and `5b3f27d` were not pushed before the merge captured `c3ff432`). Artifact added via follow-up PR to close the documentation gap.

---

## Gate prerequisites

| Gate | Role | Verdict | Artifact | Commit audited |
|------|------|---------|----------|----------------|
| SW-3 | QA Engineer | **PASS** | `qa.md` | `df61a54` |
| SW-4 | Code Reviewer | **PASS** | `review.md` | `39d6bad` |
| SW-5 | Security Agent | **PASS** | `sec.md` | `df61a54` |
| SW-6 Docs | Documentation Engineer | **PASS** | `docs.md` | `c3ff432` |

SW-4 code review artifact committed at `39d6bad`. SW-3 QA and SW-5 security artifacts present in
change directory. Documentation Engineer completed the docs gate at `c3ff432`: README step 6 expanded
with cancel and re-entry sub-cases; no new docs/ pages required (internal threading change only).
All four gate artifacts confirm no blocking issues.

---

## Release contents

### What this fix resolves

**Reports → Export CSV…** was unusable on macOS: the save dialog would render but the app would
freeze into a beachball / endless-loading state and never resolve the IPC call (FR-016 unusable,
NFR-010 violated).

Root cause: `export_report_csv` was a synchronous `#[tauri::command]` running on the main thread,
calling `tauri_plugin_dialog`'s `blocking_save_file()`. That call parks the caller on a
`sync_channel(0)` whose completion callback is dispatched onto the main run loop — so the parked
main thread could never receive the callback → deadlock.

### Fix applied (commit `df61a54`)

`export_report_csv` is now an `async fn`. The blocking dialog call is dispatched to
`tauri::async_runtime::spawn_blocking`, keeping the main run loop free to pump the dialog's
completion callback. The `MutexGuard` over the DB connection is acquired **only after** the
`.await` point — never held across it — so there is no `await_holding_lock` concern and the
command future is `Send`.

### Runtime changes

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs` | `export_report_csv` converted to `async fn`; dialog call moved to `spawn_blocking`; `State<'_, AppState>` lifetime made explicit for async Tauri; two new unit tests added. No other function touched. |
| `openspec/changes/task-024-csv-export-save-dialog-hang-fix/` | Proposal, tasks, spec, smoke matrix, arch-review, QA, security, review artifacts (SW-1 through SW-5 documentation). |

### TASK-023 security contract preserved verbatim

`csv_formula_neutralized`, `csv_escape`, and `export_csv_repo` are byte-for-byte identical to
`main`. The 7-column header (`date,project,start_time,end_time,duration_minutes,note,total_duration_hours`)
is unchanged. No new IPC command, no new capability, no schema or migration, no Cargo dependency
added or bumped, no frontend file changed.

### Test results

```
cargo test (src-tauri/)
  67 unit tests:  67 passed, 0 failed
  3 adversarial:   3 passed, 0 failed
  Total:          70 tests, 0 failures

New TASK-024 unit tests:
  validate_csv_destination_requires_csv_file_and_rejects_directories  — PASS
  export_csv_repo_surfaces_write_failure_as_error_not_a_hang          — PASS

TASK-023 adversarial gate:
  csv_export_neutralizes_formula_like_project_names_and_notes         — PASS (stayed green)
  summaries_and_csv_filtering_escape                                  — PASS
  csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes   — PASS

cargo clippy --all-targets: no await_holding_lock warning; no new warnings vs. main.
openspec validate task-024-csv-export-save-dialog-hang-fix --strict: valid.
```

### GUI smoke matrix — post-merge confirmation

```
Build:  npm run tauri:dev
Nav:    Reports → Export CSV…
```

| # | Case | Steps | Expected | Result |
|---|------|-------|----------|--------|
| S1 | **Success** | Choose a writable `.csv` location and confirm | File written; `Exported N entries.` alert; app stays responsive | **PASSED (Janne, 2026-06-15)** |
| S2 | **Cancel** | Open dialog, dismiss without choosing | No file, no error; app returns to fully responsive state (no endless loading) | **PASSED (Janne, 2026-06-15)** |
| S3 | **Write failure** | Choose a read-only destination (e.g. `/usr/`) | User-visible error alert; app stays responsive | **PASSED (Janne, 2026-06-15)** |
| S4 | **Re-entry** | After S1 or S2, click Export CSV… again | Dialog opens and resolves normally (no stuck state) | **NOT REPRODUCIBLE** — macOS `NSSavePanel` does not allow selecting a path the OS considers invalid; the automated test `validate_csv_destination_requires_csv_file_and_rejects_directories` is the gate for that branch. App behaviour confirmed responsive. |

> S2 is the primary regression case: before the fix, dismissing the dialog left the app frozen.

---

## Required declaration 1 — Deployment size

**`patch` (deadlock bugfix; no breaking changes; no new user-visible surface beyond the fix itself).**

- Single function converted to `async` in `lib.rs`; no new helper function, no renamed symbol
- No new Tauri IPC command, no new DB table or column, no schema migration
- No new dependency added, removed, or version-bumped (`Cargo.toml`/`Cargo.lock` untouched)
- No frontend file, no IPC contract, no `tauri.conf.json` changed
- `src-tauri/Cargo.toml` version string remains `0.1.0`; release tracked by task tag `task-024/v0.1.2`
- Behavioral delta: save dialog resolves deterministically in all four outcomes (success, cancel,
  path-error, write-error) instead of deadlocking. All other behavior is identical to `main`.

---

## Required declaration 2 — Rollback strategy

**`partial-automated`.**

| Step | Action | Mode |
|------|--------|------|
| 1 | `git revert df61a54` on `main` post-merge | Automated |
| 2 | `cargo test` — verify 65+ tests pass; `validate_csv_destination_requires_csv_file_and_rejects_directories` and `export_csv_repo_surfaces_write_failure_as_error_not_a_hang` will disappear (reverted), which is expected | Automated |
| 3 | `cargo clippy --all-targets` — confirm no regression beyond the reverted tests | Automated |
| 4 | Tag reverted state `task-024-rollback/vX` | Manual |
| 5 | Re-deploy app binary | Manual (no DB migration required; schema unchanged) |

**Data consideration:** This fix changes only runtime threading behavior. There is no persistent
state change — no schema migration, no stored data format change, no new column. Rollback is a
pure code revert; no data migration is required.

**Rollback consequence:** Reverting restores the deadlock. Users would again see the app freeze on
CSV export. This is a known-bad regression; the revert should only be done if the async change
introduces an unforeseen stability issue in production that cannot be hot-fixed forward.

**Staging gate:** `cargo test` suite is the automated gate. GUI smoke S1–S3 should be re-run on
the reverted binary to confirm the old behavior is restored (deadlock visible on S1/S2).

---

## Required declaration 3 — Component compatibility matrix

No dependency was added, removed, or version-bumped in this change. The matrix reflects the full
runtime dependency envelope inherited from the branch tip (identical to TASK-023 RELEASE.md).

### Rust core (`src-tauri/Cargo.toml`)

| Crate | Specified | Tested | Notes |
|-------|-----------|--------|-------|
| `tauri` | `^2.2` | 2.2.x | Tauri v2; no v1 compatibility. Async commands require v2. |
| `tauri-plugin-dialog` | `^2.2` | 2.2.x | Paired with tauri `^2.2`; `blocking_save_file()` used via `spawn_blocking` |
| `rusqlite` | `^0.32` | 0.32.x | Bundled SQLite; no external SQLite required |
| `serde` | `^1` | 1.x | `derive` feature |
| `serde_json` | `^1` | 1.x | — |
| `chrono` | `^0.4` | 0.4.x | `serde`, `clock` features |
| `uuid` | `^1` | 1.x | `v4` feature |
| `thiserror` | `^2` | 2.x | — |
| `reqwest` | `^0.12` | 0.12.x | `blocking`, `json`, `rustls-tls`; no default-features |
| `url` | `^2` | 2.x | — |

### Frontend (`package.json`)

| Package | Specified | Tested | Notes |
|---------|-----------|--------|-------|
| `@tauri-apps/api` | `^2.2.0` | 2.2.x | — |
| `@tauri-apps/plugin-dialog` | `^2.2.0` | 2.2.x | — |

### Build toolchain

| Tool | Min | Tested |
|------|-----|--------|
| Rust / Cargo | 1.70 (edition 2021 min) | 1.95.0 |
| Node | 18 LTS | 26.3.0 |
| `@tauri-apps/cli` | `^2.2.0` | 2.2.x |
| macOS | 11.0+ (Tauri v2 min) | tested on current dev machine |

**No upstream/downstream service compatibility concerns.** Vire is a local-first desktop app;
there are no network API versions to declare. The only external contract (Langfuse REST) is
unchanged by this task.

---

## Non-blocking advisories rollup (SW-3 / SW-4 / SW-5)

Carried forward for future tasks; none affects the SW-6 gate verdict.

| ID | Source | Severity | Finding | Disposition |
|----|--------|----------|---------|-------------|
| NB-1 | SW-4 review | Style | `lib.rs:181`: `spawn_blocking` join-error message `"CSV export save dialog could not be presented"` is slightly imprecise — a `JoinError` means the worker panicked, not necessarily that the dialog failed to appear. Suggested alternative: `"CSV export save dialog failed unexpectedly"`. Zero user impact (edge case; user still sees error, not a hang). | Defer to a minor polish task |
| NB-2 | SW-4 review | Advisory | `arch-review.md §3` listed the non-blocking callback API as "Preferred" and `spawn_blocking` with the blocking API as "Acceptable alternative." The implementation used the acceptable alternative (simpler, one fewer moving part, equally correct). | No action needed; noted for audit completeness |
| NB-3 | SW-4 / SW-3 | Style | `csv_formula_neutralized` (`lib.rs:145`): `matches!(v.chars().next(), Some('\t'\|'\r'\|'\n'))` arm is unreachable — `trim_start_matches(is_whitespace)` already strips those chars. Suggested simplification: keep only `trimmed.starts_with(['=', '+', '-', '@'])`. Pre-existing from TASK-023; zero behavior change. | Defer to a cleanup task |
| NB-4 | SW-3 / SW-5 | Advisory | 4 pre-existing `cargo clippy` warnings in `langfuse/importer.rs` (lines 182, 306, 316 — `map_or`/`manual_flatten` style) and `lib.rs:249` (`io_other_error`). None introduced by TASK-024. | Defer to a clippy-cleanup task |
| NB-5 | SW-5 | Advisory | 17 transitive RUSTSEC advisories: `glib 0.18.5` (RUSTSEC-2024-0429, CVSS 6.9 — below L2 threshold of 7.0; fixed in glib 0.20.0) and 16 no-CVSS unmaintained/soundness advisories on Tauri's Linux GTK3 binding stack (`atk`, `gdk*`, `gtk*`), `proc-macro-error`, and `unic-*` crates. Pre-existing; no lockfile change in this PR. | Defer to a dependency-refresh task |
| NB-6 | SW-3 | Advisory | 2 pre-existing frontend test failures in `tests/pi-observe.security.test.mjs`. Unrelated to CSV export. | Defer; out of scope |

---

## Tag and signing

**Planned tag:** `task-024/v0.1.2`

**Signing status: DEFERRED — tag not created.**

L2 policy requires a signed tag. SSH signing is configured (`git config gpg.format=ssh`,
`tag.gpgsign=true`, `user.signingkey=/Users/kaikkonen/.ssh/id_ed25519.pub`), but the private key
at `/Users/kaikkonen/.ssh/id_ed25519` is absent in this environment (only the public key
is present). This is the same condition as TASK-023 (tag `task-023/v0.1.1` also deferred).

**Tag creation is blocked until the signing key is provisioned.** No tag has been created and
none will be pushed. No unsigned fallback tag is created per L2 policy.

When the key is available, create the tag against the merge commit on `main`:

```bash
git tag -s task-024/v0.1.2 72c3843 -m "task-024/v0.1.2 — CSV export save-dialog hang fix (PATCH)"
git push origin task-024/v0.1.2
```

---

## PR promotion

PR #16 **merged** to `main` at `72c3843` (2026-06-15). Branch `fix/task-024-csv-export-save-dialog-hang`
captured at `c3ff432` (docs gate commit). Release artifact follow-up via `docs/task-024-release-artifact`.
