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
