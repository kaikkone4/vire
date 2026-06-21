# Handoff — TASK-044 keychain-public-key-to-settings (F2a)

- **Change dir**: openspec/changes/task-044-keychain-public-key-to-settings/
- **Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR (draft) #32
- **Phase / gate**: SW-2 Backend **complete** (2026-06-21) — Addendum Decisions 1–2 implemented
- **Tier**: L2

## Last gate result

**SW-2 implementation complete** (2026-06-21). Implemented `arch-review.md` Addendum (2026-06-21)
Decisions 1–2, all inside `src-tauri/src/settings/mod.rs` + `tests.rs` (repo signatures unchanged ⇒
`lib.rs` untouched). Prior gates still hold: SW Architect PASS (escalation resolved); SW-5 Security
PASS (`sec.md`). SW-4 ESCALATE (`review.md`) now resolved in code — re-run SW-4 after SW-3.

- **Decision 1** — `resolve_credentials` now reads both stores strictly and matches the pair as a
  unit: both stores ⇒ stored pair; neither ⇒ env pair iff both env keys set; exactly one store ⇒
  `None` (env not consulted for the missing side). Mixed-source pair is structurally impossible.
- **Decision 2** — `set` keeps order but stops swallowing rollback errors → `INCONSISTENT_SET_ERR`
  when SQLite compensation fails. `clear` reordered to **SQLite-first** (capture prior → delete
  public → delete secret → restore public on Keychain-delete failure → `INCONSISTENT_CLEAR_ERR` if
  restore fails). Fragile Keychain mutation last in both; residual one-store windows inert by D1.

## Active blockers

- None.

## Exact next action

Route to **SW-3** (sw-qa-engineer) to re-verify the scenario matrix + the new tests, then SW-4/SW-5
recheck. SW-6 RELEASE.md must still note the one-time re-save (existing installs) **and** the
pair-level env behavior change (env is now a whole-pair dev override).

## Required files (read these, not the whole tree)

- `src-tauri/src/settings/mod.rs` — D1 resolver (`resolve_credentials`), D2 set/clear, `INCONSISTENT_*`
- `src-tauri/src/settings/tests.rs` — 33 tests incl. T-PAIR-A/B/C, T-SET-ROLLBACK-FAIL (×2),
  T-CLEAR-COMP, T-CLEAR-SQLITE-FAIL (trigger-based SQLite-failure injection)
- `arch-review.md` — Addendum (2026-06-21) = the binding design + test matrix
- `specs/langfuse-credential-storage/spec.md` — requirements/scenarios

## Notes carried forward

- Checks (from `src-tauri/`): `cargo test settings::tests` PASS (33), `cargo fmt --all -- --check`
  PASS, clippy clean on touched files, `npm run build` PASS. Pre-existing untouched clippy lints
  (`langfuse/importer.rs`, `langfuse/tests.rs`, `lib.rs`) remain out of scope.
- `clear_aborts_before_settings_when_keychain_delete_fails` replaced by `t_clear_comp_...` (mechanism
  now restore-after, not abort-before; same "prior pair preserved" guarantee).
- Accepted behavior change: a dev with exactly one key in a store + the other in env now gets `None`
  (env is a whole-pair override). Note in code comment + RELEASE.
- Manual T6 (1 Keychain prompt on fresh macOS launch) still needs human verification.
