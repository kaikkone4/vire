# TASK-044 — Public key → settings (F2a): architecture & implementation design

> Design only. No implementation. Realizes F2a from TASK-041 `design.md` §3 under the SW-5
> sign-off + conditions C1–C4 in TASK-041 `sec.md`.

## 1. Where this sits

```
Renderer (Settings)  ──IPC──►  Rust core  ──►  resolve_credentials
  set_langfuse_secret(pub,sec)   (lib.rs)        ├─ public  ◄── SQLite settings.langfuse_public_key   [MOVED HERE]
  clear_langfuse_secret()                        └─ secret  ◄── Keychain  langfuse_secret_key (dev.vire.app) [STAYS]
  get_langfuse_settings → flags
```

Today **both** keys are Keychain items (`settings/mod.rs:23-24`, `secret_store.rs`). F2a moves
only the **public** key into the existing k/v `settings` table (`lib.rs:124`,
`read_setting`/`write_setting` at `mod.rs:81-101`), the same store and access pattern already
used for `base_url`/`source`/`environments`/`langfuse_enabled` (`mod.rs:28-31`). The secret
key and the entire `SecretStore`/Keychain layer are **unchanged**.

**Net effect:** one Keychain item remains ⇒ one ACL ⇒ **one** fresh-launch prompt (was two).
F2a is independent of F1 (TASK-042 signing); together, F1 makes that single grant persist
across builds.

## 2. Storage relocation

- Add `const KEY_PUBLIC_KEY: &str = "langfuse_public_key";` to the `settings`-table key block
  (`mod.rs:28-34`). The string may equal the legacy Keychain account name; they are different
  namespaces (a `settings` row key vs a Keychain account) — keep the Rust constants distinct
  (`KEY_PUBLIC_KEY` for the row, `PUBLIC_KEY_ACCOUNT` retained only for legacy cleanup, §4).
- The public key is written/read via the existing `write_setting`/`read_setting` helpers — no
  new persistence code, no new table, no `add_column_if_absent`/migration.

## 3. Two-store atomic set/clear + resolver (C1, C4 — load-bearing)

The credential pair now spans **two stores**. The DEC-026 invariant must hold across both:
after any `set`/`clear`, the pair is **EITHER both-present (the new or the prior pair) OR
both-absent — never exactly one store populated**, because a half state lets
`resolve_credentials` pair a real value from one store with an **env-fallback** value for the
other (`mod.rs:233-244`, `330-359`). (The hazard only materializes when the marked dev env
vars `VIRE_LANGFUSE_*` / `LANGFUSE_*` are set; the invariant must still hold — it is exactly
what the current single-store rollback protects.)

### 3.1 `set_langfuse_secret_repo(conn, secrets, public, secret)`  — public-first, rollback the cheap store

Mirror the **existing** rollback structure (`mod.rs:317-361`), swapping the public store from
Keychain to SQLite. Order so the operation that may need rollback is the **local, reliable,
prompt-free** one (SQLite):

1. Validate both non-empty (unchanged).
2. Capture `prior_public = read_setting(conn, KEY_PUBLIC_KEY)`.
3. `write_setting(conn, KEY_PUBLIC_KEY, public)` — SQLite (reliable, no prompt).
4. `secrets.set(SECRET_KEY_ACCOUNT, secret)` — Keychain (may prompt/fail).
5. **On step-4 failure**, restore the public row to its prior state (best-effort, idempotent):
   rewrite `prior_public` when it existed, else delete the row; then return the coarse,
   secret-free error. Result: pair left consistent (both-new or both-prior), never mixed.

Why SQLite-first (not Keychain-first): the rollback target is whichever store was written
first. SQLite rollback is a local rewrite/delete that never prompts and effectively never
fails; a Keychain rollback is a fragile OS call. Writing the **fragile** store (Keychain)
**last** means a step-4 failure leaves only the cheap SQLite rollback to perform.

### 3.2 `clear_langfuse_secret_repo(conn, secrets)` — SQLite-first, compensate the fragile delete

