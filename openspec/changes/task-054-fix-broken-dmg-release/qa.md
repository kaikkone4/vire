# QA — TASK-054 fix broken v0.8.1 DMG release (Plan B, docs-only)

**Verdict: PASS**

## Recheck (2026-07-01, after SW-2 wrapper-tag cleanup)

SW-4 review had found leftover `</content>`/`</invoke>` wrapper-tag residue at the tail of
`ops-review.md` (tool-call artifact, not review content) — the sole blocker. SW-2 removed it
(commit `e217603`). Re-verified from scratch, not just diffed against the prior pass:

- `rg '</content>|</invoke>|<invoke|<content'` across `ops-review.md`, `README.md`, `RELEASE.md` →
  no matches inside the artifact itself; the only hits are textual mentions of the tags in
  `handoff.md` (describing the fix) and this gate's own history in `review.md` (quoting the finding)
  — not residue.
- `ops-review.md` tail now ends cleanly at the "Follow-up (carried, not actioned here)" section with
  a single trailing newline; content (root-cause table, open-path verification, follow-up) intact.
- `git diff origin/main...HEAD --stat`: still exactly `README.md`, `RELEASE.md`, and
  `openspec/changes/task-054-fix-broken-dmg-release/*` (490 insertions / 6 deletions across 9
  files) — no `src-tauri/`, `tauri.conf.json`, `.github/`, `package.json`, or lockfile touched.
  Confirmed via explicit name-only exclusion filter: zero files outside that scope.
- `openspec validate task-054-fix-broken-dmg-release --strict` → valid.
- `git diff origin/main...HEAD --check` → clean.
- Trailing-newline check on all four touched non-openspec-boilerplate files (`README.md`,
  `RELEASE.md`, `ops-review.md`, `handoff.md`) → all end with exactly one newline.
- All original scenario-coverage findings below (diff surface, scenario matrix, targeted checks)
  re-confirmed unchanged — the only delta since the original PASS is the wrapper-tag removal itself.

**Verdict unchanged: PASS.** No new blockers. Routes to SW-4 (recheck) + SW-5 per handoff (SW-5
already PASS, no rework needed there).

## Scope verified

Gate is over a **docs-only** release-packaging fix (no application code). "Test coverage" here means:
observable, reproducible verification that each spec scenario's claimed behavior is true against the
actual diff and the diagnostic evidence already gathered by devops (`ops-review.md`), not new unit/
integration test files (none apply — no source changed).

## Diff surface confirmed

`git diff origin/main...HEAD --stat`: only `README.md`, `RELEASE.md`, and
`openspec/changes/task-054-fix-broken-dmg-release/*` (arch-review.md, design.md, handoff.md,
ops-review.md, proposal.md, specs/, tasks.md). No `src-tauri/`, `tauri.conf.json`, `.github/`,
`package.json`, or lockfile changes — matches the Plan B "docs-only" claim and the design/tasks
guardrails (no signing block, no updater, no CI). `openspec validate task-054-fix-broken-dmg-release
--strict` → valid. `git diff --check` → clean; both files end with a single trailing newline.

## Scenario coverage matrix (spec: `specs/release-distribution/spec.md`)

| Scenario / Requirement | Coverage | Result |
|---|---|---|
| "A browser-downloaded release opens by following the docs" | ops-review.md §Open-path verification: `ditto` + applied `com.apple.quarantine` to simulate a browser download, ran the documented `xattr -dr com.apple.quarantine`, bounded-launched the de-quarantined binary — process exec'd and stayed alive (not signature-killed). Covers the unsigned-build branch of the scenario end-to-end except full GUI double-click, which needs a physical Mac. | **PASS** (full GUI open recorded as standing human gate, see below — not a new blocker, matches TASK-053 precedent) |
| "The attached asset matches the release version and is a clean build" | ops-review.md table: shipped asset sha256/size byte-identical to the published GitHub asset; `CFBundleShortVersionString = 0.8.1`; fresh `npm run tauri:build` is structurally identical (same signature, Identifier, layout) → confirms pristine, not hand-re-packed. sha256 recorded in `RELEASE.md` (unchanged, `e77d15cf…`). | **PASS** |
| "The download/install path is documented for prebuilt releases" — reader finds a remedy that works | `README.md:74-97` diff read directly: replaces right-click→Open guidance with `xattr -dr com.apple.quarantine /Applications/Vire.app`, states plainly it bypasses Gatekeeper because the app is unsigned, and points to TASK-028 (signed+notarized) as the real fix. `RELEASE.md:39,45-55` carries matching guidance in the smoke checklist and a new TASK-054 subsection. No right-click→Open or "do not strip quarantine" language remains for the "damaged" failure mode in either file. | **PASS** |

## Targeted checks

- **No false signing/notarization claims**: both docs still say "not code-signed or notarized," consistent with the confirmed `adhoc, linker-signed` / `TeamIdentifier=not set` state in `ops-review.md`. No claim of Developer ID signing or notarization was introduced.
- **No unintended source/CI/updater changes**: confirmed via the diff-surface check above — zero touches to `src-tauri/`, `.github/`, `package.json`, lockfiles, or updater code.
- **`.VolumeIcon.icns` visibility**: correctly classified, not a defect requiring a fix. `ops-review.md` shows a fresh `tauri:build` reproduces the identical dot-file/attribute state as the shipped asset (Tauri's `bundle_dmg.sh` sets the volume custom-icon flag but never the file's own invisible bit); flagging it invisible would require a manual re-pack, which the spec explicitly forbids ("pristine... not manually re-packed"). The pre-existing `README.md` "Install and run" section (source-build path, untouched by this diff) already documents `.VolumeIcon.icns`/`.DS_Store` as normal DMG metadata — consistent, no contradiction introduced.
- **Local-build vs. downloaded-build docs stay coherent**: the unchanged local-build sections (`README.md:182-184,306`, `RELEASE.md:752`) still say right-click→Open, which is correct there — a locally built app carries no quarantine attribute, so that failure mode is the "unidentified developer" case, not "damaged." The new download-path section correctly does not apply that remedy to the quarantined case.
- **Release-asset action matches docs' claims**: `RELEASE.md`'s new TASK-054 subsection states the asset "was not re-uploaded and not yanked" — this matches the actual GitHub state (devops did not touch the release) and is justified by the pristine-rebuild comparison, so no re-upload was needed to satisfy the spec's "asset is untouched tauri:build output" scenario.
- **Handoff coherence**: `handoff.md` accurately reflects Plan B completion, the changed-files list, the "no release-asset action" decision, and routes to SW-3 (this gate) then merge. Matches the state found on disk.

## Non-blocking observations (do not affect gate verdict)

1. **Standing human gate, not new**: full GUI first-launch on a physical Mac (`RELEASE.md` smoke steps 2–5) remains unverified in-pipeline — same standing gate already accepted at TASK-053's merge (no physical Mac available to this pipeline). The task's own scope note says this is a carried-forward human gate, not a precondition for this docs fix to merge. Recorded, not blocking.
2. `tasks.md` checkboxes (§2B, §4) are left unchecked even though the corresponding work is done and reflected in `ops-review.md`/`handoff.md`. Cosmetic — recommend devops tick them in a follow-up, not worth a FAIL for a docs-only change.
3. Uncommitted working-tree edits to `openspec/changes/task-053-release-dmg-asset-smoke/handoff.md` and `review.md` are present locally but are **not part of this PR's diff** (`git diff origin/main...HEAD` confirms) — they belong to TASK-053's already-merged history and are out of scope for this gate. Flagging so they aren't mistaken for TASK-054 scope creep.

## Blockers

None.
