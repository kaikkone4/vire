# Tasks — TASK-057 publish v0.8.2 release

Single component (the **Vire desktop app**) plus a release-ops publication. **Two ordered parts with
a hard ownership boundary**: Part A is a normal code change (SW-2 → gates → merge); Part B is the
public GitHub Release **with the DMG asset**, published by the **Release Manager (SW-6) only after
Part A merges to `main`**. Do **not** publish before merge — see `arch-review.md`. No update-check
logic change, no in-app auto-updater / `tauri-plugin-updater`, no signing/notarization, no
schema/CSP/capture change. Generated files (`src-tauri/gen/**`, `src-tauri/target/**`) are gitignored
— do **not** hand-edit or commit them.

## Part A — Land v0.8.2 metadata on `main` (SW-2)

**Precondition:** `git fetch origin`; branch `feat/task-057-publish-v082-release` off the current
`origin/main` that includes the TASK-056 merge `df79423` (local `origin/main` ref may be stale). Confirm
`git show origin/main:src-tauri/src/main.ts` shows the TASK-056 capture panel before branching.

- [x] `src-tauri/Cargo.toml` — set `version = "0.8.2"`.
- [x] `src-tauri/tauri.conf.json` — set `"version": "0.8.2"`.
- [x] `src-tauri/Cargo.lock` — `vire` package `version` entry now `0.8.2`; `cargo build --locked`
      confirms no dependency-line drift (only that lockfile line changed).
- [x] `RELEASE.md` (repo root) — prepended a `## v0.8.2` entry: rolls up TASK-056 (Settings→Active-window
      capture panel + truthful status/privacy copy) and this publication; states plainly **no
      source/schema/CSP/dependency change beyond the version strings**; notes the aarch64 DMG is
      attached and unsigned/unnotarized (xattr Plan B). `## v0.8.1` / `## v0.8.0` kept intact below.
- [x] `README.md` — `Current version: v0.8.0` → `v0.8.2`; DMG examples updated to
      `Vire_0.8.2_aarch64.dmg` (download step + artifact-location table).
- [x] Confirmed **no** change under `src-tauri/src/update_check/` and no `tauri-plugin-updater` added.

## Part A — Verify before merge (proportional to scope)

- [x] `cargo build --locked` (in `src-tauri/`) succeeds; `Cargo.lock` `vire` = `0.8.2`.
- [x] `cargo test update_check` passes (11 passed / 0 failed — comparison logic + owner guards unchanged).
- [x] `node --import tsx --test tests/updateCheckUi.test.mjs` passes (12 passed / 0 failed).
- [x] `npm run build` passes; `cargo fmt --all -- --check` passes.
- [x] Version triple agrees at `0.8.2`: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and the
      `vire` block in `src-tauri/Cargo.lock` all read `0.8.2`.
- [x] `openspec validate task-057-publish-v082-release --strict` passes.
- [ ] Open PR → gates (SW-3 QA, SW-4 review, SW-5 security) → **merge to `origin/main`**. *(Draft PR
      opened by SW-2; gates + merge pending.)*

## Part B — Build DMG + publish the Release (SW-6, AFTER Part A is merged)

- [ ] `git fetch origin && git checkout main`; confirm `git show HEAD:src-tauri/Cargo.toml` reads
      `0.8.2` and HEAD includes TASK-056.
- [ ] `npm run tauri:build` → produces `src-tauri/target/release/bundle/dmg/Vire_0.8.2_aarch64.dmg`
      (aarch64 only, matching the v0.8.1 single-arch precedent).
- [ ] Publish, creating the tag server-side and attaching the DMG:
      `gh release create v0.8.2 --repo kaikkone4/vire --target <merged-sha> --title "Vire v0.8.2"
      --notes-file <notes> src-tauri/target/release/bundle/dmg/Vire_0.8.2_aarch64.dmg`. It **MUST**
      be a full release — do **not** pass `--draft` or `--prerelease`.
- [ ] Release notes MUST include: the DMG asset name; "unsigned/unnotarized"; the exact
      `xattr -dr com.apple.quarantine /Applications/Vire.app` remedy; official-source trust caveat;
      TASK-028 Developer-ID/notarization long-term pointer. MUST NOT include `sudo`,
      `spctl --master-disable`, or any global-Gatekeeper-disable wording. **Do not** embed any
      quarantine-bypass in the artifact (see `design.md` §xattr).

## Part B — Verify the live behaviour

- [ ] `gh release view v0.8.2 --repo kaikkone4/vire --json tagName,isDraft,isPrerelease,assets` →
      `tagName=v0.8.2`, `isDraft=false`, `isPrerelease=false`, DMG asset present & downloadable.
- [ ] `curl -sS -o /dev/null -w "%{http_code}" https://api.github.com/repos/kaikkone4/vire/releases/latest`
      → `200`; JSON `tag_name` = `v0.8.2`; `html_url` is the real release page.
- [ ] `https://github.com/kaikkone4/vire/releases` → `200`.
- [ ] In-app smoke, **0.8.1 build** (the currently distributed binary): "Check for updates" →
      **UpdateAvailable** `{ current: "0.8.1", latest: "0.8.2", release_url: <real> }`.
- [ ] In-app smoke, **0.8.2 build** (rebuilt from merged `main`): "Check for updates" →
      **UpToDate** `{ current: "0.8.2" }` — never "downgrade available".
- [ ] Physical-Mac DMG smoke (mount → drag only `Vire.app` → `xattr -dr com.apple.quarantine` →
      launch): standing human UAT carried from TASK-053/054 — record result; **not** a merge blocker.
