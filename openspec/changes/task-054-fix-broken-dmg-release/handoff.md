# Handoff — TASK-054 fix broken v0.8.1 DMG release

- **Change dir**: openspec/changes/task-054-fix-broken-dmg-release/
- **Branch / PR**: feat/task-054-fix-broken-dmg-release · draft PR #42 → `main`
- **Phase / gate**: SW-4 blocker **FIXED** (2026-07-01, ready for recheck, `review.md`); SW-5 security **PASS** (`sec.md`)
- **Tier**: L2 · **Plan**: B (unsigned + honest quarantine-removal docs; no cert)

## State
Root cause: docs, not binary. Shipped asset `e77d15cf…` = pristine `tauri:build`, ad-hoc/linker-signed
v0.8.1; "damaged" = Gatekeeper quarantine policy for unsigned aarch64, cleared by `xattr -dr
com.apple.quarantine`. Asset not yanked/re-uploaded; Release unchanged. SW-3 QA PASS.

## Last result (SW-4 review) — blocker fixed
Plan B README/RELEASE guidance passed review (`review.md`); sole blocker was stray wrapper tags in
`ops-review.md`. **RESOLVED (SW-2, 2026-07-01):** removed `</content>`/`</invoke>` from tail of
`ops-review.md`; Plan B content intact, single trailing newline. Post-fix checks pass: wrapper-tag
scan clean, `openspec validate --strict`, `git diff --check`, diff still docs-only.

## Last result (SW-5 security) — PASS (L2)
No auto-fail. semgrep ERROR=0; gitleaks false-positive outside diff; OSV/Trivy N/A. xattr remedy scoped
to `/Applications/Vire.app` (no sudo/blanket Gatekeeper disable); no false signing claims. ADV-1:
residual unsigned risk → real fix TASK-028. See `sec.md`.

## Exact next action
Rerun **SW-4** to confirm the `ops-review.md` tag cleanup (only remaining blocker). SW-5 PASS — no
security rework. When SW-4 re-passes, both gates clear → SW-6 release. Physical-Mac GUI-launch UAT
(RELEASE smoke 2–5) is a standing human gate from TASK-053 — not a blocker here.

## Notes for downstream roles
- Uncommitted `task-053.../*` edits not in this PR's diff (TASK-053 merged) — ignore. `tasks.md`
  §2B/§4 checkboxes unticked though work done; cosmetic.
- **FB-054-1** (carried): BA to make TASK-028 signing+notarization required, or accept unsigned interim.
