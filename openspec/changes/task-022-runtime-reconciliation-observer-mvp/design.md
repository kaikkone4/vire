# Design — TASK-022 pi/Claude runtime reconciliation observer (TASK-006 MVP slice)

## 1. Component placement and posture

The observer is the BA **AI Runtime Observer** (`03_architecture_plan.md` §3/§4.2). It is a
read-only Rust-core component that consumes two **local** inputs and produces one **local** output:

```
  pi-observe events.jsonl ─┐
  (coarse session log)     ├─► runtime_observer ─► ai_runtime_sessions (+ reconciliation state)
  langfuse_ai_evidence  ───┘        (Rust core)        └─► get_runtime_reconciliation (read-only IPC)
  (imported traces, TASK-019)
```

Hard posture (all enforced as Rust invariants, mirroring the importer's posture):

- **No network.** The observer makes zero HTTP calls. The only network path in the product remains
  the importer's loopback REST pull (DEC-020). The webview CSP (`connect-src ipc:`) is untouched; the
  renderer never reaches the network.
- **No process scanning, no capture.** Runtime evidence comes from a pre-existing local **session
  log**, not from enumerating processes or reading command-lines. This sidesteps the single largest
  prohibited-field hazard (command bodies live in process argv) and needs no new macOS permission.
- **Read + reconcile only; never a cost/time authority.** `ai_runtime_sessions` has **no token/cost
  column**. Cost/time stay sourced from `langfuse_ai_evidence` (DEC-003/DEC-017).

## 2. Runtime evidence source — decision

**Decision: ingest the `pi-observe` coarse event log; do not build a process scanner or an emitter.**

| Option | What it is | Verdict |
| --- | --- | --- |
| **A. Ingest `pi-observe` events.jsonl** *(chosen)* | Read the metadata-only, already-redacted session lifecycle log the wrapper writes to `~/.local/state/pi-observe/events.jsonl` | **Chosen.** Lowest privacy risk (no argv, pre-redacted, hashed session ids), minimal Rust core, reuses the TASK-001-identified signal source, aligns with DEC-017 reconciliation-only. |
| B. Rust process scanner (`sysinfo`/`ps`) | Enumerate live pi/Claude processes; derive sessions from start/exit | Rejected for MVP: re-collects what pi-observe already emits, exposes command-line argv (prohibited command bodies), new capture surface and permission burden, more code. |
| C. New pi/Claude emitter/adapter | Instrument agents to emit runtime evidence | **Forbidden by DEC-017** in the MVP. |

Decoupling rule (so Option A does not silently become a hard pi-observe dependency): the source is a
**configurable path** (`VIRE_RUNTIME_LOG_PATH`, default the pi-observe state path). If the file is
**absent or empty**, the observer has *no runtime evidence* and every otherwise-importable trace
reconciles to `unmatched_trace` while no session can be `observed_no_trace`. Absence of the log is a
**state**, never a conclusion about AI usage. A future native capture sidecar (TASK-005) can emit the
same coarse session shape and the observer ingests it unchanged.

### 2.1 Why reconciliation is genuinely bi-directional (not trivially "always matched")

`pi-observe` both writes `events.jsonl` **and** POSTs a Langfuse trace with the **same** hashed
`sessionId`. That is exactly what makes the gap detectable:

- `pi-observe`'s `sendLangfuse()` can **fail/timeout/be remote-blocked**, the Claude-Code hook can
  **silently emit nothing**, pi-langfuse v1.4.3 can **mis-propagate the environment** → a session
  exists locally but **no usable trace was imported** ⇒ `observed_no_trace` (the high-value signal).
- A trace can arrive from an **unwrapped** run (no events.jsonl entry) ⇒ `unmatched_trace` (coverage/
  project-mapping uncertainty).
- Both present and aligned ⇒ `matched` (confidence the imported cost/time maps to a real local run).

## 3. Matching key and the importer interface extension

Match precedence, per runtime session:

1. **`session_id` exact** — when the session log and the imported evidence both carry the hashed
   `session-<hash>` id. `pi-observe` sets `body.sessionId` to the same value it logs, so this is exact
   when pi-observe is the emitter.
2. **environment + time-window overlap** — fallback when a side lacks a session id: a session in
   project→environment `E` whose `[start,end]` overlaps a trace's `[ai_start_ts, ai_end_ts]` (or
   `timestamp`) in `E`. Coarse, tolerant of clock skew via a small bounded slop.

