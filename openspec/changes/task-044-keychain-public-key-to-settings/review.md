# SW-4 Code Review — TASK-044

**Verdict:** ESCALATE

**Review boundary:** TASK-044 implementation commit `1160f04` (`src-tauri/src/settings/mod.rs`,
`src-tauri/src/settings/tests.rs`, `src-tauri/src/lib.rs`) plus the branch/base scope note.

## Blocking issues

1. **The resolver still creates mixed stored/env credential pairs when exactly one stored field is
   absent.** In `src-tauri/src/settings/mod.rs:274-294`, each field independently falls back to its
   environment variable before the pair is assembled. Therefore:
   - settings public present + Keychain secret absent + env secret present returns a mixed pair;
   - settings public absent + Keychain secret present + env public present returns a mixed pair.

   The second case is also the documented existing-install state until the user re-saves, so a stale
   Keychain secret can be paired with an environment public key. This contradicts the requested
   no-mixed-pair behavior and the T3 plan in `design.md`. The test at
   `src-tauri/src/settings/tests.rs:607-672` verifies rollback restored a complete prior pair; it
   never exercises either actual half-state, so the gap is currently untested.

2. **The implementation does not uphold the stated atomic set/clear invariant across store
   failures.**
   - On set, rollback errors are discarded at `src-tauri/src/settings/mod.rs:397-404`. If restoring
     or deleting the SQLite public row fails after the Keychain write fails, the function returns
     the Keychain error while leaving a half-written pair.
   - On clear, the Keychain secret is deleted before the SQLite row at
     `src-tauri/src/settings/mod.rs:417-420`. If the SQLite delete fails, the secret is already gone
     and there is no compensation path, leaving only the public row.

   The tests cover Keychain failure before the SQLite clear
   (`src-tauri/src/settings/tests.rs:727-741`) but not SQLite failure after Keychain mutation or
   failed set compensation. The design explicitly treats SQLite failure as “effectively never”;
   that assumption conflicts with the hard requirement that every set/clear result leave both
   stores present or both absent. The failure contract or compensation design needs an architect
   decision before implementation can pass.

## Suggestions

- Add direct half-state resolver tests for both directions, with both environment keys populated.
- Add injectable/failing SQLite-operation coverage for set compensation and the second clear step.

## Escalation to SW Architect

Reconcile the hard cross-store atomicity/no-mixed-pair invariant with the chosen two-store design.
Specify pair-level environment fallback semantics and an explicit outcome for SQLite failure after a
Keychain mutation; the current “local SQLite effectively never fails” rationale is not a guarantee.

## Checks

- `cargo test settings::tests` — PASS (27 tests).
- `cargo fmt --all -- --check` — PASS.
- `npm run build` — PASS.
- `git diff --check 1160f04^ 1160f04` — PASS.
- `cargo clippy --tests -- -D warnings` — FAIL only on pre-existing untouched findings documented
  in the handoff (`langfuse/importer.rs`, `langfuse/tests.rs`, `lib.rs`).
- Commit message for `1160f04` is scoped and complete.
- PR #32 metadata could not be fetched because `api.github.com` was unreachable; local branch scope
  confirms TASK-044 is one implementation commit on top of the TASK-043 tip, so the large
  `main...HEAD` diff is inherited base history rather than TASK-044 scope.

## Changed paths

- Added `openspec/changes/task-044-keychain-public-key-to-settings/review.md`.
- Updated `openspec/changes/task-044-keychain-public-key-to-settings/handoff.md`.
