# Handoff — TASK-041 macOS Keychain repeated-prompt stability

- **Change dir**: openspec/changes/task-041-keychain-access-stability/
- **Branch / PR**: `docs/task-041-keychain-access-stability` / PR #31 (ready-for-review)
- **Phase / gate**: SW-6 RELEASE **PASS** (2026-06-21) → DOCS gate **PASS** (2026-06-21) — all gates complete
- **Tier**: L2

## Last gate result
SW-5 SEC **PASS** — Tier 1 scanners clean vs auto-fail (gitleaks: no leaks/182 commits;
semgrep: 0 ERROR; OSV: max CVSS 6.9 glib < 7.0; trivy fs: 0 HIGH/CRITICAL). **Public-key
non-secrecy sign-off GRANTED** (public key is the Basic-Auth username, `langfuse/api.rs:81`;
no security property depends on its secrecy) ⇒ **unblocks TASK-044**. F2a + F1 approved with
conditions C1–C6 for the implementing tasks (TASK-044 / TASK-042). No design escalation;
DEC-026/SEC-009 preserved. See `sec.md` (2026-06-21).

SW-4 CODE REVIEW FINAL RECHECK **PASS** (2026-06-21) — the sole blocker is resolved:
`ops-review.md` no longer proposes an unnamed entitlements file; the explicit
`keychain-access-groups` prohibition remains at `ops-review.md:115,130`. Developer ID claim
narrowing, corrected repository paths, and the keyring-v3 inference label remain intact.
No code/config changed. SW-5 SEC PASS, public-key sign-off, F2a approval, and TASK-044
unblocked state are preserved. Both gates now pass; ready for SW-6. See `review.md`,
`ops-review.md`, `sec.md`.
(Prior: SW-3 QA PASS, `qa.md`, 2026-06-21. SW-2 DEVOPS PASS, `ops-review.md`, 2026-06-21.
SW-1 ARCH PASS, 2026-06-20.)

## Active blockers
- ~~**SW-4 remediation** — remove the unnamed entitlements-file clauses; keep the
  `keychain-access-groups` prohibition.~~ **RESOLVED; SW-4 PASS (2026-06-21).**
- ~~Developer ID claim narrowing, repository path corrections, and inference label.~~
  **RESOLVED and verified by SW-4 recheck (2026-06-21).**
- **Apple Developer ID enrolment** (paid, 99 USD/yr, Account Holder) — owner: **Janne**.
  Hard prerequisite for F1; cannot be automated. Shared with TASK-042.
- ~~**SEC sign-off** that the Langfuse public key is not secrecy-sensitive~~ — **RESOLVED:
  GRANTED by SW-5 (`sec.md`)**. TASK-044 unblocked; carries SEC conditions C1–C4 to its own
  SW-5 gate. TASK-042 signing foundation carries C5–C6.

## DOCS gate outcome (2026-06-21)
**PASS — no drift.** Research/decision-only task; zero app code, config, or binary changed.
- `README.md` (lines 45, 123): Keychain references are accurate and complete; no mention of
  repeated-prompt defect needed (defect unresolved; fix deferred to TASK-042/TASK-044).
- `docs/langfuse-local-setup.md`: Keychain/credential guidance correct; no update needed.
- `docs/backup-restore.md`: no keychain-prompt references; no update needed.
- No new user-facing behavior → no user-doc change. No API surface changed → no OpenAPI drift.
- No files written; no commit on DOCS gate (nothing to change).

## SW-6 outcome (2026-06-21)
- RELEASE.md written with all three declarations (docs-only/none, forward-only, compatibility matrix).
- All 9 artifacts committed to `docs/task-041-keychain-access-stability` (commit `6d25d99`).
- PR #31 created and marked ready-for-review.
- Tag `task-041/v0.6.0` is a dry-run (SSH key unavailable in agent session); run manually after merge (command in RELEASE.md §Tag signing).
- TASK-041 SW flow **COMPLETE**. Janne merges PR #31 to close.

## Exact next action (post-SW-6)
Pi-Assistant: TASK-041 complete. Carry forward the intended
implementation split:
- **F1 (stable Developer ID signing)** → FOLD into TASK-042's "F — release-signing-foundation"
  (do NOT create a separate signing task / second pipeline); blocked on Janne's enrolment.
- **TASK-044 `keychain-public-key-to-settings` (F2a)** → NEW task (next free integer),
  application-layer; SEC public-key sign-off is granted, so it can proceed before F1.
- **F3** → ruled out (keyring v3 uses the file-based login keychain; no access group needed).

## Required files (read these, not the whole tree)
- `qa.md` — **SW-3 QA output**: scenario coverage matrix, all checks, PASS verdict.
- `review.md` — **SW-4 output**: final recheck PASS; blocker resolution and preserved findings.
- `sec.md` — **SW-5 security output**: scanner table, public-key sign-off, conditions C1–C6.
- `ops-review.md` — **SW-2 research output**: 4 sourced findings, fork decision (F1+F2a, F3
  out), next TASK split, dev posture, owner blockers. READ THIS FIRST for next steps.
- `design.md` §2 — root-cause diagnosis (now CONFIRMED by ops-review Findings 1–2)
- `design.md` §3 — fix path F1/F2/F3 (F3 now ruled out)
- `arch-review.md` — SW-1 PASS rationale + Apple Dev ID cost gate
- `src-tauri/src/settings/secret_store.rs` / `settings/mod.rs:23-24` — storage layer (correct, not the bug)
- `src-tauri/tauri.conf.json:16-26` + `RELEASE.md:382` — confirms unsigned/no `bundle.macOS` block
- `../task-042-in-app-updater/design.md` §2,§7,§8 — shared signing foundation (TASK-028/DEC-029)

## Notes carried forward
- Root cause CONFIRMED: unstable code-signing identity (ad-hoc cdhash changes per build) +
  per-item ACL count. Storage layer (DEC-026/SEC-009) is correct; defect is packaging identity.
- keyring v3 `apple-native` = **file-based login keychain, no access group** ⇒ stable signing
  alone fixes "Always Allow"; **F3 entitlement not needed.**
- Do NOT build a second signing pipeline — F1 == TASK-042's shared signing foundation.
- F2a (public key → settings) drops one prompt; cheap, no Apple gate; SEC sign-off granted →
  TASK-044 unblocked.
- No `feedback_to_ba[]`; BA architecture plan is not contradicted.
