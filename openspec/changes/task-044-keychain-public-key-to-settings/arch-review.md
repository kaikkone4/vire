# Architecture review — TASK-044 (public key → settings, F2a)

**Verdict: PASS (design complete; no implementation).** The F2a design is sound, scoped, and
consistent with the existing architecture (DEC-026 credential-pair integrity, SEC-009 secret
non-echo) and with the SW-5 sign-off + conditions C1–C4 carried from TASK-041. The change stays
inside the Rust-core `settings` module and two IPC signatures; **no component boundary is
crossed ⇒ no task split required**, and **no `feedback_to_ba[]`**.

## Scope check

- Task = *create/review the OpenSpec change for F2a; do not implement*. Delivered exactly that:
  `proposal.md`, `design.md`, `tasks.md`, `specs/langfuse-credential-storage/spec.md`, this
  review, `handoff.md`. No source/config change. ✔
- No creep: F1 signing is left to TASK-042; F3 is ruled out (SW-2); UI value-surfacing of the
  public key (C2 permits) is explicitly deferred; the TASK-043 Stream B advisory renumber is
  flagged, not done. ✔

## Architectural findings

1. **Storage relocation is posture-preserving.** The public key (`pk-lf-…`) is the HTTP
   Basic-Auth username (`langfuse/api.rs:81`) and SW-5 has GRANTED that it is non-secret. Moving
   it to the plaintext `settings` k/v table is the **same posture already shipped** for
   `base_url`/`source`/`environments`/`langfuse_enabled` (`mod.rs:28-31`). The table exists
   (`lib.rs:124`); no new table, no migration framework. ✔
2. **Two-store atomicity is the one real risk, and it is addressed (C1, C4).** The pair now
   spans SQLite (public) + Keychain (secret). `design.md` §3 preserves the DEC-026 invariant
   (both-present or both-absent, never one-store) by (a) writing the **fragile** store last and
   rolling back the **cheap** local store on failure (set), and (b) deleting the fragile store
   first and aborting before the local delete (clear). The resolver keeps the "no silent env
   downgrade on a real read failure" contract symmetrically via a strict settings read. ✔
3. **SEC-009 boundary held and reinforced (C2).** The secret stays Keychain-only and
   presence-flag-only; only the (non-secret) public key changes store. The
   `secret_is_never_written_to_the_settings_table` test (`tests.rs:261`) still governs. ✔
4. **Renderer contract is unchanged.** The JS `call('set_langfuse_secret', {publicKey,
   secretKey})` / `clear_langfuse_secret` / `get_langfuse_settings.has_public_key` shape is
   preserved; only `lib.rs:846-858` gains a `State` handle. Minimal blast radius, no frontend
   edit. ✔
5. **Prompt-count goal met (2 → 1).** Removing the public Keychain item removes its ACL and the
   second first-access prompt; the public-key path performs no Keychain access. Independent of
   F1, which later makes the single grant persist across builds. ✔

## Conditions carried into implementation (verified at SW-5 / SW-3, not blockers here)

- **C1** (hard) — atomic set/clear + no mixed-source pair via env fallback across the two
  stores. Realized in `design.md` §3.1/§3.3 and `tasks.md` A2/A4, T2/T3.
- **C2** — secret stays presence-flag-only; only the public key may surface as a value (kept as
  a flag here). `design.md` §3.4, `tasks.md` B1/T5.
- **C3** (advisory) — do not newly write the public key into exports/logs/support bundles.
  `tasks.md` B3.
- **C4** — clear wipes both the settings public row and the Keychain secret. `design.md` §3.2,
  `tasks.md` A3/T4.

## Design decisions flagged for the implementer

- **Existing-install continuity = M-c "explicit re-save"** (no auto-migration; a startup
  migration would have to *read* the legacy Keychain public item and thereby trigger the very
  prompt being removed). `design.md` §4. SW-3 should expect a one-time re-save; RELEASE.md must
  document it.
- **Strict settings read on the credential path** is a correctness/contract-symmetry measure,
  not a secrecy control (the public key is non-secret); recommended and low-cost. `design.md`
  §3.3.

## feedback_to_ba[]

None. The BA-flow architecture plan (`03_architecture_plan.md` credential/secret-storage
intent) and DEC-026/SEC-009 are preserved and reinforced. No escalation to ba-architect.

