# Ops review — TASK-054 (fix broken v0.8.1 DMG release, Plan B)

**Role:** devops (release-packaging). **Tier:** L2. **Plan:** B (Janne selected — no Apple Developer
Program / no Developer ID cert / no notarization creds; ship unsigned + honest quarantine-removal docs).
**Branch:** `feat/task-054-fix-broken-dmg-release`. **Date:** 2026-07-01.

## Verdict

The v0.8.1 release was **bricked by the docs, not by the binary**. The shipped asset is already a
pristine `tauri:build` DMG and opens correctly once the download-quarantine attribute is removed. Plan B
fix = correct the first-launch docs; **no asset re-upload or yank was required or performed.**

## What was done

1. **Rebuilt a fresh pristine DMG** via `npm run tauri:build` and compared it to the shipped asset —
   to test the "was it hand re-packed?" hypothesis and to have a verified artifact on hand.
2. **Corrected the unsigned first-launch guidance** in `README.md` (prebuilt install step 3) and
   `RELEASE.md` (v0.8.1 smoke-checklist step 3) + added a TASK-054 note. Right-click → Open and
   "do not strip quarantine" are replaced with the working `xattr -dr com.apple.quarantine` remedy,
   a security caveat, and a pointer to TASK-028 (signed+notarized) as the real fix.
3. **Verified locally** the shipped/pristine DMG layout, signature, and the documented open-path.

## Diagnosis confirmed against artifacts (not assumed)

| Check | Result |
|---|---|
| Shipped on-disk DMG == published asset | `sha256 e77d15cf2066…82e27f`, size `5,787,708` — byte-identical to the v0.8.1 asset. |
| Shipped `Vire.app` signature | `adhoc, linker-signed` (`flags=0x20002`), Identifier `vire-3e8c42af81136f6b`, `TeamIdentifier=not set`. |
| Shipped app version | `CFBundleShortVersionString = 0.8.1`. |
| Shipped DMG Finder-visible layout | `Vire.app` + `Applications` only (dot-files hidden by Finder default). |
| `.VolumeIcon.icns` | dot-file, creator `icnC`, invisible attr **not** set (`avbstclinmedz`); volume root has custom-icon `C`. |
| Was the DMG hand re-packed? | **No.** A fresh `npm run tauri:build` produced a structurally identical DMG (same layout, same `.VolumeIcon.icns` flags, same `adhoc` sig + Identifier, same version). The visible icon is inherent Tauri `bundle_dmg.sh` behavior — it runs `SetFile -c icnC` + volume `SetFile -a C` but never sets the file invisible (`-a V`). Finder hides it by dot-prefix default; it only shows with "show all files". |
| Bytes corrupt ("damaged" == corruption)? | **No.** Mach-O carries a valid ad-hoc signature; "damaged" is Gatekeeper's quarantine policy verdict for an unsigned/ad-hoc app on Apple Silicon. |

## Fresh build (verification artifact — NOT uploaded)

| Field | Value |
|---|---|
| Path | `src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg` |
| sha256 | `7280ff43a58d058320e9e25d3fd0c6d5e405949ff27d2235fe1d5225eab7bc2b` |
| Size | `5,787,688` bytes (20 B off the shipped image — normal DMG compression variance) |
| Signature / version | `adhoc, linker-signed`, Identifier `vire-3e8c42af81136f6b`, `0.8.1` |

The fresh DMG differs from the shipped one only in compression bytes; structurally it is the same
pristine artifact. It was **not** uploaded (the shipped asset is already correct) and the shipped
sha256 in `RELEASE.md` is unchanged.

## Open-path verification (Plan B remedy works)

1. `codesign --verify` on the bundle reports "code has no resources but signature indicates they must be
   present" — this is the expected `codesign` result for a **linker-signed ad-hoc** Tauri bundle (no
   `_CodeSignature/CodeResources` seal; `Sealed Resources=none`). It is present identically on the
   already-shipped asset and does **not** block execution — Apple Silicon execs on the Mach-O ad-hoc
   signature, which is valid.
2. Simulated a browser download: `ditto` the app, applied `com.apple.quarantine`, then
   `xattr -dr com.apple.quarantine` → attribute cleared.
3. Bounded launch of the de-quarantined binary: the process **execed and stayed alive** (not
   SIGKILL'd) → the kernel accepts the ad-hoc signature; the app is not "damaged" once quarantine is
   removed. (Full GUI launch on a physical Mac remains the standing human-UAT gate — RELEASE.md smoke
   steps 2–5.)

## Release-asset action

- **Yank:** NOT performed. Janne did not approve yanking `Vire_0.8.1_aarch64.dmg` as separate
  containment (proposal Q2 unanswered), and it is not needed — the asset is pristine and becomes
  installable via the corrected docs.
- **Re-upload / replace:** NOT performed and not required. The shipped asset is already the correct
  pristine `tauri:build` output; the only defect was the docs. Re-uploading the fresh build would only
  churn the sha256 with no user benefit and is an outward-facing action outside this change.
- **Net GitHub Release change:** none. The existing asset (`e77d15cf…`) is unchanged; the repo docs now
  describe the working install path for it.

## Scope / guardrails honored

- Docs-only Plan B. **No** `src-tauri/src`, schema, IPC, capability, or `tauri.conf.json` change (Plan A's
  signing block was not added — no cert). No auto-updater, release-feed, or CI change/claim introduced.
- Did **not** hand re-pack the DMG (spec requires pristine `tauri:build` output). The visible
  `.VolumeIcon.icns` cannot be flagged invisible without a manual re-pack, so it is left as standard
  Tauri output and documented as a Finder "show all files" artifact.
- Historical release notes (v0.1.0 TASK-026) and local-build first-launch notes (README build-from-source
  / packaged-app sections) were left unchanged — a locally-built app is not quarantined, so right-click →
  Open is correct in those non-download contexts.

## Follow-up (carried, not actioned here)

- **FB-054-1** (arch-review): BA to promote Developer ID signing + notarization (**TASK-028**) from
  deferred to required for user-facing releases, or formally accept the unsigned + quarantine-removal
  posture as the interim contract. Plan B is the honest interim; Plan A/TASK-028 remains the real fix.
