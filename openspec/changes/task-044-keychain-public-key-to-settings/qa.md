# QA Report — TASK-044 keychain-public-key-to-settings (SW-3 recheck)

**Date**: 2026-06-21  
**Branch**: feat/task-044-keychain-public-key-to-settings  
**Commits in scope**: 1160f04 (initial impl) + db2eeef (Addendum D1/D2 fix)  
**Tier**: L2  
**Verdict**: **PASS**

---

## 1. Scope boundary check

Changes across both commits affect only:

- `src-tauri/src/settings/mod.rs` — resolver, set, clear, `INCONSISTENT_*` constants, module doc
- `src-tauri/src/settings/tests.rs` — 33 tests (up from 27; D1/D2 additions)
- `src-tauri/src/lib.rs` — `State` handle added to `set_langfuse_secret` (IPC body only, no signature change)
- `openspec/changes/task-044-*/` — artifacts only

`src/main.ts` is untouched in both commits (IPC shape unchanged). Branch diff vs `main` includes
task-043 files; the PR targets `main` and reduces to task-044-only once task-043 merges.

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
| S5 | Clear removes both stores (happy path) | **SQLite public row deleted first**, then Keychain secret; legacy public item cleaned | T4: `clear_removes_both_stores_and_deletes_legacy_keychain_public` | ✅ PASS |
| S5a | Keychain delete fails → settings public row restored (prior pair preserved) | `clear` restores `prior_public` into settings on Keychain-delete failure; resolver returns prior pair | T-CLEAR-COMP: `t_clear_comp_failed_secret_delete_restores_and_preserves_the_prior_pair` | ✅ PASS |
| S5b | SQLite delete fails → abort before Keychain delete (both stores remain) | If `clear_setting(KEY_PUBLIC_KEY)` fails, function returns `Err` before `secrets.delete` is reached | T-CLEAR-SQLITE-FAIL: `t_clear_sqlite_fail_aborts_before_keychain_and_keeps_both_stores` | ✅ PASS |
| S6 | Store read failure does not enable env fallback | `read_setting_strict` short-circuits with coarse secret-free error; env path never taken | T3: `settings_read_failure_short_circuits_and_never_downgrades_to_env_public` | ✅ PASS |
| S6a | Lone stored public + env secret → no credentials (T-PAIR-A) | Exactly-one-store ⇒ `None`; env not consulted for missing side | `t_pair_a_lone_settings_public_with_env_yields_no_credentials` | ✅ PASS |
| S6b | Lone stored secret + env public → no credentials (T-PAIR-B) | Existing-install upgrade hazard; stale Keychain secret + env public ⇒ `None` | `t_pair_b_lone_keychain_secret_with_env_yields_no_credentials` | ✅ PASS |
| S6c | Both stores absent + both env keys → env pair resolves (T-PAIR-C) | Dev-mode regression; env is whole-pair override when neither store holds its key | `t_pair_c_both_stores_absent_resolves_the_env_pair` | ✅ PASS |
| S7 | One Keychain prompt structurally (2→1) | `has_public_key` derived from `settings` row; no Keychain call for public key on settings view | Code inspection + T1 resolver round-trip | ✅ PASS |

### C1–C4 from TASK-041 sec.md (Addendum-revised descriptions)

| Condition | Verification | Result |
|---|---|---|
| C1: no mixed-source pair | `resolve_credentials` matches pair as a unit (match block); T-PAIR-A/B prove lone-store ⇒ `None`; T-PAIR-C proves env pair when neither store present; `failed_replacement_*` proves no env fill after failed replacement | ✅ PASS |
| C2: secret never in plaintext settings | `LangfuseSettings` has no secret field; `set_langfuse_secret_repo` never calls `write_setting` with secret; T5 (`secret_is_never_written_to_the_settings_table`) confirms; `INCONSISTENT_*` error strings contain no key material | ✅ PASS |
| C3: strict DB read — no env fallback on real failure | `read_setting_strict` uses `.optional()?`; `resolve_credentials` maps `Err` via `map_err` then `?` before the env branch; T3 (`settings_read_failure_short_circuits_…`) confirms | ✅ PASS |
| C4: atomic clear — **SQLite-first** + compensation | `clear_langfuse_secret_repo` clears SQLite first; on SQLite failure aborts before Keychain; on Keychain failure restores public row from `prior_public`; T-CLEAR-COMP + T-CLEAR-SQLITE-FAIL confirm both paths | ✅ PASS |

### Decision 2: `INCONSISTENT_*` error surface (compensation failure)

