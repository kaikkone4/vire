# TASK-054 — Tasks

Owner: **devops (release-packaging)**. Gated on Janne answering Q1 (cert?) and Q2 (containment?) in
proposal.md. Do not perform outward-facing GitHub Release actions without Janne's approval.

## 0. Decide the branch (Janne / Pi-Assistant)
- [ ] Q1: Apple Developer ID Application cert + notarization creds available? → **Plan A** (yes) / **Plan B** (no).
- [ ] Q2: Approve yanking the broken `Vire_0.8.1_aarch64.dmg` asset now? → containment (§5).

## 1. Containment (outward-facing — after Q2 approval)
- [ ] Yank the broken asset: `gh release delete-asset v0.8.1 Vire_0.8.1_aarch64.dmg`.
- [ ] Add a one-line note to the `v0.8.1` release body: fixed installer incoming (TASK-054).

## 2A. Plan A — sign + notarize (if Q1 = yes)
- [ ] `src-tauri/tauri.conf.json` `bundle.macOS`: add `signingIdentity` (Developer ID Application),
      `hardenedRuntime: true`, and `entitlements` only if required (none expected).
- [ ] Provide notarization creds via env (APPLE_ID+APPLE_PASSWORD+APPLE_TEAM_ID or API key) — secrets, never commit/print.
- [ ] `npm run tauri:build`; confirm the DMG is signed, notarized, stapled.
- [ ] Verify: `codesign --verify --deep --strict -v Vire.app`; `spctl -a -vvv -t install <dmg>` → accepted/Notarized; `xcrun stapler validate Vire.app`.
- [ ] Simplify README/RELEASE first-launch steps (notarized → double-click opens; drop Gatekeeper dance).

## 2B. Plan B — pristine unsigned re-ship + honest docs (if Q1 = no)
- [ ] `npm run tauri:build`; upload the untouched `src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg` — do **not** hand-re-pack.
- [ ] Fix `README.md:74-78` + `RELEASE.md:39`: for the "damaged" dialog document `xattr -dr com.apple.quarantine /Applications/Vire.app` with a security caveat; correct/remove the ineffective right-click → Open + "do not strip quarantine" lines for this case; point at Plan A / TASK-028 as the real fix.

## 3. Re-publish (outward-facing — after fix verified + Janne approval)
- [ ] `gh release upload v0.8.1 <fixed-dmg> --clobber`.
- [ ] Record the new DMG `sha256` in `RELEASE.md`.
- [ ] Re-run the TASK-053 smoke checklist end-to-end from a **fresh browser download** on a clean Mac; confirm the app opens following the documented steps.

## 4. Close out
- [ ] Update `handoff.md` with the branch taken, verification results, and the shipped `sha256`.