`langfuse_ai_evidence` today stores `trace_id, environment, ai_start_ts, ai_end_ts, tokens, cost,
health` — **not** `session_id`. Two ways to get the id:

- **Recommended (privacy-positive): surface `session_id` on the evidence row.** `model.rs` already
  parses `Trace.session_id`; add a nullable `session_id` column to `langfuse_ai_evidence` and
  populate it in `upsert_ai_evidence`. The observer then matches against the **normalized** row and
  **never reads `langfuse_raw_traces.payload`** (which may contain prompts/metadata). This is an
  additive, low-risk migration inside the importer component and a read-only contract the observer
  consumes — not a boundary crossing.
- **Fallback (zero importer change):** the observer reads `langfuse_raw_traces.payload` and extracts
  **only** `sessionId` / `timestamp` / `environment`, discarding all other keys. Acceptable but puts
  the observer in contact with prompt-bearing payloads, so it is the second choice.

The backend dev should default to the recommended path unless BA declines item (3) in
`feedback_to_ba[]`.

## 4. Reconciliation-state taxonomy (the new contract)

TASK-022 **owns** the runtime-side reconciliation states and **references** the importer's trace-side
health states (it does not re-derive them). Resolution is by fixed precedence:

| State | Owner | Meaning / when |
| --- | --- | --- |
| `reconciliation_unknown` | TASK-022 | Highest precedence. The import covering the session's window+environment was `unavailable` / `unknown` / `auth_or_network_error`, **or** no runtime log exists. Cannot assert a gap — absence ≠ zero. |
| `matched` | TASK-022 | Session aligns with ≥1 imported trace by session_id or env+time overlap. |
| `observed_no_trace` | TASK-022 | Session observed **and** the import for its window+environment was `healthy`/complete, but no matching trace. The DEC-017/FR-012 gap signal (= BA "observed-agent-without-trace"). |
| `unmatched_runtime` | TASK-022 | Session that cannot be evaluated against any environment (e.g. no project→environment mapping). Coarse "needs manual review". |
| `unmatched_trace` | TASK-022 | An imported trace with no runtime session (per imported evidence row, not per session). |
| `stale` / `missing` / `wrong_env` / `delayed` / `duplicate` / `unavailable` / `unknown` | **Importer (TASK-019/020)** | Trace-side health, **referenced** in the combined view; never recomputed here. |

The combined reconciliation view a session/trace carries is `(reconciliation_state, trace_health?)`
so the UI can say e.g. "observed agent run · trace wrong_env" without TASK-022 owning `wrong_env`.

### 4.1 Import-health gating (the load-bearing correctness rule)

`observed_no_trace` MUST be gated on the importer's recorded health for the session's window+env:

```
if import_health(window, env) ∈ {unavailable, unknown, auth_or_network_error}: reconciliation_unknown
elif matched_trace_exists:                                                       matched
elif import_health(window, env) == healthy and !matched_trace_exists:            observed_no_trace
else:                                                                            reconciliation_unknown
```

This is the `ai_runtime_sessions` realization of NFR-008 / "absence ≠ zero": a session with no trace
under a down or uncertain import is **never** a real gap and **never** zero cost.

## 5. Data model (additive, observer-owned)

