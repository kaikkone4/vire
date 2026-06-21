# TASK-044 — Move the Langfuse public key out of the Keychain (F2a)

## Why

A freshly built Vire prompts for macOS Keychain access **twice** on first launch — once per
Keychain item. Vire stores the Langfuse credential pair as **two** Keychain items under
service `dev.vire.app`: `langfuse_public_key` and `langfuse_secret_key`
(`src-tauri/src/settings/mod.rs:23-24`). Each item owns its own ACL, so the first access to
each triggers its own dialog (TASK-041 `design.md` §2; root cause confirmed by SW-2).

TASK-041's research/design split this defect into two independent fixes:

- **F1** — stable Apple Developer ID signing so an "Always Allow" grant *persists across
  builds*. Owned by **TASK-042** (release-signing foundation); gated on Janne's paid Apple
  enrolment. Not this task.
- **F2a** — **cut the prompt count from 2 → 1** by removing one Keychain item: move the
  **non-secret** Langfuse public key into the plaintext SQLite `settings` table, keeping the
  secret key in the Keychain. This is **TASK-044** — independent of F1, no Apple gate.

SW-5 has already **GRANTED** the prerequisite sign-off: the Langfuse public key is *not*
secrecy-sensitive (it is the HTTP Basic-Auth **username** `pk-lf-…`, paired with the secret
`sk-lf-…`; it cannot authenticate alone — `src-tauri/src/langfuse/api.rs:81`,
TASK-041 `sec.md` Decision 1). TASK-044 is therefore **unblocked** and carries SEC conditions
**C1–C4** into its own SW-5 gate.

## What changes

Backend-only credential-storage relocation. The renderer-facing IPC **shape is unchanged**;
only where the public key lives changes.

- **Public key → SQLite `settings`** table as one additive key/value row
  (`langfuse_public_key`), the same posture already used for `base_url`, `source`,
  `environments`, `langfuse_enabled` (`mod.rs:28-31`). The k/v `settings` table already
  exists (`lib.rs:124`) — **no new table, no migration framework.**
- **Secret key stays in the Keychain** (account `langfuse_secret_key`, unchanged).
- **One Keychain item ⇒ one ACL ⇒ one fresh-launch prompt.** Goal 2 → 1 met (and once F1
  lands, that single grant persists across builds).
- The credential pair is still **set and cleared atomically**, now **across two stores**
  (SQLite public + Keychain secret), preserving the DEC-026 credential-pair invariant so the
  resolver never combines one store's key with the other's stale/env value (C1, C4).
- The **secret key stays presence-flag-only** to the renderer (SEC-009 / C2); only the
  public key *may* surface as a value (non-secret) — but this task keeps the existing
  presence-flag UI contract to stay proportional.

**No code is implemented in this task.** This change produces the OpenSpec package
(`proposal.md`, `design.md`, `tasks.md`, `specs/*`, `arch-review.md`, `handoff.md`) that the
backend developer implements next.

## Impact

- **Files (at implementation time, per `design.md`)**: `src-tauri/src/settings/mod.rs`
  (storage relocation + two-store atomic set/clear + resolver), `src-tauri/src/lib.rs`
  (`set_langfuse_secret` / `clear_langfuse_secret` gain a `State<AppState>` DB handle),
  `src-tauri/src/settings/tests.rs`. Frontend (`src/main.ts`): **none required** — the JS
  `call('set_langfuse_secret', {publicKey, secretKey})` / `clear_langfuse_secret` /
  `get_langfuse_settings.has_public_key` contract is preserved.
- **Data model**: one additive `settings` row key (`langfuse_public_key`). No schema change to
  any typed table; no IPC payload-shape change visible to the renderer.
- **Security**: SEC-009 secret-key non-echo is **preserved and reinforced** (C2); the only
  value newly stored in plaintext is a non-secret project identifier (sign-off GRANTED).
  Carries **C1–C4** to the SW-5 gate.
- **Existing installs**: a user who already saved credentials must **re-save once** after
  upgrade (two-field form) so the public key lands in `settings`; the legacy Keychain public
  item is cleaned up best-effort. Rationale and rejected alternatives in `design.md` §4.
- **Relationship to other tasks**: independent of TASK-042 (F1 signing). F3
  (`keychain-access-groups` entitlement) is **ruled out** by SW-2 (file-based login keychain,
  no access group). **ID-collision note:** TASK-044 = F2a; the earlier *TASK-043 Stream B*
  Tauri/GTK advisory bump collides and must be **renumbered to a later free integer in its
  own task — not handled here**.

## Out of scope (creep guard)

- F1 signing / notarization / `tauri.conf.json` `bundle.macOS` (→ TASK-042).
- F3 entitlement (ruled out).
- Any change to the `Secret`/`Credentials` redaction types, the SEC-009 secret path, the
  loopback/SEC-002 validation, or the importer/resolver semantics beyond the public-key
  source swap.
- Surfacing the public-key *value* in the UI (C2 permits it; not required for prompt
  reduction — left as an optional follow-up).
- Windows/Linux keyring backends (Mac-first; `apple-native` only).
- The TASK-043 Stream B advisory renumber.
