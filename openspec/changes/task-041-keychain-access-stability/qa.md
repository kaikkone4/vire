# QA report — TASK-041 (Keychain repeated-prompt stability)

**Gate:** SW-3 QA · **Tier:** L2 · **Scope:** SW-2 research artifact only — no code/config implemented.
**Verdict:** PASS

---

## Scenario coverage matrix

| Scenario (from `design.md` §6 research mandate) | Source cited in `ops-review.md` | Internally consistent | Status |
|---|---|---|---|
| **S1** Finding 1: ACL "Always Allow" is keyed to code-signature DR; unsigned cdhash changes per build ⇒ re-prompt | Apple DTS (Quinn) threads 69841 + 115425; TN2206; TN3137 | Consistent with `design.md` §2 root-cause table; `tauri.conf.json:16-26` has no `bundle.macOS` ⇒ ad-hoc ⇒ cdhash per build | PASS |
| **S2** Finding 2: keyring v3 `apple-native` uses file-based login keychain, no `kSecAttrAccessGroup`; resolves F1/F3 fork to F1 only | keyring-rs v3.6.3 `src/macos.rs` GitHub source; security-framework `SecKeychain` source; docs.rs features page | Consistent with `design.md` §2 "secondary factor"; `Cargo.toml:29` pins `keyring = { version = "3", features = ["apple-native"] }` as stated | PASS |
| **S3** Finding 3: Tauri v2 macOS signing config (`bundle.macOS.signingIdentity`, entitlements, `APPLE_SIGNING_IDENTITY`, auto sign→notarize→staple) | https://v2.tauri.app/distribute/sign/macos/; https://v2.tauri.app/reference/config/; https://v2.tauri.app/reference/environment-variables/ | Consistent with repo state: `tauri.conf.json:16-26` has `bundle` block but no `macOS` sub-block, no `signingIdentity`, no `entitlements`; ad-hoc status confirmed | PASS |
| **S4** Finding 4: Apple Developer ID requires paid program ($99/yr, Account Holder role); notarytool workflow; enrolment is human gate | https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution; Apple certificate + membership docs | Consistent with `arch-review.md` "Prerequisite flagged"; blocker owner is Janne | PASS |

---

## Additional checks

| Check | Result |
|---|---|
| **F1+F2a recommendation traceable** | `design.md` §3 names F1 (primary fix) + F2a (recommended candidate); `ops-review.md` verdict section confirms both; F3 conditionally was for data-protection keychain path — ruled out by Finding 2 | PASS |
| **F3 dropped traceable** | `design.md` §2 "secondary factor" set up the conditional; `ops-review.md` Finding 2 resolves it: file-based keychain has no access group ⇒ stable signing alone suffices ⇒ `keychain-access-groups` entitlement not required | PASS |
| **Blockers/owners clear** | Two blockers with named owners: (1) Apple Dev ID enrolment (owner: Janne, $99/yr, hard gate for F1); (2) SEC sign-off on Langfuse public-key secrecy (prerequisite for TASK-044/F2a, no Apple cost gate) | PASS |
| **Repo has no macOS signing config** | `tauri.conf.json:16-26` verified: `bundle` block present, no `macOS` sub-block, no `signingIdentity`, no `entitlements` | PASS |
| **Aligns with TASK-042/DEC-029** | `task-042-in-app-updater/design.md §2` names mechanism A (Apple Dev ID) as "shared foundation with TASK-041 (F1)"; `§7` explicitly says "Do not build two signing pipelines"; `§8` names "F — `release-signing-foundation`" as the single shared implementation task. `ops-review.md §94-101` cross-refs this explicitly. No conflict | PASS |
| **No code/config drift outside task dir** | `git diff --name-only HEAD` shows no modifications outside `openspec/changes/` directories; no source, config, or CI file touched | PASS |
| **No scope creep in ops-review.md** | Section headers "Proposed-but-NOT-applied config prep" explicitly marks F1 config proposals as out-of-scope for TASK-041; `design.md §7` creep guard lists same exclusions | PASS |

---

## Changed paths (task dir only)

```
openspec/changes/task-041-keychain-access-stability/
  arch-review.md      (SW-1 output — read-only for this gate)
  design.md           (SW-1 output — read-only for this gate)
  handoff.md          (updated by SW-2; updated by this gate)
  ops-review.md       (SW-2 output — primary artifact verified above)
  proposal.md         (BA input — read-only for this gate)
  qa.md               (this file — SW-3 output)
```

No source files, no Cargo/npm configs, no CI files, no files outside this directory were added or modified.

---

## Blockers going forward

1. **Apple Developer ID enrolment** — owner: **Janne**; $99/yr paid program; hard prerequisite for F1 / TASK-042 signing foundation. Cannot be automated.
2. **SEC sign-off** on Langfuse public-key secrecy — prerequisite for TASK-044 (`keychain-public-key-to-settings` / F2a). No Apple/cost gate; this task can otherwise proceed independently.

---

## Gate verdict

**QA STATUS: pass** — 4/4 research mandate claims confirmed with primary sources, internally consistent with codebase evidence, F1+F2a recommendation traceable, F3 ruled out by evidence, blockers/owners named, no code drift outside task dir.

Route: SW-4 (code-reviewer) ∥ SW-5 (security-agent).
