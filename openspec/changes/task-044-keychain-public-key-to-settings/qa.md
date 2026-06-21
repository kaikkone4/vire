# QA Report — TASK-044 keychain-public-key-to-settings (SW-3)

**Date**: 2026-06-21  
**Branch**: feat/task-044-keychain-public-key-to-settings  
**Commit (task-044 scope)**: 1160f04  
**Tier**: L2  
**Verdict**: **PASS**

---

## 1. Scope boundary check

`git show --name-only 1160f04` confirms the single task-044 commit touches only:

- `src-tauri/src/settings/mod.rs`
- `src-tauri/src/settings/tests.rs`
- `src-tauri/src/lib.rs`
- `openspec/changes/task-044-*/` (artifacts only)

`src/main.ts` is **not** in the task-044 commit. Files from task-043 appear in the branch diff vs `main` because the branch was cut from the task-043 SW-6 tip; the PR is already targeted at `main` and will reduce to TASK-044-only once task-043 merges. Confirmed as expected per handoff.

---

## 2. Scenario coverage matrix

### Spec: `specs/langfuse-credential-storage/spec.md`

| # | Scenario | Requirement | Test | Result |
|---|---|---|---|---|
| S1 | Public key persists outside the Keychain | Set writes public→settings, secret→Keychain; no public Keychain item | T1: `set_writes_public_to_settings_secret_to_keychain_and_resolves_the_pair` | ✅ PASS |
| S2 | Secret never enters plaintext settings | Secret value never written to `settings` table or key | T5: `secret_is_never_written_to_the_settings_table` | ✅ PASS |
| S3 | Settings view never returns the secret | Response contains `has_secret_key` / `has_public_key` flags only | `get_settings_returns_presence_flags_never_secret_values`, `presence_flags_false_when_keychain_empty` | ✅ PASS |
| S4 | Failed secret write rolls back the public key (no prior pair) | Settings public row deleted; Keychain untouched; error is secret-free | T2: `secret_write_failure_rolls_back_the_public_key_write` | ✅ PASS |
| S4a | Failed replacement restores prior pair; no mixed-store pairing with env | Prior settings public reinstated; resolver cannot combine env public + Keychain secret | T2/T3: `failed_replacement_restores_the_prior_pair_and_never_mixes_stores_with_env` | ✅ PASS |
| S5 | Clear removes both stores | Keychain secret deleted first; settings row deleted after; legacy public item cleaned | T4: `clear_removes_both_stores_and_deletes_legacy_keychain_public` | ✅ PASS |
| S5a | Clear aborts before settings on Keychain delete failure | Settings row untouched if Keychain delete fails; prior pair preserved | T4: `clear_aborts_before_settings_when_keychain_delete_fails` | ✅ PASS |
| S6 | Store read failure does not enable env fallback | Strict DB read (`read_setting_strict`) short-circuits with coarse error; env path not taken | T3: `settings_read_failure_short_circuits_and_never_downgrades_to_env_public` | ✅ PASS |
| S7 | One Keychain prompt structurally (2→1) | `has_public_key` derived from `settings` row in `get_langfuse_settings_repo`; no Keychain call for public key | Code inspection + T1 resolver round-trip | ✅ PASS |

### C1–C4 from TASK-041 sec.md

| Condition | Verification | Result |
|---|---|---|
| C1: atomic set — rollback on Keychain failure | `set_langfuse_secret_repo` writes settings first, rolls back on `secrets.set` error; T2 + T2/T3 confirm both "no prior" and "replace prior" cases | ✅ PASS |
| C2: secret never in plaintext settings | `LangfuseSettings` has no secret field; `set_langfuse_settings_repo`/`set_langfuse_secret_repo` never call `write_setting` with a secret value; T5 confirms | ✅ PASS |
| C3: strict DB read — no env fallback on real failure | `read_setting_strict` uses `.optional()?` (propagates `Err`); `resolve_credentials` maps error via `map_err` then `?` before the env branch; T3 confirms | ✅ PASS |
| C4: atomic clear — Keychain first, abort before settings on failure | `clear_langfuse_secret_repo` calls `secrets.delete(SECRET_KEY_ACCOUNT).map_err(…)?` before `clear_setting`; T4 confirms | ✅ PASS |

### Additional checks

| Check | Result |
|---|---|
| Legacy Keychain public-key item best-effort deleted on set | `let _ = secrets.delete(PUBLIC_KEY_ACCOUNT)` after successful secret write; T1 asserts `secrets.get(PUBLIC_KEY_ACCOUNT).is_none()` | ✅ PASS |
| Legacy item best-effort deleted on clear | `let _ = secrets.delete(PUBLIC_KEY_ACCOUNT)` after `clear_setting`; T4 asserts absent | ✅ PASS |
| IPC / renderer contract unchanged | `src/main.ts` not modified in 1160f04; `call('set_langfuse_secret', {publicKey, secretKey})` / `call('clear_langfuse_secret')` arg shapes identical | ✅ PASS |
| No frontend drift | `src/main.ts` absent from task-044 commit file list | ✅ PASS |
| Existing installs one-time re-save | No auto-migration; documented in handoff.md Notes + SW-6 RELEASE.md requirement | ✅ DOCUMENTED |
| Disabled Test connection short-circuits before Keychain | `TripwireSecretStore` test: `disabled_test_connection_plan_short_circuits_without_touching_the_secret_store` | ✅ PASS |
| Keychain read failure distinguished from absent credential | `keychain_read_failure_is_propagated_not_masked_as_missing_credentials` | ✅ PASS |

---

## 3. Test run results

```
cargo test  (src-tauri/)
  169 passed; 0 failed   (27 settings module, 5 new/rewritten for TASK-044)
  3 adversarial passed
  0 doc-tests

npm run build
  ✓ tsc + vite — no errors; 32.8 kB bundle (gzip 10.9 kB)

npx tsc --noEmit
  0 errors

npm run lint
  no lint script present (expected — no ESLint in Cargo/Tauri-only project)
```

---

## 4. Manual T6 note (structural only — macOS required)

T6 (fresh launch → 1 Keychain prompt) is a macOS runtime observation, not executable in CI.
Structural coverage is provided by:
- `get_langfuse_settings_repo` code path: `has_public_key` reads from `settings` via the lenient `read_setting` (no Keychain call); only `has_secret_key` calls `secrets.get(SECRET_KEY_ACCOUNT)`.
- T1 exercises the resolver round-trip with only one Keychain `get` call on the secret account.

Functional verification must be performed by a human on a macOS install with real Keychain.

---

## 5. Pre-existing clippy notes (out of scope)

`cargo clippy -D warnings` is clean on task-044 files. Pre-existing pedantic-lint failures in untouched files (`langfuse/importer.rs`, `langfuse/tests.rs`, `lib.rs:1155/1713`) remain. These are out of task-044 scope and logged for separate cleanup (see handoff.md).

---

## Gate verdict: **PASS → route to SW-4 (Code Review) ∥ SW-5 (Security) in parallel**