## ID-collision note (recorded, not actioned)

TASK-044 is now **F2a** (this change). The earlier **TASK-043 Stream B** (Tauri/GTK advisory
bump) collides with the originally-proposed numbering and must be **renumbered to a later free
integer in its own task** — out of scope here and not modified by this change.

---

# Addendum — SW-4 escalation resolution (2026-06-21)

**Verdict: PASS (design decided — no task split, no BA escalation). Route to SW-2.**

SW-4 Code Review returned **ESCALATE** (`review.md`) and SW-5 Security concurred (`sec.md` §3) on
two cross-store correctness gaps in the *implementation* (commit `1160f04`). This addendum gives the
binding architecture decision. It **supersedes finding #2's clear-ordering choice above** (the
original "delete the fragile store first and abort" for clear) — see Decision 2. Decision 1 was
already required by C1 but is now pinned to an exact match table. All changes stay inside
`src-tauri/src/settings/mod.rs` + `tests.rs`; repo signatures are unchanged so `lib.rs` (verified
`lib.rs:847-867`, `db_conn` already threaded) needs no edit. **No component split.**

These two decisions are coupled: **the resolver (Decision 1) renders any half-populated pair inert,
which is what makes the residual two-store atomicity windows (Decision 2) safe** — a half-state is
at worst "credentials not configured," never a usable or mixed-source credential. The storage layer
therefore does not need true cross-store atomicity to satisfy DEC-026.

## Decision 1 — Pair-level env fallback (Blocker 1)

**Problem.** `resolve_credentials` (`mod.rs:267-296`) falls back to env **per field**, so when
exactly one store is genuinely absent the present field is stored and the absent field is env → a
**mixed-source pair**. Reachable cases: settings-public present + Keychain-secret absent + env
secret set ⇒ stored-public + env-secret; and (the documented existing-install upgrade state)
settings-public absent + Keychain-secret present + env public set ⇒ env-public + stale-Keychain
secret.

**Decision.** Env is a **pair-level (all-or-nothing) alternative**, consulted only when **neither**
store holds its key — matching the `from_env` "both keys present" contract. Read both stores
**strictly** (error-propagating; a genuine read failure stays `Err`, never env — C3 unchanged), then
match the pair as a unit:

| settings public | Keychain secret | Result |
|---|---|---|
| present | present | **stored pair** (env never consulted) |
| absent | absent | env pair iff **both** env keys present, else `None` |
| present | absent | **`None`** — lone stored key discarded; env **not** consulted |
| absent | present | **`None`** — lone stored key discarded; env **not** consulted |

"No mixed-source pair" becomes a **structural property of the match**, not a per-field accident. The
existing-install half-state resolves to `None` regardless of env → import/Test reports *not
configured* → user re-saves once (M-c) → both stores set → works. Required semantics (replacing the
per-field match at `mod.rs:274-295`):

```text
let public_stored = read_setting_strict(conn, KEY_PUBLIC_KEY)
        .map_err(|_| "could not read the Langfuse public key from local settings".to_string())?;  // Option<String>
let secret_stored = secrets.get(SECRET_KEY_ACCOUNT).map_err(|e| e.0)?;                             // Option<String>
let pair = match (public_stored, secret_stored) {
    (Some(p), Some(s)) => Some((p, s)),                      // both from stores
    (None, None) => {                                        // neither store ⇒ env, all-or-nothing
        let p = env.get("VIRE_LANGFUSE_PUBLIC_KEY").or_else(|| env.get("LANGFUSE_PUBLIC_KEY"));
        let s = env.get("VIRE_LANGFUSE_SECRET_KEY").or_else(|| env.get("LANGFUSE_SECRET_KEY"));
        match (p, s) { (Some(p), Some(s)) => Some((p, s)), _ => None }
    }
    _ => None,                                               // exactly one store ⇒ no env fill, no mix
};
Ok(pair.map(|(public_key, secret_key)| Credentials { public_key, secret_key: Secret::new(secret_key) }))
```

**Accepted behavior change (note in code comment + RELEASE):** a developer who kept exactly one key
in a store and the other in env now gets `None` instead of a working pair. Env is a whole-pair dev
override — the deliberate cost of the no-mixed-pair guarantee.

