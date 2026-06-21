# Architecture review — TASK-041 (Keychain repeated-prompt stability)

**Verdict: PASS (triage/design complete).** No code touched. Diagnosis and fix path are
sound, scoped, and consistent with the existing architecture and the DEC-026/SEC-009
credential model. One external-semantics dependency is correctly deferred to SW-2 research
rather than asserted.

## Scope check

- Task = *diagnose + design fix path, do not implement*. Delivered exactly that:
  `proposal.md`, `design.md`, this review, `handoff.md`. No source/config/`tauri.conf.json`
  change. ✔
- No scope creep: signing **mechanism** is explicitly handed to the TASK-028/DEC-029 lineage
  (`design.md` §3 F1, §7); only the *Keychain-trust requirement* is owned here. ✔

## Architectural findings

1. **Root cause is packaging identity, not the storage layer.** `KeyringSecretStore`
   (`secret_store.rs`) and the atomic pair logic (`settings/mod.rs`) are correct. The defect
   is that Vire ships **unsigned/ad-hoc** (`tauri.conf.json:16-26` has no `bundle.macOS`
   block; `RELEASE.md:275` "Not code-signed or notarized"), so macOS sees a *new app
   identity per build* and the Keychain "Always Allow" ACL never matches again. ✔ consistent
   with the user report ("newly built version").
2. **Two prompts ⇐ two Keychain items.** `langfuse_public_key` + `langfuse_secret_key`
   (`settings/mod.rs:23-24`) each own an ACL; first access to each prompts. Addressed by F2.
3. **Component boundaries hold.** Fix path stays within Rust-core + packaging config; no IPC,
   schema, network, or renderer contract changes. No boundary crossing ⇒ **no split of this
   task** required (follow-up *implementation* tasks are proposed in `design.md` §5, which is
   the deliverable, not a blocker).
4. **Verification discipline upheld.** Load-bearing macOS/keyring/Tauri claims are marked
   **[verify: SW-2]** with a sourced research mandate (`design.md` §6), matching the repo
   norm set in `task-003` (no unverified claim asserted as fact).

## Recommendations (carried into implementation tasks)

- Primary fix **F1** (stable Developer ID signing) — coordinate with TASK-028, do not fork a
  second signing pipeline.
- Prefer **F2a** (public key → `settings` table; it is not secret) to remove one prompt — but
  gate on a SEC confirmation that nothing depends on public-key secrecy.
- Resolve the **F1-vs-F1+F3** fork (is `keychain-access-groups` entitlement also needed?) via
  SW-2 research item §6.2 / §6.4 *before* TASK-043 implementation.
- Decide **dev-build** posture explicitly (recommend D1+F1; add D3 only if a friction-free
  dev loop is wanted).

## feedback_to_ba[]

None. The BA-flow architecture plan (`03_architecture_plan.md` credential/secret-storage
intent) is **not** contradicted — this is an implementation/packaging-layer defect, not an
architecture-plan divergence. No escalation to ba-architect.

## Prerequisite flagged (decide with Janne, not here)

A durable fix likely needs **Apple Developer ID enrolment** (paid program). This is a
cost/decision gate for TASK-043, surfaced now so it is not discovered mid-implementation.
