# SW-2 ops/research review — TASK-041 (Keychain repeated-prompt stability)

> **Scope:** execute the `design.md` §6 research mandate only — confirm the four
> load-bearing external claims with primary sources, resolve the F1-vs-F1+F3 fork, and
> define the exact implementation TASK split. **No code/config implemented** (design.md §7
> defers all `src-tauri/tauri.conf.json`/`src-tauri/Cargo.toml`/CI changes to the signing-foundation task).
> Tier L2. Every load-bearing claim below carries a primary-source URL.

## Verdict

The architecture diagnosis in `design.md` §2 is **CONFIRMED** by primary sources: the
repeated prompts are caused by an **unstable code-signing identity + per-item Keychain ACL
count**, not by the storage layer. The keyring backend uses the **legacy file-based login
keychain**, so the **F1-vs-F1+F3 fork resolves to F1 only — F3 is NOT needed.**

**Recommended fork: `F1 + F2a` (F3 ruled out).**
- **F1** (stable Apple Developer ID signing) fixes symptom (a) "every new build re-prompts".
- **F2a** (move the Langfuse *public* key out of the Keychain into the `settings` table)
  fixes symptom (b) by cutting fresh-launch prompts from 2 → 1; it is application-layer,
  has **no Apple/cost gate**, and can land independently of F1.
- **F3** (`keychain-access-groups` entitlement) is **unnecessary** — see Finding 2.

## Research findings (design.md §6, all sourced)

### 1. Keychain "Always Allow" ACL ↔ code signature — CONFIRMED
"Always Allow" stores a per-item **ACL keyed to the requesting app's designated requirement
(DR)** — its code-signing identity — *not* the file path or bundle id alone; the keychain
stores no checksum of the app, which is why access survives signed updates. An
**unsigned/ad-hoc** binary's DR is just its `cdhash`, which **changes on every rebuild**, so a
stored ACL no longer matches the new build ⇒ re-prompt. **Any stable signing identity** — a
self-signed cert (design.md §4 D2) *or* a Developer ID — anchors the DR to a stable
identity + bundle id, so "v1.1 is treated like v1.0" ⇒ the ACL keeps matching ⇒ no re-prompt.
Per **TN2206** the Keychain ACL subsystem tracks this stable DR regardless of issuer, so a
**stable signing identity (self-signed included) is sufficient for file-based ACL persistence**;
**Developer ID is required *additionally* for the notarized/Gatekeeper-clean distributed build**
(Findings 3–4), not for ACL stability per se. The file-based keychain binds access via per-item ACL/DR; the data-protection
keychain instead uses entitlement-derived access groups (no per-item trusted-app list).
- Apple DTS (Quinn): https://developer.apple.com/forums/thread/69841 ,
  https://developer.apple.com/forums/thread/115425
- TN2206 (code signing / DR / cdhash): https://developer.apple.com/library/archive/technotes/tn2206/_index.html
- TN3137 "On Mac Keychains" (file-based ACL vs data-protection groups): https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains (corroborated by https://developer.apple.com/forums/thread/696431)

### 2. keyring v3 `apple-native` backend — CONFIRMED (resolves the F1/F3 fork)
The `apple-native` feature pulls in `security-framework`; keyring v3's `src/macos.rs` calls
**only the legacy file-based Keychain Services API** (`SecKeychain*` generic-password:
`set_generic_password` / `find_generic_password`) on the User-domain **login keychain**. Items
are **generic passwords** (`kSecClassGenericPassword`). The backend sets **no
`kSecAttrAccessGroup`**, never touches `kSecUseDataProtectionKeychain`, and does not use the
data-protection keychain. **Conclusion (inferred from the pinned keyring v3.6.3 backend source
cited above — `src/macos.rs`):** item trust follows the **file-based ACL model**, so a
**stable code-signing identity alone makes "Always Allow" persist** — a `keychain-access-groups`
entitlement (F3) is **not part of this backend's trust path and is not required.** (Caveat: true for `keyring = "3"`; v4/5 delegate to a separate
`apple-native-keyring-store` crate — out of scope, we pin v3 at `src-tauri/Cargo.toml:29`.)
- Features → security-framework: https://docs.rs/crate/keyring/3.6.3/features
- Backend source (SecKeychain generic password, no access group): https://github.com/open-source-cooperative/keyring-rs/blob/v3.6.3/src/macos.rs
- security-framework SecKeychain (legacy, deprecated): https://github.com/kornelski/rust-security-framework/blob/main/security-framework/src/os/macos/keychain.rs