> **SUPERSEDED by `arch-review.md` Addendum (2026-06-21), Decision 2.** The original
> "fragile-store-first, abort-before-second" ordering below is **replaced** by the SQLite-first +
> compensation design, because Keychain-first leaves **no recovery path** when the SQLite delete
> fails after the secret is gone (a deleted secret cannot be restored). The reordered design:

1. Capture `prior_public = read_setting_strict(conn, KEY_PUBLIC_KEY)` — a genuine read failure
   aborts **before any mutation** (both stores untouched ⇒ prior pair).
2. `clear_setting(conn, KEY_PUBLIC_KEY)` — SQLite (reliable) **first**. On failure, abort **before**
   touching the Keychain ⇒ both stores remain ⇒ prior consistent pair (safe).
3. `secrets.delete(SECRET_KEY_ACCOUNT)` — Keychain (fragile) **last**. On failure, **restore the
   public row** from `prior_public` (reliable local write) ⇒ back to the prior pair. If that restore
   itself fails (catastrophic local SQLite failure), return a **distinct, secret-free** error
   (`INCONSISTENT_CLEAR_ERR`) — never swallow it.
4. Best-effort `secrets.delete(PUBLIC_KEY_ACCOUNT)` to remove any **legacy** Keychain public item
   (§4); idempotent, no-op when absent.

Ordering rationale: the fragile Keychain mutation is performed **last in both set and clear**, and
its failure is compensated by a reliable local SQLite op. The realistic failure (denied prompt /
locked keychain) is therefore **recoverable** (prior pair restored). The only unrecoverable window
requires a local SQLite op to fail immediately after another succeeded (catastrophic) — surfaced via
the distinct error and rendered inert by the pair-level resolver (§3.3: a one-store state ⇒ `None`).

### 3.3 `resolve_credentials(conn, secrets, env)` — preserve "no silent env downgrade on a real read failure"

