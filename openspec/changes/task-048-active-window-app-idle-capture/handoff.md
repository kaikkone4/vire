<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-048 zero-permission app + idle capture

- **Change dir**: openspec/changes/task-048-active-window-app-idle-capture/
- **Branch / PR**: `feat/task-048-active-window-app-idle-capture` (off origin/main) · draft PR opened.
- **Gate**: SW-2 Implementation **COMPLETE** → next SW-3 (sw-qa-engineer). L2 · zero TCC grants.

## Shipped (in-process Rust, design §3 — no sidecar/externalBin/IPC)

- `src-tauri/src/active_window/capture.rs` (new): poll loop — `NSWorkspace.frontmostApplication` +
  `CGEventSource::seconds_since_last_event_type` (idle *age* only; no event tap/AX/Quartz/content).
  Coalesces into TASK-046 evidence blocks; `no_gui_session` + `sampling_gap` health rows; retention
  via `prune_expired`. Writes ONLY via TASK-046 `store`. `window_title=NULL`,
  `title_state=absent_no_permission`, `source=nsworkspace`.
- `config.rs`: `CaptureConfig::from_settings` — `active_window_capture_enabled` (default **OFF**),
  `sample_seconds`=5, idle 60/300; settings>env>default. `mod.rs`: `pub mod capture;`.
- `lib.rs` `.setup()`: spawns loop on its own OS thread (mirrors langfuse). No
  renderer/IPC/network/CSP/invoke_handler change.
- `Cargo.toml`: ONLY 2 new direct deps (macOS-scoped) — `objc2-app-kit` 0.3.2,
  `objc2-core-graphics` 0.3.2 (objc2 0.6.4 + objc2-foundation 0.3.2 transitive). SW-5 scan input.

## Gate evidence

- **§0 LIVE no-TCC PASS** (this Mac): real readers → `source=nsworkspace`, `frontmost_present=true`,
  real `idle_state`, **no prompt/hang/AX-SR-IM grant**. Repro via `VIRE_TASK048_LIVE_PROBE=1`.
- `cargo test --lib` 230 ✓ · `--test adversarial` 5 ✓ (10 new capture tests) · clippy ✓ · fmt ✓ ·
  `openspec validate … --strict` ✓. Unchanged: CSP, no externalBin, src/main.ts, handler list.

## For SW-3 / notes

- Default OFF: set `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED=1` (or settings key) to exercise live.
- FB-048 → ba-architect (DEC-019/C1) and FB-002 (UX) carried, both non-blocking.
- NSWorkspace not main-thread-gated in objc2 → off-main read sound (design §3 note resolved).
