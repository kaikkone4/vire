# Documentation Gate — TASK-054 fix broken v0.8.1 DMG release (SW-6, L2)

**Verdict: PASS**

## Scope

Plan B docs-only fix for the unsigned macOS DMG Gatekeeper "damaged" dialog. This gate verifies
`README.md` and `RELEASE.md` are accurate, safe, internally consistent, and free of drift, ahead of
SW-6 release. No doc edits were required — the diff already in the PR (verified against
`origin/main`) satisfies every check below.

## Checks performed

- **Official release URL** — `README.md:68` and the security caveat (`README.md:95`) both link
  `https://github.com/kaikkone4/vire/releases`; `git remote -v` confirms `origin` is
  `https://github.com/kaikkone4/vire.git`. `RELEASE.md:31` download URL
  (`.../kaikkone4/vire/releases/download/v0.8.1/Vire_0.8.1_aarch64.dmg`) matches the same owner/repo.
- **Scoped xattr command** — `xattr -dr com.apple.quarantine /Applications/Vire.app` (README:88,
  RELEASE:39) targets only the installed app path; no `sudo`, no `spctl --master-disable`, no
  blanket Gatekeeper disable.
- **Security caveat present** — README:93-98 states the remedy bypasses a Gatekeeper safety check
  specifically because the build is unsigned/unnotarized, conditions trust on downloading from the
  official releases page, and names TASK-028 (signed + notarized) as the real fix. RELEASE.md:39
  carries the same caveat in the smoke checklist.
- **Unsigned/unnotarized truthfulness** — both files consistently say "not code-signed or
  notarized," matching the ad-hoc/linker-signed, `TeamIdentifier=not set` state recorded in
  `ops-review.md`. No Developer ID signing or notarization claim appears anywhere in the diff.
- **No right-click-only false remedy for the "damaged" dialog** — README:81-82 and RELEASE:39
  explicitly state right-click → Open does **not** clear the quarantine "damaged" verdict (it only
  bypasses the separate "unidentified developer" warning). The old ineffective guidance was removed
  from this failure-mode path.
- **No false signing/notarization/auto-update/CI claims** — `RELEASE.md`'s new TASK-054 subsection
  explicitly disclaims source, schema, IPC, capability, `tauri.conf.json`, signing/notarization,
  auto-updater, release-feed, and CI changes. Confirmed against the actual diff
  (`git diff origin/main...HEAD --stat`): only `README.md`, `RELEASE.md`, and this change directory
  are touched — no `src-tauri/`, `.github/`, `package.json`, or lockfile changes.
- **`.VolumeIcon.icns` classification** — README:130-136 and RELEASE:52 both describe it as normal,
  Finder-hidden Tauri DMG metadata (not corruption, not a defect requiring manual re-packing),
  consistent with the pristine-rebuild comparison in `ops-review.md`.
- **Local-build vs. downloaded-build coherence** — the unchanged local-build sections
  (`README.md:182-184,306`) still describe right-click → Open / *Open Anyway*, which remains correct
  there since a locally built app carries no quarantine attribute and hits the different
  "unidentified developer" warning, not the "damaged" dialog. No contradiction between the two paths.
- **Diff scope re-confirmed independently**: `git diff origin/main...HEAD -- README.md` and
  `-- RELEASE.md` read in full; both diffs are limited to the documented Gatekeeper/xattr guidance
  and the new TASK-054 changelog subsection — nothing else changed in either file.

## Non-blocking observation (out of scope, pre-existing)

`README.md:5` ("Current version: v0.8.0") is stale relative to the shipped `0.8.1`
(`src-tauri/tauri.conf.json` reads `0.8.1`) and the `v0.8.1` DMG referenced two paragraphs below it.
This predates TASK-054's diff (not touched by this PR) and is outside Plan B's docs-only Gatekeeper
scope — flagged for a follow-up docs pass, not a blocker for this gate.

## Blockers

None.

## Result

Docs are release-ready. Routes to SW-6 release close-out.
