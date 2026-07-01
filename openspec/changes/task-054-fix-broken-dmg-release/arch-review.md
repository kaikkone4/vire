# Architecture review — TASK-054 (fix broken v0.8.1 DMG release)

**Verdict: PASS** — this is a bounded release-packaging + docs change on a single distribution
surface. No component boundary is crossed, no app-logic/schema/IPC/capability change. Owner is
**devops (release-packaging)**. Scope is proportional to the incident. Two human decisions gate the
outward-facing steps (see below).

## Diagnosis (confirmed against the repo, not assumed)

- **Root cause:** the app is **unsigned/ad-hoc + un-notarized** (`src-tauri/tauri.conf.json`
  `bundle.macOS` has no `signingIdentity`/notarization; `README.md:74-78`, `RELEASE.md:39` say so).
  The asset is `aarch64` (Apple Silicon). A browser download applies `com.apple.quarantine`; Gatekeeper
  then reports the app as **"damaged"** — the exact wording for an unsigned/ad-hoc quarantined,
  un-notarized app on Apple Silicon. TASK-053 already verified the downloaded bytes are intact
  (`sha256 e77d15cf…`, size `5,787,708`), so this is a **policy verdict, not corruption**.
- **Compounding doc defect:** the documented remedy (right-click → Open; "do not strip quarantine")
  does **not** clear the "damaged" dialog — it only clears "developer cannot be verified". Users have
  no working documented path → the release is effectively bricked regardless of the binary.
- **Secondary/cosmetic:** the visible `.VolumeIcon.icns` indicates the DMG volume-icon attribute
  wasn't applied or the DMG was hand-re-packed after build; not the cause of "damaged", but a manual
  re-pack could invalidate even the ad-hoc signature — verify the shipped DMG is a pristine
  `tauri:build` output.

## Architectural placement

- Fix lives entirely in the **release/distribution boundary**: `tauri.conf.json` bundle config
  (Plan A only), install docs, and the GitHub Release asset. Consistent with the deferred posture in
  RELEASE.md v0.1 ("signing/notarization out of scope for v0.1") and DEC-029 / TASK-028.
- **No source-logic change** → not a backend/Tauri developer task. Route to **devops**.
- Branch-aware by necessity: correct fix depends on cert availability (Q1). Both branches are
  specified so the change is complete regardless of the answer.

## Containment recommendation (outward-facing — needs Janne approval)

Yank the broken `Vire_0.8.1_aarch64.dmg` asset from Release `v0.8.1` now (design §5). The release
degrades gracefully to notes-only (update-check reads `tag_name`, not assets), which is safer than
leaving a trap that bricks every download and is actively promoted by the in-app updater. **Not
performed in this change** per task instruction ("do not modify the release yet").

## feedback_to_ba[]

- **FB-054-1 (assumption falsified):** the standing "unsigned/un-notarized is acceptable for
  distribution, mitigated by right-click → Open" posture (RELEASE.md v0.1; DEC-029) is **false for
  browser-downloaded Apple Silicon DMGs** — they hit the un-bypassable "damaged" dialog. Recommend BA
  promote **Developer ID signing + notarization (TASK-028)** from deferred to required for any
  user-facing release, or formally accept the unsigned + quarantine-removal-docs posture as the
  interim contract. Routed to ba-architect via Pi-Assistant.

## Open decisions gating implementation

- **Q1 (Plan A vs B):** Apple Developer ID Application cert + notarization credentials available?
- **Q2 (containment):** approve yanking the broken asset now?
