<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

- **Change dir**: openspec/changes/task-053-release-dmg-asset-smoke/
- **Branch / PR**: feat/task-053-release-dmg-asset-smoke / PR #41 — **promoted ready-for-review**
- **Phase / gate**: SW-4 PASS + SW-5 PASS + Docs PASS + **SW-6 Release PASS** (2026-07-01) — ready for Janne merge
- **Tier**: L2

## Last gate result
SW-6 Release PASS (see RELEASE.md): 3 declarations recorded (size: patch; rollback: automated;
compat matrix). Live recheck of qa.md's asset/checksum evidence — no drift. PR #41 → ready.

## Prior evidence
SW-5/SW-4/SW-3/Docs all PASS — sec.md/review.md/qa.md/docs.md. Asset live, HTTP 200, sha256
match (`e77d15cf…e82e27f`, 5,787,708 B). Evidence files + new RELEASE.md committed this gate
(were untracked, not yet in PR #41 diff).

## Active blockers
None blocking merge. Carried forward, non-blocking:
1. **Signed tag `task-053/v0.8.1` not created** — SSH key passphrase unavailable
   non-interactively (`git tag -s` failed). No unsigned fallback created. Command for Janne in
   `RELEASE.md` §Tag status. Same disposition as TASK-045/047/052 (doesn't block merge).
2. Root RELEASE.md smoke steps 2–5 — human-only UAT, recommended before wide announcement.

## Exact next action
SW-6 complete. Merge PR #41 when ready; Janne can run the deferred tag command + manual smoke
pass (both in RELEASE.md) at his convenience.

## Notes carried forward
- Download URL: github.com/kaikkone4/vire/releases/download/v0.8.1/Vire_0.8.1_aarch64.dmg
- Exclusions honored: no auto-updater/signing/notarization/release-feed/CI; aarch64 only;
  Gatekeeper documented (right-click→Open), never bypassed.
- Scope note: README "Current version: v0.8.0" header is pre-existing/out of scope.
- Sec advisory (non-blocking, future dep-bump task like task-047): quinn-proto 0.11.14->0.11.15
  (7.5, optional/unreachable) + glib 0.18.5 (6.9, Linux-only). Not TASK-053 scope (no lockfile).
