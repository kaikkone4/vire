# Docs gate — task-024-csv-export-save-dialog-hang-fix

**Tier:** L2
**Branch:** fix/task-024-csv-export-save-dialog-hang (PR #16)
**Date:** 2026-06-15

## Assessment

TASK-024 is a threading bugfix inside the Tauri IPC layer: `export_report_csv` was converted from a
synchronous (main-thread) command to an `async` command so the blocking save dialog runs off the main
thread and the IPC promise always resolves. No new user-visible feature, no new configuration, no new
export column, no change to `docs/`.

### README.md — manual verification step 6 expanded

The existing step 6 described CSV export as "export CSV to a selected local destination". That was
accurate before the fix but insufficient now: the primary regression surface is the cancel path and
re-entry (previously, any dialog interaction froze the app). Step 6 is updated to spell out the three
sub-cases Janne must verify locally:

- **Success** — file written, `Exported N entries.` alert, app responsive.
- **Cancel** — no file, no error, app fully responsive (no endless loading).
- **Re-entry** — dialog opens and resolves normally on a second invocation.

This mirrors the smoke matrix (S1/S2/S4) in `smoke.md` and `qa.md §6` so Janne has the acceptance
checklist in the standard README location without needing to navigate to the openspec directory.

S3 (write-failure path) is covered by automated tests (`export_csv_repo_surfaces_write_failure_as_error_not_a_hang`)
and the manual instruction "read-only / unwritable target" is omitted from the README because triggering
it requires environment setup that would distract from the primary regression check; the test suite
is the gate for that path.

### docs/ — no changes

`docs/langfuse-local-setup.md` and `docs/backup-restore.md` are unrelated to CSV export.
The threading change (async command, `spawn_blocking`) is an internal/IPC implementation detail
with no operator action, configuration change, or new runtime dependency. No new operator or user
guide section is warranted.

## Coherence check

| Requirement | README | docs/ | Implementation |
|---|---|---|---|
| Save dialog runs off main thread; app never hangs | ✓ step 6 (success + responsive) | — | `async fn export_report_csv` + `spawn_blocking` (lib.rs:169–188) |
| Cancel resolves as no-export, no hang | ✓ step 6 (cancel sub-case) | — | `let Some(dest) = dest else { return Ok(None); }` (lib.rs:182) |
| Re-entry works after success or cancel | ✓ step 6 (re-entry sub-case) | — | `spawn_blocking` per invocation, no retained handle (QA SC-8) |
| Write failure surfaced, not hung | — (automated test gate; omitted from README per above) | — | `export_csv_repo` propagates `fs::write` `Err` (lib.rs:187) |
| TASK-023 neutralization/escaping unchanged | — (internal security control) | — | `csv_escape`/`csv_formula_neutralized`/`export_csv_repo` byte-for-byte unchanged (SW-4 review §2) |
| Manual verification step describes CSV export | ✓ step 6 (expanded) | — | — |

No drift between OpenSpec spec (`specs/csv-export/spec.md`) and implementation detected.
All spec requirements (R1–R3) are covered by existing tests and/or the manual smoke matrix.

## Docs not required at L2 / deferred

- Maintenance runbook (L3 only)
- Compliance/audit docs (L3 only)
- DEC-024 ADR entry — off-main-thread dialog is recorded in `arch-review.md §7`; BA ratification
  is out of scope for the SW-6 docs gate
