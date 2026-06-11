# Architecture Review (SW-1) — TASK-019 Local Docker Langfuse importer (TASK-007 MVP slice)

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-019-local-langfuse-importer-mvp`
- **Branch (proposed):** `feat/task-019-local-langfuse-importer-mvp` · **base:** `main`
- **Tier:** L2 · **Gate context:** SW-1 task-design review before developer roles implement the
  TASK-007 MVP importer slice.
- **Date:** 2026-06-11
- **Verdict:** **PASS** — single component (the BA "Langfuse importer"), one cohesive OpenSpec
  change. No component boundary is crossed; **not split-required**. No BA escalation (DEC-020/DEC-022
  already establish the posture). Two non-blocking `feedback_to_ba[]` items carried (§7).

---

## 1. Inputs read

- TASK-018 authoritative direction: `task-018-local-langfuse-source-addendum/{RELEASE.md, arch-review.md §5,
  proposal.md, specs/langfuse-trace-source/spec.md}` (DEC-020 supersedes DEC-018 cloud-first).
- BA architecture: `artifacts/ba/03_architecture_plan.md` §4.3/§4.4, `artifacts/ba/04_technical_plan.md`
  §4, §7 (health taxonomy, REST flow, operational model), §8 (schema direction), §12 (test strategy),
  §13 (WP order).
- Feeder spike `feat/task-007-langfuse-importer-validation`:
  `task-007-langfuse-importer-validation/langfuse-validation-report.md` — empirical REST schema,
  field shapes, pagination/cursor/dedup, 9-state health model (read through DEC-020).
- Current product runtime: `src-tauri/src/lib.rs` (manual tracker only), `src-tauri/Cargo.toml`
  (no HTTP client), `src-tauri/tauri.conf.json` (CSP), `src-tauri/capabilities/default.json`.

## 2. Architecture-consistency findings

The proposed slice is consistent with the BA architecture plan and TASK-018 §5. Three findings shape
the design and must be honored by developers:

1. **The importer must be in the Rust core — the codebase forces it.** The webview CSP is
   `connect-src ipc: http://ipc.localhost` (`tauri.conf.json`); the renderer cannot make outbound
   HTTP. The REST client therefore lives in the Rust core, which also keeps credentials server-side
   (SEC-003) and the renderer off every network path (SEC-002). This matches DEC-019.
2. **This is greenfield runtime — `src-tauri/Cargo.toml` has no HTTP client.** TASK-019 is the first
   AI-evidence runtime change. Recommendation: add `reqwest` + `tokio` used **only** in the Rust
   core, and realize the DEC-020 "Tauri HTTP URL allowlist" as an **importer-internal Rust
   invariant** (every URL built from the one configured base URL + fixed `/api/public/*` paths).
   Do **not** add `tauri-plugin-http` or a new webview capability — that would expose an HTTP surface
   to the renderer for no benefit. `tauri.conf.json` CSP and `capabilities/default.json` stay
   unchanged. (See `design.md` §2.)
3. **Health taxonomy must be the BA §7 names, not the feeder's.** The feeder proved 9 states under
   internal names; BA `04_technical_plan.md` §7 is the authoritative 10-state taxonomy and adds the
   two DEC-020-critical states the feeder lacked — **`unavailable`** (Docker/stack down) and
   **`unknown`** (currency/completeness indeterminate). Map `valid→healthy`,
   `schema_mismatch→schema_changed`, `auth_or_config_error`+`rate_limited`→`auth_or_network_error`.
   The absence-≠-zero invariant is hard across all ten. (See `design.md` §6.)

## 3. Split analysis — one task, not split-required

Per role working rules, "split-required" means the scope would cross component boundaries defined in
`03_architecture_plan.md`. It does not. All six listed pieces are sub-parts of the single **Langfuse
importer** component (`03_architecture_plan.md` §4, `04_technical_plan.md` §4):

| Listed piece | Component | Boundary crossing? |
| --- | --- | --- |
| Read-only Rust REST client (base URL default loopback) | Langfuse importer | No |
| Config model (base URL / env filter / credentials) | Langfuse importer | No |
| 10-state health taxonomy | Langfuse importer (publishes the contract) | No |
| Cursor / pagination / dedup + schema/time/usage/cost validation | Langfuse importer | No |
| No raw-activity egress | Cross-cutting constraint (SEC-002), not a component | No |
| Docker-down surfacing | Read-only IPC status surface (thin) | No (full UI = TASK-009) |

The macOS capture adapter, AI runtime observer, classification engine, review UI, and CSV exporter
are untouched. **Verdict: one cohesive change** with an internal implementation sequence
(`tasks.md`), not a split.

## 4. Boundary touchpoints (contracts/dependencies, not splits)

1. **SQLite store / TASK-004 schema.** The importer needs persistence (`langfuse_import_runs`,
   `langfuse_raw_traces`, `langfuse_ai_evidence`). BA §13 sequences TASK-004 (schema) before MVP
   implementation, but TASK-004 has not landed. Decision: TASK-019 owns a **narrow importer-only
   slice** of the schema (its own tables + additive migration extending `init_db`; no
   `projects`/`time_entries` change) so the importer is shippable without blocking on a full schema
   task. Flagged to BA (§7).
2. **AI runtime observer (TASK-006).** The health taxonomy is the *interface* the observer reconciles
   against (feeder report §4). TASK-019 **defines and produces** the states; TASK-006 **consumes**
   them. TASK-019 must not build the observer or a duplicate cost/time ledger (DEC-017).