```sql
CREATE TABLE IF NOT EXISTS ai_runtime_sessions (
    session_key      TEXT PRIMARY KEY,   -- run_id (stable) or hashed session_id; dedup unit
    project_key      TEXT NOT NULL,      -- safe token from the log (already redacted/tokenized)
    tool             TEXT,               -- coarse tool label only (e.g. "claude", "pi")
    session_id       TEXT,               -- hashed "session-<hash>" if present (opaque, not content)
    environment      TEXT,               -- resolved project→environment, if mappable
    started_at       TEXT,               -- ISO-8601
    ended_at         TEXT,               -- ISO-8601, NULL if still open/orphaned
    status           TEXT,               -- success | failed | canceled | open  (coarse)
    reconciliation   TEXT NOT NULL,      -- the TASK-022 state taxonomy
    matched_trace_id TEXT,               -- the trace it reconciled to, if matched
    observed_at      TEXT NOT NULL       -- when the observer recorded this row
);
-- NO prompt/response, NO command body, NO env dump, NO cost, NO token, NO secret column — by design.
```

Idempotent on `session_key`: re-ingesting the same log re-resolves reconciliation without duplicating
rows (the same absence-preserving discipline as the importer's `(environment, trace_id)` dedup).

## 6. Ingest allowlist (security boundary)

`events.jsonl` lines are parsed and filtered to a **fixed allowlist** before anything is persisted:

- **Kept:** `event` (type), `project` / `project_key`, `tool`, `run_id`, `session_id`, `ts`,
  `status`, `exit_code`→coarse status, `billable`, `duration_ms`.
- **Dropped (never persisted, never logged):** `metadata.summary`, `metadata.command_label`,
  `metadata.git_branch` / `git_remote_hash`, `metadata.cwd_basename`, and **any** field not on the
  allowlist. No prompts, command bodies, shell history, env lines, or secret-shaped strings can reach
  the store even if a malformed/hostile log injects them.

Lifecycle pairing mirrors pi-observe's own model: `tool_started`/`manual_active` open a session,
`tool_finished`/`manual_inactive` close it, `tool_orphaned` closes as `open`→orphaned. Malformed JSON
lines are skipped, not fatal. Path safety mirrors pi-observe's marker reads: reject symlinks, cap
file bytes, tolerate absence.

## 7. IPC + UI surface (minimal)

- **`get_runtime_reconciliation` (read-only).** Returns coarse aggregate counts per reconciliation
  state plus the runtime-log presence flag and the source path's posture (local only) — **no** secrets,
  **no** session content, **no** command text. Shape mirrors `SourceHealthSnapshot`'s "facts only"
  discipline.
- **Settings line.** One line appended to the existing AI-evidence panel in `src/main.ts`:
  "Observed agent runs: N · without a matching trace: M · reconciliation: <state-or-unknown>", with
  the explicit "a down/absent stack means unknown, never zero" framing already used by the source
  banner. Full review/approval UI is TASK-009.

## 8. Boundary touchpoints (contracts, not splits)

1. **Langfuse importer (TASK-019/020/021).** Read-only consumer of `langfuse_import_runs`
   (health/window) and `langfuse_ai_evidence` (trace timestamps/env, + recommended `session_id`).
   TASK-022 changes no importer logic; the one recommended importer edit is an additive evidence
   column it consumes.
2. **SQLite store / TASK-004 schema.** TASK-022 owns the narrow additive `ai_runtime_sessions` slice
   (its own table + migration through `init_db`), no `projects`/`time_entries` change — the same
   pattern TASK-019 used. Flagged to BA.
3. **Review UI (TASK-009) / Summary model (TASK-010).** TASK-022 produces the reconciliation state and
   a thin read-only surface; it does not build the review UI or fold runtime into approved summaries.

## 9. Test strategy (L2)

See `tasks.md` §7. Anchored on the importer's existing `langfuse/tests.rs` + `tests/adversarial.rs`
style: unit tests for allowlist filtering, session pairing/orphan, the matcher (session_id-first then
env+time), reconciliation precedence **including import-health gating**, absence ≠ zero, idempotent
re-ingest, malformed-line and symlink/oversize rejection; adversarial fixtures proving injected
prompt/command-body/secret/env fields are dropped and never persisted; integration against a seeded
importer DB (healthy→matched, missing→observed_no_trace, unavailable→reconciliation_unknown,
trace-only→unmatched_trace); an IPC test asserting the surface carries no secrets/content.