### 3. Tauri v2 macOS signing config — CONFIRMED
Signing is configured under `bundle.macOS`: `signingIdentity`
(`"Developer ID Application: NAME (TEAMID)"`; `"-"` = ad-hoc), `entitlements` (path),
`providerShortName`, and `hardenedRuntime` (**defaults to `true`** — required for
notarization, no action needed). CI env: `APPLE_SIGNING_IDENTITY`, `APPLE_CERTIFICATE`
(base64 `.p12`) + `APPLE_CERTIFICATE_PASSWORD`. Notarization env, either path:
`APPLE_ID` + `APPLE_PASSWORD` (app-specific) + `APPLE_TEAM_ID`, **or** App Store Connect API
key `APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`. With the notarization vars
present, `tauri build` **signs → notarizes → staples automatically** (no manual `notarytool`).
**Ad-hoc (`-`) yields an unstable, per-build signature**, so it cannot anchor a persistent
Keychain ACL. **A stable signing identity — including a self-signed cert (design.md §4 D2) —
is sufficient for file-based Keychain ACL persistence** (TN2206: the ACL tracks a stable
designated requirement regardless of issuer). **Developer ID is the *required choice* for
Vire's distributable, notarized release path** — ad-hoc/self-signed cannot be notarized and is
Gatekeeper-blocked — **not a precondition for ACL stability per se.**
- https://v2.tauri.app/distribute/sign/macos/
- https://v2.tauri.app/reference/config/ , https://v2.tauri.app/reference/environment-variables/

### 4. Apple Developer ID / notarization prerequisites — CONFIRMED (owner cost gate)
A **Developer ID Application** certificate requires **paid Apple Developer Program**
membership (**99 USD/yr** individual/org) and the **Account Holder** role to generate it; a
free Apple ID **cannot** obtain one. Notarization workflow: Developer ID-signed + hardened
runtime → `xcrun notarytool submit` → `xcrun stapler staple`; notarytool accepts either an
App Store Connect API key (Issuer + Key ID + `.p8`) or Apple ID + app-specific password +
Team ID. Result for the user: Gatekeeper-clean first launch (no "unidentified developer"
block). **Enrolment is a human/owner decision (legal authority + payment + cert generation)
that a build agent cannot perform** ⇒ hard blocker for F1, owner: **Janne**.
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- https://developer.apple.com/help/account/certificates/create-developer-id-certificates/
- https://developer.apple.com/help/account/membership/program-enrollment/
- https://developer.apple.com/documentation/security/customizing-the-notarization-workflow

## Current repo signing config — verified against the tree
- `src-tauri/tauri.conf.json:16-26` — `bundle` block has **no `macOS` sub-block**, no `signingIdentity`,
  no `entitlements` ⇒ build is **unsigned/ad-hoc**. `identifier = "dev.vire.app"` matches
  `secret_store.rs:7 SERVICE`, so the two Keychain items live under that service.
- `RELEASE.md:382` — "**Not code-signed or notarized.** … Signing/notarization is out of
  scope for v0.1." `RELEASE.md:452-454` — signing/notarization is split to **TASK-028
  (DEC-029)**, now realized as **TASK-042 (in-app-updater)**.
- `src-tauri/Cargo.toml:29` — `keyring = { version = "3", features = ["apple-native"] }` (matches
  Finding 2; file-based keychain).
- Two accounts (`settings/mod.rs:23-24`: `langfuse_public_key`, `langfuse_secret_key`) ⇒ two
  ACLs ⇒ up to two fresh-launch prompts (matches `design.md` §2).

