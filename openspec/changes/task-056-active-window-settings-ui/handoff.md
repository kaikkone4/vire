# Handoff — TASK-056 active-window capture Settings + privacy/status UI

- **Change dir**: openspec/changes/task-056-active-window-settings-ui/
- **Branch / PR**: feat/task-056-active-window-settings-ui · **draft PR #43** — https://github.com/kaikkone4/vire/pull/43
  - Branch rebased clean onto `main` (base 6eaffc8); diff = only the 16 TASK-056 files. The 5 inherited
    TASK-053/054 release commits were dropped. Single impl commit (byte-identical to original) + this handoff commit.
- **Phase / gate**: SW-2 A+B+C+D DONE → ready for **SW-3 QA gate**
- **Tier**: L2 (SEC-001/007/012; new IPC + truthful transparency copy)

## Last gate result
SW-1 PASS. SW-2 A+B backend (prior session): cargo test/fmt/clippy clean, 259 lib tests green.
SW-2 C+D frontend (this session): `npm run build` (tsc+vite) green; new suite 25/25; full
`test:frontend` 140 pass + **2 pre-existing unrelated `pi-observe.security` failures** (fail with my
`main.ts` stashed — not caused by TASK-056). `openspec validate --strict` passes.

## Active blockers
- none. DEC-044 + FB-002 realization → ba-architect (arch-review.md), **non-blocking**.

## Exact next action
1. **SW-3 QA gate** (sw-qa-engineer) over the whole SW-2 change.
2. **Physical-Mac smoke** (tasks.md §Smoke) — the mandated testable route; run on the target Mac.

_Branch hygiene DONE (this session): rebased clean onto main + draft PR #43 opened. `test:frontend`
re-run post-rebase = 142 tests, 140 pass, 2 fail (both pre-existing unrelated `pi-observe.security`)._

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
