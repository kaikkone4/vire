# Handoff — TASK-056 active-window capture Settings + privacy/status UI

- **Change dir**: openspec/changes/task-056-active-window-settings-ui/
- **Branch / PR**: feat/task-056-active-window-settings-ui · **PR #43 — ready for review (promoted from
  draft this gate)** — https://github.com/kaikkone4/vire/pull/43
  - Diff = the 16 original TASK-056 files + 4 doc/gate files committed this gate (`README.md`,
    `docs/active-window-capture.md`, `qa.md`, `review.md`, `sec.md`, `docs.md` — plus `tasks.md`/
    `handoff.md` updates), all in commit `97ee6eb`, pushed to origin. 20 files total vs `main`, verified
    clean (`git diff main...HEAD --name-only`).
- **Phase / gate**: SW-4 Code Review **PASS** + SW-5 Security **PASS** + Documentation gate **PASS** +
  **SW-6 Release: PASS** (see `RELEASE.md`) → **release gate complete**
- **Tier**: L2 (SEC-001/007/012; new IPC + truthful transparency copy)

## Last gate result
**SW-6 Release** (this session, see `RELEASE.md`): committed the Documentation gate's uncommitted doc
fixes plus the SW-3/4/5 gate artifacts (`qa.md`, `review.md`, `sec.md`, `docs.md`) that existed
untracked on the branch — commit `97ee6eb`, pushed. Wrote `RELEASE.md` with all three required
declarations: **Deployment size = MINOR** (new additive Settings UI + 2 new IPC commands, no breaking
change, no version-file bump in this PR — consistent with the project's pattern of separating feature
merges from dedicated version-bump/publish tasks); **Rollback = partial-automated** (`git revert` of the
merge commit, PR review cycle required, no schema/data cleanup needed since capture defaults OFF);
**Component compatibility matrix** = unchanged from the `0.8.1` baseline (zero lockfile diff; Tauri
2.2/`tauri` 2.11.2, Rust edition 2021, macOS 10.13+, `@tauri-apps/*` `^2.2.0`), plus the two new internal
IPC commands documented as bundled-artifact (no independent frontend/backend version skew possible).
Attempted the signed task-scoped tag `task-056/v0.8.1` — **blocked by SSH key passphrase** (non-
interactive session), same failure mode as `task-053`/`task-054` precedent; no unsigned fallback tag
created (confirmed no dangling tag object). Recorded **non-blocking**, per role policy — see
`RELEASE.md` §Tag status for the exact command for Janne to run locally. **PR #43 promoted from draft
to ready-for-review** this gate.

Prior gate — **Documentation gate** (see `docs.md`): verified default-OFF/macOS-only/no-titles claims,
status fields, and validation bounds are accurate against the shipped code. Fixed three stale/incomplete
spots: `README.md`'s top summary line still said "no in-app UI in this release" (false — TASK-056 ships
the Settings panel); `README.md` manual-verification step 2 asserted the literal *removed* sidebar string
`Manual Mode / Capture deferred` (replaced by `sidebarCaptureStatus()` in the SW-3 fix); and
`docs/active-window-capture.md`'s bounds prose omitted the 86400s upper caps on idle/away thresholds.
Added a `README.md` "Active-window capture Settings UI (TASK-056)" manual-verification section (steps
26–30) mirroring `tasks.md` §Smoke, so the still-outstanding physical-Mac smoke has a durable checklist.
Also checked off `tasks.md` workstreams A/B (confirmed fully implemented — `review.md` had flagged the
stale unchecked boxes as a non-blocking suggestion). `openspec validate --strict` re-run clean after all
edits. **These file edits are uncommitted on the branch** — SW-6 (or whoever commits next) should commit
`README.md`, `docs/active-window-capture.md`, `openspec/changes/task-056-active-window-settings-ui/{tasks.md,docs.md,handoff.md}`
before/with the release.

