# Tasks — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

Single surface (the **v0.8.1 GitHub Release** + repo install docs). **No** change to `src-tauri/`
source, `tauri.conf.json`, capabilities, schema, migrations, or CI. **No** auto-updater,
signing/notarization, or release-feed automation (out of scope — proposal §"What changes").
Recommended order:

## 1. Validate the release artifact (proposal A1)

- [ ] Confirm `v0.8.1` still has no conflicting assets: `gh release view v0.8.1 --json assets`.
- [ ] Rebuild-or-validate the DMG. Either `npm run tauri:build` on the tagged commit, or validate the
      existing `src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg`:
  - [ ] `shasum -a 256` matches the recorded sha256
        (`e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`), and
  - [ ] the embedded app version is `0.8.1`
        (`/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' \
        src-tauri/target/release/bundle/macos/Vire.app/Contents/Info.plist`).

## 2. Upload the DMG asset (outward-facing — requires Janne's OK; arch-review FB-note)

- [ ] `gh release upload v0.8.1 src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg`
      (add `--clobber` only if re-uploading a same-name asset).
- [ ] Verify the asset is attached, downloadable, and correctly sized:
      `gh release view v0.8.1 --json assets` shows `Vire_0.8.1_aarch64.dmg`.

## 3. README download/install docs (single subsection)

- [ ] Add a **"Download & install (prebuilt)"** subsection ahead of the build-from-source flow:
      link to `https://github.com/kaikkone4/vire/releases/latest`, download
      `Vire_0.8.1_aarch64.dmg`, mount, drag **only `Vire.app`** to `/Applications`, first-launch
      right-click → Open (unsigned-app Gatekeeper step). Reuse the existing drag-only warning wording.
- [ ] Bump the example artifact name in §Artifact location from `Vire_0.8.0_aarch64.dmg` to
      `Vire_0.8.1_aarch64.dmg` (README:81).

## 4. Release smoke checklist (RELEASE.md)

- [ ] Add a **"Release smoke checklist (v0.8.1)"** section: (a) asset present & downloadable on the
      release, (b) mount + drag `Vire.app`, (c) first launch opens (Gatekeeper right-click → Open),
      (d) app runs without the dev server, (e) **Settings → Check for updates** returns *up to date*
      on v0.8.1 (proposal §"Verified facts" — update-check reads `tag_name`, asset-independent).
- [ ] Record the shipped artifact's sha256 in RELEASE.md for traceability (proposal A1).

## 5. Verify & report

- [ ] Re-read README/RELEASE edits; confirm both files valid Markdown, non-empty, single trailing
      newline.
- [ ] Confirm `gh release view v0.8.1` lists the asset. Update `handoff.md` with the final state.
