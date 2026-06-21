# TASK-041 — macOS Keychain repeated-prompt stability: triage & fix-path design

## Why

User report (Janne): **every time a newly built Vire is opened, macOS asks for Keychain
access — sometimes more than once per launch — even after clicking "Always Allow" and
entering the login password.** The grant never "sticks" across builds.

This is a desktop-credibility defect, not a product-behaviour change. Vire stores the
Langfuse credential pair in the macOS Keychain (`keyring` v3 `apple-native`, service =
bundle id `dev.vire.app`, accounts `langfuse_public_key` / `langfuse_secret_key` —
`src-tauri/src/settings/secret_store.rs:7`, `src-tauri/src/settings/mod.rs:23-24`). The
storage design is sound (DEC-026 / SEC-009); the problem is **app identity instability at
package time**.

### Leading diagnosis (architecture-level, to be confirmed by SW-2 research)

macOS binds a Keychain item's "Always Allow" decision to an **ACL entry that trusts a
specific application identity**, derived from the app's **code signature** (its designated
requirement / cdhash). Vire is currently **unsigned / ad-hoc** — `tauri.conf.json` has **no
`bundle.macOS` block** (no `signingIdentity`, no `entitlements`), and `RELEASE.md:275`
confirms: *"Not code-signed or notarized."* Consequences that match the report exactly:

1. **Re-prompt on every new build.** An unsigned or ad-hoc (`codesign -s -`) binary gets a
   fresh, content-derived cdhash on each build and carries **no stable Team-ID designated
   requirement**. macOS therefore treats each freshly built `Vire.app` as a *different*
   application, so the prior "Always Allow" ACL no longer matches → it re-prompts. This is
   precisely "asks every time I open a **newly built** version."
2. **Multiple prompts in one launch.** There are **two** Keychain items (public + secret),
   each with its own ACL; the first access to each triggers its own dialog → up to two
   prompts per fresh launch.

## What this change decides

This is a **triage + design** change (no implementation). It produces:

- a root-cause diagnosis grounded in the current code/packaging state (`design.md` §1–2);
- a designed **fix path** for stable app identity → persistent Keychain trust, plus
  prompt-count reduction options (`design.md` §3);
- an explicit **dev-build** behaviour decision (signing is meaningless for `tauri dev`)
  (`design.md` §4);
- a **research mandate for SW-2** with mandatory authoritative sources for the macOS /
  `keyring` / Tauri semantics this design leans on (`design.md` §6);
- a proposed **follow-up task split** for the actual implementation (`design.md` §5).

## What changes

- New OpenSpec change dir `openspec/changes/task-041-keychain-access-stability/`
  (`proposal.md`, `design.md`, `arch-review.md`, `handoff.md`). **Docs only.**
- **No code, config, or `tauri.conf.json` change in this task.** Implementation is deferred
  to the follow-up tasks proposed in `design.md` §5.

## Impact

- **Relationship to TASK-028 / DEC-029 (updater + signing/notarization split).** Apple
  Developer ID signing is already on the roadmap for the updater
  (`task-027.../design.md:170-175`, `RELEASE.md:347`). TASK-041 **must not duplicate or
  pre-empt** that work — it isolates the *Keychain-trust* requirement on stable signing and
  hands the signing mechanism to the TASK-028 lineage. Scope guard: this task ends at design.
- **No data-model, schema, IPC, or network change.** SEC-009 (no secret echo) and the
  settings-first resolution path are untouched.
- A genuine fix likely requires Apple Developer ID enrolment (paid) — flagged as a
  prerequisite cost, decided with Janne before implementation, not here.
