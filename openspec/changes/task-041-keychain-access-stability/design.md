# TASK-041 — Keychain access stability: architecture & fix-path design

> Triage/design only. No implementation. Load-bearing external claims are marked
> **[verify: SW-2]** and gathered in §6.

## 1. Where this sits in the existing system

```
Renderer (Settings)  ──IPC──►  Rust core  ──►  KeyringSecretStore  ──►  macOS Keychain
  set / not set flags          (lib.rs:730…)    secret_store.rs         service dev.vire.app
  never the value (SEC-009)                      apple-native            ├─ langfuse_public_key
                                                                         └─ langfuse_secret_key
```

- Store: `src-tauri/src/settings/secret_store.rs` — `KeyringSecretStore` wraps
  `keyring::Entry::new(service, account)` with `get/set_password` / `delete_credential`.
- Service id: `SERVICE = "dev.vire.app"` (`secret_store.rs:7`) = `tauri.conf.json` `identifier`.
- Accounts: `PUBLIC_KEY_ACCOUNT`, `SECRET_KEY_ACCOUNT` (`settings/mod.rs:23-24`) → **two items**.
- Dependency: `keyring = { version = "3", features = ["apple-native"] }` (`Cargo.toml:29`).

The application logic is correct and well-tested (atomic pair write/rollback, in-memory
fake for CI). **Nothing in this layer is the bug.** The defect is in *packaging identity*.

## 2. Root cause (architecture diagnosis)

macOS Keychain "Always Allow" persists a per-item **ACL** that names a *trusted
application*, identified by the requesting binary's **code signature** (designated
requirement / cdhash). The grant matches on a later launch only if the binary still
satisfies that requirement. **[verify: SW-2 — Apple Keychain Services / ACL & trusted-app
matching semantics]**

Current state of Vire's identity:

| Fact | Evidence | Effect |
|------|----------|--------|
| No `bundle.macOS` block; no `signingIdentity`/`entitlements` | `tauri.conf.json:16-26` | Build is unsigned or ad-hoc |
| "Not code-signed or notarized" | `RELEASE.md:275` | Confirmed posture |
| Ad-hoc/unsigned cdhash changes per build; no stable Team-ID DR | macOS codesign model **[verify: SW-2]** | Each new build = a *different* app to macOS |
| Two Keychain items, two ACLs | `settings/mod.rs:23-24` | First access to **each** prompts separately |

⇒ **(a)** new build ⇒ identity no longer matches the stored ACL ⇒ re-prompt;
**(b)** two items ⇒ up to two prompts per fresh launch. Both symptoms in the report fall
out of this single root cause: **unstable code-signing identity + per-item ACL count.**

### Secondary factor to confirm (does *not* change the leading diagnosis)

`keyring` v3 `apple-native` may target either the **file-based login keychain** (classic
ACL / designated-requirement model — signing alone fixes trust) **or** the
**data-protection keychain** (requires a `keychain-access-groups` entitlement and ties item
ownership to a signed team prefix). Which one it uses **determines whether stable signing is
sufficient, or whether a `keychain-access-groups` entitlement is also required.**
**[verify: SW-2 — keyring v3 apple-native backend: keychain class + access-group behaviour]**

## 3. Fix path (designed, not built)

Ordered by leverage. Final selection is an implementation-task decision informed by §6.

- **F1 — Stable signing identity (primary fix).** Sign **every** build whose Keychain trust
  must persist with a *consistent* Apple **Developer ID Application** certificate, so the
  designated requirement anchors to a stable Team ID + bundle id rather than a per-build
  cdhash. Configure via `tauri.conf.json` `bundle.macOS.signingIdentity` (+ entitlements) or
  the `APPLE_SIGNING_IDENTITY` env. **[verify: SW-2 — Tauri v2 macOS signing config]**
  *Coordinate with TASK-028 / DEC-029 — do not build a second signing pipeline.*
- **F2 — Item / ACL design to cut prompt count.** Choose one:
  - **F2a (recommended candidate):** move the **public key out of the Keychain** into the
    `settings` table — it is **not secret** (it is already a documented credential-adjacent
    public value). Eliminates one item ⇒ one fewer prompt, and removes a needless ACL.
    Requires confirming nothing relies on public-key secrecy (SEC review).
  - **F2b:** consolidate both keys into **one** Keychain item (JSON pair) ⇒ one prompt, but
    couples public+secret lifecycle and touches the atomic-pair logic in `settings/mod.rs`.
  - **F2c:** keep two items but ensure both carry the same ACL (least change, still 2 items).
