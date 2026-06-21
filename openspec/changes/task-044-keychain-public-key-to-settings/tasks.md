# Tasks — TASK-044 keychain-public-key-to-settings (F2a)

Design + constraints: `design.md`, `arch-review.md`, `specs/langfuse-credential-storage/spec.md`.
SEC conditions **C1–C4** (TASK-041 `sec.md`) are verified at this task's SW-5 gate. Backend-only;
**no frontend change** (the JS IPC contract is preserved). Do the minimum — no extra refactors,
no UI value-surfacing, no signing work (that is TASK-042). Check off as you go.

## Workstream A — Storage relocation + two-store atomicity (backend)

- [x] A1. Add `const KEY_PUBLIC_KEY: &str = "langfuse_public_key";` to the `settings`-table key
  block (`settings/mod.rs:28-34`). Keep `PUBLIC_KEY_ACCOUNT` only for legacy Keychain cleanup
  (§4). Add a strict, error-propagating settings read for the credential path (e.g.
  `read_setting_strict -> Result<Option<String>>`) and a `clear_setting`/row-delete helper;
  leave the lenient `read_setting` for optional config (`design.md` §3.3).
- [x] A2. `set_langfuse_secret_repo(conn, secrets, public, secret)` — public-first, rollback the
  cheap store (`design.md` §3.1): validate non-empty → capture prior settings public →
  `write_setting(KEY_PUBLIC_KEY)` → `secrets.set(SECRET_KEY_ACCOUNT)`; on the Keychain failure
  restore the settings row to its prior state and return the coarse secret-free error. Also
  best-effort idempotent `secrets.delete(PUBLIC_KEY_ACCOUNT)` to drop any legacy Keychain
  public item (§4). **(C1)**
- [x] A3. `clear_langfuse_secret_repo(conn, secrets)` — fragile store first, abort-before-second
  (`design.md` §3.2): `secrets.delete(SECRET_KEY_ACCOUNT)` first (on error return before
  touching settings) → delete the settings public row → best-effort `secrets.delete(
  PUBLIC_KEY_ACCOUNT)` (legacy). Idempotent. **(C4)**
- [x] A4. `resolve_credentials` reads public from `settings` (strict read, env fallback only on
  genuine absence) + secret from Keychain (unchanged); a real read failure of either store
  short-circuits to `Err`, never the env fallback. Thread `conn` through
  `resolve_config_with` (`mod.rs:215-223`). **(C1)**

## Workstream B — Read-back view + IPC wiring (backend)

- [x] B1. `get_langfuse_settings_repo`: `has_public_key` ← settings row presence (no Keychain
  read); `has_secret_key` ← Keychain presence (unchanged). Secret stays presence-flag-only;
  `LangfuseSettings` struct unchanged. **(C2)**
- [x] B2. `lib.rs`: `set_langfuse_secret` and `clear_langfuse_secret` gain `state:
  State<AppState>` + `db_conn(&state)?` (`lib.rs:846-858`). JS argument shape unchanged
  (`{publicKey, secretKey}` / none) ⇒ **no `src/main.ts` edit**. No `generate_handler!` change.
- [x] B3. **C3 (advisory):** confirm the public key is not newly written into exports, logs, or
  support bundles — it sits in `settings` for resolution only; no broadcast of the identifier.

## Workstream C — Tests (backend; `design.md` §7)

- [x] C1. T1 set→resolve happy path; T2 atomic-set rollback on Keychain failure (C1).
- [x] C2. T3 resolver: half-state + dev env set produces no mixed pair; strict-read failure
  short-circuits to `Err`, not env (C1).
- [x] C3. T4 atomic clear: both-removed; Keychain-delete failure aborts before settings (C4);
  legacy Keychain public deleted best-effort.
- [x] C4. T5 SEC-009/C2: secret never returned by any view; secret never written to `settings`
  (extends `tests.rs:261`); `has_secret_key` presence-only.
- [x] C5. `cargo test` (169 pass, incl. 27 settings / 5 new TASK-044) + `cargo fmt --check` clean;
  `npm run build` green (no frontend change). `cargo clippy -D warnings`: **my changed files are
  clippy-clean**; the only `-D warnings` failures are **pre-existing** pedantic lints from the
  local clippy 1.95.0 (no toolchain pin) in untouched files — `langfuse/importer.rs`,
  `langfuse/tests.rs`, `lib.rs:1155` (`io_other_error`), `lib.rs:1713`
  (`assertions_on_constants`). Out of scope for TASK-044; flagged for separate cleanup.

## Cross-cutting verification (before SW-3 QA)

- [x] V1. Re-read all changed files; confirm C1–C4 hold end to end and the pair invariant is
  preserved across both stores.
- [x] V2. **Prompt-count check (T6):** verified **structurally** — `get_langfuse_settings_repo`
  reads `has_public_key` from the `settings` row (no Keychain), and `resolve_credentials` reads the
  public key from `settings`; the only Keychain access on the credential paths is the single secret
  read. Live macOS prompt-count (2 → 1) is a manual confirmation at SW-3 QA.
- [~] V3. **Handoff** notes the **one-time re-save** for existing installs and that no
  auto-migration was added (§4). **RELEASE.md** note is deferred to the **SW-6** release-prep gate
  (per this repo's cadence — RELEASE.md entries are added at SW-6, e.g. `c9196bc` task-034); flagged
  in the handoff so SW-6 carries the line.
