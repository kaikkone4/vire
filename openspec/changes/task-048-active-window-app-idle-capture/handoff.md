<!-- Compact state; keep <= 2 KB. -->

# Handoff — TASK-048 zero-permission app + idle capture

- **Branch / PR**: `feat/task-048-active-window-app-idle-capture` / #36
- **Gate**: SW-2 fix complete → **awaiting SW-4 recheck** (+ cheap SW-5 recheck, see sec.md note).
- **Review**: `review.md`; **QA**: `qa.md` (SW-3 PASS).

## Implemented scope

- In-process Rust loop: `NSWorkspace.frontmostApplication` + `CGEventSource` event age.
- Default OFF through settings/env; dedicated setup thread.
- TASK-046 store writes/coalescing, `no_gui_session`/`sampling_gap`, daily retention.
- `window_title=NULL`, `title_state=absent_no_permission`, `source=nsworkspace`.
- No renderer/IPC/network/CSP/handler/sidecar/`externalBin` change.
- Two macOS-scoped direct dependencies: `objc2-app-kit`, `objc2-core-graphics`.

## SW-4 blockers — all FIXED in capture.rs (no new deps)

1. Idle reader queries `kCGAnyInputEventType` = `CGEventType(0xFFFF_FFFF)` via the existing newtype — one any-input call replaces the 5-type subset. Age only, no content.
2. Disabled path now resets `open` + `last_sample_ts` (`reset_disabled_state`, used by `maybe_sample` + `run_tick`); re-enable emits no false `sampling_gap`.
3. `sample_once` wraps the whole tick (health + raw + evidence) in one `unchecked_transaction`; `open`/`last_sample_ts` published only after `commit`. Failure rolls back fully; next tick retries.
- Low-risk suggestion done: `config_default_is_disabled` holds local `ENV_LOCK` + `EnvGuard` save/restore.
- Regressions (13/13): `idle_reader_queries_any_input_event_type`, `disable_then_reenable_records_no_false_sampling_gap`, `failed_block_write_rolls_back_raw_and_preserves_state`.

## SW-5 Security — PASS (L2), see `sec.md`

- Recheck SW-5 on this revision (idle reader + write path) — cheap re-run; no new dep, no new scanner-relevant surface.

## Evidence (post-fix)

- Capture tests 13/13 ✓; full lib 233/233 ✓; `cargo fmt --check` ✓; OpenSpec strict ✓; `git diff --check` ✓.
- Clippy still 8 pre-existing warnings, none in TASK-048 files (no new warnings introduced).
- Working-tree change is `capture.rs` only — no renderer/IPC/CSP/network/externalBin/AX/title/UI; no new dependency.
- Live no-TCC probe (`VIRE_TASK048_LIVE_PROBE=1`) must be re-run on target Mac to confirm any-input reader.
