# QA — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

**Tier:** L2 · **Scope:** release distribution/docs only (README.md, RELEASE.md, GitHub Release v0.8.1 asset) · **Verdict:** PASS

## Scenario coverage matrix (specs/release-distribution/spec.md)

| Requirement / Scenario | Check performed | Result |
|---|---|---|
| A user installs the published release without building from source — asset listed & downloadable | `gh release view v0.8.1 --json assets,isDraft,isPrerelease` → asset `Vire_0.8.1_aarch64.dmg`, `state=uploaded`, not draft/prerelease. `curl -sI -L` on the public download URL → HTTP 200 (resolves through GitHub's release-assets CDN redirect). | PASS |
| …mount/drag/first-launch (same scenario, human sub-steps) | Requires physical Mac interaction; not executable in this environment. | OUTSTANDING (human UAT, not a code blocker — consistent with tasks.md task 4 "(Human step)" annotations) |
| The attached asset matches the release version | Local on-disk DMG `sha256sum` = `e77d15c…e82e27f`, matches both the RELEASE.md-recorded sha256 and the GitHub asset `digest` field (`sha256:e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`) and size (5,787,708 bytes, all three sources agree). Filename `Vire_0.8.1_aarch64.dmg` encodes version+arch per Tauri v2 convention. Embedded `CFBundleShortVersionString=0.8.1` recorded in handoff/RELEASE.md (not independently re-extracted this pass — plist no longer on disk from prior build, would require rebuild). | PASS |
| A reader finds how to install without a toolchain (README) | `README.md:63` — "## Download & install (prebuilt)" precedes "## Build and run the packaged app" (`README.md:83`); links to `releases/latest`, names `Vire_0.8.1_aarch64.dmg` (Apple Silicon only), documents unsigned/Gatekeeper right-click→Open, cross-references existing drag-only warning at `#install-and-run` (anchor resolves — heading `### Install and run` exists at README:103). Artifact-location example bumped `0.8.0`→`0.8.1` at README:99. | PASS |
| A release is verified by a repeatable smoke checklist (RELEASE.md) | New "DMG asset attached + release smoke checklist (TASK-053)" subsection present with artifact table (name/size/sha256/embedded version/download URL) and a 5-step checklist (asset present ✅ verified in-pipeline; steps 2–5 explicitly marked human/physical-Mac steps). sha256 recorded matches live asset digest. | PASS |

## Scope / exclusion checks

- `git diff main...feat/task-053-release-dmg-asset-smoke -- src-tauri tauri.conf.json .github` → empty. No source, `tauri.conf.json`, capabilities, schema, migrations, or CI changes.
- PR #41 file list (`gh pr view 41 --json files`) touches only `README.md`, `RELEASE.md`, and the `openspec/changes/task-053-release-dmg-asset-smoke/` change dir — matches proposal's declared single surface.
- No auto-updater / signing / notarization / release-feed changes anywhere in the diff — exclusions honored.
- `README.md` "Current version: v0.8.0" header (README:5) left unchanged — this is a pre-existing staleness (release is v0.8.1) but is explicitly out of scope per tasks.md task 3 and flagged in handoff as a follow-up candidate, not a defect of this change.

## Tests / checks run

- `gh release view v0.8.1 --json assets,tagName,isDraft,isPrerelease` — asset present, correct state, published (not draft/prerelease).
- `shasum -a 256` on-disk DMG vs. recorded/remote sha256 — three-way match (local build, RELEASE.md, GitHub asset digest).
- `curl -sI -L` on the public release-download URL — HTTP 200, live.
- `git diff` scope check against excluded paths — empty, confirms no out-of-scope changes.
- `README.md` / `RELEASE.md` — both valid Markdown, non-empty, single trailing newline (`tail -c1` / `wc -l`).
- No integration/unit test files applicable — this is a docs/release-asset change with no app code path affected; no test container or code execution needed for this surface.

## Blockers

None for a PASS gate. Outstanding (non-blocking, tracked): RELEASE.md smoke-checklist steps 2–5 (mount, drag, first-launch Gatekeeper, dev-server-free run, in-app "Check for updates") require a physical Mac and remain human UAT, to be completed by SW-6 (release manager). This was already known and recorded in tasks.md/handoff.md prior to this QA pass.
