# QA — TASK-056 active-window capture Settings + privacy/status UI (SW-3 re-gate)

- **Branch / PR**: feat/task-056-active-window-settings-ui · draft PR #43 (open)
- **Diff scope verified**: `git diff main...HEAD --stat` = exactly the same 16 files as the prior gate
  (7 openspec docs + `docs/active-window-capture.md` + 5 backend + 2 frontend + 1 test file). No stray
  non-TASK-056 changes. `capture.rs`/`config.rs` (native capture path) still untouched.
- **Tier**: L2

## Verdict: PASS

Re-gate after SW-2 fix for Finding #1 (prior FAIL). Fix verified closed, regression-guarded, and
scoped to exactly the files claimed. All previously-clean checks re-confirmed.

## Fix verification (prior Finding #1)

Prior FAIL: the always-on left-sidebar `.status` box in `shell()` (`src/main.ts`) hard-coded
"Manual Mode / Capture deferred — No automatic activity capture runs in v0.1" on every view,
contradicting the enabled state once a user turns capture on via this task's own panel.

Fix commit `1fb08b6` (3 code files + handoff, on top of the SW-2 `25782fc` feat commit):

- New pure builder `sidebarCaptureStatus(view: CaptureSettingsView | null)` in
  `active-window-settings-ui.ts`, mirroring `captureBanner`'s state machine: off / on / macOS-only /
  null-neutral ("Open Settings for capture status" — never a denial either way).
- `main.ts` `shell()` now renders `${sidebarCaptureStatus(captureView)}` instead of the literal
  string; `captureView` is the same module-level state `capture()`/`capturePanel` already use,
  populated by `loadCaptureView()` on both `renderToday()` and `renderSettings()` (default view on
  cold start is `Today`, so the box resolves to real state on first paint, not just after visiting
  Settings) — confirmed by reading `src/main.ts:33,58-99`.
- 6 new tests in `tests/activeWindowSettingsUi.test.mjs`: on/off/macOS-only/null states, a dedicated
  regression guard asserting the stale phrase is absent in all 4 states, and a source-scan test that
  reads `main.ts` directly and asserts (a) the stale literal string is gone and (b) the box is wired
  to `sidebarCaptureStatus(captureView)` — this makes the fix un-revertible by silent copy-paste.
- Confirmed via `npm run test:frontend`: all 6 sidebar tests pass, including the source-scan guard.
- No backend/native/`title_mode`/Accessibility/release-asset file touched by the fix commit.

## Checks re-run this session

| Check | Result |
| --- | --- |
| `cargo test --lib` (full backend) | **259 passed**, 0 failed (unchanged since prior gate — fix is frontend-only) |
| `cargo test --lib active_window` (focused) | **66 passed**, 0 failed |
| `cargo fmt --check` | clean |
| `npm run build` (tsc + vite) | clean |
| `npm run test:frontend` (full suite) | **146/148 passed**; 2 pre-existing failures in `tests/pi-observe.security.test.mjs` (file outside this diff, unrelated to TASK-056 — same 2 failures as prior gate) |
| `openspec validate task-056-active-window-settings-ui --strict` | passes |
| `git diff main...HEAD --name-only` | exactly 16 files, unchanged file list from prior gate |
| `gh pr view 43` | OPEN, draft, base `main` ← head `feat/task-056-active-window-settings-ui` |

Clippy/dependency/schema/CSP checks were exhaustively verified at the prior gate and are unaffected by
a frontend-copy-only fix commit; not re-run (no `Cargo.toml`/lock/`tauri.conf.json`/migration/schema
change in the fix commit — confirmed via `git diff main...HEAD --name-only` above).

## Scenario coverage matrix (`specs/active-window-settings/spec.md`)

Unchanged from prior gate except row 7 below, now fully closed:

| Requirement / Scenario | Coverage |
| --- | --- |
| Reading current capture settings (incl. macOS-only disabled) | `settings_api_default_view_is_capture_off_and_redacted`, `settings_api_valid_set_writes_five_keys_and_resolves` (backend); `panel: non-macOS disables every control...` (frontend) |
| Changing a setting takes effect without restart | `settings_api_valid_set_writes_five_keys_and_resolves` — full live-loop timing deferred to physical-Mac smoke (not yet run, see below) |
| Invalid interval / ordering / retention rejected, prior config unchanged | backend `settings_api_rejects_*` suite; mirrored inline-validation tests in `activeWindowSettingsUi.test.mjs` |
| Default remains off | `settings_api_default_view_is_capture_off_and_redacted` |
| Degraded capture is explained (named cause + since-time) | `status_snapshot_separates_open_and_recent_health` (backend); `status: degraded names the open state...` (frontend) |
| Enabled but no samples yet (absent, not error) | `status_snapshot_empty_db_is_zero_counts_and_none`, `settings_api_view_embeds_status_snapshot` (backend); frontend status test |
| Copy reflects real state when capture is enabled, on **every** view | `capturePanel`/`captureBanner` tests (per-view banner/panel) **+ `sidebarCaptureStatus` tests (always-on sidebar, this fix)**. **Now fully closed — no remaining view can assert "no capture runs" while enabled.** |
| No window-title opt-in offered; no Accessibility/title code | `settings_api_set_never_writes_title_mode` (backend); explicit `no title_mode input` assertion (frontend); repo-wide grep confirms no new `kAX`/`AXUIElement`/event-tap code |

## Not run (deferred — correctly flagged as human/UAT in tasks.md)

- **Physical-Mac smoke** (`tasks.md` §Smoke): enable via UI → status shows running / `last_sample_ts`
  advances; interval/threshold change takes next-tick effect; degraded-state induction; disable stops
  sampling; no TCC prompt; `window_title` stays `NULL`; sidebar box updates alongside banner/panel on
  a real toggle. Not automatable from this environment (no physical Mac attached to this session) —
  remains the mandated testable route, not fabricated or skipped silently. No available-here check
  contradicts any of these claims.

## Guardrail checks (all held, re-confirmed)

- `title_mode` never accepted by `set_active_window_capture_settings`, never written, no UI control for it.
- No Accessibility/`kAX`/`AXUIElement`/event-tap/Screen-Recording code added (`capture.rs` diff is empty).
- No schema/table/column/migration change.
- No new dependency, no CSP change, no `tauri.conf.json` change.
- Capture defaults OFF; only an explicit user toggle can enable it.
- Clean branch diff — only TASK-056's 16 files, matching the prior gate exactly.
