# SW-2 Fix Note — TASK-032 (SW-4 loop 1)

Fixes the two SW-4 blockers from `review.md`. Scope limited to those two issues plus the
non-blocking `qa.md` whitespace suggestion. No UI change, no semantic broadening.

## B1 — regeneration is now an atomic replace-set

`src-tauri/src/suggestions/engine.rs` — `generate()` previously ran `delete_pending` then a loop of
inserts on the bare connection, so a mid-regeneration insert failure left the prior pending set
deleted and only a partial replacement persisted (Refresh destructive under failure).

Fix: wrap `delete_pending` + every guarded `insert_if_not_decided` + the final `list_pending` read in
one transaction via `conn.unchecked_transaction()`, then `tx.commit()`. On any insert error the
function returns `Err` via `?`; the `Transaction` is dropped without commit and rolls back
(default = Rollback), so the **original** pending set survives intact.

`unchecked_transaction` (not `&mut Connection` + `transaction()`) keeps the existing `&Connection`
signature — the engine is only ever called on the single-writer app connection (no outer tx to nest),
mirroring the accept path's one-transaction guarantee without rippling `&mut` through all callers/tests.

Failure-path test added: `failed_regeneration_preserves_the_original_pending_set`
(`src-tauri/src/suggestions/tests.rs`). Generates the original set (2 pending blocks), installs a
`BEFORE INSERT ... RAISE(ABORT)` trigger to force the next regeneration's insert to fail, asserts
`generate` returns `Err` and the original pending set (same count, same ids) survives unchanged.

## B2 — stale lint suppression + hidden dead code removed

`src-tauri/src/suggestions/mod.rs` — removed the stale "awaiting B/C wiring" comment and the
crate-module-wide `#![allow(dead_code, unused_imports)]` (B/C are implemented).

`src-tauri/src/suggestions/store.rs` — removing the suppression exposed `EvidenceRow.trace_id`
(selected + stored, never read). Removed the field, its `query_map` projection, and `e.trace_id` from
the SELECT column list. Kept `trace_id` in `ORDER BY` so equal-`environment` rows retain deterministic
clustering order. Column indices renumbered (0–9). No other dead-code/unused-import warning surfaced.

## qa.md whitespace (non-blocking suggestion)

Stripped trailing whitespace (markdown hard-break spaces) from `qa.md` lines 3-5, 90-92, 106 —
whitespace-only, 7 lines, no content change. `git diff --check` is now clean.

## Checks (focused)

- `cargo test --manifest-path src-tauri/Cargo.toml suggestions:: --lib` — PASS 13/13 (+1 new).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — PASS 159/159, no regressions.
- `cargo fmt ... --all -- --check` — PASS.
- `cargo clippy ... --all-targets` — PASS; **0 warnings in `suggestions/`** (pre-existing warnings in
  `langfuse/` + `lib.rs` are outside TASK-032 and unchanged).
- `npm run build` — PASS.

## Out of scope (left as-is)

- review.md "Suggestions" item: midnight-spanning interval test — not a blocker, not requested this loop.
- Untracked `qa-032.md` / `review.md` are other roles' artifacts; not modified/committed here.
