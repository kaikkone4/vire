# Architecture Review — TASK-049 v0.8.0 version/docs metadata + macOS DMG install-artifact cleanup

- **Reviewer**: SW Architect (SW-1) · **Date**: 2026-06-22 · **Verdict**: **PASS** (single task)
- **Type**: release-hygiene / packaging-metadata + docs. No source, schema, IPC, capability, CSP,
  capture-behavior, UI, or third-party dependency change.

## Summary

TASK-048's post-merge code-to-spec diverged: the v0.8.0 docs never landed and the build version
fields were never bumped. Plus a reported macOS install artifact (faded/grey `.VolumeIcon.icns`).
I built and mounted the current artifact and verified every claim, then researched the Tauri v2
levers against official docs. The three workstreams are one cohesive release-hygiene change.

## Architectural findings

1. **Version drift is real and pure-metadata.** `tauri.conf.json:4` and `Cargo.toml:3` both say
   `0.1.0`; the produced `Info.plist` and `Vire_0.1.0_aarch64.dmg` confirm the stamp. `README.md:79`
   also hardcodes the stale name. Fix = bump both to `0.8.0` (Tauri uses `tauri.conf.json` `version`,
   falling back to Cargo.toml when absent — keep both in sync). No component boundary touched.

2. **The "faded icon" is expected macOS behavior, not a defect we can compile away.** The DMG
   volume root carries hidden `.VolumeIcon.icns` (byte-identical to the app `.icns`) + `.DS_Store`;
   these are standard `create-dmg` volume metadata, dimmed in Finder only when *Show hidden files*
   is on, and copied to `/Applications` only by a Select-All drag. **Tauri v2 exposes no config to
   suppress them** (tauri#9253, tauri#11190 — both OPEN). Correct, in-scope fix = the supported
   `bundle.macOS.dmg` layout + install docs that say "drag only `Vire.app`" and explain the hidden
   files. A post-bundle DMG re-pack to delete them is rejected (out of scope, brittle, kills the
   custom volume icon).

3. **No NFR or boundary impact.** Runtime code path, startup, capture, and schema are untouched; the
   `.app` payload is byte-equivalent apart from the version string. `Cargo.lock` will change only its
   own `vire` self-version line — no dependency delta, so the TASK-047 advisory posture is intact.

## Component-boundary check (`03_architecture_plan.md`)

No boundary crossed. This change lives entirely in packaging metadata (`tauri.conf.json`,
`Cargo.toml`) and documentation. No new module, no cross-component contract, no data-model change.

## Split decision

**No split.** Version + docs + install-UX share the v0.8.0 release cut and the `README.md` file.
Splitting would fragment a single release-hygiene change and add coordination cost on one file.

## BA escalation

**None.** No `feedback_to_ba[]`. macOS-only packaging is an established project decision; nothing
here contradicts a BA-flow architecture assumption.

## Verified vs assumption (full ledger in `design.md` §7)

- **Verified locally** (build + read-only mount): all version facts; DMG contents; `.VolumeIcon.icns`
  byte-identity; absence of `.background`; leftover `rw.*.dmg` is unrelated build noise.
- **Verified (official docs)**: Tauri `version` semantics + Cargo.toml fallback; `bundle.macOS.dmg`
  field set/defaults; no suppression config (#9253, #11190 OPEN) — URLs in `design.md` §3.
- **Assumption (stated)**: Janne's Finder has *Show hidden files* on and the file reached
  `/Applications` via a Select-All drag — the most consistent reading of the verified evidence; the
  adopted fix is correct under either interpretation.

## Handoff

To **SW-frontend/devops** for implementation (T1–T5 in `tasks.md`) and **SW-3** for a real
packaged-build verification (V1–V6). `handoff.md` holds compact state.