## Interaction with TASK-028 / DEC-029 (now TASK-042) — do NOT build two pipelines
The Apple Developer ID **sign + notarize foundation is shared**: TASK-041 F1 needs it for a
stable Keychain ACL; **TASK-042** (in-app-updater = TASK-028/DEC-029) needs the *same*
mechanism A for Gatekeeper-clean auto-install (`task-042.../design.md` §2, §7, §8 "F —
release-signing-foundation"). **Build the signing foundation ONCE in the TASK-042 lineage; it
satisfies both features.** Note also `task-042.../design.md §0` flagged the numbering
collision (the old `design.md §5` label "TASK-042 keychain-signing-research" clashed with the
in-app-updater) — that research is **this artifact**, so no separate research task is needed.

## Recommended next TASK split (supersedes design.md §5; numbers for BA to confirm)
| Task | Scope | Status / gate |
|------|-------|---------------|
| **Research (former §5 "TASK-042 research")** | §6 research mandate | **DONE — this `ops-review.md`.** No new task. |
| **F1 — stable Developer ID signing** | Add `bundle.macOS.signingIdentity` + CI sign/notarize/staple; **never add a `keychain-access-groups` entitlement** (F3 ruled out by Finding 2) | **Fold into TASK-042 "F — release-signing-foundation".** No separate signing task. **Blocked on Apple Dev ID enrolment (paid, owner: Janne).** |
| **TASK-044 — `keychain-public-key-to-settings` (F2a)** | Move `langfuse_public_key` out of the Keychain into the `settings` table (it is not secret); drop one item/ACL ⇒ fresh-launch prompts 2→1; touch resolver + `settings/mod.rs` atomic-pair logic | **NEW, next free integer (044).** Application-layer; **no Apple/cost gate.** Gate: SEC sign-off that nothing relies on public-key secrecy — **GRANTED (SW-5 `sec.md`, 2026-06-21): the Langfuse public key is the Basic-Auth username (`langfuse/api.rs:81`) and non-secret ⇒ TASK-044/F2a UNBLOCKED.** Can land before F1. |
| **F3 — `keychain-access-groups` entitlement** | — | **RULED OUT** (Finding 2: file-based keychain, no access group). |

## Dev-build posture
Unsigned `tauri dev` / local `tauri build` **cannot** hold a stable Keychain ACL (no durable
identity), per `design.md` §4 — confirmed by Findings 1–2. Recommend **D1 + F1** (accept the
dev prompt, document it as expected; signing fixes the shipped artifact). With F2a, dev still
sees a single secret-key prompt. Add **D3** (non-Keychain dev-env fallback for the secret)
only if Janne wants a friction-free dev loop.

## Proposed-but-NOT-applied config prep (out of TASK-041 scope)
The following are **proposals for the F1/TASK-042 signing-foundation task — deliberately not
applied here** (design.md §7 excludes `src-tauri/tauri.conf.json`/`src-tauri/Cargo.toml`/CI changes):
1. Add a `bundle.macOS` block: `signingIdentity` (from `APPLE_SIGNING_IDENTITY`); leave
   `hardenedRuntime` default `true`. **Never add a `keychain-access-groups` entitlement**
   (F3 ruled out, Finding 2).
2. Add the Apple signing/notarization secrets to the CI secret store (never in repo) — same
   posture class as SEC-009; private keys never shipped.
3. Cross-ref (TASK-042, not here): single-source the app version (`src-tauri/tauri.conf.json:3 = 0.1.0`
   vs `RELEASE.md` v0.6.1) before any signed-release pipeline — flagged, not actioned.

## Owner blockers
- **Apple Developer Program enrolment (99 USD/yr, Account Holder)** — owner **Janne**;
  prerequisite for F1 and for TASK-042 auto-install. Cannot be automated.
- ~~**SEC sign-off** that the Langfuse public key is not secrecy-sensitive — prerequisite for
  TASK-044 (F2a).~~ **RESOLVED — GRANTED by SW-5 (`sec.md`, 2026-06-21); TASK-044/F2a unblocked.**

## Checks performed
- 4/4 research items confirmed against primary sources (Apple dev docs/forums, Tauri v2 docs,
  keyring-rs/security-framework source) — URLs inline above.
- Repo config verified: `src-tauri/tauri.conf.json:16-26` (no `bundle.macOS`), `src-tauri/Cargo.toml:29`
  (keyring v3 apple-native), `RELEASE.md:382/452`, `secret_store.rs:7`, `settings/mod.rs:23-24`.
- No source/config/CI files modified (verified `git status` unchanged outside this task dir).
