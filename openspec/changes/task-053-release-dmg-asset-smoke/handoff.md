<!-- handoff.md — compact per-task state for the QV SW pipeline. KEEP <= 2 KB. -->

# Handoff — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

- **Change dir**: openspec/changes/task-053-release-dmg-asset-smoke/
- **Branch / PR**: feat/task-053-release-dmg-asset-smoke / draft PR opened
- **Phase / gate**: SW-2 dev/ops (DONE) — asset uploaded + docs landed
- **Tier**: L2

## Last gate result
SW-1 arch-review PASS. SW-2 dev/ops complete (2026-07-01): DMG validated, uploaded to
Release v0.8.1 (Janne-authorized), download round-trip verified, README + RELEASE.md updated.

## Active blockers
- None. Steps 2–5 of the RELEASE.md smoke checklist (mount/drag/first-launch/dev-server/
  update-check) are human-only UAT on a physical Mac — outstanding, not a code blocker.

## Exact next action
Route to sw-qa-engineer (SW-3) to gate the docs/ops change, then sw-release-manager (SW-6)
for the human smoke checklist. No further ops action required.

## Required files (read these, not the whole tree)
- this change dir: tasks.md, ops-review not required (single-surface); handoff (this file)
- README.md — new "## Download & install (prebuilt)" section (README:63) + artifact example bump
- RELEASE.md — v0.8.1 "DMG asset attached + release smoke checklist (TASK-053)" subsection

## Notes carried forward
- Asset LIVE on Release v0.8.1: Vire_0.8.1_aarch64.dmg, 5,787,708 B, state=uploaded,
  sha256 e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f (download round-trip
  matched local build byte-for-byte). Embedded CFBundleShortVersionString=0.8.1.
- Download URL: github.com/kaikkone4/vire/releases/download/v0.8.1/Vire_0.8.1_aarch64.dmg
- Exclusions honored: no auto-updater, no signing/notarization, no release-feed/CI, aarch64-only,
  Gatekeeper documented (right-click→Open), never bypassed.
- Scope note: README "Current version: v0.8.0" header left unchanged (out of scope; only the
  §Artifact-location example name was bumped per tasks.md task 3). Flag if a follow-up bump wanted.
