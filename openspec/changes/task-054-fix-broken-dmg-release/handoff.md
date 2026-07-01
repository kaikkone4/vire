# Handoff — TASK-054 fix broken v0.8.1 DMG release

- **Change dir**: openspec/changes/task-054-fix-broken-dmg-release/
- **Branch / PR**: feat/task-054-fix-broken-dmg-release · PR #42 → `main` — **ready for review**
- **Gates**: SW-3 QA PASS · SW-4 review PASS (recheck) · SW-5 security PASS (recheck) · Docs PASS
  · **SW-6 Release PASS** (all closed 2026-07-01). See `qa.md`/`review.md`/`sec.md`/`docs.md`/`RELEASE.md`.
- **Tier**: L2 · **Plan**: B (unsigned + honest quarantine-removal docs; no cert)

## State
Root cause: docs, not binary. Shipped asset `e77d15cf…` = pristine `tauri:build`, ad-hoc/linker-signed
v0.8.1; "damaged" = Gatekeeper quarantine policy for unsigned aarch64, cleared by `xattr -dr
com.apple.quarantine`. Asset not yanked/re-uploaded; Release unchanged. Earlier SW-4 blocker
(wrapper-tag residue in `ops-review.md`) was fixed in `e217603` and reconfirmed clean by every
later gate.

## Last result (SW-6 Release) — PASS
`RELEASE.md` written with all 3 declarations: deployment size **patch**; rollback **automated**
(`git revert`, no asset action); compatibility matrix inherited unchanged from TASK-052/053
(v0.8.1, aarch64/Ventura 13+, no new deps). Signed tag `task-054/v0.8.1` **not created** — same
recurring passphrase-protected SSH key blocker as TASK-052/053 (confirmed no dangling tag left).
Per established precedent, non-blocking; deferred to Janne (command in `RELEASE.md` §Tag status).
PR #42 promoted **draft → ready for review** (`gh pr ready 42`).

## Exact next action
Awaiting Janne: (1) merge PR #42, (2) run deferred signed-tag command locally, (3) optional
physical-Mac GUI smoke (`RELEASE.md` root steps 2-5 — standing human gate from TASK-053, not a
merge blocker).

## Notes for downstream roles
- Uncommitted `task-053.../*` edits are pre-existing, not touched by this gate — ignore.
- `tasks.md` §2B/§4 checkboxes unticked despite complete work; cosmetic/non-blocking.
- **FB-054-1** (carried): BA to decide TASK-028 signing+notarization required vs. accept unsigned interim.
- No GitHub Release asset touched, re-uploaded, or yanked by this gate.
