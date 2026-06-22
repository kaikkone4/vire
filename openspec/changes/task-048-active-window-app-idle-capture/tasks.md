# Tasks — TASK-048 zero-permission app + idle capture

Single component (the **macOS Evidence Capture** loop, in the existing `src-tauri/src/active_window/`
module). Sub-tasks are an implementation sequence within one OpenSpec change, **not** a component
split. In-process Rust (no Swift sidecar / `externalBin` / IPC — `design.md` §3); writes only through
the **already-built TASK-046 store API**. **No** AX/titles, **no** UI, **no** network, **no** CSP
change, **no** renderer surface. Recommended order:

## 0. Empirical zero-permission confirmation (FIRST — de-risks the one assumption)

- [x] On the target macOS, build with capture enabled and confirm **no TCC prompt** appears and the
      app never calls `AXIsProcessTrusted` / creates a `CGEventTap`. NSWorkspace + idle must populate
      rows with `source='nsworkspace'` and a real `idle_state`. If a prompt *does* appear, STOP and
      report (the zero-permission premise would be wrong — escalate, do not request a grant).

## 1. Dependencies + module scaffold

- [x] Add `objc2-app-kit` (NSWorkspace) and `objc2-core-graphics` (CGEventSource) to
      `src-tauri/Cargo.toml` — the only new deps; record exact versions for the SW-5 dep scan. **No**
      other crate, **no** `externalBin`, **no** `tauri.conf.json` change.
- [x] New `src-tauri/src/active_window/capture.rs`; declare `pub mod capture;` in
      `active_window/mod.rs`.

## 2. Signal readers (zero-permission only — design §2)

- [x] `read_frontmost_app()` → `Option<(app_name, app_bundle_id)>` via
      `NSWorkspace.shared.frontmostApplication` (`localizedName` / `bundleIdentifier`). `None` ⇒
      `no_gui_session`. Resolve the `objc2-app-kit` main-thread affinity note (design §3) here.
- [x] `idle_seconds()` via `CGEventSource::seconds_since_last_event_type(.combinedSessionState, …)`,
      min across `keyDown/mouseMoved/leftMouseDown/rightMouseDown/scrollWheel` (mirrors the TASK-002
      probe). **Never** create a `CGEventTap`; **never** read event content.
- [x] `idle_state(secs)` → `active` / `idle_candidate` (≥60 s) / `away` (≥300 s); thresholds from
      config. **No** AX call anywhere; `window_title = None`, `title_state = absent_no_permission`.

## 3. Sample → store, via the TASK-046 API only (design §4)

- [x] Each tick build a `RawObservation` (`source = source::NSWORKSPACE`, `window_title = None`,
      `title_state = title_state::ABSENT_NO_PERMISSION`) and persist with `insert_raw_observation`
      under the resolved `TitleMode` (redacted default — gate already nulls titles).
- [x] Maintain the open normalized block and `upsert_evidence_block` on each tick; close + reopen when
      `(app_bundle_id, idle_state)` changes (set `end_ts`, `duration_seconds`). **No** new table, **no**
      generic writer, **no** `time_entries`/`project_id` association (read-time only, DEC-001).

## 4. Capture-health (degraded states, never silent — design §2, C4)

- [x] `record_capture_health(no_gui_session)` when no frontmost app; no fabricated activity row.
- [x] `record_capture_health(sampling_gap)` when wall-clock gap between ticks > 2× interval (covers
      sleep/suspend without notification observers); `detail` = a coarse bound only, never a title.

## 5. Lifecycle, config, retention (design §5)

- [x] `active_window::config`: add `capture_enabled` (settings key `active_window_capture_enabled` /
      env `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED`, **default OFF**), `sample_seconds` (default 5), idle
      thresholds — same settings>env>default precedence as `ActiveWindowConfig::from_settings`.
- [x] Spawn the loop from `lib.rs` `.setup()` on a dedicated OS thread (mirror the langfuse
      auto-import scheduler at `lib.rs:1189`): if `capture_enabled` is false, the thread idles / does
      not sample. **No** renderer call, **no** new `invoke_handler` command, **no** network.
- [x] Drive `prune_expired(now, retention_days)` on a coarse cadence (e.g. day-boundary crossing);
      touches only `active_window_*` tables.

## 6. Tests (mirror `active_window/tests.rs` discipline — synthetic data only)

- [x] `idle_state` threshold mapping (0 / 90 / 360 s → active / idle_candidate / away).
- [x] Sample-to-store: a synthetic observation persists with `source='nsworkspace'`,
      `window_title IS NULL`, `title_state='absent_no_permission'`; default redacted mode keeps title
      NULL.
- [x] Coalescing: contiguous same `(bundle_id, idle_state)` → one block; a change closes + opens.
- [x] Health: `no_gui_session` (nil frontmost) and `sampling_gap` (oversized gap) each write a row;
      no fabricated activity; `detail` carries no title.
- [x] Disabled-by-default: with `capture_enabled=false`, a setup cycle writes **no** rows and app
      behavior is unchanged.
- [x] No-AX / no-title structural: the capture path contains no `AXIsProcessTrusted` / `kAXTitle` /
      `CGEventTap` / Quartz call (grep-assert or absence-by-construction); logs emit counts/states
      only, never an app-name/title value.
- [x] Retention driver removes only `active_window_*` rows; a sentinel `time_entries` row survives.

## 7. Gate / validation

- [x] `cargo test` (lib + adversarial) ✓ · `cargo clippy --lib` ✓ · `cargo fmt --check` ✓.
- [x] `openspec validate task-048-active-window-app-idle-capture --strict` ✓.
- [x] Confirm unchanged: `tauri.conf.json` CSP + absent `externalBin`; `src/main.ts`; the
      `invoke_handler!` list; no network client added.
