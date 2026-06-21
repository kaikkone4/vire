# Handoff — TASK-044 keychain-public-key-to-settings (F2a)

- **Change dir**: openspec/changes/task-044-keychain-public-key-to-settings/
- **Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR (draft) #32
- **Phase / gate**: SW-3 QA **PASS** (2026-06-21)
- **Tier**: L2

## Last gate result

SW-3 QA **PASS** (2026-06-21). 169 Rust tests pass (27 settings, 5 new/rewritten). All 9 spec
scenarios and C1–C4 have direct test coverage. Build clean. No blockers.

## Active blockers

- None.

## Exact next action

SW-4 (Code Reviewer) **and** SW-5 (Security Agent) in parallel — read `qa.md` for coverage
matrix and `src-tauri/src/settings/mod.rs` + `tests.rs` + `src-tauri/src/lib.rs` for scope.

## Required files (read these, not the whole tree)

- `openspec/changes/task-044-keychain-public-key-to-settings/qa.md` — scenario coverage matrix + test results
- `openspec/changes/task-044-keychain-public-key-to-settings/specs/langfuse-credential-storage/spec.md` — requirements/scenarios
- `src-tauri/src/settings/mod.rs` — implementation (C1–C4 logic)
- `src-tauri/src/settings/tests.rs` — 27 unit tests incl. T1–T5 new/rewritten
- `src-tauri/src/lib.rs` — IPC command wiring (`set_/clear_langfuse_secret` + State/db_conn)

## Notes carried forward

- Task-044 scope is one commit: 1160f04. Branch was cut from task-043 SW-6 tip; PR diff vs main
  includes task-043 changes until that PR merges — TASK-044-only diff is `git show 1160f04`.
- Public key = Basic-Auth username, non-secret (SW-5 sign-off already GRANTED in arch-review.md).
  Secret Keychain-only, presence-flag-only (SEC-009/C2).
- Existing installs re-save once (M-c; no auto-migration). **SW-6 must add the RELEASE.md
  one-time-re-save note.**
- Manual T6 (1 Keychain prompt on fresh macOS launch) requires human verification; structural
  proof is in `get_langfuse_settings_repo` code + qa.md §4.
- Pre-existing clippy pedantic-lint failures in untouched files logged; out of task-044 scope.