- **public**: read from `settings` first, env fallback **only when the row is genuinely
  absent**. The existing `read_setting` swallows DB errors into `None`
  (`mod.rs:81-92`, `.ok().flatten()`) — which on the **credential path** would silently enable
  the env fallback on a real DB read failure, violating the symmetry of the secret-side
  contract (`mod.rs:207-210`). **Use an error-propagating read for the credential public key**
  (e.g. a `read_setting_strict` that returns `Result<Option<String>>`, mapping a true DB error
  to `Err` and `Ok(None)` only for an absent row). Keep the lenient `read_setting` for
  genuinely-optional config (`base_url` etc.).
  *Altitude note:* because the public key is **non-secret**, the security impact of not doing
  this is limited (a wrong/env public key merely fails Basic-Auth — no disclosure). The strict
  read is for **correctness/contract symmetry** (honoring C1's spirit), not a secrecy control;
  it is low-cost and recommended, not a security blocker.
- **secret**: unchanged — `secrets.get(SECRET_KEY_ACCOUNT)?` (the `?` already short-circuits a
  real Keychain read failure to `Err`, never env).
- **Pair rule — SUPERSEDED by `arch-review.md` Addendum (2026-06-21), Decision 1: env fallback is
  PAIR-LEVEL, not per-field.** Read both stores strictly, then match the pair as a unit: both stores
  present ⇒ stored pair (env never consulted); both stores absent ⇒ env pair iff **both** env keys
  present, else `None`; **exactly one store present ⇒ `None`** (the lone stored key is discarded; env
  is **not** consulted for the missing side). This makes "no mixed-source pair" structural — a
  half-state (incl. the existing-install state: settings-public absent + Keychain-secret present)
  can never combine a stored key with an env key. The earlier per-field draft below is replaced.
- `resolve_credentials` now needs the `conn` (it previously took only `secrets`, `env`); thread
  the `Connection` through `resolve_config_with` (`mod.rs:215-223`).

### 3.4 Renderer view & presence flags (C2)

`get_langfuse_settings_repo` (`mod.rs:258-273`):

- `has_public_key` ← **`read_setting(conn, KEY_PUBLIC_KEY).is_some()`** (was a Keychain read).
  This removes the public-key Keychain access that contributed the second prompt at Settings
  render.
- `has_secret_key` ← Keychain presence (unchanged) — the secret stays **presence-flag-only**
  (SEC-009 / C2). Do **not** add the secret value to any view.
- The public-key **value** is non-secret and *may* be surfaced (C2 permits), but this task
  **keeps the `has_public_key: bool` contract** to stay proportional and avoid frontend churn
  (out of scope, §proposal). The `LangfuseSettings` struct and `src/main.ts` are unchanged.

## 4. Existing-install data continuity (decision)

After F2a the code reads the public key from `settings`; an already-configured user's public
key sits in the **legacy Keychain item** and would read as absent ⇒ `resolve_credentials`
returns `None` ⇒ import/Test break until action.

**Decision — M-c "explicit re-save" (recommended, chosen).** No automatic value migration. The
user re-enters credentials once via the existing two-field form after upgrade; `set` writes
public→settings + secret→Keychain fresh. `set` and `clear` issue a **best-effort idempotent
`secrets.delete(PUBLIC_KEY_ACCOUNT)`** so the legacy Keychain public item is removed and never
read again. Documented in RELEASE.md and handoff so SW-3 QA expects the one-time re-save.

Rejected alternatives:

- **M-a (auto-migrate on startup: read legacy Keychain public → write settings → delete).**
  Reading the legacy item **triggers the very ACL prompt we are removing** — counterproductive.
  Rejected.
- **M-b (lazy: keep reading the legacy Keychain public until next save).** Keeps the second
  prompt alive for existing users until they re-save — defeats the goal in the interim.
  Rejected.

M-c is proportional for a **v0.x, Mac-first, effectively single-user** prototype where credential
re-entry is a two-field form; it avoids the migration-prompt paradox entirely.

## 5. IPC layer (`lib.rs`)

- `set_langfuse_secret` and `clear_langfuse_secret` currently take **no `State`**
  (`lib.rs:846-858`) — they must gain `state: State<AppState>` and a `db_conn(&state)?` handle
  to read/write the settings public row. **The JS-visible argument shape is unchanged**
  (`{publicKey, secretKey}` / no args), so `src/main.ts:80` needs no edit.
- `get_langfuse_settings` already has `state` (`lib.rs:811-815`) — only its repo body changes.
- No new command, no change to `generate_handler!` registration (`lib.rs:1179`).

## 6. Boundaries & scope (no split)

- All changes are within the **Rust core `settings` module + two IPC signatures**; reuses the
  existing k/v `settings` table; no new component, no network, no IPC payload-shape change, no
  renderer contract change. Per `03_architecture_plan.md` the credential/secret-storage
  component boundary is **not crossed** ⇒ **no task split required**.
- Independent of TASK-042 (F1). F3 ruled out (SW-2: file-based login keychain, no access
  group). No `feedback_to_ba[]` — the DEC-026 pair model and SEC-009 are preserved/reinforced.

## 7. Test plan (for the implementer; verified at SW-3)

Extend `src-tauri/src/settings/tests.rs` (uses `MemorySecretStore` + in-memory `settings`):

- **T1** set writes public→settings, secret→Keychain; `resolve_credentials` returns the pair.
- **T2 (C1 atomic set):** secret (Keychain) write fails ⇒ settings public row restored to its
  prior state (prior value reinstated, or row absent when none) — never a one-store pair.
- **T3 (C1 resolver):** with the dev env vars set, a half state (only settings public, or only
  Keychain secret) does **not** produce a mixed real+env pair; a genuine settings read failure
  short-circuits to `Err` rather than falling through to the env public (§3.3).
- **T4 (C4 atomic clear):** clear with both present ⇒ both absent; Keychain-delete failure ⇒
  abort before settings (both remain); legacy Keychain public item deleted best-effort.
- **T5 (C2 / SEC-009):** the secret value is never returned by any view; `has_secret_key`
  stays presence-only; `secret_is_never_written_to_the_settings_table`
  (`tests.rs:261`) still holds — the **secret** never enters `settings`.
- **T6 prompt-count (manual/QA):** fresh launch + open Settings + Test connection touches the
  Keychain **once** (secret only); the public key path issues **no** Keychain access.
- `cargo test` + `cargo fmt` + `cargo clippy` clean; `npm run build` green (no frontend change).
