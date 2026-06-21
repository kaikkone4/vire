<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-046 active-window evidence storage (schema foundation)

- **Change dir**: openspec/changes/task-046-active-window-evidence-storage/
- **Branch / PR**: `feat/task-046-active-window-evidence-storage`; draft PR open.
- **Phase / gate**: SW-2 implementation complete. Ready for SW-3 QA.
- **Tier**: L2 · SEC-001 (field-allowlist at the storage layer).

## Last gate result
SW-2 implementation **complete**. `cargo test` 200/200 green. `cargo clippy` clean (new module
adds zero new non-dead-code warnings beyond pre-existing langfuse/lib.rs ones). All 13 module
tests + 2 new adversarial tests pass.

## What was built

- `src-tauri/src/active_window/` — new module, mirroring `runtime_observer/`:
  - `mod.rs` — module root, declares submodules + test.
  - `model.rs` — `TitleMode`, controlled-vocabulary consts (`title_state`, `idle_state`,
    `source`, `health_state`), `RawObservationIn` (allowlist DTO, serde-drops prohibited
    keys), `RawObservation`, `EvidenceBlock`, `CaptureHealthEvent`, `EvidenceBlockView`,
    `PruneStats`.
  - `store.rs` — `migrate(conn)`, `insert_raw_observation`, `upsert_evidence_block`,
    `record_capture_health`, `evidence_blocks_in_range`, `capture_health_in_range`,
    `prune_expired`, `apply_title_gate` (private).
  - `config.rs` — `ActiveWindowConfig::from_env()` reading `VIRE_ACTIVE_WINDOW_RETENTION_DAYS`
    (default 30) and `VIRE_ACTIVE_WINDOW_TITLE_MODE` (default `redacted`).
  - `tests.rs` — 13 unit tests covering all §6 requirements.
- `src-tauri/src/lib.rs` — added `mod active_window;`; wired `migrate` + `prune_expired`
  into `init_db` after `suggestions::store::migrate`.
- `src-tauri/tests/adversarial.rs` — 2 new external tests: no-prohibited-column and
  migration-additive-via-init_db.

## Out-of-scope items (verified not built)
No macOS capture, no IPC, no renderer change, no CSP change, no `tauri.conf.json` change.
`capture_status` stays `manual_mode_deferred`. Tables start empty.

## Active blockers
None.

## Required files for QA
- `src-tauri/src/active_window/` (all files)
- `src-tauri/src/lib.rs:113-155` (init_db wiring)
- `src-tauri/tests/adversarial.rs` (bottom section)
- `design.md` §2–§6 (QA reference for contract requirements)
- `specs/active-window-evidence/spec.md` (6 scenarios to verify)
