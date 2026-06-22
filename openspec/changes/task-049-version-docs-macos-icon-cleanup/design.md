# Design — TASK-049 v0.8.0 version/docs metadata + macOS DMG install-artifact cleanup

- **Reviewer**: SW Architect (SW-1) · **Date**: 2026-06-22
- **Type**: release-hygiene / packaging-metadata + docs only — **no shipped source/dep delta**
- **Decision**: **PASS** — single task. Fix version fields, land v0.8.0 docs, and resolve the
  DMG install-artifact concern via the only supported levers (docs + `bundle.macOS.dmg` layout).
  The `.VolumeIcon.icns` itself is **not removable via Tauri v2 config** and is explicitly bounded out.

## 1. Verified ground truth — version drift (built & inspected, 2026-06-22)

| Fact | Value | Source / method | Status |
|---|---|---|---|
| `tauri.conf.json` `version` | `"0.1.0"` | `src-tauri/tauri.conf.json:4` | **verified** |
| `Cargo.toml` `[package].version` | `"0.1.0"` | `src-tauri/Cargo.toml:3` | **verified** |
| `package.json` version | *absent* (no `version` key) | `package.json` | **verified** |
| Built `CFBundleShortVersionString` | `0.1.0` | `PlistBuddy` on produced `Vire.app/Contents/Info.plist` | **verified** |
| Built `CFBundleVersion` | `0.1.0` | same | **verified** |
| Produced DMG filename | `Vire_0.1.0_aarch64.dmg` | `src-tauri/target/release/bundle/dmg/` | **verified** |
| Docs target version | **v0.8.0** | working-tree `README.md`/`RELEASE.md`, `docs/active-window-capture.md` | **verified** |
| README hardcoded artifact name | `Vire_0.1.0_<arch>.dmg` | `README.md:79` | **verified** |

Conclusion: the binary version metadata is three releases behind the docs. The fix is a pure
field bump to `0.8.0` in both `tauri.conf.json` and `Cargo.toml`.

## 2. Verified root cause — macOS DMG "faded/grey icon" artifact (mounted the produced DMG)

Mounted `Vire_0.1.0_aarch64.dmg` read-only and listed the volume root incl. hidden files:

| Entry | Notes | Status |
|---|---|---|
| `Vire.app` | the app bundle | verified |
| `Applications -> /Applications` | install drop-target symlink | verified |
| `.VolumeIcon.icns` | **byte-identical to `src-tauri/icons/icon.icns`** (sha `241793f09b44…`); volume-root dir has `com.apple.FinderInfo` (custom-icon bit set) | **verified** |
| `.DS_Store` | Finder window state / icon positions | verified |
| `.background/` | **absent** — no DMG background image / curated icon layout configured | verified |
| `bundle/macos/rw.24972.Vire_0.1.0_aarch64.dmg` | 40 MB leftover read-write temp from an interrupted `create-dmg` run; lives under gitignored `target/`, **not shipped**, **not the cause** | verified |

**Mechanism (verified).** `.VolumeIcon.icns` + the volume custom-icon attribute is the standard
macOS mechanism to give a mounted volume a custom icon; Tauri's DMG bundler (`bundle_dmg.sh`,
a vendored `create-dmg`) writes it from the app's `.icns`. It is a **hidden dotfile**: invisible
in Finder unless `AppleShowAllFiles` (Show hidden files, ⌘⇧.) is on. Janne — a developer — almost
certainly has that toggle on, so the file shows **faded/grey** (Finder renders hidden items
dimmed). The artifact reaches `/Applications` only when the user **Select-All-drags** the DMG
window contents instead of dragging just `Vire.app`. The leftover `rw.*.dmg` is unrelated build
noise (a clean rebuild / `cargo clean` removes it).

## 3. Tauri v2 capability check — what config can and cannot do (official sources)

