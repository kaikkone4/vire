# TASK-054 — Fix the broken Vire v0.8.1 DMG release asset (Gatekeeper "damaged")

## Why

The `Vire_0.8.1_aarch64.dmg` asset attached to GitHub Release `v0.8.1` in TASK-053 is **unusable by
every downloading user**. Two user-provided screenshots (Chrome download today 14:15) show:

1. the mounted DMG visibly contains `.VolumeIcon.icns` alongside `Vire` and the `Applications`
   shortcut; and
2. launching the app installed from the release-downloaded DMG fails with the macOS Gatekeeper
   dialog **"Vire on vioittunut eikä sitä voida avata. Siirrä se roskakoriin."** ("Vire is damaged
   and can't be opened. Move it to the Trash.").

This is not an isolated machine problem — it is the expected outcome of the current release posture,
and it makes v0.8.1 **not installable via the documented path**. TASK-050's in-app "update available"
affordance actively nudges users toward this broken asset, so the blast radius is every user who
follows the intended install/update flow.

## Root cause (diagnosed, not guessed)

The "damaged" verdict is a **Gatekeeper policy rejection of an unsigned, un-notarized, quarantined
app on Apple Silicon** — not literal file corruption:

- `src-tauri/tauri.conf.json` `bundle.macOS` has **no `signingIdentity` and no notarization
  configuration**; `README.md` and `RELEASE.md` both explicitly state the app is "not code-signed or
  notarized". The build is therefore unsigned / ad-hoc-signed only.
- The asset is `aarch64` (Apple Silicon). On Apple Silicon macOS requires a valid signature to run,
  and applies the `com.apple.quarantine` extended attribute to anything downloaded via a browser.
  Unsigned/ad-hoc **+ quarantine + not notarized → the specific "is damaged" message** (distinct from
  the "developer cannot be verified" message shown for Developer-ID-signed-but-un-notarized apps).
- The bytes are **not** corrupt: TASK-053 verified the downloaded asset's `sha256` matches
  (`e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`) and size `5,787,708`. This
  rules out a truncated/corrupt upload and confirms "damaged" is a signature/quarantine verdict.

**Compounding documentation defect (equally important):** README step 3 (`README.md:74-78`) and the
RELEASE smoke checklist (`RELEASE.md:39`) instruct users to **right-click → Open** and explicitly
say **"Do not … strip quarantine."** For the "damaged" failure mode this guidance is *wrong*:
right-click → Open resolves only the "developer cannot be verified" case, **not** "damaged". For an
unsigned/ad-hoc quarantined app on Apple Silicon the *only* user-side remedy is removing the
quarantine attribute — which the docs forbid. So a user following the documented steps has **no
working path to open the app.** This is why the release is effectively bricked, independent of the
binary itself.

The visible `.VolumeIcon.icns` is **cosmetic and secondary** — the DMG's volume-icon file is present
but its hidden/volume-icon attribute was not applied (or the DMG was assembled/re-packed by hand
after `tauri:build`). It is *not* the cause of "damaged", but if the DMG was manually re-packed that
step could additionally have invalidated even the ad-hoc signature. Treat as a hypothesis to verify,
not the primary cause.

## What this change decides

A focused **release-packaging + docs** OpenSpec change on a **single surface** — the macOS build/sign
posture (`tauri.conf.json`), the install docs (`README.md` / `RELEASE.md`), and the `v0.8.1` GitHub
Release asset. It does **not** change app logic, schema, IPC, capabilities, or any `src-tauri/src`
source. Because the correct fix depends on a fact this role cannot verify — whether an Apple
Developer ID Application certificate + notarization credentials are available — the proposal is
**branch-aware**:

- **Plan A — proper fix (preferred, requires Apple Developer ID + notarization creds).** Add
  Developer-ID signing + hardened runtime + Apple notarization to the Tauri macOS build, rebuild,
  sign, notarize, `stapler staple`, verify with `spctl`, and re-upload the DMG. Eliminates the error
  entirely; no user workaround needed. Docs updated to drop the Gatekeeper dance.
- **Plan B — interim honest fix (if no cert is available).** Keep the app unsigned but make the docs
  *truthful*: for the "damaged" error the correct step is stripping quarantine
  (`xattr -dr com.apple.quarantine /Applications/Vire.app`) with a clear security caveat, and re-ship
  a **pristine `tauri:build` DMG** (not hand-re-packed) so the ad-hoc signature and volume icon are
  intact. Right-click → Open guidance is corrected/removed for this failure mode.

It also decides **immediate containment** (outward-facing → Janne approval, see design §5): yank the
broken `Vire_0.8.1_aarch64.dmg` asset from Release `v0.8.1` now, pending the fix, so no further users
download a bricked installer.

## What changes

- New OpenSpec change dir `openspec/changes/task-054-fix-broken-dmg-release/`
  (`proposal.md`, `design.md`, `tasks.md`, `specs/release-distribution/spec.md`, `arch-review.md`,
  `handoff.md`).
- The designed follow-up (dev/ops task) touches:
  - `src-tauri/tauri.conf.json` — **Plan A only**: add `bundle.macOS.signingIdentity` +
    hardened-runtime/notarization wiring (small config block; no source change).
  - `README.md` §"Download & install (prebuilt)" step 3 + `RELEASE.md` Gatekeeper lines — corrected
    first-launch guidance matching the actual failure mode (both plans).
  - the **GitHub Release `v0.8.1`** — yank the broken asset (containment) and, after the fix,
    re-upload a corrected DMG. **Outward-facing publish actions requiring Janne's authorization.**
- **No** change to `src-tauri/src`, capabilities, schema, migrations, IPC, or CI in this change.

## Scope guardrails (explicit exclusions)

- No `tauri-plugin-updater` / auto-installer (still TASK-042 territory).
- No release-feed automation or CI release workflow (there is none today; out of scope).
- No `x86_64` / universal build — `aarch64` only, unchanged from TASK-053.
- No app-logic, schema, IPC, or capability change.

## Blocking questions (route to Janne via Pi-Assistant before dev/ops executes)

- **Q1 — cert availability (selects Plan A vs B):** Is there an Apple Developer Program membership +
  a **Developer ID Application** certificate and notarization credentials (Apple ID + app-specific
  password *or* App Store Connect API key + Team ID) available to the build/CI environment?
- **Q2 — containment approval (outward-facing):** Approve yanking the broken
  `Vire_0.8.1_aarch64.dmg` asset from Release `v0.8.1` immediately, pending the fix? (Recommended —
  see design §5.)
