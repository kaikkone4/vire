<!-- handoff.md — compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-046 active-window evidence storage

- **Branch / PR**: `feat/task-046-active-window-evidence-storage` / #34
- **Phase / gate**: SW-2 (SW-4 blockers fixed) → re-run SW-3 QA then SW-4
- **Tier**: L2 · SEC-001
- **Commit**: `2fa9f9d` fix(task-046): SW-4 blockers — upsert idempotency, config precedence, vocab enforcement, exact-ts prune

## Fixes applied (SW-4 blocking issues)

1. **Nullable `app_bundle_id` upsert**: Normalise `None → ""` sentinel before binding the
   unique-conflict column (`BUNDLE_NULL_SENTINEL = ""`); map `"" → None` on read in
   `evidence_blocks_in_range`. Regression tests: idempotent upsert with `None`, round-trip read.

2. **`ActiveWindowConfig.from_settings`**: Replaces `from_env()`. Reads DB `settings` table
   first (keys: `active_window_retention_days`, `active_window_title_mode`), falls back to env,
   then compile-time defaults. `lib.rs:154` wired up. Tests: defaults, stored override for
   retention and title_mode, invalid-value fallback.

3. **Vocabulary enforcement at write boundary**: `TITLE_STATE_VOCAB`, `IDLE_STATE_VOCAB`,
   `SOURCE_VOCAB`, `HEALTH_STATE_VOCAB` const arrays + `check_vocab`/`check_detail` helpers.
   Called at the top of `insert_raw_observation`, `upsert_evidence_block`, `record_capture_health`
   before any SQL. `detail` bounded to `MAX_DETAIL_BYTES = 200`. `apply_title_gate` Stored branch
   now forces `title_state=captured` when title is present (prevents title + absence-state
   inconsistency). Tests: invalid title_state, idle_state, source, health state, oversized detail,
   title/state consistency.

4. **Prune by exact per-table timestamps**: `DELETE … WHERE datetime(sample_ts) < datetime(?1,?2)`
   / `end_ts` / `start_ts`. Wrapped in `unchecked_transaction()` (atomic). Tests: intra-day
   cutoff (same-day before cutoff time → deleted), exact-cutoff row kept, one-second-before
   deleted.

## Checks

- `cargo test`: PASS — 209 unit + 5 adversarial (was 195 + 5; 14 new regression tests added).
- `cargo clippy`: 48 warnings; 40 pre-existing, 8 new active-window future-seam dead code.
- Security: no change to allowlist, redaction gate, prune scope, or deps → SW-5 PASS carries forward.
- Scope: no UI, IPC, capture, network, deps, CSP, or Tauri config change.

## Required next action

Re-run SW-3 (QA) then SW-4 (code review) against commit `2fa9f9d`.