| Question | Answer | Source | Status |
|---|---|---|---|
| Does top-level `version` set the bundle version? | Yes — "semver version number or a path to a `package.json`". Maps to macOS `CFBundleShortVersionString`. | [Tauri v2 config reference](https://v2.tauri.app/reference/config/) | **verified** |
| What if `version` is omitted? | "If removed the version number from `Cargo.toml` is used." | [Tauri v2 config reference](https://v2.tauri.app/reference/config/) | **verified** |
| `bundle.macOS.dmg` layout fields | `windowSize` (default `{height:400,width:660}`), `appPosition` (default `{x:180,y:170}`), `applicationFolderPosition` (default `{x:480,y:170}`) | [Tauri v2 config reference](https://v2.tauri.app/reference/config/) · [DMG distribute](https://v2.tauri.app/distribute/dmg/) | **verified** |
| Config to set/disable the DMG **volume icon** (`.VolumeIcon.icns`)? | **None.** "config option for DMG Volume icon" is an **OPEN** feature request — not implemented; Tauri uses the first `.icns` in the folder. | [tauri-apps/tauri#9253](https://github.com/tauri-apps/tauri/issues/9253) | **verified** |
| Config to **hide** the DMG's hidden files when `AppleShowAllFiles` is on? | **None.** "Move hidden files/folder out of DMG window's view" is an **OPEN** feature request; no maintainer fix, no config flag. | [tauri-apps/tauri#11190](https://github.com/tauri-apps/tauri/issues/11190) | **verified** |
| Is `.VolumeIcon.icns` Apple's standard custom-volume-icon file? | Yes — root-level `.VolumeIcon.icns` + the volume's custom-icon bit is the documented macOS mechanism. | Apple convention; corroborated by the mount in §2 | **verified** (mechanism) / industry-standard |

**Implication.** Tauri v2 gives **no supported knob** to remove or relocate `.VolumeIcon.icns`.
The only thing that would suppress it is a custom post-bundle script that mutates the produced
DMG (delete the file + clear the custom-icon bit + re-create/re-sign the image) — out of scope
for a metadata/docs task, fragile, and it sacrifices the (desirable) custom volume icon.

## 4. Decision — options considered for the icon artifact

| Option | Verdict | Rationale |
|---|---|---|
| **A. Document install behavior + explain hidden files** | **ADOPT (primary)** | Zero packaging risk; directly addresses the user-visible concern. Tells the user to drag only `Vire.app` (so nothing leaks into `/Applications`) and that `.VolumeIcon.icns`/`.DS_Store` are normal hidden DMG metadata, not corruption. |
| **B. Add `bundle.macOS.dmg` window layout** (`windowSize`/`appPosition`/`applicationFolderPosition`) | **ADOPT (supporting)** | The only Tauri-supported lever. Presents an unambiguous "drag the app onto Applications" layout, reducing the Select-All temptation. Pure packaging metadata; low risk. Does **not** remove the hidden files. |
| **C. Post-bundle script to delete `.VolumeIcon.icns`** | **REJECT** | No Tauri config; requires re-packing/re-signing the DMG; removes the custom volume icon; brittle; out of scope for a metadata task. Tracked upstream (#9253/#11190) — defer to Tauri. |
| **D. Do nothing / treat as user setting** | **REJECT as sole action** | The drift + missing docs must land regardless; leaving the install UX unexplained ignores a real, reported confusion. |

Adopted: **A + B**. The faded icon is *expected* macOS behavior, not a defect we can compile
away today; we make the install path safe (drag-only-the-app) and the artifact understood
(docs), and we keep the layout clean via the supported config.

## 5. Implementation outline (full checklist in `tasks.md`)

1. `src-tauri/tauri.conf.json`: `version` `0.1.0` → `0.8.0`; add `bundle.macOS.dmg` block with
   `windowSize`/`appPosition`/`applicationFolderPosition` (defaults are fine — set them
   explicitly so the layout is curated and reproducible).
2. `src-tauri/Cargo.toml`: `[package].version` `0.1.0` → `0.8.0`. (`Cargo.lock` will update its
   own `vire` package version line on next build — that is an expected, allowed self-version
   bump, **not** a dependency change.)
3. Land working-tree docs: `README.md`, `RELEASE.md`, `docs/active-window-capture.md`.
4. `README.md`: fix `:79` artifact name to `Vire_0.8.0_<arch>.dmg` (or `Vire_<version>_<arch>.dmg`);
   expand the "Install and run" step to "drag **only** `Vire.app` onto the Applications shortcut —
   do not Select-All" and add a short note that `.VolumeIcon.icns`/`.DS_Store` are normal hidden
   DMG metadata shown only when *Show hidden files* is enabled.
5. Verify with a real packaged build (SW-3/devops): `npm run tauri:build` → confirm
   `Vire_0.8.0_aarch64.dmg`, `Info.plist` `CFBundleShortVersionString=0.8.0`, and a clean DMG
   window layout.

## 6. Split decision, NFR, boundaries

- **Split: NO.** Three workstreams (version, docs, install-UX) share the v0.8.0 release cut and
  the `README.md` file; they are one cohesive metadata/docs/packaging change. Splitting adds
  coordination cost on the same file for no benefit. **STATUS: pass.**
- **NFR impact: none.** No runtime code path, startup, capture, or schema change. The `.app`
  payload is byte-equivalent apart from the version string; the DMG layout is cosmetic.
- **BA escalation: none.** No `03_architecture_plan.md` component boundary is crossed; no BA
  architecture assumption is contradicted. macOS-only packaging is an established project
  decision. No `feedback_to_ba[]`.
- **Cargo.lock note.** Bumping `Cargo.toml` `[package].version` updates only the `vire` package's
  own version line in `Cargo.lock` (self-reference), not any third-party dependency. The
  TASK-047 advisory posture (`deny.toml`, target-scoped) is unaffected — no new crate, no version
  change to any dependency.

## 7. Verified-vs-assumption ledger

- **Verified (built/mounted/inspected locally):** all of §1; the DMG volume contents and the
  `.VolumeIcon.icns` byte-identity in §2; the absence of `.background`; the leftover `rw.*.dmg`.
- **Verified (official docs, URLs in §3):** Tauri `version` semantics + Cargo.toml fallback;
  `bundle.macOS.dmg` field set + defaults; **no** config to suppress the volume icon (#9253) or
  hide DMG hidden files (#11190) — both OPEN.
- **Assumption (stated, not proven):** that Janne's Finder has `AppleShowAllFiles` enabled and
  that the artifact reached `/Applications` via a Select-All drag. This is the most consistent
  explanation for "faded/grey extra icon in /Applications" given the verified DMG contents, but
  the exact drag gesture was not observed. The adopted fix (A+B) is correct under either reading
  (drag-only-the-app + explanation covers both the DMG-window sighting and the /Applications copy).
