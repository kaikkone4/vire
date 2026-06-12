# TASK-022 — pi/Claude runtime reconciliation observer (TASK-006 MVP slice)

## Why

TASK-006 (WP-006) is the **runtime-reconciliation support** for the Langfuse-first AI-evidence path.
DEC-017 makes Langfuse the canonical AI time/usage/cost source and limits local pi/Claude Code
runtime observation to **reconciliation and trace-health gap detection only** — never a duplicate
cost/time ledger and never a new pi/Claude adapter in the MVP. TASK-019/020/021 landed the local
Docker Langfuse importer with the 10-state health taxonomy and a read-only source-health snapshot,
but that pipeline can only describe the **trace side** of a gap (stale / missing / wrong_env /
delayed / duplicate / unavailable). It cannot answer the question DEC-017/FR-012/NFR-008 actually
care about:

> *An agent ran locally — did a usable Langfuse trace arrive for it, or is the cost/time silently
> missing?*

Answering that requires a second, **runtime-side** evidence stream to reconcile against the imported
traces. This change adds that stream as a minimal, read-only **AI Runtime Observer** component
(`03_architecture_plan.md` §3/§4.2, `04_technical_plan.md` §6/§8 `ai_runtime_sessions`).

This is an implementation change, not an architecture decision. It implements the BA-flow
architecture and DEC-003/DEC-017; it does **not** reopen DEC-017 (Langfuse-first), DEC-019
(implementation path), or DEC-020 (local-Docker default), and it does **not** build a pi/Claude
emitter/adapter.

## What Changes

- **Coarse runtime-session ingest (read-only, local file).** A new Rust-core observer module reads a
  **local coarse session log** of pi/Claude Code runs and extracts session boundaries only:
  `project_key`, `tool`, `run_id`, hashed `session_id`, start/end timestamps, terminal status. The
  default source is the already-redacted, metadata-only `pi-observe` event log
  (`$PI_OBSERVE_STATE_DIR/events.jsonl`, default `~/.local/state/pi-observe/events.jsonl`) — the
  signal source TASK-001's salvage inventory flagged as the strong TASK-006 input. The observer
  **does not scan processes**, **does not** read command-lines, and **does not** adopt `pi-observe`
  as a runtime dependency: if the log is absent the observer reports *no runtime evidence* (→
  `reconciliation_unknown`), never zero AI usage/cost.
- **Strict ingest allowlist (no prohibited fields).** Ingest keeps only a fixed allowlist of coarse
  metadata fields and **drops everything else** — no prompts/responses, no terminal command bodies,
  no shell history, no env dumps, no secrets, no free-text summaries, no git/cwd identifiers beyond
  the safe project token. Defensive even though the source is pre-redacted.
- **Observer-owned persistence (`ai_runtime_sessions`).** A narrow additive migration adds the
  `ai_runtime_sessions` table named in `03_architecture_plan.md` §5 / `04_technical_plan.md` §8:
  coarse session boundary + reconciliation state only. **No token/cost columns** — by construction it
  cannot become a competing cost/time authority (DEC-003/DEC-017). This is TASK-022's narrow slice of
  TASK-004, the same pattern TASK-019 used for the importer tables; it touches no
  `projects`/`time_entries`/`langfuse_*` write path.
- **Reconciliation engine (runtime ↔ imported traces).** For each runtime session, match against the
  importer's persisted evidence — **`session_id` first** (exact, when both sides carry it), then
  **environment + time-window overlap** — and resolve a reconciliation state:
  `matched / observed_no_trace / unmatched_runtime / reconciliation_unknown`, plus `unmatched_trace`
  for imported traces with no runtime session. The observer **references** the importer's existing
  trace-side health states (stale/wrong_env/delayed/duplicate) rather than re-deriving them.
