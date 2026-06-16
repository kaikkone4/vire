# Ops Review — TASK-026 Desktop production readiness (Workstreams B & C)

**Role:** DevOps Engineer (SW)
**Change:** `task-026-desktop-production-readiness`
**Tier:** L2
**Branch:** `feat/task-026-desktop-production-readiness` · **PR:** #21 (draft)
**Scope of this review:** Workstream B (macOS app icon) and Workstream C (production packaged build,
no dev server, install/run docs, release compat/rollback). Workstream A (in-app Langfuse settings +
Keychain) was implemented and committed separately (`ab17c5e`, `8fc7a24`) and is out of this review's
write-scope.

---

## Workstream B — macOS application icon

- **Source asset (temporary placeholder):** `src-tauri/icons/source/vire-icon.png`, 1024×1024 RGBA.
  No branded asset exists, so a simple temporary Vire mark (indigo rounded square + white "V") is
  shipped per `design.md` §6 / `tasks.md` Workstream B. Brand owns the final asset
  (`artifacts/brand/` is read-only to engineering — not written here).
- **Generator (reproducible placeholder):** `src-tauri/icons/source/generate-vire-mark.mjs` — a
  dependency-free Node script (built-in `zlib` PNG encoder). Used because no ImageMagick/rsvg/PIL is
  installed on the build host; keeps the placeholder reproducible until the branded asset lands.
- **Icon set generated:** `npx tauri icon src-tauri/icons/source/vire-icon.png` (Tauri CLI 2.11.1)
  wrote `icon.icns`, `icon.ico`, `icon.png` (512×512), `32x32.png`, `64x64.png`, `128x128.png`,
  `128x128@2x.png`, and the `Square*Logo.png` set. The macOS `.icns` is what the Dock/app switcher use.
- **macOS-only cleanup:** the iOS/Android icon directories that `tauri icon` also emits were removed —
  Vire is a local-only macOS desktop app, so those variants are unused. Re-running `tauri icon`
  regenerates them if ever needed.
- **`bundle.icon` populated:** `src-tauri/tauri.conf.json` `bundle.icon` (was `[]`) now references
  `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico` (standard Tauri macOS set).
  Config validated as JSON; all five referenced paths exist.
- **Replacement path documented:** README "Application icon" section — drop a branded ≥1024×1024 PNG at
  `src-tauri/icons/source/vire-icon.png`, re-run `npx tauri icon …`, rebuild. No code change.

## Workstream C — production packaged build, no dev server

- **No dev server at runtime (verified):** `tauri.conf.json` uses `beforeBuildCommand: npm run build`
  and `frontendDist: ../dist`. `npm run build` (tsc + vite) succeeds and emits `dist/index.html` +
  hashed `assets/*`; a grep of `dist/` finds **no** `1420` / `localhost:1420` dev-server references.
  The packaged app serves bundled assets and needs no Vite dev server or `tauri:dev` process.
- **Full bundle build:** `npm run tauri:build` (release) — see "Build result" below for the run status
  on this host and the artifact paths.
- **Artifacts (where produced):**
  - `.app` → `src-tauri/target/release/bundle/macos/Vire.app`
  - `.dmg` (where supported) → `src-tauri/target/release/bundle/dmg/Vire_0.1.0_<arch>.dmg`
- **Install/run documented:** README "Build and run the packaged app" — build command, artifact table,
  install/run steps, explicit "no dev server at runtime," Langfuse config sourced from in-app settings
  (Keychain-stored secret), local Langfuse stack required for AI import, and the unsigned/Gatekeeper
  first-launch note (signing/notarization out of scope for v0.1).
- **Release compat/rollback (for SW-6 `RELEASE.md`):** documented in README and below.
  - Data: same `app_data_dir()/vire.sqlite`; idempotent `init_db` (`CREATE TABLE IF NOT EXISTS` +
    `INSERT OR IGNORE`); new Langfuse config = additive rows in the existing key/value `settings`
    table; no schema change to `projects`/`time_entries`; no destructive migration.
  - Secrets: app-scoped macOS Keychain entries (service `dev.vire.app`); persist across reinstall;
    not bundled in the artifact.
  - Rollback: a prior build opens the same `vire.sqlite`, ignores unknown additive `settings` rows
    (key/value, no schema dependency) → no data loss; prior build falls back to `VIRE_LANGFUSE_*` env
    (the marked dev fallback) for Langfuse config.

## Cross-cutting verification

- **CSP unchanged:** `tauri.conf.json` `app.security.csp` is byte-identical to HEAD
  (`connect-src ipc: http://ipc.localhost`); the only `tauri.conf.json` diff is `bundle.icon`.
- **Capabilities unchanged:** `git diff HEAD -- src-tauri/capabilities/default.json` is empty. No new
  webview network surface — Keychain + HTTP remain in the Rust core (Workstream A).
- **No secret exposure in B/C scope:** the icon assets, generator script, `bundle.icon`, and README
  packaging docs contain no credential material. README references secrets only as "stored in the
  macOS Keychain, never plaintext / never displayed."

## Build result

`npm run tauri:build` **succeeded** on this host (macOS, aarch64; Tauri CLI 2.11.1). Release profile
built in ~1m55s (incl. the new `keyring v3.6.3` native dependency from Workstream A) and produced both
bundles:

- `src-tauri/target/release/bundle/macos/Vire.app` — `Contents/Info.plist` `CFBundleIconFile=icon.icns`;
  bundled `Contents/Resources/icon.icns` is byte-identical to the generated `src-tauri/icons/icon.icns`
  (the Vire mark). Frontend assets are compiled into the binary from `frontendDist: ../dist` — **no
  dev server / `tauri:dev` at runtime**.
- `src-tauri/target/release/bundle/dmg/Vire_0.1.0_aarch64.dmg` — 5.3 MB.

`npm run build` (tsc + vite) also verified independently: `dist/` contains only bundled production
assets with **no** `1420`/dev-server references. Build artifacts live under the git-ignored
`src-tauri/target/` and are **not** committed.

## Follow-ups for QA (SW-3 / SW-4) and Release (SW-6)

- **QA (manual macOS smoke, per `design.md` §8):**
  - Launch the built `Vire.app` directly with no dev server running; confirm UI renders from bundled
    assets and the **Vire icon** shows in the Dock/app switcher (not the generic default).
  - In-app Langfuse settings persist across relaunch; set secret → import works; clear secret →
    falls back / `auth_or_network_error`, never zero (Keychain-backed; CI cannot assert real Keychain).
- **Release (SW-6):** lift the "Release compatibility and rollback" section into `RELEASE.md`. Version
  stays `0.1.0` in `tauri.conf.json`/`Cargo.toml` unless Janne bumps; SW-6 owns the release-tag/version
  decision. Signing/notarization remain out of scope (local prototype) — note Gatekeeper first-launch.
- **Brand:** replace the temporary Vire mark with the branded asset via the documented icon-replacement
  path once `artifacts/brand/` provides it.
