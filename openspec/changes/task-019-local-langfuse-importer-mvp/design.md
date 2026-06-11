# Design — TASK-019 Local Docker Langfuse importer (TASK-007 MVP slice)

- **Author:** SW Architect (SW-1) scaffold for developer roles
- **Change:** `task-019-local-langfuse-importer-mvp`
- **Decisions honored:** DEC-017 (Langfuse-first, no duplicate ledger, no new pi/Claude adapter),
  DEC-019 (reuse Tauri + native helper + Rust-core importer), **DEC-020 / DEC-022** (local Docker
  self-hosted Langfuse default; Cloud explicit override), SEC-002 (network boundary), SEC-003
  (credentials).
- **Source of truth:** BA `03_architecture_plan.md` §4.3/§4.4 + `04_technical_plan.md` §7;
  TASK-018 `arch-review.md` §5 (authoritative TASK-007 direction); feeder spike
  `task-007-langfuse-importer-validation/langfuse-validation-report.md` (empirical REST/field/health
  findings — re-targeted from cloud-first to local-Docker default).

## 1. Component boundary

TASK-019 lives **entirely inside the BA-defined "Langfuse importer" component** (`03_architecture_plan.md`
§4, `04_technical_plan.md` §4). It does not modify the macOS capture adapter, AI runtime observer,
classification engine, review UI, or CSV exporter. The only shared substrate it touches is the
SQLite store, where it adds importer-owned tables (§5). No component boundary is crossed → single
cohesive change, not a split (see `arch-review.md` §3).

```
 webview (renderer)            Rust core (server-side)                 external
 ────────────────              ───────────────────────                 ────────
 status banner  ──IPC──▶  get_langfuse_source_health (read-only)
 (TASK-009 later)         ┌───────────────────────────────┐
                          │ Langfuse importer module      │  reqwest   ┌──────────────────────┐
                          │  - config (base URL/env/creds) │ ─Basic──▶  │ langfuse-web :3000    │
                          │  - availability probe          │  (read)    │ 127.0.0.1 (loopback)  │
                          │  - REST pull + pagination      │ ◀───────   │ /api/public/*         │
                          │  - dedup + cursor              │            └──────────────────────┘
                          │  - schema/usage/cost normalize │  (worker/PG/CH/Redis/MinIO are
                          │  - 10-state health             │   Langfuse-internal; surfaced to
                          └───────────────┬───────────────┘   Vire only as stack-health)
                                          ▼
                                  SQLite (importer-owned tables)
```

**Why the importer is in the Rust core, not the renderer:** the webview CSP is
`connect-src ipc: http://ipc.localhost` (`src-tauri/tauri.conf.json`) — the renderer cannot make
outbound HTTP at all. The importer must call the Langfuse REST API from Rust. This also keeps API
credentials server-side (SEC-003) and keeps the renderer off every network path (SEC-002).

## 2. HTTP client choice — `reqwest` in the Rust core (not a webview HTTP plugin)

- Add `reqwest` (rustls, JSON) + an async runtime (`tokio`) to `src-tauri/Cargo.toml`, used **only**
  from the Rust core. **No new Tauri capability** is added: capabilities gate the *webview's* access
  to Tauri commands/plugins; the Rust core's own outbound calls are not capability-gated, and adding
  `tauri-plugin-http` would needlessly expose an HTTP surface to the renderer.
- **URL allowlist is enforced in Rust:** the importer constructs every request URL from the single
  configured base URL and a fixed set of `/api/public/*` paths. It refuses any other host/scheme.
  Default base URL `http://127.0.0.1:3000`; loopback only unless an explicit Cloud/LAN override is
  set. This is the DEC-020 "Tauri HTTP URL allowlist" realized as an importer-internal invariant.
- The existing `tauri.conf.json` CSP and `capabilities/default.json` are unchanged.

## 3. Config model (SEC-003)

| Field | Default | Notes |
| --- | --- | --- |
| `base_url` | `http://127.0.0.1:3000` | loopback; the only direct connection target |
| `allowed_environments` (per project) | `["vire"]` | environment-first mapping; primary Vire→project mechanism |
| `source` | `local` | `local` (default) \| `cloud` (explicit non-default override — only off-host egress path) |
| `public_key` / `secret_key` | — | from Keychain or chmod-600 gitignored `.env`; header use only |

Credentials: used solely for the `Authorization: Basic base64(pub:sec)` header. **Never** printed,
logged, persisted to evidence rows, exported to CSV, or included in diagnostics/support output.
Local stack secrets (Postgres/ClickHouse/Redis/MinIO/`NEXTAUTH_SECRET`/`SALT`/`ENCRYPTION_KEY`)
likewise never enter repo/logs/evidence (they are Langfuse-internal; Vire never connects to them).

## 4. REST contract (empirically validated by the feeder — re-targeted to local)

- **List:** `GET /api/public/traces?environment=&fromTimestamp=&toTimestamp=&page=&limit=` →
  `{ data: Trace[], meta: { page, limit, totalItems, totalPages } }`. Paginate until
  `page >= totalPages`. Store a per-environment cursor/checkpoint (latest trace timestamp) in
  `langfuse_import_runs`; re-import with an overlap window to catch `delayed` (late-arriving) traces.
- **Dedup:** by `(environment, trace_id)` across pages, re-imports, and overlap windows; count once,
  record `duplicate` when seen.
- **Usage/cost live on generation observations, not the trace body.** Read per-call tokens/cost from
  `GET /api/public/observations` (or `GET /api/public/traces/{id}` with `observations[]`) and
  aggregate to the trace; the trace's `totalCost` is an aggregate convenience, not the authoritative
  per-call breakdown. Observed `usage` keys are `{ input, output, total, unit }`; token counts also
  appear as top-level observation fields. Read by **observed shape**, not assumed key names.
