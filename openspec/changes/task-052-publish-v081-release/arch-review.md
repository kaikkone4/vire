# Architecture Review — TASK-052 publish v0.8.1 release

- **Tier:** L2 · **Component:** Vire desktop app + release-ops · **Verdict:** **PASS** (no split, no
  blocking BA escalation)
- **Reviewed against:** `artifacts/ba/03_architecture_plan.md` (update-check / release path), DEC-029
  (U-lite update check, Phase-1), the in-app-update-check spec deltas of TASK-050 / TASK-051, and
  the live repo state (`origin/main` @ `a3bd398`).

## 1. Scope alignment

TASK-052 is proportional and stays inside one component boundary. It moves **version strings** and a
**changelog entry**, then performs one **release-ops publication**. No source logic, schema, IPC,
CSP, capability, or dependency changes. This does not cross any component boundary defined in
`03_architecture_plan.md`, so **no split is required**. The update-check capability itself is already
correct on `main` (TASK-051 landed the canonical owner and the regression guards); TASK-052 only
supplies the two missing real-world inputs the capability needs: (a) matching compiled version
metadata, and (b) a published Release for the endpoint to return.

## 2. Root-cause confirmation (why the feature is still inert on `main`)

Two independent gaps, both verified live:

1. **`origin/main` ships `0.8.0`.** `git show origin/main:src-tauri/Cargo.toml` → `0.8.0`. The
   TASK-051 SW-6 bump (`3beece9`) is a **local-only, unmerged** commit; PR #39 merged only the URL
   fix (`c0d2350`). The `v0.8.1` tag was never created (`git tag -l` shows only `task-045/v0.6.3`).
2. **No GitHub Release exists.** `gh release list --repo kaikkone4/vire` → empty, so
   `/releases/latest` → `404` and every check fail-softs to `Unknown`.

Landing the metadata as a **fresh, reviewable diff off `main`** (rather than cherry-picking the
tag-blocked local commits) is the architecturally cleaner path: it produces one clean PR the gates
can evaluate, and it leaves no ambiguity about which commit the `v0.8.1` tag points at.

## 3. Architectural decision — ownership & ordering of the Release (the key question)

**Decision: the public GitHub Release is owned by the Release Manager (SW-6) and published only
*after* the code change merges to `main`. The version-metadata change is owned by the developer
(SW-2) and goes through the normal code gates first.** The two are sequenced, not concurrent.

Rationale:

- **Outward-facing, hard-to-reverse action.** Publishing a public GitHub Release is an external
  publication; it must reference the exact merged `main` commit. Creating it before merge would tag
  an unmerged or soon-rewritten commit. This is precisely the class of action that belongs to the
  release gate, after code is final — not to the implementation step.
- **Tag must be authoritative.** `/releases/latest` returns the release for the tag; the tag must
  point at the `0.8.1` commit on `main`. Ordering (merge → tag → publish) guarantees that.
- **Auth & blocker handling live with SW-6.** The TASK-051 signed-tag step stalled on an interactive
  GPG passphrase. The defined path avoids it: `gh release create v0.8.1 --target <merged-sha>`
  creates the tag **server-side** over the authenticated `gh` session (already `kaikkone4`), so no
  local signing prompt is hit. If a *signed* tag is later required by policy, that is a separate,
  interactive SW-6 session — not a TASK-052 blocker.

### Exact path (authoritative ordering)

1. **SW-2** — branch off `origin/main`; bump `Cargo.toml`, `tauri.conf.json`, and the `Cargo.lock`
   `vire` entry to `0.8.1`; add the `## v0.8.1` `RELEASE.md` entry. No `update_check/` change.
2. **Gates** — SW-3 QA (build + existing update-check tests still green; version triple = `0.8.1`),
   SW-4 review (diff = version strings + changelog only), SW-5 security (no new dep/egress). Merge PR.
3. **SW-6** — resolve the merged SHA; `gh release create v0.8.1 --repo kaikkone4/vire
   --target <sha> --title "Vire v0.8.1" --notes-file <notes>`; **full release, not draft, not
   prerelease**.
4. **Verify** — endpoint `200` + `tag_name=v0.8.1`; releases page `200`; `0.8.0` build →
   `UpdateAvailable`; `0.8.1` build → `UpToDate`.

## 4. Correctness analysis of the check against a real `v0.8.1` release

`compare_release` (`src-tauri/src/update_check/mod.rs:67`) strips a leading `v` from `tag_name`,
parses both sides with `semver`, and returns `UpdateAvailable` **iff** `latest > current`, else
`UpToDate` (equal or lower → up-to-date, never downgrade). Therefore:

- `current 0.8.0` vs `tag v0.8.1` → `0.8.1 > 0.8.0` → **UpdateAvailable** (real `html_url`). ✓
- `current 0.8.1` vs `tag v0.8.1` → equal → **UpToDate**. ✓

Both arms of the feature get exercised by the two builds, with **no logic change**. The tag MUST be
parseable semver after `v`-stripping (`v0.8.1` → `0.8.1`); a non-semver tag would fail-soft to
`Unknown` — call this out in the SW-6 notes (use `vMAJOR.MINOR.PATCH`).

## 5. Known limitation (deliberate, not a gap)

The Release may be published **notes-only**. The update-check reads only `tag_name` and `html_url`;
it never downloads or verifies assets. So `/releases/latest`, the semver verdict, and "Open
Releases" all function without any binary attached. Building / signing / notarizing / uploading
installers (`.dmg` / `.app`) and any auto-download/install path are **out of scope** for TASK-052
and deferred to the in-app-updater track (TASK-042 territory). This must be stated in the handoff so
the asset-less Release is not mistaken for an incomplete one — a user clicking "Open Releases" will
reach a real page that may not yet offer a downloadable binary. That is an accepted Phase-1 state.

## 6. feedback_to_ba (non-blocking)

- **Proposed DEC-031 (optional):** formalize the release-publication ownership/ordering (code gates →
  merge → SW-6 publishes a full GitHub Release; server-side tag via `gh release create --target` to
  avoid local signing prompts; `/releases/latest` requires non-draft/non-prerelease). Highest
  formalized decision in `artifacts/ba/07_decision_log.md` is DEC-030, so DEC-031 is free. This is a
  recommendation for ba-architect to record if they want it pinned; it does **not** block TASK-052
  and requires no BA-artifact change to proceed.

## 7. Verdict

**PASS.** Single component, proportional scope, no boundary crossing, no redesign. Architecture
constraints for implementation: (1) keep `update_check/` untouched; (2) version triple
(`Cargo.toml` / `tauri.conf.json` / `Cargo.lock`) must agree at `0.8.1`; (3) the Release is a
non-draft, non-prerelease, `v`-prefixed semver tag on the merged `main` commit, published by SW-6
after gates; (4) no assets, no updater, no signing in this task.
