# Documentation Gate — TASK-057 Part A (publish v0.8.2 release: version bump + docs)

- **Gate**: Documentation Engineer (L2 pre-SW-6 docs check)
- **Branch / PR**: `feat/task-057-publish-v082-release` · draft PR #44, base `main`
- **Scope gated**: Part A only — `README.md` + `RELEASE.md` wording for the v0.8.2 version/docs
  reconciliation. Part B (DMG build, `gh release create`, live update-check smoke) is SW-6, out of
  scope here.

## Verdict: complete — no doc changes required

Read `handoff.md`, `qa.md`, `review.md`, `sec.md`, then independently re-read `README.md` and
`RELEASE.md` on the branch tip. All required doc checks already hold; SW-4/SW-5 verified the same
content and no drift has occurred since.

## Checklist verified

- **Version says v0.8.2**: `README.md:5` ("Current version: v0.8.2"), `RELEASE.md:3` (`## v0.8.2`
  heading). Version triple confirmed on disk: `src-tauri/tauri.conf.json:4` and
  `src-tauri/Cargo.toml:3` both read `"0.8.2"`/`0.8.2`.
- **TASK-056 Settings UI included**: `RELEASE.md:3,10` state this release publishes the TASK-056
  Active-window capture Settings panel (PR #43, merge `df79423`) so the update check can point at
  it; `README.md:382-402` documents the panel's UAT steps under "TASK-056 — required before
  release".
- **Update-check behavior documented accurately**: `RELEASE.md:13` describes the exact transition —
  a distributed `0.8.1` build resolves **UpdateAvailable (0.8.1 → 0.8.2)** with a real
  `release_url`; a rebuilt `0.8.2` build resolves **UpToDate**; "Open GitHub Releases" reaches a
  real release page. Matches `update_check` module scope (untouched, comparison-logic only).
  `RELEASE.md:38` restates the same transition as the Part B live-verification target.
  `README.md` install flow does not describe or imply an in-app auto-download/auto-install path.
- **Manual install / "Open Releases" — not an in-app updater**: `RELEASE.md:15` explicitly states
  "No `tauri-plugin-updater`, no download/install/auto-updater"; `RELEASE.md:54` ("the in-app
  downloader/installer remains TASK-042 territory") and `:58`/`:86` ("no auto-updater") are
  consistent carried-forward wording from v0.8.1. `README.md:66-105` documents the manual
  download-DMG-and-drag install path with no auto-update claim.
  `grep -rn "tauri-plugin-updater|auto-updat" README.md RELEASE.md` → all hits are explicit
  negations, none are an in-app updater claim.
- **Unsigned/unnotarized Plan B guidance**: `README.md:77-101` and `RELEASE.md:19-25` both carry
  the `xattr -dr com.apple.quarantine /Applications/Vire.app` remedy, an explicit "only run this if
  you downloaded from the official releases page and trust it" caveat, and the TASK-028
  signing/notarization pointer as the real fix.
  `grep -n "sudo\|spctl --master-disable" README.md RELEASE.md` → only the sentence in
  `RELEASE.md:25` stating the notes *never* instruct `sudo` or a global Gatekeeper disable; no
  actual unsafe instruction present in either file.
- **Part B (release asset) deferred to SW-6**: `RELEASE.md:31-38` scopes "Verification (Part A)" to
  build/test/lint checks only, and explicitly states "Live release verification (... DMG asset
  present and downloadable ...) is performed in SW-6 (Part B), after this change merges and the
  Release is published." `handoff.md` "Exact next action" agrees.
- **No stale version references**: `grep -rln "0.8.1|0.8.0" docs/` → no hits; `docs/` carries no
  version-pinned copy that would drift. Older `RELEASE.md` entries (`v0.8.1`, `v0.8.0`, ...) are
  historical changelog content, correctly left unmodified below the new `v0.8.2` entry.

## Checks run

- `grep -n "0.8.2\|v0.8.2" README.md RELEASE.md` — confirms consistent version references.
- `grep -n "version" src-tauri/tauri.conf.json src-tauri/Cargo.toml` — confirms `0.8.2` triple.
- `grep -rn "Open GitHub Releases|Open Releases|Check for updates|UpdateAvailable|update_check|tauri-plugin-updater|auto-updat" README.md RELEASE.md` — confirms update-check wording and absence of in-app-updater claims.
- `grep -n "sudo\|spctl --master-disable" README.md RELEASE.md` — confirms no unsafe Gatekeeper-bypass instruction.
- `grep -rln "0.8.1|0.8.0" docs/` — confirms no stale version references under `docs/`.

## Changes made

None. `README.md` and `RELEASE.md` already satisfy every Part A doc requirement; no edit was
needed.

## Blockers

None.

## Suggestions

None for Part A. (Non-blocking, future/SW-6 scope: once the `v0.8.2` Release and DMG are published,
SW-6 should backfill the asset name/size/sha256 table into the `RELEASE.md` v0.8.2 entry, matching
the pattern already used for v0.8.1's "DMG asset attached" section.)