## Decision 2 — Achievable two-store consistency contract (Blocker 2)

SQLite and the Keychain are independent stores with **no shared transaction**; true cross-store
atomicity is impossible. **One principle for both operations: perform the fragile Keychain mutation
LAST and compensate its failure with a reliable local SQLite operation.** This makes the realistic
failure (denied prompt / locked keychain / OS error) recoverable in both directions and shrinks the
unrecoverable window to a *catastrophic local SQLite failure*, surfaced explicitly and rendered
inert by Decision 1. This **reorders `clear` to SQLite-first** (it is currently Keychain-first,
`mod.rs:417-420`); `set` keeps its order and only stops swallowing rollback errors.

**2.1 `set_langfuse_secret_repo` — keep order, harden rollback (`mod.rs:397-404`).** Replace the
`let _ = …` compensation so a failed rollback is **not** masked:

```text
let prior_public = read_setting_strict(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?; // read fail ⇒ abort before any write
write_setting(conn, KEY_PUBLIC_KEY, public).map_err(|e| e.to_string())?;                   // SQLite first (reliable)
if let Err(secret_err) = secrets.set(SECRET_KEY_ACCOUNT, secret) {                          // Keychain last (fragile)
    let rollback = match &prior_public {
        Some(prior) => write_setting(conn, KEY_PUBLIC_KEY, prior),                          // restore prior pair
        None        => clear_setting(conn, KEY_PUBLIC_KEY),                                 // back to both-absent
    };
    return match rollback {
        Ok(())  => Err(secret_err.0),                                                       // pair consistent; report real cause
        Err(_)  => Err(INCONSISTENT_SET_ERR.into()),                                        // distinct, secret-free (§2.3)
    };
}
let _ = secrets.delete(PUBLIC_KEY_ACCOUNT); // legacy cleanup, best-effort (unchanged)
Ok(())
```

**2.2 `clear_langfuse_secret_repo` — reorder to SQLite-first + compensate (`mod.rs:417-423`).**

```text
let prior_public = read_setting_strict(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?; // read fail ⇒ abort, both untouched
clear_setting(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?;                           // SQLite first; fail ⇒ abort before Keychain (both prior)
if let Err(secret_err) = secrets.delete(SECRET_KEY_ACCOUNT) {                              // Keychain last (fragile)
    let restore = match &prior_public {
        Some(prior) => write_setting(conn, KEY_PUBLIC_KEY, prior),                          // back to prior pair
        None        => Ok(()),                                                              // nothing to restore (was public-absent)
    };
    return match restore {
        Ok(())  => Err(secret_err.0),                                                       // prior state preserved; report real cause
        Err(_)  => Err(INCONSISTENT_CLEAR_ERR.into()),                                      // distinct, secret-free (§2.3)
    };
}
let _ = secrets.delete(PUBLIC_KEY_ACCOUNT); // legacy cleanup, best-effort (unchanged)
Ok(())
```

