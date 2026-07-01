# Tasks — TASK-056 active-window capture Settings + privacy/status UI

Design + constraints: `design.md`, `arch-review.md`. One cohesive capability, three thin workstreams
(**A backend IPC → B status projection → C frontend**; A+B gate C). Do the minimum; reuse the Langfuse /
env-mapping settings pattern. **No capture change, no new permission, no window titles, no network, no
schema change.** Check off as you go.

## A. Settings read/write IPC (backend)

- [x] Add `get_active_window_capture_settings(state) -> CmdResult<CaptureSettingsView>` in `lib.rs`:
      resolve `active_window::config::CaptureConfig::from_settings(conn)` into the view; set
      `platform_supported = cfg!(target_os = "macos")` and `title_mode = "redacted"` (informational).
- [x] Add `set_active_window_capture_settings(state, input: CaptureSettingsInput) -> CmdResult<CaptureSettingsView>`:
      **validate** (design §1 bounds: `sample_seconds ∈ [1,3600]`, `idle_candidate_seconds ≥ 1`,
      `idle_away_seconds > idle_candidate_seconds`, `retention_days ∈ [1,3650]`) then upsert the five
      `settings` keys in one transaction via `INSERT … ON CONFLICT(key) DO UPDATE`; re-resolve + return the
      fresh view. Do **not** accept or write `title_mode`.
- [x] Register both commands in `generate_handler!` (`lib.rs:1205`). Optionally house the DTOs + validation
      in a new `src-tauri/src/active_window/settings_api.rs` (declare `pub mod settings_api;` in
      `active_window/mod.rs`) to keep `lib.rs` tidy — no behavior change either way.
- [x] Tests: reject 0-interval, non-ordered away/idle, 0 retention, out-of-range → clear error, no row
      changed; valid `set` writes exactly the five keys and `get`/`CaptureConfig::from_settings` resolve
      them; enable→disable round-trips; `title_mode` untouched by `set`.

## B. Capture status / health projection (backend, read-only)

- [x] Add `active_window::store::capture_status_snapshot(conn, now_day, retention_from_day) -> CaptureStatusView`
      (design §2): `MAX(sample_ts)`, `samples_today` (raw WHERE day), `evidence_blocks_retained`
      (evidence WHERE day ≥ retention_from), `open_health` (`capture_health` WHERE `end_ts IS NULL`),
      `recent_health` (last N). Read-only; **no schema change, no write path, redaction gate untouched.**
- [x] Wire the snapshot into `get_active_window_capture_settings` (`status` field).
- [x] Tests: seeded fixture → correct counts / open-vs-recent health / last-sample; empty DB → zeros +
      `None` last sample (not an error); structural check still holds (no prohibited column read/returned).

## C. Settings panel + truthful copy (frontend)

- [x] Add an **"Active-window capture"** panel to `renderSettings()` (`src/main.ts`): enable toggle;
      numeric inputs for sample interval, idle-candidate seconds, away seconds, retention days; **Save**;
      live status/health readout; **privacy table** (Captured vs Never-captured, mirroring
      `docs/active-window-capture.md` §Privacy posture). Load via `get_active_window_capture_settings`; save
      via `set_active_window_capture_settings`; bind fn mirrors `bindLangfuse()`; inline-validate before the
      call; surface backend errors verbatim; `rerender()` on success.
- [x] **Replace the inaccurate copy:** removed the static *"does not collect … active windows, idle state"*
      sentence in `renderSettings()`; capture-status copy is now driven by the real setting. The Today
      `capture()` banner now reflects actual enabled/off/macOS-only state (driven by the real
      `get_active_window_capture_settings` view, not the hard-coded v0.1 string; backend `get_capture_status`
      left untouched — it stays out of the frontend workstream and native-capture scope).
- [x] macOS-only: when `platform_supported === false`, panel renders read-only with an "available on macOS
      only" note and disabled controls.
- [x] Extracted pure builders into `src/active-window-settings-ui.ts` (mirrors `env-mapping-ui.ts`/
      `langfuse-settings.ts`) for pure-builder tests.
- [x] Frontend tests (`tests/activeWindowSettingsUi.test.mjs`, 25 tests): panel render, four-field
      validation (bounds + ordering + NaN), status/health copy (healthy / degraded-with-cause / off /
      no-samples-yet / macOS-only / recent-health), privacy table, macOS-only disabled state, truthful
      banner, XSS escaping. `npm run test:frontend` (140 pass; 2 pre-existing unrelated `pi-observe`
      failures) + `npm run build` green.

## D. Docs

- [x] Updated `docs/active-window-capture.md`: §"Enabling capture" now documents the Settings panel + the
      two IPC commands + safe bounds (replacing *"not yet available"*); §"Not yet available" no longer lists
      the shipped UI/IPC bullets. Privacy table kept authoritative and aligned with the UI table (added the
      display-name + terminal-command-bodies + prompts/secrets rows to match `privacyTable()`).

## Smoke — physical-Mac testable route (mandated)

Run on the target Mac (unsigned/ad-hoc build is fine — zero-permission capture is signing-independent):

- [ ] From Settings, **enable** capture → status shows *running*; within `sample_seconds`, `last_sample_ts`
      advances and `samples_today` increments; evidence blocks accrue.
- [ ] Confirm **no TCC/permission prompt** ever appears and no AX call / event tap is made (reuse the
      TASK-048 `VIRE_TASK048_LIVE_PROBE` posture); verify captured rows keep `window_title = NULL`.
- [ ] Change **sample interval** and **idle thresholds**, Save → the new cadence/thresholds take effect on
      the next tick (no restart); invalid inputs are rejected with a clear message.
- [ ] Induce a gap (lock screen / no frontmost app) → status names the degraded state (`no_gui_session` /
      `sampling_gap`) with its start time; nothing is silently blank.
- [ ] **Disable** capture → sampling stops (no new raw rows); the privacy copy and Today banner reflect the
      disabled state truthfully.

## Validation

- [x] `cargo test` / `cargo fmt --check` / `cargo clippy` clean for the backend additions (A+B session;
      259 lib tests green per handoff — not re-run in this frontend-only session).
- [x] `npm run test:frontend` (new suite 25/25; full 140 pass, 2 pre-existing unrelated `pi-observe`
      failures) + `npm run build` (tsc + vite) green.
- [x] `openspec validate task-056-active-window-settings-ui --strict` passes.
