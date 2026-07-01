<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-054 fix broken v0.8.1 DMG release (Gatekeeper "damaged")

- **Change dir**: openspec/changes/task-054-fix-broken-dmg-release/
- **Branch / PR**: feat/task-054-fix-broken-dmg-release · draft PR opened
- **Phase / gate**: SW-2 devops **Plan B COMPLETE** (docs fix + local verify) (2026-07-01)
- **Tier**: L2 · **Plan**: B (unsigned + honest quarantine-removal docs; no cert)

## Last result (SW-2 devops)
Root cause = **docs, not the binary**. Shipped asset `e77d15cf…` is pristine `tauri:build` output
(fresh rebuild is structurally identical → not hand re-packed); ad-hoc/linker-signed, v0.8.1.
"damaged" = Gatekeeper quarantine policy for unsigned aarch64, cleared by `xattr -dr
com.apple.quarantine`. Verified: bytes intact, strip works, de-quarantined binary execs.

## Changed (docs-only)
- `README.md` prebuilt install step 3 — xattr remedy + security caveat + TASK-028 pointer.
- `RELEASE.md` v0.8.1 smoke step 3 + new TASK-054 subsection. Shipped sha unchanged (`e77d15cf…`).
- `openspec/changes/task-054-fix-broken-dmg-release/ops-review.md` — full evidence.

## Release-asset action
- **None.** Not yanked (Q2 unapproved + unneeded). Not re-uploaded (asset already pristine; fix is
  docs). Fresh verify-build `7280ff43…` NOT uploaded. Net GitHub Release change: none.

## Blockers / decisions
- None blocking. Optional (Janne, outward-facing): re-upload a fresh asset via
  `gh release upload v0.8.1 <dmg> --clobber` — not required.
- **FB-054-1**: BA to make signing+notarization (TASK-028) required, or accept unsigned interim.

## Exact next action
Pi-Assistant: route to **sw-qa-engineer (SW-3)** to gate the docs change, then merge PR. Physical-Mac
launch UAT (RELEASE smoke 2–5) remains a standing human gate.

## Required files (read these, not the whole tree)
- ops-review.md — verdict, diagnosis, verification, asset action
- README.md:74-98; RELEASE.md:39,45-55 — corrected guidance
- design.md §4 (Plan B) / tasks.md §2B — approved scope
</content>
