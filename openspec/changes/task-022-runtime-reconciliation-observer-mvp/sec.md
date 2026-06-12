# SW-5 Security Review — TASK-022 runtime reconciliation observer (MVP)

- **Change:** `task-022-runtime-reconciliation-observer-mvp`
- **PR:** #14 · branch `feat/task-022-runtime-reconciliation-observer-mvp` → `main`
- **Head reviewed:** `d18ea5f` (feature) atop `a8fc11e` (SW-3 QA gate, PASS L2)
- **Tier:** L2 (secrets + CVE ≥ 7 + Trivy + semgrep ERROR)
- **Verdict: PASS** — no L2 auto-fail condition hit.

## Scope reviewed

PR #14 product surface only (18 files, +2128/−10): the new Rust-core module
`src-tauri/src/runtime_observer/` (`mod/config/model/ingest/store/reconcile/tests`),
the additive `langfuse_ai_evidence.session_id` column + `AiEvidence.session_id`
field, one read-only Tauri IPC command in `lib.rs`, and the thin Settings line in
`src/main.ts`. The TASK-022 commit touches **no** `Cargo.toml`/`Cargo.lock`,
`tauri.conf.json`, or capability files (confirmed via `git show --name-only`).

## Tier 1 scanner results

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** 8.30.1 | `git main..HEAD` (54 commits, 1.15 MB) | **no leaks found** | No |
| **semgrep** 1.165.0 | `--config auto` over all 17 changed source files | **0 ERROR** findings | No |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (487 pkgs) | 17 advisories — max scored **CVSS 6.9** (`glib`, MEDIUM); rest unscored | No (none ≥ 7.0) |
| **Trivy** 0.71.1 | `fs --scanners vuln,secret --severity HIGH,CRITICAL` over `src-tauri`; secret scan over `runtime_observer/` | **0 HIGH/CRITICAL**, **0 secrets** | No |

Notes:

- **gitleaks** did not flag the adversarial test fixtures in `runtime_observer/tests.rs`
  / `*.snapshot` lines (`sk-ant-SECRETTOKEN`, `AKIASECRETKEY000`, `AWS_SECRET_ACCESS_KEY`).
  These are intentionally fake — `AKIASECRETKEY000` is not a valid AWS key shape (12 chars
  after `AKIA`, the rule needs 16) and `sk-ant-SECRETTOKEN` is not a real Anthropic key
  format. Trivy's secret scanner likewise reported clean. No real secret is committed or
  surfaced through any product path.
- **semgrep** scanned all 17 files including every `runtime_observer/*.rs`. Three
  `Internal matching error` *warnings* were emitted by the engine for three JavaScript
  rules (crypto-js weak-algorithm, express/koa CORS) against the minified `src/main.ts`;
  the file was still fully scanned and those rules are irrelevant to this code. Not a
  coverage gap.
- **No container image** ships in this Tauri desktop app, so the L2 "Trivy container HIGH/
  CRITICAL" check was satisfied by a filesystem scan of the Rust dependency tree (0 HIGH/
  CRITICAL), cross-checking OSV.

## Manual secure-code review (against the SW-5 focus list)

All focus items **PASS**:

1. **No prohibited content collected/persisted.** The allowlist is the *type*, not a
   runtime filter: `RawEvent` (`model.rs`) declares only coarse fields and serde silently
   drops every other key (`prompt`, `command_body`, `shell_history`, `env`, `metadata`,
   `api_key`, `response`, `git_*`, `cwd_*`) at deserialization — they can never reach a
   typed value, the store, or a log. Proven by `allowlist_drops_injected_prohibited_fields_*`
   (hostile line stuffed with secrets → dump contains none of `SECRET/AKIA/sk-ant/rm -rf/
   AWS_SECRET/git_branch/summary/cwd`).
2. **Runtime log ingest is local, read-only, metadata-only; symlink + byte cap effective.**
   `read_safely` (`ingest.rs`) uses `symlink_metadata` and rejects `is_symlink()`, rejects
   non-regular files, and refuses files over `MAX_LOG_BYTES` (8 MiB) — never a partial read.
   Absence/empty → `log_present=false` (no evidence), never zero. Covered by
   `symlinked_log_is_refused`, `oversize_log_is_refused`, `absent_log_is_no_evidence_not_zero`,
   `empty_log_is_no_evidence`, `malformed_lines_are_skipped_not_fatal`.
