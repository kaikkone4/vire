# Release — TASK-041 macOS Keychain repeated-prompt stability (research/decision)

- **Task:** TASK-041
- **Tier:** L2 project (task L1 — docs/research-only; no app code changed)
- **Branch:** `docs/task-041-keychain-access-stability`
- **PR:** #31
- **Tag:** `task-041/v0.6.0` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `6d25d99`
- **Base version:** v0.6.0 (TASK-034) — **unchanged by this release**
- **Date:** 2026-06-21
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

**OpenSpec research and decision artifacts only — zero app code, config, or binary changes.**

This task investigated the root cause of the macOS Keychain "repeatedly prompting for access"
defect and produced a signed research record, architecture review, and forward decisions.

### Artifacts produced

| File | Gate | Summary |
|---|---|---|
| `proposal.md` | (input) | Problem statement and investigation scope |
| `design.md` | SW-1 | Root-cause diagnosis: ad-hoc `cdhash` identity instability + per-item ACL count; storage layer confirmed correct; fix paths F1/F2/F3 defined |
| `arch-review.md` | SW-1 PASS | Architecture review; Apple Developer ID cost gate ($99/yr) documented |
| `ops-review.md` | SW-2 PASS | 4 sourced findings; F3 (keychain-access-groups) ruled out; fork decision F1+F2a confirmed |
| `qa.md` | SW-3 PASS | Scenario coverage matrix, verification checks |
| `review.md` | SW-4 PASS | Code review of the design documents; blocker (unnamed entitlements file) resolved |
| `sec.md` | SW-5 PASS | Scanner table clean (gitleaks 0 leaks, semgrep 0 ERROR, OSV max CVSS 6.9, trivy 0 HIGH/CRITICAL); **public-key non-secrecy sign-off GRANTED** → unblocks TASK-044 |
| `handoff.md` | SW-6 | State carry-forward and next-action decisions |

### Forward decisions carried

- **F1 (stable Developer ID signing)** → folds into TASK-042 release-signing-foundation;
  blocked on Apple Developer Program enrolment ($99/yr, Account Holder) — owner: Janne.
- **F2a (public key → settings)** → TASK-044 `keychain-public-key-to-settings`; SEC sign-off
  granted (public key is Basic-Auth username, `langfuse/api.rs:81`; not secrecy-sensitive).
- **F3 (keychain-access-groups)** → ruled out; keyring v3 `apple-native` uses file-based login
  keychain, no access group needed.

**Scope guard:** zero `src/`, `src-tauri/`, `Cargo.*`, `package.json`, `tauri.conf.json`, or
capability-file changes. The shipped `.app` is byte-identical to v0.6.0.

---

## Required declarations

### 1. Deployment size: none (docs-only)

No source code, runtime dependency, Tauri config, Rust crate, IPC surface, DB schema, or
user-facing behaviour changed. The shipped `.app` is byte-identical to v0.6.0. No product version
increment is appropriate; semantic versioning carries no MAJOR/MINOR/PATCH classification for a
purely documentary release. The tag `task-041/v0.6.0` records the research decision at the v0.6.0
product baseline.

### 2. Rollback strategy: forward-only (docs artifacts)

There are no deployed artifacts and no runtime state changes.

| Layer | Rollback action | Rating |
|---|---|---|
| Shipped `.app` | No change; v0.6.0 binary remains valid | ✅ N/A |
| App source / config | No change | ✅ N/A |
| DB / IPC / capability | No change | ✅ N/A |
| OpenSpec artifacts | `git revert <commit>` removes documents from the tree | ⚠️ Forward-only preferred |

**Rating: forward-only** — the artifacts encode architectural decisions (DEC-026/SEC-009 preserved,
F3 ruled out, TASK-044 unblocked). Reverting the documents would not undo the decisions already
recorded in downstream tasks. If a decision turns out to be wrong, the correct path is a new
OpenSpec change that supersedes this one, not a revert.

### 3. Component compatibility matrix

No upstream or downstream component version constraint is introduced, changed, or removed by this
release. The table below records the inherited constraints from v0.6.0 that are relevant to the
decisions made in this task.

| Component | Constraint | Min | Max | Notes |
|---|---|---|---|---|
| macOS | keyring v3 `apple-native` target | 10.15 (Catalina) | — | File-based login keychain; no access group entitlement needed (F3 ruled out) |
| Apple Developer ID | Signing identity for F1 | N/A (blocked) | — | Requires paid Apple Developer Program ($99/yr); prerequisite owned by Janne; shared with TASK-042 |
| Langfuse API key (public portion) | `langfuse/api.rs:81` — Basic-Auth username | — | — | SEC-009 confirmed non-secrecy; F2a move to settings is safe; no min/max version constraint |
| keyring crate | `keyring = "3"` (Cargo.toml) | 3.0 | — | Version unchanged; storage layer correct; defect is packaging identity, not the crate |
| Tauri | `"tauri": "^2"` (package.json) | 2.0 | — | Unchanged; `bundle.macOS` block absent confirms unsigned baseline at v0.6.0 |

**No incompatibility introduced.** All constraints were present before this task; this release
documents them, it does not create them.

---

## Tag signing (dry-run — SSH signing key unavailable in agent session)

Git is configured for SSH signing (`gpg.format = ssh`,
`user.signingkey = ~/.ssh/id_ed25519.pub`). The signing key is not available in the non-interactive
agent session.

**Intended tag command** (to be run by Janne or a privileged session after PR merge):

```
git tag -s task-041/v0.6.0 \
  -m "release(task-041): v0.6.0 docs — Keychain repeated-prompt root-cause research; F1→TASK-042, F2a→TASK-044, F3 ruled out" \
  <merge-commit-sha>
git push origin task-041/v0.6.0
```

The tag records the research decision at the v0.6.0 product baseline. No version increment.
