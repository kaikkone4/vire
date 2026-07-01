# Handoff — TASK-056 active-window capture Settings + privacy/status UI

- **Change dir**: openspec/changes/task-056-active-window-settings-ui/
- **Branch / PR**: feat/task-056-active-window-settings-ui · **draft PR #43** — https://github.com/kaikkone4/vire/pull/43
  - Branch rebased clean onto `main` (base 6eaffc8); diff = only the 16 TASK-056 files. The 5 inherited
    TASK-053/054 release commits were dropped. Single impl commit (byte-identical to original) + this handoff commit.
- **Phase / gate**: SW-2 fix for qa.md Finding #1 DONE → **awaiting SW-3 re-gate**
- **Tier**: L2 (SEC-001/007/012; new IPC + truthful transparency copy)

## Last gate result
SW-3 QA **FAIL** (see `qa.md`, Finding #1) — now **fixed** (SW-2, this session). All other checks were
already clean (backend 259 lib tests, frontend build, `openspec validate --strict`, full scenario
matrix). The one truthful-copy gap is closed.

**Fix for qa.md Finding #1 (this session):** the always-on left-sidebar `.status` box in `shell()`
(`src/main.ts`) no longer hard-codes `Manual Mode / Capture deferred — No automatic activity capture
runs in v0.1`. New pure builder `sidebarCaptureStatus(captureView)` in `active-window-settings-ui.ts`
drives it from the real setting (off / on / macOS-only / null-neutral), exactly like `captureBanner`.
No view can now assert "no capture runs" once capture is enabled. Backend/native/title_mode/
Accessibility/release scope untouched.

## Active blockers
- **None** for SW-3 re-gate. (DEC-044 + FB-002 realization → ba-architect / arch-review.md remains
  non-blocking, unaffected.)

## Exact next action
1. **QA (SW-3)**: re-gate. Verify Finding #1 closed — `sidebarCaptureStatus` builder + `main.ts`
   wiring; new sidebar tests in `tests/activeWindowSettingsUi.test.mjs` (incl. capture-enabled +
   source-scan regression guard).
2. **Physical-Mac smoke** (tasks.md §Smoke) — still the mandated testable route; not yet run (no
   physical Mac in this session).

_This session (SW-2 fix): `npm run build` clean; `npm run test:frontend` = 148 tests, 146 pass, 2 fail
(both pre-existing unrelated `pi-observe.security`, file untouched by this diff). +6 sidebar tests, all
pass. Working-tree diff = 3 code files (`main.ts`, `active-window-settings-ui.ts`, its test) + this
handoff, committed as the latest commit on `feat/task-056-active-window-settings-ui` (draft PR #43)._

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