Prior gate — SW-3 QA **PASS** (see `qa.md`). Prior FAIL (Finding #1, stale always-on sidebar denial copy)
verified fixed and regression-guarded: `sidebarCaptureStatus(captureView)` now drives the sidebar box; 6
new tests incl. a source-scan guard that fails if the stale literal ever returns to `main.ts`. Full re-run
that session: backend 259/259 + active_window 66/66, `cargo fmt` clean, `npm run build` clean, frontend
146/148 (2 pre-existing unrelated `pi-observe.security` failures, file outside diff), `openspec validate
--strict` passes, PR #43 confirmed open/draft. Full scenario matrix (`specs/active-window-settings/spec.md`)
fully covered — no gaps.

## Active blockers
- **None.** (DEC-044 + FB-002 realization → ba-architect / arch-review.md remains non-blocking,
  unaffected.)
- **Non-blocking**: signed tag `task-056/v0.8.1` deferred — SSH key passphrase unavailable in this
  non-interactive session (see `RELEASE.md` §Tag status for the command to run locally).
- **Non-blocking**: physical-Mac smoke (`tasks.md` §Smoke; `README.md` steps 26–30) still not run in
  any session to date — carried forward as human/UAT.

## SW-5 security result
SW-5 **PASS**. Security artifact: `sec.md`.

- Blockers: none. No design-level escalation (`feedback_to_ba[]` empty).
- Tier-L2 scanners (this session): gitleaks `main..HEAD` no leaks; semgrep 112 rules / 6 changed files
  **0 findings**; Trivy secret+misconfig **0/0** (no container image → Trivy image role N/A); OSV over
  both lockfiles.
- SEC-001/007/012 verified by manual review: serde-allowlisted IPC input (`title_mode` undeserializable),
  parameterized rusqlite SQL, validate-before-persist bounds that reject (never silently clamp), all
  dynamic UI values `escapeHtml`-escaped, capture OFF by default, zero new grant/native/CSP/dep/network.
- **Carry-forward advisory (NOT blocking)**: OSV reports 1 HIGH (`quinn-proto` 7.5, RUSTSEC-2026-0185,
  transitive via `reqwest` QUIC path, unreached by app source) + GTK/glib cluster — **all pre-existing
  baseline** (lockfiles byte-identical to `main`; TASK-056 adds no deps). Belongs to the dependency-
  hygiene (task-047) lineage, independent of this Settings-UI change.

## SW-4 code review result
SW-4 **PASS**. Review artifact: `review.md`.

- Blockers: none.
- Suggestions only: `tasks.md` A/B backend checkboxes are stale unchecked despite implemented work, and
  one frontend comment says unknown keys are "rejected" where serde actually drops/ignores them.
- Checks run by SW-4: `cargo fmt --check --manifest-path src-tauri/Cargo.toml`, `cargo test --lib
  active_window --manifest-path src-tauri/Cargo.toml` (66/66), `npm run build`, `node --import tsx --test
  tests/activeWindowSettingsUi.test.mjs` (31/31), `openspec validate task-056-active-window-settings-ui
  --strict`.
- Full `npm run test:frontend` observed 144/148 with 4 failures in `tests/pi-observe.security.test.mjs`
  due to sandbox-denied `127.0.0.1` listen attempts; outside the TASK-056 diff.
- Scope guard held: 16 TASK-056 diff paths; no `title_mode` input; no native capture/AX/window-title,
  schema, dependency, capability, `tauri.conf.json`, or CSP change.

## Exact next action
1. **Janne**: merge PR #43 (now ready-for-review) when convenient; then run the signed-tag command in
   `RELEASE.md` §Tag status (`git tag -s task-056/v0.8.1 ...` on `97ee6eb`, then `git push origin
   task-056/v0.8.1`).
2. **Physical-Mac smoke** (tasks.md §Smoke; also `README.md` §"Active-window capture Settings UI
   (TASK-056)" steps 26–30) — still the mandated testable route; not yet run (no physical Mac in any
   session) — carry forward as human/UAT, does not block this release gate.
3. **Future version-bump task** (pattern: TASK-049/052): roll this merge into the next published
   `0.9.0` GitHub Release when ready.

_This session (SW-3 re-gate): re-ran `cargo test --lib` (259 passed), `cargo test --lib active_window`
(66 passed), `cargo fmt --check` (clean), `npm run build` (clean), `npm run test:frontend` (146/148,
same 2 pre-existing unrelated failures), `openspec validate --strict` (passes), `git diff main...HEAD
--name-only` (same 16 files), `gh pr view 43` (OPEN/draft). No files written besides `qa.md` (rewritten
PASS verdict) and this handoff._

## What SW-2 C+D shipped (this session)
- **C (frontend)** `src/active-window-settings-ui.ts` (NEW, pure builders): `capturePanel`,
  `captureStatusBlock`, `privacyTable`, `captureBanner`, `validateCaptureInput` (+ `CAPTURE_BOUNDS`
  mirroring backend), `healthMarkerLabel`. `src/main.ts`: new "Active-window capture" panel in
  `renderSettings()` (after Storage, before App-updates/Langfuse); `bindCaptureSettings()` mirrors
  `bindLangfuse()` (inline-validate → `set_active_window_capture_settings` → rerender; backend error
  verbatim via run/alertError); removed the false "does not collect … active windows, idle state"
  copy; `capture()` banner now driven by the real `get_active_window_capture_settings` view (off /
  on / macOS-only / unavailable), loaded in `renderToday()`+`renderSettings()`.
- **D (docs)** `docs/active-window-capture.md`: §Enabling documents the panel + the two IPC commands
  + safe bounds; §Not-yet-available drops the shipped UI/IPC bullets; privacy table aligned to the UI.
- `tests/activeWindowSettingsUi.test.mjs` (NEW, 25 tests): validation bounds/ordering/NaN, status copy
  (off/awaiting/healthy/degraded-with-cause/macOS-only/recent), privacy table, panel (toggle + 4 knobs
  + no title_mode input + non-macOS 6 disabled controls), truthful banner, XSS escaping.

## Notes / decisions carried forward
- **Backend `get_capture_status` (lib.rs:681) intentionally untouched** — it is orphan (unused by the
  frontend) and changing it is backend scope; the Today banner is driven truthfully from the real
  capture setting instead. Satisfies the FB-002 "truthful copy" scenario without reopening A/B.
- No schema change; r/w existing `settings`, read `active_window_*`. Loop re-reads config each tick →
  next-tick effect, no restart. `title_mode` stays redacted + unexposed (read-only, no input). OFF by
  default. Zero new perms. No new CSS (reuses `panel`/`lf-form`/`switch`/`banner`; base `table`).
- Merge coord w/ 055: both append to `renderSettings()`; this session added only the capture panel +
  bind + banner load — no overlap with 055's Suggestions surface.
- This session committed the FULL SW-2 change (A+B+C+D + change dir) in one commit — the branch had no
  prior task-056 commit (A+B were left uncommitted by the backend session).