3. **No new external egress; CSP/capabilities unchanged.** No network/process primitives in
   the module (`grep` for `reqwest/TcpStream/std::process/Command/proc/connect/http` → none).
   `tauri.conf.json` untouched; CSP remains `connect-src ipc: http://ipc.localhost` (IPC/
   loopback only). Renderer has no `fetch`/XHR/WebSocket; it reaches the backend solely via
   the Tauri `call()` IPC bridge.
4. **`get_runtime_reconciliation` IPC leaks no raw content or secrets.** Returns
   `RuntimeReconciliationSnapshot` — coarse per-state `u32` counts, a `runtime_log_present`
   bool, a constant `source:"local"`, and a fixed advisory `message`. No session content,
   command body, prompt, or raw log line. Proven by
   `snapshot_serializes_to_counts_only_with_no_secret_or_content` (serialized JSON contains
   none of `SECRET/sk-ant/rm -rf/command_body/prompt`).
5. **`ai_runtime_sessions` cannot become a duplicate cost/time ledger.** The migration
   (`store.rs`) defines only `session_key, project_key, tool, session_id, environment,
   started_at, ended_at, status, reconciliation, matched_trace_id, observed_at` — **no**
   token/cost/duration/price column. `billable`/`duration_ms` are parsed for fidelity but
   explicitly not persisted. Enforced by the structural test
   `runtime_sessions_table_has_no_cost_token_or_duration_column` (PRAGMA bans
   cost/token/duration/price/prompt/command/secret columns).
6. **Langfuse `session_id` addition persists no credentials/raw prompt content.** Additive
   nullable column populated from the already-parsed `Trace.session_id` (opaque hashed
   `session-<hash>`), surfaced onto the **normalized** evidence row so the observer never
   parses the prompt-bearing `langfuse_raw_traces.payload`. Privacy-positive; no credential
   or prompt text added to any table.
7. **No committed real secrets; fixtures fake and not surfaced.** Confirmed by gitleaks +
   Trivy; fixtures live only in `tests.rs` and never flow to a product code path.
8. **No auth bypass / new endpoint / rate-limit issue.** One read-only Tauri command, local
   on-demand, no HTTP listener, no auth surface, no DoS-relevant unbounded loop (input is
   byte-capped and line-bounded).

## Advisory findings (non-blocking — documented per L2 rubric)

- **A1 — `add_column_if_absent` builds `ALTER TABLE` via string interpolation**
  (`langfuse/store.rs`). Not exploitable: `table`/`column`/`decl` are compile-time string
  literals from the single call site (`"langfuse_ai_evidence","session_id","TEXT"`); no
  external input reaches it. Recommendation: keep the function private and literal-only; do
  not let caller-supplied identifiers flow in without an allowlist.
- **A2 — `runtimeLine()` interpolates snapshot counts without `esc()`** (`src/main.ts`).
  Safe today: only the numeric `u32` fields (`observed_runs`, `observed_no_trace`,
  `reconciliation_unknown`) are interpolated and the `runtime_log_present` branch is static
  text — no snapshot *string* field (`source`, `message`) is rendered. Recommendation: if a
  string field is ever surfaced in this line, route it through `esc()` like `sourcePanel()`.
- **A3 — Pre-existing dependency advisories on the Tauri GTK3 stack.** OSV reports
  unmaintained/unsound RUSTSEC advisories on `gtk/gdk/atk/glib 0.18`, `proc-macro-error`,
  and `unic-*`, max CVSS 6.9 (`glib` RUSTSEC-2024-0429). **Not introduced by TASK-022** —
  `Cargo.lock` entered the tree at `f8fd591` (TASK-020) and this PR adds no crates. None
  reach the L2 ≥ 7.0 / HIGH/CRITICAL bar. Recommendation: track a Tauri / gtk-rs bump in a
  dedicated dependency-hardening task.

## Evidence

- Security-relevant behavior is locked by the module's own tests
  (`runtime_observer/tests.rs`: allowlist-drop, path safety, IPC no-content-leak,
  no-cost-column, absence-≠-zero). The SW-3 QA gate (`a8fc11e`) already reported these
  passing at L2; not re-run here.

## Gate decision

**PASS.** No auto-fail condition (committed secret / CVE ≥ 7.0 / Trivy HIGH-CRITICAL /
semgrep ERROR) is present. Proceed — hold for SW-4 (Code Review) before SW-6 (Release).
No design-level escalation to BA-flow Architect required.
