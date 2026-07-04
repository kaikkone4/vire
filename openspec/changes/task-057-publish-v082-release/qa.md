# QA Gate — TASK-057 Part A (publish v0.8.2 release: version bump + docs)

**Branch:** `feat/task-057-publish-v082-release` · **PR:** #44 (draft, base `main`) · **Tier:** L2
**Scope gated:** Part A only (version triple + docs). Part B (DMG build + `gh release create` +
live update-check smoke) is explicitly out of scope here — carried by SW-6.

## Verdict: PASS

## Branch provenance

- `git merge-base origin/main feat/task-057-publish-v082-release` = `df79423` (the TASK-056 merge
  commit) after `git fetch origin`. `git merge-base --is-ancestor df79423 <branch>` confirms
  `df79423` is an ancestor of the branch tip. Branch correctly cut from the real current `main`,
  including TASK-056. (Local `main` ref was stale at `6eaffc8`/TASK-052 before fetch — expected per
  `design.md`'s noted precondition, not a defect.)

## Change scope — verified against `df79423..HEAD`

Exactly 5 files changed outside `openspec/changes/`, matching `proposal.md`/`tasks.md` Part A scope
1:1, no more, no less:
- `src-tauri/Cargo.toml` — `version = "0.8.1"` → `"0.8.2"`.
- `src-tauri/tauri.conf.json` — `"version": "0.8.1"` → `"0.8.2"`.
- `src-tauri/Cargo.lock` — single-line diff, only the `vire` package `version` entry
  (`0.8.1`→`0.8.2`); no other dependency line touched, confirming zero dep drift.
- `RELEASE.md` — new `## v0.8.2` section prepended; `## v0.8.1` / `## v0.8.0` and all older entries
  (16 version headings total) intact and unmodified below it.
- `README.md` — `Current version: v0.8.0` → `v0.8.2`; both DMG filename examples
  `Vire_0.8.1_aarch64.dmg` → `Vire_0.8.2_aarch64.dmg`.

Confirmed **no** changes anywhere under `src-tauri/src/update_check/` (diff is empty against that
path), and no `tauri-plugin-updater` (or any updater dependency) added in `Cargo.toml`/`Cargo.lock`.
No schema/CSP/capture/classifier/Langfuse touch — none of those paths appear in the diff.

## Content accuracy checks

- Version triple all read `0.8.2` on the branch tip (`tauri.conf.json`, `Cargo.toml`, `Cargo.lock`
  `vire` block) — grepped directly, not just diffed.
- `RELEASE.md` v0.8.2 entry: states no source/schema/CSP/dependency change beyond version strings;
  documents the DMG asset name and the unsigned/unnotarized posture; contains the exact
  `xattr -dr com.apple.quarantine /Applications/Vire.app` remedy, an official-source trust caveat,
  and the TASK-028 Developer-ID/notarization pointer — matches spec requirement wording. Grepped the
  full file for `sudo` / `spctl --master-disable`: the only hit is the sentence stating the notes
  *never* instruct `sudo` or a global Gatekeeper disable — not an actual instruction. No prohibited
  guidance present.
- `README.md` edits are accurate and localized to the two intended spots (header line, DMG example)
  plus the artifact-location table's parenthetical example; no unrelated copy changed.

## Scenario coverage matrix (`specs/in-app-update-check/spec.md`)

| Requirement / Scenario | Coverage | Notes |
|---|---|---|
| Req 1 — Scenario: 0.8.1 build sees the update | **Covered (code/test level)** | `update_check::tests::newer_release_is_update_available` exercises the exact unchanged `parse_release_payload`/`compare_release` path with a higher-version pair; version-value-agnostic logic, untouched by this change. Literal 0.8.1→0.8.2 live-binary check is Part B (deferred per task scope). |
| Req 1 — Scenario: 0.8.2 build is up to date | **Covered (code/test level)** | `same_version_is_up_to_date` exercises the equal-version path; `lower_release_is_up_to_date` confirms the never-downgrade guarantee holds. Literal 0.8.2-build live smoke is Part B. |
| Req 1 — Scenario: draft/prerelease does not satisfy | **N/A to Part A** | Governs how SW-6 invokes `gh release create` (no `--draft`/`--prerelease`); no code path in this repo to unit-test. Correctly a Part B/operational responsibility — flagged, not a Part A gap. |
| Req 2 — Scenario: DMG asset present and downloadable | **Deferred to Part B** | No DMG is built or published in Part A; nothing to verify yet. |
| Req 2 — Scenario: Notes carry remedy, not a bypass | **Content pre-verified here** | The exact notes text SW-6 will carry into `gh release create --notes-file` is authored in this change's `RELEASE.md` v0.8.2 entry and verified accurate above (command, caveat, TASK-028 pointer, no bypass wording). Actual publish action is Part B. |

No scenario in this spec delta is unaccounted for: every one is either code-level-covered now or
explicitly and correctly deferred to Part B per the task's stated boundary.

## Checks run (independently re-executed, not just trusted from handoff)

- `cargo build --locked` — **OK** (only pre-existing dead-code warnings in unrelated `active_window`
  modules, not introduced by this change).
- `cargo test update_check` — **11 passed / 0 failed**.
- `node --import tsx --test tests/updateCheckUi.test.mjs` — **12 passed / 0 failed**.
- `cargo fmt --all -- --check` — clean.
- `npm run build` (tsc + vite) — clean, no errors.
- `openspec validate task-057-publish-v082-release --strict` — valid.

No flaky behavior observed across the above (single run each, deterministic unit/UI tests, no
timing- or network-dependent assertions in scope).

## Tier L2 proportionality note

This change is a version/docs-only reconciliation with **no new application logic** (Part A adds no
code path). L2's "performance baselines + negative-path tests" are satisfied by the pre-existing
`update_check` suite's negative/never-downgrade case (`lower_release_is_up_to_date`) and equal-case
(`same_version_is_up_to_date`), which this change does not alter. No new integration test file was
added — none is warranted since no new behavior exists to cover; writing one would duplicate
existing coverage of unchanged logic.

## Blockers

None for Part A. Full live release/DMG/binary verification remains correctly scoped to SW-6 Part B
per `tasks.md` and is not a Part A gate blocker.
