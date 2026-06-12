# TASK-019 — Local Docker Langfuse importer (TASK-007 MVP slice)

## Why

TASK-007 (WP-007) is the **lead AI-evidence work package**: import pi/Claude Code AI
time/usage/cost from Langfuse as Vire's primary AI-evidence path. TASK-018 (DEC-020 / DEC-022)
fixed-forward the source posture this importer inherits — **local Docker self-hosted Langfuse is
the canonical default**, Langfuse Cloud is an explicit non-default override only. The feeder spike
`task-007-langfuse-importer-validation` (branch `feat/task-007-langfuse-importer-validation`,
authored under the older DEC-018 cloud-first framing) empirically validated the REST schema,
field shapes, pagination/cursor/dedup, and a health-state model; its findings are reusable, but its
default-source posture must be read through DEC-020.

This change turns that validated direction into the **first shippable importer slice in the product
runtime**. Until now the Rust core (`src-tauri/src/lib.rs`) is only the manual time-tracker
(projects + `time_entries` + CSV export); there is no HTTP client and no AI-evidence path. TASK-019
adds the read-only Langfuse REST importer in the Rust core, behind the existing locked webview CSP.

This is an implementation change, not an architecture decision. It implements the BA-flow
architecture (`03_architecture_plan.md` §4.3/§4.4, `04_technical_plan.md` §7) and the TASK-018
authoritative direction. It does **not** reopen DEC-019 (implementation path) or DEC-020 (source
posture).

## What Changes

- **Read-only Langfuse REST client in the Rust core.** A new importer module calls the Langfuse
  public REST API (`GET /api/public/traces`, `GET /api/public/observations` /
  `GET /api/public/traces/{id}`) with HTTP Basic auth, default base URL `http://127.0.0.1:3000`
  (loopback). Reads only; never writes to Langfuse. Lives entirely server-side in Rust — the
  webview CSP (`connect-src ipc:`) is untouched and the renderer never reaches the network.
- **Importer config model.** Base URL (default loopback), per-project allowed environments
  (primary mapping, starting with `vire`), Cloud-override flag (explicit, non-default), and API
  credentials sourced from local secure config (Keychain or chmod-600 gitignored `.env`).
  Credentials are used only for the `Authorization` header — never rendered, logged, persisted to
  evidence rows, exported, or placed in diagnostics.
- **Stack-availability check + 10-state health taxonomy.** Each run checks local Docker/Langfuse
  availability first, then resolves one of the BA `04_technical_plan.md` §7 states:
  `healthy / missing / stale / wrong_env / delayed / duplicate / schema_changed /
  auth_or_network_error / unavailable / unknown`. **Absence is never zero AI usage/cost.**
- **Import cursor, pagination, dedup, and schema/time/usage/cost validation.** Paginate to window
  completion per environment; store a per-environment cursor/checkpoint; dedup by
  `(environment, trace_id)`; read usage/cost from **generation observations** (aggregated to the
  trace), tolerate sparse traces (nullable `sessionId`, empty `name`, variable `metadata`), and
  degrade visibly to `schema_changed` on field/shape drift rather than producing a wrong total.
- **Importer-owned persistence.** A narrow migration adds `langfuse_import_runs` (cursor,
  environment, status, latest trace timestamp, warnings — no credentials), raw trace evidence, and
  normalized AI time/usage/cost evidence. This is TASK-019's slice of the broader TASK-004 schema,
  scoped so it does not pre-empt the full schema work.
- **Docker-down surfacing (minimal).** A read-only IPC status command returns active base URL,
  configured environment(s), last import, latest trace timestamp, and current health state (never
  secrets) so the existing frontend can surface `unavailable`/`stale`/`unknown` without a full
  review UI. Review-UI polish stays in TASK-009.

## Impact

- **Affected specs:** `langfuse-importer` (new capability, ADDED). Codifies the MVP importer
  controls: local-Docker default + loopback, Cloud explicit override, environment-first mapping,
  read-only REST client in the Rust core, absence-≠-zero across the 10-state health taxonomy,
  cursor/pagination/dedup, observation-sourced usage/cost, no-raw-activity egress, and secrets out
  of logs/evidence/exports.
- **Affected code (product runtime — first AI-evidence runtime change):**
  `src-tauri/Cargo.toml` (add an HTTP client, e.g. `reqwest`, + async runtime; used in the Rust
  core only), `src-tauri/src/lib.rs` / new importer module(s), the SQLite migration for the
  importer-owned tables, and a thin frontend status surface for Docker-down. **No** new
  webview-facing Tauri capability is required (the importer calls `reqwest` from the Rust core, not
  a webview HTTP plugin); `tauri.conf.json` CSP is unchanged.
- **Out of scope (clean boundaries, not this task):** the AI runtime observer (TASK-006) consumes
  the health taxonomy but is not built here (DEC-017 — no duplicate cost/time ledger, no new
  pi/Claude adapter); the classification engine (TASK-008); the full review/approval UI (TASK-009);
  CSV export of AI evidence (TASK-010); the macOS capture adapter (TASK-005). None of their
  component boundaries are crossed.
- **Guardrails preserved:** local Docker default + loopback `127.0.0.1`, no LAN binding, Cloud
  explicit override only (the sole off-host egress path), environments as primary mapping, MinIO/S3
  private + three-store backup-consistency risks documented (`docs/`), Docker/Langfuse down ⇒
  `unavailable`/`stale`/`unknown` never zero cost, local trace payloads may include
  prompt/session/metadata for MVP (stricter redaction/retention is an L2 follow-up), and raw macOS
  activity stays in local SQLite and never mixes into Langfuse traces.
- **Carried `feedback_to_ba[]`:** (1) DEC-019 (implementation path) is still not recorded in
  `07_decision_log.md` — ratify it and note its importer posture reads against DEC-020.
  (2) Confirm whether the importer-owned persistence (TASK-019's slice of TASK-004) should carry its
  own decision id, or whether TASK-004 is expected to land first.
- **Branch:** `feat/task-019-local-langfuse-importer-mvp`, base `main` (TASK-018 PR #10 and TASK-003
  PR #9 already on/landing to `main`; no merge-order dependency on this branch).