3. **Renderer/UI (TASK-009).** Docker-down must be surfaced "somehow." For MVP a read-only IPC
   command (`get_langfuse_source_health`, secrets excluded) plus a thin banner is sufficient; full
   review/approval UI stays in TASK-009. This is the importer's status surface, not a UI redesign.

## 5. Constraints explicitly preserved (TASK-018 §5 / DEC-020)

| # | Constraint | Where enforced in this change |
| --- | --- | --- |
| 1 | Local Docker self-hosted Langfuse is the default source | `proposal.md`, `design.md` §1–2, spec R1 |
| 2 | Cloud is explicit non-default override only (sole off-host egress) | `design.md` §2–3, spec R1 |
| 3 | Langfuse environments are the primary Vire→project mapping | `design.md` §3–4, spec R3 |
| 4 | Loopback `127.0.0.1:3000` default; no LAN binding | `design.md` §2, spec R1 |
| 5 | MinIO/S3 internal/private + three-store backup-consistency docs stay visible | `design.md` §7, `tasks.md` §7 (keep `docs/` accurate) |
| 6 | Docker/Langfuse down ⇒ `unavailable`/`stale`/`unknown`, never zero cost | `design.md` §6, spec R2 |
| 7 | Local trace payloads may include prompt/session/metadata for MVP; stricter redaction/retention later | `design.md` §7, spec R5 |
| 8 | No raw macOS activity egress; raw activity stays in local SQLite | `design.md` §7, spec R5 |
| 9 | Credentials/local-stack secrets never in logs/evidence/exports | `design.md` §3, spec R5 |

**Privacy note (relaxation):** the feeder (DEC-018 cloud-first) forbade persisting raw trace
`input`/`output`. Under DEC-020 the **local** boundary relaxes this for MVP (prompt/session/metadata
allowed locally) so the import flow can be made to work first; the no-raw-activity-egress and
no-secrets invariants do **not** relax. Developers must not over-redact and stall the MVP.

## 6. Empirical facts the developer must honor (feeder, re-targeted to local)

- Usage/cost live on **generation observations**, aggregated to the trace; the trace's `totalCost` is
  an aggregate convenience. Read by **observed shape** (`usage` = `{input,output,total,unit}`,
  top-level token counts), not assumed key names.
- Tolerate sparse traces: nullable `sessionId`, empty `name`, `metadata` 0–14 keys.
- Emitters set **no `environment`** → traces land in `default` (pi-observe always; pi-langfuse v1.4.3
  propagation risk; Claude Code hook silent-fail). Treat `default`/unexpected-env as `wrong_env`.
- REST: `GET /api/public/traces?environment=&fromTimestamp=&toTimestamp=&page=&limit=` →
  `{data, meta:{page,limit,totalItems,totalPages}}`; paginate to completion; per-environment cursor;
  dedup by `(environment, trace_id)`; overlap window for `delayed`.
- **Absence ≠ zero** asserted literally (empty env ⇒ health flag, never `0`).

## 7. Open items / `feedback_to_ba[]`

- **(Carried from TASK-018) DEC-019 not ratified.** `07_decision_log.md` still omits DEC-019
  (implementation path). Record it and note its importer posture reads against DEC-020, not DEC-018.
- **TASK-004 schema sequencing.** Confirm whether the importer-owned persistence slice
  (`langfuse_import_runs` + raw/normalized AI-evidence tables) should carry its own decision id /
  whether full TASK-004 is expected to land before this MVP, or whether TASK-019 owning the narrow
  slice is accepted. No blocker either way; flagged for BA ratification.

Neither item blocks developer start; both are routed to ba-architect via Pi-Assistant.

## 8. Recommendation — next roles and branch

- **Change name:** `task-019-local-langfuse-importer-mvp` (this dir). `openspec validate --strict`
  passes (verified 2026-06-11).
- **Branch:** `feat/task-019-local-langfuse-importer-mvp`, base `main` (no merge-order dependency;
  TASK-003 PR #9 and TASK-018 PR #10 already on/landing to `main`).
- **Next role (primary):** **backend-developer (Rust/Tauri)** owns the read-only REST client, config
  model, importer-owned migration, pagination/dedup/cursor, observation-sourced usage/cost
  normalization, and the 10-state health taxonomy in the Rust core.
- **Supporting role:** **integration-engineer** for Langfuse host-runtime fit (local Docker stack
  availability probe, environment/propagation realities, pi-langfuse/Claude-hook failure modes) and
  the read-only `get_langfuse_source_health` IPC surface + thin Docker-down banner.
- **Then:** SW-3 (QA) per `design.md` §9 / `04_technical_plan.md` §12; SW-4 (code review); SW-5
  (security: SEC-002/SEC-003); SW-6 (release).

## 9. Verdict

**PASS.** TASK-019 is one cohesive change inside the single Langfuse-importer component boundary —
not split-required. It faithfully implements TASK-018 §5 and the BA architecture (local-Docker
default, loopback, Cloud explicit override, environment-first mapping, 10-state health with
absence-≠-zero, no raw-activity egress, secrets protected, MVP-local prompt/session/metadata with
redaction deferred). Deliverables for developer handoff (`proposal.md`, `design.md`, `tasks.md`,
`specs/langfuse-importer/spec.md`, this review) are in place. Two non-blocking `feedback_to_ba[]`
items carried; route TASK-019 to backend-developer (Rust/Tauri) + integration-engineer on
`feat/task-019-local-langfuse-importer-mvp`.