- **Import-health gating (absence ≠ zero, hard rule).** `observed_no_trace` is asserted **only** when
  the importer's run for that session's window+environment was actually `healthy`/complete. When the
  source was `unavailable` / `unknown` / `auth_or_network_error`, an observed session reconciles to
  `reconciliation_unknown` — a missing trace under a down/uncertain import is never read as a real
  gap and never as zero cost.
- **Read-only reconciliation surface (minimal).** A read-only IPC command
  (`get_runtime_reconciliation`) returns coarse reconciliation **counts and states** (no secrets, no
  session content, no command bodies) so the existing Settings panel can show "N observed agent
  runs, M without a matching trace" alongside the source-health banner. Full review/approval UI stays
  in TASK-009.

## Impact

- **Affected specs:** `runtime-reconciliation` (new capability, ADDED). Codifies: read-only coarse
  runtime-session ingest with a strict no-prohibited-field allowlist; the reconciliation-state
  taxonomy and its precedence; `session_id`-first then time+environment matching; import-health
  gating so `observed_no_trace` is never asserted under a down/uncertain import; runtime evidence is
  reconciliation/health only and never a cost/time authority; no network egress and renderer stays
  off-network; absence ≠ zero.
- **Affected code (product runtime):** new Rust-core module `src-tauri/src/runtime_observer/`
  (`mod.rs`, `ingest.rs`, `model.rs`, `store.rs`, `reconcile.rs`, `tests.rs`), an additive
  `ai_runtime_sessions` migration wired through the single `init_db` entry point, one new read-only
  Tauri command registered in `src-tauri/src/lib.rs`, and a thin reconciliation line in the existing
  Settings panel (`src/main.ts`). **No** new webview capability and **no** `tauri.conf.json` CSP
  change — the observer reads a local file + local SQLite and makes **no** network calls.
- **Minor importer interface extension (privacy-positive, recommended):** surface the
  already-parsed `Trace.session_id` into a new nullable `session_id` column on
  `langfuse_ai_evidence` so the observer matches against the **normalized** evidence row and never
  has to parse prompt-bearing `langfuse_raw_traces.payload`. Additive nullable column + populate from
  an existing field; same importer component, a read-only contract the observer consumes (see
  `design.md` §3). Fallback if BA/dev declines: observer extracts only `sessionId`/`timestamp`/
  `environment` defensively from raw payloads.
- **Out of scope (clean boundaries, not this task):** process scanning / native macOS capture
  (TASK-005); any pi/Claude emitter or adapter (DEC-017); a runtime cost/time ledger
  (DEC-003/DEC-017); classification (TASK-008); the full review/approval UI (TASK-009); CSV export of
  reconciliation (TASK-010); runtime-evidence retention automation. None of their component
  boundaries are crossed.
- **Guardrails preserved:** no prompts/responses, no terminal command bodies, no shell history, no
  env dumps, no secrets, no raw local-activity egress; renderer off-network (no CSP change);
  Docker/Langfuse down or absent ⇒ `reconciliation_unknown`, never zero AI usage/cost; runtime
  evidence stays coarse and never duplicates valid Langfuse time/cost.
- **`feedback_to_ba[]` (non-blocking):** (1) `pi-observe`'s tooling role is still formally undecided
  (carried from TASK-001/003) — TASK-022 treats its `events.jsonl` as a read-only **optional** signal
  source, not a committed dependency; request a DEC ratifying that the runtime observer MAY ingest a
  local coarse session log without fixing pi-observe's permanent role. (2) `ai_runtime_sessions` is
  named in BA §5/§8 under TASK-004; confirm TASK-022 owning the narrow additive slice is accepted
  (or assign a decision id). (3) Confirm the `session_id` surfacing into `langfuse_ai_evidence` is
  acceptable as a privacy-positive importer interface extension.
- **Branch:** `feat/task-022-runtime-reconciliation-observer-mvp`, base `main`
  (origin/main `bc064e5`, post-PR #13; no merge-order dependency).