**Why reorder clear (answers SW-4's "no recovery after deleting the Keychain secret first").** A
deleted Keychain secret **cannot** be restored — its value is gone and must not be re-read (that
re-introduces the removed prompt and touches the secret, against SEC-009 minimization). Deleting
SQLite **first** keeps the only recoverable artifact (the *non-secret* public key in `prior_public`)
restorable, so the fragile delete's failure leaves the **prior pair intact**. The original
Keychain-first analysis (finding #2 / design.md §3.2) treated the delete orders as symmetric; they
are **not** — only SQLite-first yields a compensation path.

**2.3 The contract (state in module docs + RELEASE):**

> The `settings` table and the Keychain are independent stores with no shared transaction; true
> cross-store atomicity is impossible. The app guarantees instead:
> 1. **Error-atomicity** — every returning `set`/`clear` (`Ok` or handled `Err`) leaves both-present
>    (new or prior) or both-absent, **provided every local SQLite op succeeds.** The fragile Keychain
>    mutation is always last and compensated by a reliable local SQLite op; a strict-read failure
>    aborts before any mutation.
> 2. **Catastrophic-failure disclosure** — the only one-store state reachable through a returned
>    result requires a local SQLite op to fail immediately after another succeeded (disk-full /
>    corruption / I/O). It is surfaced as a **distinct, secret-free error** (`INCONSISTENT_SET_ERR`
>    / `INCONSISTENT_CLEAR_ERR`); the compensation error is **never** swallowed.
> 3. **No crash-atomicity, made safe by the resolver** — no guarantee across a hard crash/kill
>    between the two mutations; that window and case (2) can leave one store populated. Decision 1
>    renders it **inert**: a one-store state resolves to `None`, never a mixed/partial pair, fixed by
>    one re-save. The unconditional invariant the system depends on is *"a one-store state is never a
>    usable credential and never a mixed-source pair."*

Error strings (coarse, secret-free):
`INCONSISTENT_SET_ERR = "Langfuse credentials are in an inconsistent state and could not be fully saved; please re-save your credentials."`
`INCONSISTENT_CLEAR_ERR = "Langfuse credentials are in an inconsistent state and could not be fully cleared; please re-save, then clear again."`

## Required tests for SW-2 (verified at SW-3; covers SW-4 "Suggestions")

Unit tests in `settings/tests.rs`; no real Keychain/network. **SQLite-failure injection:** a
`BEFORE UPDATE/DELETE … WHEN … RAISE(ABORT)` trigger on the in-memory `settings` table targeting the
`langfuse_public_key` row — no production seam, deterministic. (Alt if brittle: a thin `SettingsStore`
trait seam — heavier, not recommended for v0.x.)

- **Decision 1:**
  - **T-PAIR-A** — settings-public present, Keychain-secret absent, both env keys set ⇒
    `credentials.is_none()` (no stored-public + env-secret).
  - **T-PAIR-B** — settings-public absent, Keychain-secret present, both env keys set ⇒
    `credentials.is_none()` (the existing-install hazard: no env-public + stored-secret).
  - **T-PAIR-C** — both stores absent, both env keys set ⇒ env pair resolves (dev-mode regression).
  - Keep both-stores happy path + `failed_replacement_restores_the_prior_pair_and_never_mixes_stores_with_env`
    + the two strict-read short-circuit tests (all stay green).
- **Decision 2 set:**
  - **T-SET-ROLLBACK-FAIL** — prior pair seeded; Keychain `set` fails **and** a trigger ABORTs the
    restore `UPDATE` ⇒ `Err(INCONSISTENT_SET_ERR)` (not the raw Keychain string), secret-free.
    No-prior variant: trigger ABORTs the `DELETE` rollback.
- **Decision 2 clear:**
  - **T-CLEAR-COMP** — full prior pair (public in settings + secret in a delete-failing store);
    `clear` ⇒ `Err` (raw secret-free Keychain cause) and **both** stores still hold the prior pair;
    `resolve_config_with` returns the prior pair. Replaces/renames
    `clear_aborts_before_settings_when_keychain_delete_fails` (mechanism: abort-before → restore-after;
    same net "prior pair preserved" assertion).
  - **T-CLEAR-SQLITE-FAIL** — prior pair; trigger ABORTs the `DELETE` of the public row ⇒ `clear`
    `Err`, aborts **before** the Keychain delete, **both** stores remain.
  - Keep `clearing_secret_removes_it_and_flips_presence`,
    `clear_removes_both_stores_and_deletes_legacy_keychain_public`,
    `secret_is_never_written_to_the_settings_table` (all pass under SQLite-first).

**Gates:** `cargo test settings::tests`, `cargo fmt --all -- --check`, `npm run build` green; clippy
clean on touched code (pre-existing untouched lints stay out of scope).

## Doc changes made by this addendum

- `design.md` §3.2 (clear) → SQLite-first + compensation; §3.3/§3 → pair-level resolver + the §2.3
  contract.
- `specs/langfuse-credential-storage/spec.md` — "Clear removes both stores" scenario reworded from
  the mechanism ("settings row left untouched") to the **invariant** (Keychain-delete failure ⇒ the
  prior consistent pair is preserved); the atomicity requirement gains the resolver-renders-inert
  clause.

## feedback_to_ba[] / split

None. DEC-026 and SEC-009 are preserved and **strengthened**; the credential/secret-storage boundary
(`03_architecture_plan.md`) is not crossed. No new `DEC-###` minted — this refines DEC-026's
realization; assign a formal number from the BA log only if desired. Routes to **SW-2**.