- **Tolerate sparse traces:** `sessionId` is nullable, `name` may be empty, `metadata` key-count
  varies (0–14). Mapping must not assume a fixed metadata shape.
- **Environment reality:** pi-observe emits **no `environment`**, so its traces land in `default`,
  not `vire`; pi-langfuse v1.4.3 has the same propagation risk and the Claude Code hook can
  silently emit nothing. The importer must treat `default`/unexpected-env traces as first-class
  **`wrong_env`** (surfaced for review), never a silent pass, and detect **`missing`**/**`stale`**
  traces rather than trusting an empty result.
- **Schema/time/usage/cost validation:** validate field presence/type against the recorded shape
  before relying on totals; nullable cost/model or empty `usageDetails`/`costDetails` ⇒
  `schema_changed`, **not** zero cost.

## 5. Importer-owned persistence (TASK-019's slice of TASK-004)

A narrow additive migration (extends the existing `init_db` pattern in `lib.rs`):

| Table | Purpose | Secrets? |
| --- | --- | --- |
| `langfuse_import_runs` | per-environment cursor/checkpoint, status, latest trace timestamp, warning counts, health state | **no** |
| `langfuse_raw_traces` | raw imported trace evidence (local-only; may include prompt/session/metadata per §7) | **no** (credentials never stored) |
| `langfuse_ai_evidence` | normalized AI time/usage/cost evidence (start/end, tokens, cost, env, session, health) | **no** |

Scoped narrowly so it does not pre-empt the full TASK-004 schema (no `projects`/`time_entries`
changes; importer tables only). Flag to BA whether TASK-004 should land first or TASK-019 owns this
slice (`feedback_to_ba[]`, see `arch-review.md` §7).

## 6. Health taxonomy — implement the BA §7 names (authoritative)

The feeder spike proved a 9-state model with internal names; TASK-019 must implement the **BA
`04_technical_plan.md` §7** 10-state taxonomy, which is authoritative and adds the two DEC-020
states the feeder lacked:

| BA §7 state (implement this) | Feeder equivalent | Trigger |
| --- | --- | --- |
| `healthy` | `valid` | recent import; usage/cost present where a generation exists |
| `missing` | `missing` | expected runtime/activity but no matching trace |
| `stale` | `stale` | latest trace/import older than threshold |
| `wrong_env` | `wrong_env` | traces in `default`/unexpected env vs mapping |
| `delayed` | `delayed` | trace precedes a checkpoint but arrives after it |
| `duplicate` | `duplicate` | repeated trace id across pages/re-imports |
| `schema_changed` | `schema_mismatch` | expected usage/cost/timestamp field absent/wrong type |
| `auth_or_network_error` | `auth_or_config_error` (+ `rate_limited`) | 401/403/429/network failure — no secret exposed; **fold 429/backoff here**, do not surface a separate state |
| `unavailable` | *(new — DEC-020)* | Docker/Langfuse web/worker/PG/CH/Redis/MinIO down/unreachable |
| `unknown` | *(new — DEC-020)* | cannot establish whether local stack is current/complete |

**Absence ≠ zero invariant (hard):** every "no data" path (`missing`, null usage/cost, `stale`,
`unavailable`, `unknown`, auth/network failure) resolves to a health flag, never a numeric zero
contributed to an AI total. On `unavailable`, Vire may offer to open/start Docker where safe or
refuse to show AI totals, but always surfaces the state.

## 7. Privacy posture under DEC-020 (relaxation vs the cloud-first feeder)

- The feeder (DEC-018 cloud-first) said the importer must not persist raw trace `input`/`output`.
  **Under DEC-020 the local boundary relaxes this for MVP:** local self-hosted trace payloads **may**
  include prompt/session/metadata, accepted within the local Langfuse boundary to make the flow work.
  Do **not** over-redact and stall the MVP. Stricter redaction/retention (prompt-text exclusion,
  metadata scrubbing, retention windows) is an explicit **L2 follow-up**.
- **Hard invariants that do not relax:** raw macOS activity/window titles stay in Vire's local
  SQLite and are **never** mixed into Langfuse traces or sent to Langfuse (SEC-002); secrets never
  enter logs/evidence/exports (SEC-003); off-host egress occurs **only** on explicit Cloud override.
- MinIO/S3 stays private/non-public; bucket (`langfuse`), prefixes, Docker volumes, and three-store
  (PG/ClickHouse/MinIO) backup-consistency/divergence risks remain documented in `docs/` and must
  stay accurate as the importer lands.

## 8. Out-of-scope boundaries (do not build here)

- **AI runtime observer (TASK-006):** consumes this health taxonomy for reconciliation; not built
  here. DEC-017: no duplicate cost/time ledger, no new pi/Claude adapter.
- **Classification (TASK-008), review/approval UI (TASK-009), CSV export of AI evidence (TASK-010),
  macOS capture adapter (TASK-005):** separate components/tasks. TASK-019 stops at importing,
  normalizing, persisting, and exposing a read-only health status.

## 9. Test direction (handoff to SW-3)

Mirror `04_technical_plan.md` §12 for the importer: pagination/dedup and schema validation against
**mocked local self-host responses**; all 10 health-state transitions incl. `unavailable`/`unknown`;
absence-≠-zero asserted literally (empty env ⇒ health flag, never `0`); credential-redaction checks
(no secret in logs/evidence/exports); loopback-default and Cloud-explicit-override checks; no
raw-activity egress. Prefer the feeder probe's `--mock` fixture style (no network/credentials).
