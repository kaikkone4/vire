# Handoff — TASK-056 active-window capture Settings + privacy/status UI

- **Change dir**: openspec/changes/task-056-active-window-settings-ui/
- **Branch / PR**: feat/task-056-active-window-settings-ui · **draft PR #43** — https://github.com/kaikkone4/vire/pull/43
  - Branch rebased clean onto `main` (base 6eaffc8); diff now = the 16 TASK-056 files + this session's doc
    fixes (`README.md`, `docs/active-window-capture.md`, `tasks.md`, `docs.md`, this handoff) — **not yet
    committed**, see below.
- **Phase / gate**: SW-4 Code Review **PASS** + SW-5 Security **PASS** + **Documentation gate: docs updated,
  no blockers** (see `docs.md`) → **ready for SW-6 Release Manager**
- **Tier**: L2 (SEC-001/007/012; new IPC + truthful transparency copy)

## Last gate result
**Documentation gate** (this session, see `docs.md`): verified default-OFF/macOS-only/no-titles claims,
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
- **Non-blocking**: this session's doc fixes are uncommitted — see note above.

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
1. **Release Manager (SW-6)**: commit this session's doc fixes (`README.md`,
   `docs/active-window-capture.md`, `openspec/changes/task-056-active-window-settings-ui/{tasks.md,docs.md,handoff.md}`)
   alongside/before the release, then proceed with the release bundle.
2. **Physical-Mac smoke** (tasks.md §Smoke; also now `README.md` §"Active-window capture Settings UI
   (TASK-056)" steps 26–30) — still the mandated testable route; not yet run (no physical Mac in this
   session) — carry forward as human/UAT, does not block SW-4/SW-5/docs gate.

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
