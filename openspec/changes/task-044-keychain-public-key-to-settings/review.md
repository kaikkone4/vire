# SW-4 Code Review — TASK-044 recheck

**Verdict:** PASS

**Review boundary:** Full TASK-044 implementation in commits `1160f04` + `db2eeef`, with focused
recheck of the prior SW-4 escalation and the architect addendum.

## Blocking issues

None.

## Findings

- Pair-level resolution is now structural at `src-tauri/src/settings/mod.rs:307-328`: stored
  credentials are used only as a complete pair, env is consulted only when both stores are absent,
  and either half-state resolves to `None`. Tests cover both half-state directions and the complete
  env fallback at `src-tauri/src/settings/tests.rs:729-784`.
- Set compensation no longer swallows SQLite rollback failures
  (`src-tauri/src/settings/mod.rs:427-445`). Both restore and delete rollback failures return the
  distinct, secret-free `INCONSISTENT_SET_ERR`, covered at
  `src-tauri/src/settings/tests.rs:865-902`.
- Clear is SQLite-first and aborts before Keychain mutation on SQLite failure
  (`src-tauri/src/settings/mod.rs:463-470`). A Keychain delete failure restores the captured public
  key (`src-tauri/src/settings/mod.rs:470-482`), with both paths covered at
  `src-tauri/src/settings/tests.rs:904-973`.
- Renderer IPC names and payloads remain unchanged. The added Tauri `State` argument is injected
  server-side (`src-tauri/src/lib.rs:847-868`); `src/main.ts` is unchanged.

## Suggestions

- Add a direct test for the `INCONSISTENT_CLEAR_ERR` branch at
  `src-tauri/src/settings/mod.rs:475-482` by combining the delete-failing secret store with an
  aborting restore trigger. This is non-blocking because the architect-required clear matrix covers
  successful compensation and pre-Keychain SQLite failure, and the untested branch mirrors the
  already-covered set compensation handling.

## Escalations to SW Architect

None. The prior escalation is resolved by `arch-review.md` Decisions 1 and 2.

## Checks

- `cargo test settings::tests` — PASS, 33 passed.
- `cargo fmt --all -- --check` — PASS.
- `npm run build` — PASS.
- `git diff --check 1160f04^..db2eeef -- src-tauri/src/settings/mod.rs
  src-tauri/src/settings/tests.rs src-tauri/src/lib.rs` — PASS.
- `cargo clippy --lib --tests -- -D warnings` — touched code clean; command fails only on six
  pre-existing findings in untouched `langfuse/importer.rs`, `langfuse/tests.rs`, and `lib.rs`.
- Commit messages for `1160f04` and `db2eeef` are scoped and describe behavior, failure handling,
  tests, and IPC compatibility.
- PR #32 metadata could not be fetched because `api.github.com` was unreachable.

## Changed paths

- `openspec/changes/task-044-keychain-public-key-to-settings/review.md`
- `openspec/changes/task-044-keychain-public-key-to-settings/handoff.md`
