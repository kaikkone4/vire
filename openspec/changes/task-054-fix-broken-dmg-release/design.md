# TASK-054 — Design: fix the broken v0.8.1 DMG release asset

## 1. Symptom → mechanism

| Observed | Mechanism |
|---|---|
| "Vire on vioittunut … Siirrä se roskakoriin." on launch of the release-downloaded app | Gatekeeper rejects an **unsigned/ad-hoc**, **un-notarized** app carrying `com.apple.quarantine` on **Apple Silicon**. Apple's wording for this exact combination is "is damaged and can't be opened" — not "developer cannot be verified". |
| Right-click → Open (documented) does not help | Right-click → Open bypasses only the *"developer cannot be verified"* policy (Developer-ID-signed but un-notarized). It does **not** clear the *"damaged"* policy for unsigned/ad-hoc quarantined apps. |
| `.VolumeIcon.icns` visible in the mounted volume | Cosmetic. The volume-icon file exists but the hidden flag / `SetFile -a C` volume-icon attribute was not applied, or the DMG was re-packed by hand after `tauri:build`. Not the cause of "damaged". |
| Downloaded bytes are fine | TASK-053 verified `sha256 e77d15cf…` and size `5,787,708` on the round-trip download → the file is intact; "damaged" is a policy verdict, not corruption. |

**Evidence in-repo:** `src-tauri/tauri.conf.json` `bundle.macOS` has no `signingIdentity`/notarization;
`README.md:74-78` and `RELEASE.md:39` state "not code-signed or notarized" and give the (ineffective
for this case) right-click → Open guidance plus an explicit "do not strip quarantine" instruction.

## 2. Why this is a release-packaging problem, not an app bug

Nothing in `src-tauri/src` is wrong. The app binary runs fine locally (built without quarantine).
The failure is entirely in the **distribution posture**: unsigned + un-notarized + delivered over a
browser (which sets quarantine). This is the deferred item flagged in RELEASE.md v0.1
("Signing/notarization is out of scope for v0.1") and DEC-029 / TASK-028 — now biting in production.
Therefore the owner is **devops (release-packaging)**, carrying a small `tauri.conf.json` config edit
only in Plan A. No backend/Tauri source-logic change is required.

## 3. Plan A — proper fix (Developer ID + notarization) — preferred

Prerequisite: Apple Developer Program membership + a **Developer ID Application** certificate in the
build keychain + notarization credentials.

1. `src-tauri/tauri.conf.json` → `bundle.macOS`: set `"signingIdentity": "Developer ID Application: <Name> (<TEAMID>)"`,
   `"hardenedRuntime": true` (Tauri v2 applies hardened runtime with the Developer ID identity), and,
   if any restricted entitlements are needed, an `entitlements` plist (none expected — no camera/mic).
2. Provide notarization creds to the build env (Tauri v2 reads them at bundle time): either
   `APPLE_ID` + `APPLE_PASSWORD` (app-specific) + `APPLE_TEAM_ID`, **or** `APPLE_API_ISSUER` +
   `APPLE_API_KEY` (App Store Connect API key). **Secrets — never commit; never print.**
3. `npm run tauri:build` → signs, notarizes, and staples the DMG/app automatically.
4. Verify locally before publishing:
   - `codesign --verify --deep --strict --verbose=2 Vire.app` → valid Developer ID.
   - `spctl -a -vvv -t install Vire_0.8.1_aarch64.dmg` → `accepted`, `source=Notarized Developer ID`.
   - `xcrun stapler validate Vire.app` → `The validate action worked`.
5. Re-upload the DMG to Release `v0.8.1` (`gh release upload v0.8.1 <dmg> --clobber`) after the asset
   is yanked (§5). Record the new `sha256` in `RELEASE.md`.
6. Docs: simplify README/RELEASE first-launch steps — a notarized app opens on double-click; drop the
   Gatekeeper dance.

Result: the "damaged" error cannot occur; no user workaround.

## 4. Plan B — interim honest fix (no cert available)

Keep unsigned, but stop shipping misleading instructions and stop shipping a possibly-re-packed DMG.

1. **Re-ship a pristine DMG:** rebuild via `npm run tauri:build` and upload the untouched
   `src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg` — do **not** manually re-pack it
   (avoids invalidating the ad-hoc signature and fixes the stray-icon layout).
2. **Correct the docs** (`README.md:74-78`, `RELEASE.md:39`): for the "damaged" dialog the working
   remedy is removing quarantine, e.g. after copying to `/Applications`:
   `xattr -dr com.apple.quarantine /Applications/Vire.app`. State plainly that this bypasses
   Gatekeeper because the app is unsigned, and that a signed/notarized build (Plan A / TASK-028) is
   the real fix. Remove/limit the now-inaccurate "right-click → Open" and "do not strip quarantine"
   lines for this failure mode.
3. Record the shipped DMG `sha256` in `RELEASE.md`.

Result: users can open the app by following *accurate* instructions. Weaker than Plan A (asks users
to bypass Gatekeeper), so Plan A is preferred whenever a cert exists.

## 5. Immediate containment (outward-facing — needs Janne approval)

The asset is actively harmful: every download bricks, and the in-app updater points at it.
**Recommendation:** as soon as Janne approves, **delete (yank) the `Vire_0.8.1_aarch64.dmg` asset**
from Release `v0.8.1` (`gh release delete-asset v0.8.1 Vire_0.8.1_aarch64.dmg`) so no further user
downloads a bricked installer, and add a one-line release-note pointer that a fixed asset is
incoming. Rationale for delete-over-edit: a notes-only release still functions for update-check
(reads `tag_name` only), so removing the asset degrades gracefully to the pre-TASK-053 state rather
than leaving a trap. **This change does NOT perform it** — the task says do not modify the release
yet. It is a recommendation gated on Q2.

## 6. Ownership & routing

- **Primary: devops (release-packaging)** — signing/notarization pipeline, secrets handling, rebuild,
  verify (`codesign`/`spctl`/`stapler`), containment + re-upload on the GitHub Release.
- **Config edit (Plan A only):** the `tauri.conf.json` `bundle.macOS` signing block — small, applied
  by devops as part of the pipeline; no backend source-logic change, so a backend/Tauri developer is
  **not** the primary owner.
- **Docs:** README/RELEASE corrections (both plans) — devops or docs role.

## 7. Verification (must pass before re-publishing)

- Plan A: `spctl -a -vvv -t install <dmg>` → accepted/Notarized; `stapler validate` ok;
  a *fresh download via browser* of the re-uploaded asset opens on double-click (quarantine present,
  notarized → allowed).
- Plan B: a fresh browser download, after the documented `xattr` step, opens; DMG mounts with no
  stray visible files; `sha256` recorded.
- Both: README/RELEASE instructions, followed literally on a clean Mac, actually open the app.
