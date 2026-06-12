# Tasks — TASK-022 pi/Claude runtime reconciliation observer (TASK-006 MVP slice)

Single component (the **AI Runtime Observer** in the Rust core). Sub-tasks are an implementation
sequence within one OpenSpec change, **not** a component split. Recommended order:

## 1. Observer module + runtime-log ingest (read-only)

- [x] New Rust-core module `src-tauri/src/runtime_observer/` (`mod.rs`, `ingest.rs`, `model.rs`,
      `store.rs`, `reconcile.rs`, `tests.rs`), mirroring the `langfuse/` module shape. **No** network
      client, **no** new Tauri capability, **no** `tauri.conf.json` CSP change.
- [x] Runtime-log source = configurable path `VIRE_RUNTIME_LOG_PATH`, default
      `$PI_OBSERVE_STATE_DIR/events.jsonl` → `~/.local/state/pi-observe/events.jsonl`. Absent/empty
      file ⇒ *no runtime evidence* (never an AI-usage conclusion).
- [x] Parse `events.jsonl` line-by-line; skip malformed JSON lines (never fatal). Path safety: reject
      symlinks, cap bytes read, tolerate a missing file (mirror pi-observe's marker-read guards).
- [x] **Strict ingest allowlist:** keep only `event`, `project`/`project_key`, `tool`, `run_id`,
      `session_id`, `ts`, `status`/`exit_code`, `billable`, `duration_ms`. **Drop** `metadata.summary`,
      `command_label`, `git_branch`, `git_remote_hash`, `cwd_basename`, and every non-allowlisted field
      — no prompts, command bodies, shell history, env lines, or secrets ever reach the store.
- [x] Session pairing: `tool_started`/`manual_active` open; `tool_finished`/`manual_inactive` close;
      `tool_orphaned` ⇒ close as orphaned/`open`. Coarse status only.

## 2. Observer-owned persistence (TASK-022 slice of TASK-004)

- [x] Additive migration (extend `init_db`): `ai_runtime_sessions` (`session_key` PK, `project_key`,
      `tool`, `session_id`, `environment`, `started_at`, `ended_at`, `status`, `reconciliation`,
      `matched_trace_id`, `observed_at`). **No** token/cost/prompt/command/secret column. No
      `projects`/`time_entries`/`langfuse_*` write-path change.
- [x] Idempotent upsert on `session_key`; re-ingest re-resolves reconciliation without duplicating
      rows.

## 3. Importer interface extension (recommended; privacy-positive)

- [x] Add nullable `session_id` column to `langfuse_ai_evidence`; populate from the already-parsed
      `Trace.session_id` in `upsert_ai_evidence` (additive; dedup/upsert unchanged). Lets the observer
      match on the **normalized** evidence row and never read prompt-bearing `langfuse_raw_traces`.
- [x] Fallback if BA declines `feedback_to_ba[]` (3): observer extracts **only**
      `sessionId`/`timestamp`/`environment` from raw payloads and discards all other keys.
      *(Not exercised — the recommended privacy-positive path above was implemented, so the observer
      matches the normalized `langfuse_ai_evidence` row and never reads `langfuse_raw_traces`.)*

## 4. Reconciliation engine (runtime ↔ imported traces)

- [x] Match precedence per session: (1) `session_id` exact, then (2) `environment` + time-window
      overlap (bounded clock slop). Reuse importer `environment` semantics; do not re-derive trace
      health.
- [x] Resolve the reconciliation taxonomy: `matched` / `observed_no_trace` / `unmatched_runtime` /
      `reconciliation_unknown`, plus `unmatched_trace` for imported evidence with no session.
- [x] **Import-health gating (load-bearing):** read the importer's recorded health for the session's
      window+environment (`langfuse_import_runs`); if `unavailable`/`unknown`/`auth_or_network_error`,
      resolve `reconciliation_unknown`. `observed_no_trace` is asserted **only** under a `healthy`
      import. Absence ≠ zero, asserted literally.
- [x] Produce the combined view `(reconciliation_state, referenced trace_health?)` for the surface.

## 5. Read-only reconciliation surface (minimal)

- [x] Read-only IPC command `get_runtime_reconciliation` → coarse per-state counts, runtime-log
      presence flag, local-only source posture. **Never** secrets, session content, or command text.
      Register in `src-tauri/src/lib.rs` `invoke_handler!`.
- [x] One thin line in the existing Settings AI-evidence panel (`src/main.ts`): observed runs /
      without-trace / reconciliation state, with the "down or absent ⇒ unknown, never zero" framing.
      Full review UI = TASK-009.

## 6. Guardrail invariants (assert, don't assume)

- [x] No network call in the observer path; renderer CSP unchanged.
- [x] `ai_runtime_sessions` has no cost/time/prompt/command/secret column — runtime is reconciliation/
      health only, never a competing Langfuse cost/time ledger (DEC-003/DEC-017).
- [x] Docker/Langfuse down or runtime-log absent ⇒ `reconciliation_unknown`, never zero AI usage/cost.

## 7. Verification

- [x] `cargo test` — unit: allowlist drop, session pairing/orphan, matcher (session_id-first then
      env+time), reconciliation precedence **incl. import-health gating**, absence ≠ zero, idempotent
      re-ingest, malformed-line + symlink/oversize rejection.
- [x] Adversarial tests (mirror `tests/adversarial.rs`): injected prompt/command-body/secret/env-line
      fields in a hostile `events.jsonl` are dropped and never persisted; no high-entropy/secret-shaped
      string is stored.
- [x] Integration against a seeded importer DB: healthy→`matched`, missing→`observed_no_trace`,
      unavailable→`reconciliation_unknown`, trace-only→`unmatched_trace`.
- [x] IPC test: `get_runtime_reconciliation` exposes counts/states only — no secrets, no session
      content, no command text.
- [x] `openspec validate task-022-runtime-reconciliation-observer-mvp --strict`.

## Out of scope (do not build here)

- Process scanning / native macOS capture (TASK-005); any pi/Claude emitter or adapter (DEC-017); a
  runtime cost/time ledger (DEC-003/DEC-017); classification (TASK-008); full review/approval UI
  (TASK-009); CSV export of reconciliation (TASK-010); runtime-evidence retention automation.