| # | Test | Assertion | Result |
|---|---|---|---|
| T-SET-ROLLBACK-FAIL (prior pair) | `t_set_rollback_fail_prior_pair_returns_inconsistent_set_err` | Keychain set fails + trigger ABORTs restore UPDATE ⇒ exact `INCONSISTENT_SET_ERR`, secret-free | ✅ PASS |
| T-SET-ROLLBACK-FAIL (no prior) | `t_set_rollback_fail_no_prior_returns_inconsistent_set_err` | Keychain set fails + trigger ABORTs rollback DELETE ⇒ `INCONSISTENT_SET_ERR`, secret-free | ✅ PASS |
| T-CLEAR-COMP (restore succeeds) | `t_clear_comp_failed_secret_delete_restores_and_preserves_the_prior_pair` | Keychain delete fails + restore succeeds ⇒ raw Keychain error (not `INCONSISTENT_CLEAR_ERR`); both stores hold prior pair | ✅ PASS |

### Additional checks

| Check | Result |
|---|---|
| Legacy Keychain public-key item best-effort deleted on set | `let _ = secrets.delete(PUBLIC_KEY_ACCOUNT)` after successful secret write; T1 asserts `secrets.get(PUBLIC_KEY_ACCOUNT).is_none()` | ✅ PASS |
| Legacy item best-effort deleted on clear | `let _ = secrets.delete(PUBLIC_KEY_ACCOUNT)` after Keychain secret deleted; T4 asserts absent | ✅ PASS |
| IPC / renderer contract unchanged | `set_langfuse_secret(state, public_key, secret_key)` — JS call shape `{publicKey, secretKey}` unchanged; `clear_langfuse_secret` unchanged; `get_langfuse_settings` shape unchanged | ✅ PASS |
| No frontend drift | `src/main.ts` absent from both task-044 commit file lists | ✅ PASS |
| Existing installs one-time re-save note carried | No auto-migration; behavior: existing-install half-state (public absent, secret present) resolves to `None` (T-PAIR-B) → user prompted to re-save once → SW-6 RELEASE.md must document | ✅ DOCUMENTED |
| Env pair-level behavior change carried | Dev who kept one key in store + other in env now gets `None`; accepted behavior change; code comment in `resolve_credentials`; SW-6 RELEASE.md must note | ✅ DOCUMENTED |
| Disabled Test connection short-circuits before Keychain | `TripwireSecretStore` test: `disabled_test_connection_plan_short_circuits_without_touching_the_secret_store` | ✅ PASS |
| Keychain read failure distinguished from absent credential | `keychain_read_failure_is_propagated_not_masked_as_missing_credentials` | ✅ PASS |
| Debug redaction of ImporterConfig with settings-sourced credentials | `settings_sourced_credentials_stay_redacted_in_debug` | ✅ PASS |
| Module doc matches §2.3 contract verbatim | `mod.rs:16–30` carries the exact cross-store consistency contract from Addendum §2.3 | ✅ PASS |

---

## 3. Test run results

```
cargo test settings::tests  (src-tauri/)
  33 passed; 0 failed  (27 pre-addendum + 6 new: T-PAIR-A/B/C, T-SET-ROLLBACK-FAIL ×2, T-CLEAR-COMP, T-CLEAR-SQLITE-FAIL)
  0 filtered-out settings failures

cargo fmt --all -- --check
  PASS (no formatting drift)

cargo clippy --lib --tests -- -D clippy::correctness -A clippy::all
  0 new errors on touched files

npm run build
  ✓ tsc + vite — no errors; 32.8 kB bundle (gzip 10.9 kB)
```

---

## 4. Manual T6 note (structural only — macOS required)

T6 (fresh launch → 1 Keychain prompt) requires macOS runtime observation; cannot be executed in CI.
Structural coverage:
- `get_langfuse_settings_repo`: `has_public_key` reads from `settings` via lenient `read_setting` (zero Keychain calls); only `has_secret_key` calls `secrets.get(SECRET_KEY_ACCOUNT)`.
- T1 exercises the full resolver round-trip with exactly one Keychain `get` on the secret account.

Functional verification must be performed by a human on a macOS install with real Keychain.

---

## 5. Pre-existing clippy notes (out of scope)

`cargo clippy -D clippy::correctness` is clean on task-044 touched files. Pre-existing pedantic-lint
warnings in untouched files (`langfuse/importer.rs`, `langfuse/tests.rs`, `lib.rs`) remain out of
scope.

---

## Gate verdict: **PASS → route to SW-4 (Code Review) ∥ SW-5 (Security) in parallel**