- **F3 — `keychain-access-groups` entitlement** *iff* §6 shows apple-native uses the
  data-protection keychain — pins item ownership to the team prefix so trust survives
  resigning. Skip if the file-based keychain is in use and F1 suffices.

## 4. Dev-build behaviour (explicit decision required)

`tauri dev` / unsigned local builds **cannot** hold stable Keychain trust — there is no
durable identity to anchor an ACL to. Options for the implementation task to choose:

- **D1:** Accept the prompt in dev; document it as expected. Cheapest; signing fixes the
  *release/distributed* path that the user actually ships.
- **D2:** Sign dev builds with one stable **self-signed** identity the developer trusts once.
- **D3:** In dev, source the secret from a non-Keychain fallback (e.g. the marked dev-env
  path already present in `langfuse/mod.rs`) so dev never touches the Keychain.

Recommendation to carry into implementation: **D1 + F1** (fix the shipped artifact; treat dev
prompts as expected) unless Janne wants a friction-free dev loop, then add D3.

## 5. Proposed follow-up task split (implementation — separate tasks, not this one)

| Task (proposed) | Scope | Depends on |
|-----------------|-------|------------|
| **TASK-042** `keychain-signing-research` (SW-2) | Execute §6 research mandate; confirm/deny leading diagnosis; pick F1/F3 path with sources | — |
| **TASK-043** `stable-macos-signing` | Configure stable Developer ID signing for builds (F1; reconcile with TASK-028/DEC-029) | 042, Apple Dev ID enrolment |
| **TASK-044** `keychain-item-acl-reduction` | Apply F2 (public key → settings, or single-item) to cut prompt count | 042 (SEC sign-off) |

If Janne wants signing handled wholly inside the TASK-028 updater work, fold TASK-043 into
that lineage rather than creating it. **This is a recommendation; Pi-Assistant/BA schedules.**

## 6. Research mandate for SW-2 (authoritative sources REQUIRED)

Each claim below is load-bearing and currently an **assumption**. SW-2 must confirm or
refute each with a cited primary source (Apple developer docs preferred; crate
docs/source for keyring; official Tauri docs for signing). No claim ships as fact without a
source URL, per the repo norm (cf. `task-003.../design.md` verification register).

1. **Keychain ACL ↔ code-signature matching.** Confirm that an item's "Always Allow" trust
   is keyed to the requesting app's code signature/designated requirement, and that an
   unsigned/ad-hoc binary's identity changes per build ⇒ re-prompt. *Source: Apple Keychain
   Services / "trusted applications" / TN on keychain ACLs.*
2. **keyring v3 `apple-native` internals.** Which keychain class and item type it uses
   (`SecItem` generic password? login vs data-protection keychain?), and whether it sets a
   `keychain-access-group`. Determines F1-sufficient vs F3-required. *Source: keyring-rs v3
   docs.rs / GitHub source for the apple-native backend.*
3. **Tauri v2 macOS signing config.** Exact `bundle.macOS.signingIdentity` / `entitlements`
   keys and `APPLE_SIGNING_IDENTITY` env; ad-hoc vs Developer ID difference; minimum to get
   a *stable* designated requirement. *Source: https://v2.tauri.app/distribute/sign/macos/.*
4. **Does stable signing alone make "Always Allow" persist across rebuilds**, or is a
   `keychain-access-groups` entitlement also needed? Resolve the F1-vs-F1+F3 fork. *Source:
   Apple entitlements / keychain-access-groups docs + (1)–(2) above.*

## 7. Out of scope (creep guard)

- Building any signing pipeline, entitlement file, or `tauri.conf.json` change (→ TASK-043).
- The updater / notarization / GitHub Releases work (owned by TASK-028 / DEC-029).
- Changing the secret-storage trait, SEC-009 non-echo behaviour, or the resolver.
- Windows/Linux keyring backends (Mac-first prototype; `apple-native` only — `Cargo.toml:28`).
