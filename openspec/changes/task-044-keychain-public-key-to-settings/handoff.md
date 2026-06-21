# Handoff — TASK-044 keychain-public-key-to-settings (F2a)

- **Change dir**: openspec/changes/task-044-keychain-public-key-to-settings/
- **Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR (draft) #32
- **Phase / gate**: SW-3 QA **PASS** (2026-06-21, recheck after db2eeef)
- **Tier**: L2

## Last gate result

**SW-3 PASS** (2026-06-21 recheck). Full scenario matrix re-verified against commits 1160f04 +
db2eeef. 33 tests pass (0 fail). `cargo fmt` PASS. Clippy clean on touched files. `npm run build`
PASS. All spec scenarios S1–S7 + C1–C4 + Decision 1/2 addendum tests covered.

Key corrections vs prior qa.md: S5/C4 now correctly reflect **SQLite-first** clear order
(db2eeef); T-PAIR-A/B/C, T-SET-ROLLBACK-FAIL ×2, T-CLEAR-COMP, T-CLEAR-SQLITE-FAIL all added
and verified.

## Active blockers

None.

## Exact next action

Route to **SW-4** (sw-code-reviewer) AND **SW-5** (sw-security-agent) **in parallel**. SW-6
RELEASE.md must note:
1. One-time re-save required for existing installs (public key absent in settings until re-saved).
2. Pair-level env behavior change: dev with exactly one key in a store + other in env now gets
   `None` (env is a whole-pair override only).

## Required files (read these, not the whole tree)

- `src-tauri/src/settings/mod.rs` — D1 resolver, D2 set/clear, `INCONSISTENT_*`, module doc
- `src-tauri/src/settings/tests.rs` — 33 tests
- `src-tauri/src/lib.rs:847–868` — IPC commands (State handle, no signature change)
- `arch-review.md` — Addendum (2026-06-21) = binding design
- `openspec/changes/task-044-keychain-public-key-to-settings/qa.md` — full scenario matrix

## Notes carried forward

- Manual T6 (1 Keychain prompt on fresh macOS launch) still needs human verification on real macOS.
- Pre-existing clippy warnings in untouched files (`langfuse/importer.rs`, `langfuse/tests.rs`,
  `lib.rs`) remain out of scope.
- `clear_aborts_before_settings_when_keychain_delete_fails` replaced by `t_clear_comp_…`
  (mechanism: restore-after, not abort-before; same "prior pair preserved" guarantee).
- Accepted behavior change: dev with exactly one key in a store + other in env now gets `None`.
  Code comment in `resolve_credentials` + RELEASE.md note required.
