<!-- Compact state; keep <= 2 KB. -->

# Handoff — TASK-049 v0.8.0 version/docs + macOS DMG install-artifact cleanup

- **Phase**: SW-2 DevOps implementation COMPLETE → **STATUS: complete**. All of T1–T5 and V1–V6
  done and verified against a real `npm run tauri:build` (exit 0). See `ops-review.md` for evidence.
- **Next role**: SW-3 (QA) re-verifies V1–V6 on a clean packaged build; then Release Manager (SW-6).
- **Branch**: `feat/task-049-version-docs-macos-icon-cleanup` (off `main` @ `8c45c0b`). Draft PR open.
  v0.8.0 docs carried over from the task-048 working tree as planned.
- **Build evidence**: `Vire_0.8.0_aarch64.dmg`; Info.plist `CFBundleShortVersionString`/`CFBundleVersion`
  both `0.8.0`; mounted DMG root = `Vire.app` + `Applications` symlink + expected hidden
  `.VolumeIcon.icns`/`.DS_Store` (Tauri can't suppress — docs explain). `Cargo.lock` delta = `vire`
  self-version line only.

## Scope (metadata/docs/packaging only — no source/schema/IPC/dep change)

1. **Version sync** → `0.8.0`: `src-tauri/tauri.conf.json:4` (`version`) + `src-tauri/Cargo.toml:3`
   (`[package].version`). Fix hardcoded `Vire_0.1.0_<arch>.dmg` at `README.md:79`.
2. **Land docs**: `README.md`, `RELEASE.md`, `docs/active-window-capture.md` (already drafted in WT).
3. **DMG install UX**: add `bundle.macOS.dmg` window-layout block (`windowSize`/`appPosition`/
   `applicationFolderPosition`); README install step → "drag only `Vire.app`"; explain hidden
   `.VolumeIcon.icns`/`.DS_Store`.

## Verified facts (don't re-derive)

- Build stamps `0.1.0` everywhere (Info.plist + `Vire_0.1.0_aarch64.dmg`) — confirmed by build+mount.
- "Faded/grey icon" = DMG-root `.VolumeIcon.icns` (= app icon bytes) + `.DS_Store`; hidden dotfiles,
  shown only with Finder *Show hidden files*; leak to `/Applications` only via Select-All drag.
- **Tauri v2 has NO config to suppress the volume icon or hide DMG hidden files** — tauri#9253 &
  tauri#11190 both OPEN. Do not attempt a post-bundle re-pack (out of scope).
- `version` omitted ⇒ Cargo.toml version used; `bundle.macOS.dmg` fields verified in Tauri v2 docs.

## Required files

- `proposal.md`, `design.md` (§1 version, §2 mount evidence, §3 Tauri sources, §4 decision),
  `tasks.md` (T1–T5, V1–V6), `specs/release-packaging/spec.md`, `arch-review.md`.

## Gate

- `openspec validate task-049-version-docs-macos-icon-cleanup --strict` — **passes** (verified).
- Definition of done: V1–V6 in `tasks.md`, anchored on a real `npm run tauri:build`.
