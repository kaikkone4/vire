# TASK-053 — Attach built DMG to GitHub Release v0.8.1 + release smoke checklist

## Why

The `v0.8.1` GitHub Release exists and is published (tag `v0.8.1`, not draft/prerelease,
`https://github.com/kaikkone4/vire/releases/tag/v0.8.1`) but carries **zero release assets**
(verified: `gh release view v0.8.1 --json assets` → `"assets":[]`). There is nothing to download.
TASK-050's in-app "update available" affordance already points users at the Releases page, and
TASK-052 reconciled all version metadata to `0.8.1` — but a user who follows that nudge lands on a
release with no installer. Vire is therefore **not concretely installable** without building from
source (`npm run tauri:build`), which is exactly the "command-line kikkailu" the update-check work
set out to remove.

This change makes v0.8.1 installable **now** by attaching the already-built aarch64 DMG as a release
asset and documenting the download/install path, with a repeatable smoke checklist so future
releases are verified rather than assumed.

## What this change decides

A minimal **release-distribution / docs + ops** OpenSpec change on a **single surface** — the
existing `v0.8.1` GitHub Release plus the repo's install docs. It does **not** change the app
binary, schema, IPC, capabilities, or any source under `src-tauri/src`. It produces:

- the decision to **attach `Vire_0.8.1_aarch64.dmg`** (the Tauri v2 DMG bundle) to Release `v0.8.1`
  as a downloadable asset;
- a concise **README "Download & install (prebuilt)"** subsection covering the GitHub Releases
  download path and the unsigned-app Gatekeeper first-launch step (right-click → Open);
- a repeatable **release smoke checklist** (`RELEASE.md`) that verifies download → mount → drag
  `Vire.app` → first-launch open → in-app **Check for updates** returns *up to date* on v0.8.1;
- the **explicit exclusions** carried from TASK-042/050: no auto-updater / `tauri-plugin-updater`,
  no code-signing / notarization / Developer ID, no release-feed automation or CI release workflow.

## What changes

- New OpenSpec change dir `openspec/changes/task-053-release-dmg-asset-smoke/`
  (`proposal.md`, `tasks.md`, `specs/release-distribution/spec.md`, `arch-review.md`, `handoff.md`).
- The designed follow-up (dev/ops task) touches **only**:
  - the **GitHub Release `v0.8.1`** — one uploaded asset (`Vire_0.8.1_aarch64.dmg`), an
    **outward-facing publish action** requiring Janne's authorization (see arch-review FB-note);
  - `README.md` — one new "Download & install (prebuilt)" subsection + example artifact name
    bump `0.8.0` → `0.8.1`;
  - `RELEASE.md` — one new "Release smoke checklist" section for v0.8.1.
- **No** change to `src-tauri/` source, `tauri.conf.json`, capabilities, schema, migrations, or CI.

## Verified facts (not assumptions)

- **Bundle name is `Vire_0.8.1_aarch64.dmg`** — the Tauri v2 macOS DMG target names artifacts
  `{productName}_{version}_{arch}.dmg`. Confirmed empirically on disk at
  `src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg` (5.5 MB) and in README §Artifact
  location. No guess.
- **The on-disk DMG is the 0.8.1 build** — the packaged `Vire.app` inside reports
  `CFBundleShortVersionString = 0.8.1`; `tauri.conf.json` and `Cargo.toml` both pin `0.8.1`.
- **The update-check path does not depend on release assets** — `check_for_update` reads only the
  release `tag_name` from `api.github.com/repos/kaikkone4/vire/releases/latest`
  (`src-tauri/src/update_check/mod.rs:26`). With `v0.8.1` latest and the app running `0.8.1`, it
  already returns `UpToDate`. Attaching the DMG changes the **download** experience, not update-check
  behavior. The smoke step verifies this end-to-end.
- **`gh release upload v0.8.1 <dmg>`** attaches assets to an existing tag; same-name re-uploads need
  `--clobber` (there are currently no assets, so a plain upload suffices). Authenticated user is
  `kaikkone4`, owner of `kaikkone4/vire` — has permission.

## Assumptions (flagged for the dev/ops role to confirm)

- **A1 — rebuild-or-validate before upload.** The on-disk DMG lives under gitignored `target/`.
  Although its embedded version is `0.8.1`, the dev/ops role SHOULD either rebuild from the tagged
  commit or record the artifact's sha256 (`e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`)
  in `RELEASE.md` so the shipped bytes are traceable.
- **A2 — aarch64 (Apple Silicon) only.** Scope is a single `aarch64` DMG per the task; no
  `x86_64`/universal build. Intel Macs are out of scope for this release.
- **A3 — unsigned / un-notarized.** Consistent with the repo's v0.1 posture; Gatekeeper
  right-click → Open is documented, not worked around. Signing is explicitly out of scope.
