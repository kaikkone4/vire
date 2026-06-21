<!-- handoff.md — compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-046 active-window evidence storage

- **Branch / PR**: `feat/task-046-active-window-evidence-storage` / #34
- **Phase / gate**: SW-2 round-3 fix complete → route to SW-3 + SW-4 recheck
- **Head commit**: `59747b4`
- **QA**: SW-3 PASS at `6c1200d` (209+5); recheck needed at `59747b4`

## SW-2 round-3 fixes applied (`59747b4`)

All three §8 arch-review blockers resolved — no design/spec change:

1. **§8.1 `capture_health` vocab** (`store.rs:155-157, 197-199`): `check_vocab` guard on
   `Option<&str>` before SQL bind in both `insert_raw_observation` and `upsert_evidence_block`.
   None accepted; Some(s) must be in `HEALTH_STATE_VOCAB`. 4 new tests (reject + None-ok for
   each path).

2. **§8.2 `apply_title_gate` total truth table** (`store.rs:393-450`): rewrote to return
   `Result`; classifies title into Present/Empty/Absent; rejects all contradictory
   (state, title) pairs fail-closed (no silent normalization). `Some("")` now correctly
   routes to Empty → `(None, "empty")`, closing the `Some("")+empty→captured` defect.
   Both callers propagate with `?`. Updated 2 existing tests; 3 new matrix tests covering
   no-title+captured, empty+empty, title+absence across both modes and both write paths.
   Also applied nonblocking `Some("")` bundle normalization at `upsert_evidence_block`
   call site (`store.rs:203`).

3. **§8.3 config env precedence tests** (`tests.rs`): added `ENV_LOCK` (static `Mutex<()>`)
   + `EnvGuard` (RAII save/restore). 4 new serialized tests: env-only retention, env-only
   title_mode, DB-over-env retention, DB-over-env title_mode. Hardened 2 existing tests
   (default + invalid-fallback) to lock + clear env instead of assuming ambient absence.

## Checks at `59747b4`

- `cargo test` ✓ 220 library + 5 adversarial (was 209+5)
- `cargo clippy --lib` ✓ 48 warn (unchanged — 45 staged dead-code + 3 pre-existing)
- `cargo fmt --check` ✓
- Preserved: nullable-bundle sentinel, DB-first config, vocab/detail checks, exact-ts prune

## Required next action

SW-3 recheck at `59747b4` → SW-4 recheck. No SW-5 re-review (allowlist/redaction/prune/deps
unchanged per §8.4 scope guard). Files changed: `store.rs`, `tests.rs` (both under
`src-tauri/src/active_window/`).
