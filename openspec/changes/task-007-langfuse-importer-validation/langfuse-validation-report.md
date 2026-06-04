# Langfuse importer validation report — TASK-007 (Phase A spike)

- **Change:** `task-007-langfuse-importer-validation`
- **Branch:** `feat/task-007-langfuse-importer-validation`
- **Author:** Integration Engineer (SW-2)
- **Date:** 2026-06-04
- **Tier:** L2 · **Gate context:** APP-005 (SEC-002 network boundary, SEC-003 credentials)
- **Pinned stack:** `langfuse/langfuse:3.63.0` + `langfuse/langfuse-worker:3.63.0`,
  loopback-bound `127.0.0.1:${LANGFUSE_PORT:-3000}` (`observability/langfuse/docker-compose.yml`)
- **Reference-only inputs (not modified):** `observability/pi-observe/bin/pi-observe.mjs`
  (emitter v0.1.2), `observability/langfuse/`, legacy manual-tracker surface
- **Spike probe:** `spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs` (non-shipping)

> Scope reminder: this is the **Phase A validation spike** only. It ships no durable
> importer, no host-runtime decision (TASK-003), no durable schema/migration (TASK-004),
> no classification (TASK-008), no summaries (TASK-010), and no new pi/Claude adapter
> (DEC-017). It validates the schema, designs+proves the import flow, defines the
> health-state model, assesses mapping signals, and proposes table shapes as inputs to
> TASK-004.

---

## 0. Validation-status matrix (read this first)

The local Langfuse stack **could not be brought up in this environment**, so the live
round-trip confirmations are **PENDING** on a missing project-level dependency (see
§9 blockers). Everything that can be established from the **pinned version's public API
contract** and the **emitter source** is delivered now at design grade; the live probe
(`langfuse-probe.mjs`) is written and parse-checked so the PENDING rows can be closed by
hand the moment the stack is up, without further design work.

| # | Deliverable | Method | Status |
| --- | --- | --- | --- |
| S1 | Trace/observation schema (identity, env, timestamps, session, metadata) | emitter source (authoritative for local traces) + 3.63.0 read-API contract | **Established** (live round-trip PENDING) |
| S2 | Usage/cost field shapes | 3.63.0 observation/usage contract; emitter confirms pi-observe emits **none** | **Established** (live round-trip PENDING) |
| S3 | Time/usage/cost sufficiency as primary source | semantics analysis | **Established** (caveat in §3) |
| I1 | REST query by environment + time window | read-API contract + probe | **Designed + probe ready** (live PENDING) |
| I2 | Pagination to window completion + cursor | read-API contract + probe | **Designed + probe ready** (live PENDING) |
| I3 | Dedup by trace id scoped to env/project | algorithm + probe | **Designed + probe ready** (live PENDING) |
| H1 | 9-state health model w/ detection basis + consequence | design | **Complete** |
| H2 | absence ≠ zero usage/cost invariant | design + detection wiring | **Complete** |
| H3 | Workspace failure modes (wrong/default env, hook silent-fail) | design + emitter evidence | **Complete** |
| M1 | Mapping-signal assessment (environment-first, hashed sessions) | emitter source | **Complete** |
| T3 | TASK-003 host-runtime friction signal | analysis | **Complete** |
| T4 | TASK-004 normalized-evidence + `langfuse_import_runs` shapes | design proposal | **Complete** |
| B1 | Network/credential boundary (SEC-002/003) | code + probe review | **Complete** |

"Established" = derived from the pinned version's deterministic API contract and the
authoritative emitter source; the only thing PENDING is a live byte-for-byte round-trip,
which does not change the design.

---

## 1. Validation environment

- **Server:** `langfuse/langfuse:3.63.0` (web + worker), Postgres 16, Redis 7,
  ClickHouse 24.8, MinIO — all pinned in `observability/langfuse/docker-compose.yml`,
  port published **only** to `127.0.0.1` (`"127.0.0.1:${LANGFUSE_PORT:-3000}:3000"`).
- **Emitter:** `pi-observe` v0.1.2 — a **metadata-only** wrapper. It does **not** use the
  Langfuse SDK; it POSTs hand-built batches to `POST /api/public/ingestion` with HTTP
  Basic auth and a hard loopback restriction (`isLoopbackLangfuseHost`), failing open.
- **What the spike validates against:** the pinned server's **public API** plus the
  exact payloads the emitter produces. Because the emitter source is in-repo, the
  *content* of local `vire`/`default` traces is fully known without guessing; the
  pinned server fixes the *read-API envelope*. The live probe closes the loop.

---

## 2. Observed trace/observation schema (S1, S2)

### 2.1 What `pi-observe` actually writes (authoritative — emitter source)

The emitter sends two ingestion events per wrapped run to
`POST /api/public/ingestion` (`pi-observe.mjs:262`, `:268`):

**`trace-create` body**

| Field | Shape | Source / notes |
| --- | --- | --- |
| `id` | string (hex, 32 chars, UUID dashes stripped) | `traceId`, client-generated |
| `name` | string | `"${tool}"` or `"${tool}.${role}"` (e.g. `claude-code.delegate-developer`) |
| `userId` | string | `safeToken(PI_OBSERVE_USER_ID || 'local-janne')` |
| `sessionId` | string | `session-<sha256(redact(session))[:24]>` or random UUID — **hashed/opaque** |
| `tags` | string[] | `['local', tool, role, project_key]` (falsy dropped) |
| `metadata` | object | sanitized `meta` (see below) |
| `timestamp` (envelope) | ISO-8601 string | event envelope `timestamp`, not trace body |

**`trace-update` body** (on completion): `id`, `tags` (+`status`), `metadata` (+`status`,
`exit_code`, `duration_ms`, `end_time`).

**`metadata` keys** (`pi-observe.mjs:259`): `wrapper_version`, `project_key`,
`project_confidence`, `tool`, `role`, `cwd_basename`, `git_branch` (hashed unless
`PI_OBSERVE_CAPTURE_GIT_BRANCH=true`), `git_remote_hash` (hashed), `command_label`,
`billable` (bool), `summary` (redacted, optional), and on update `status`, `exit_code`,
`duration_ms`, `end_time`.

**Critical finding — `environment`:** the emitter **does not set an `environment`
field** on the trace body. Under Langfuse 3.63.0, ingestion without an explicit
environment lands traces in the project's **`default`** environment. This is the
empirical root of the "wrong/default environment" workspace risk (§5, §6.3): for the
spike's local emitter, **every** trace currently maps to `default`, and the project key
travels in `tags` + `metadata.project_key`, *not* in the trace `environment` dimension.
The importer therefore **must not** rely on `environment` alone for local pi-observe
traces and **must** treat `default` as a first-class health signal, not a silent pass.
(The product pi/Claude integrations that DO set `environment` — pi-langfuse — are where
DEC-004/DEC-017 "`environment` is primary" applies; pi-observe is the reference emitter,
not that integration.)

### 2.2 Read-API envelope served by 3.63.0 (S1 — pinned contract, live round-trip PENDING)

The importer reads via the **public REST API** (Basic auth, `pub:sec`):

- `GET /api/public/traces` → `{ data: Trace[], meta: { page, limit, totalItems, totalPages } }`.
  Query params used by the importer: `environment`, `fromTimestamp`, `toTimestamp`,
  `page`, `limit`, optional `tags`, `sessionId`, `userId`, `orderBy`.
- `GET /api/public/traces/{traceId}` → single `Trace` **including `observations[]` and
  aggregated `totalCost`/latency** for that trace.
- `GET /api/public/observations` (and `/observations/{id}`) → observation/generation
  rows where **token usage and model cost live** (see §2.3).

`Trace` envelope fields the importer consumes: `id`, `timestamp`, `name`,
`environment`, `sessionId`, `userId`, `tags[]`, `metadata`, `release`, `version`,
`public`, plus `createdAt`/`updatedAt`. `htmlPath`/`latency`/`totalCost` appear on the
detail/aggregated views. **Field names are recorded from the version contract; the probe
records the live key→shape map (`schemaSketch`) to confirm names/units/nullability and
flag any drift as `schema_mismatch`.**

### 2.3 Where usage and cost actually are (S2)

This is the single most important schema finding for cost correctness:

- **pi-observe traces carry no usage and no cost.** The emitter writes no `usage`, no
  token counts, and no cost on its trace bodies, and emits **no generation/observation**
  events at all. Timing is conveyed as `metadata.duration_ms` + `metadata.end_time`
  (wall-clock of the wrapped command), **not** as model latency or token cost.
- **Usage/cost come from generation observations**, not traces. In Langfuse 3.63.0 a
  generation-type **observation** carries `usage` (historically
  `{ promptTokens, completionTokens, totalTokens, unit }`) and the newer
  `usageDetails` / `costDetails` maps plus `calculatedTotalCost` (and per-trace aggregate
  `totalCost`). The exact present-vs-absent set is **version- and SDK-dependent**
  (`04` §14 "Langfuse cost schema varies"), which is precisely why the importer records
  the **observed** shape and raises `schema_mismatch` when an expected
  usage/cost/timestamp field is absent or incompatible — degrading visibly instead of
  emitting a wrong total.

**Consequence for the MVP / TASK-006:** a valid pi/Claude **cost** total requires the
product integration (pi-langfuse / Claude Code Langfuse hook) to emit **generation
observations with usage+cost**. The metadata-only pi-observe wrapper is sufficient to
validate **time** semantics and the **trace identity/environment/session/metadata**
envelope, but **not** to validate live token cost on its own. Confirming real
usage/cost shape needs either (a) a generation-emitting integration pointed at the local
stack, or (b) `needs_input` to confirm against Janne's real environment keys (deferred to
the MVP per design.md). The importer's contract must read usage/cost from **observations**
and aggregate to the trace, never assume they sit on the trace body.

---

## 3. Time / usage / cost semantics (S3)

- **Time.** Two distinct clocks exist and must not be conflated: (a) wrapped-command
  wall-clock from pi-observe (`metadata.duration_ms`, start `timestamp` → `end_time`),
  which is a **runtime activity** signal; and (b) model/observation latency + timestamps
  from generation observations, the **AI** time signal. DEC-017 makes Langfuse the
  primary **AI** time/usage/cost source; the runtime/wall-clock from pi-observe is
  reconciliation/health (TASK-006), never a duplicate ledger. The importer's AI-time
  field is the observation/trace time, not pi-observe's `duration_ms`.
- **Usage.** Token usage is per-observation; a trace total is the sum across its
  generation observations. Null/absent usage on a non-generation observation is normal
  and is **not** zero usage for the trace — it means "no model call here."
- **Cost.** Per-observation `calculatedTotalCost`/`costDetails`, aggregated to trace
  `totalCost`. Cost may be **null** when a model/price is unmapped in Langfuse — this is a
  `schema_mismatch`/unmapped-price condition, **not** zero cost.
- **Primary-source verdict (where valid):** the 3.63.0 trace+observation model **is
  sufficient** to serve as the primary AI time/usage/cost source for traces that carry
  generation observations with usage+cost. The hard precondition is the **absence ≠ zero**
  invariant (§4 H2): missing/null usage or cost is surfaced as a health state, never
  folded into a total as zero.

---

## 4. Source-health state model (H1, H2) — the 9-state taxonomy (TASK-006 contract)

This taxonomy is the interface TASK-006's runtime observer reconciles against. Defined
here; implemented for reconciliation there; this change does not depend on the observer.

| State | Detection basis (against the read API / import bookkeeping) | User-visible consequence |
| --- | --- | --- |
| `valid` | recent import; traces align with expected env/session; usage/cost present where a generation exists | Langfuse usable as AI time/usage/cost source for the window |
| `missing` | local runtime/expected activity (pi-observe event, manual marker) but **no matching trace** in window | flagged as missing AI evidence; **never** rendered as zero usage/cost |
| `stale` | latest trace / `langfuse_import_run.latest_trace_ts` older than expected threshold | shown as stale, not treated as current totals |
| `wrong_env` | traces in `default`/unexpected environment vs project mapping (pi-langfuse v1.4.3 propagation risk; pi-observe always `default`) | surfaced for review; never silently trusted as the project's total |
| `delayed` | trace `timestamp` precedes a prior checkpoint but arrives after it (late arrival) | re-imported safely via overlap window; reconciled, not dropped |
| `duplicate` | same trace id across pages / re-imports / overlapping windows | deduped by `(environment, trace_id)`; counted once; `duplicate` signal recorded |
| `schema_mismatch` | expected usage/cost/timestamp field absent or wrong type vs recorded 3.63.0 shape | importer degrades visibly; the trace is not converted to a (wrong) total |
| `auth_or_config_error` | read API returns 401/403, or keys/host misconfigured | reported **without** exposing any secret material; no total fabricated |
| `rate_limited` | read API returns 429 | backoff/retry; surfaced as health, **not** as zero for the window |

**H2 — absence ≠ zero (validated as a design invariant and wired into detection):** in
every row above, the *absence* of data (`missing`, null usage, null cost, `stale`,
`rate_limited`, `auth_or_config_error`) resolves to a **health flag**, never a numeric
zero contributed to an AI total. The probe encodes this literally: an empty environment
yields cursor `"(none — env empty; absence != zero cost)"`, not `0`.

---

## 5. Workspace-specific failure modes (H3)

- **Wrong/`default` environment (pi-langfuse propagation risk).** Confirmed structurally
  from the emitter: pi-observe sets **no `environment`**, so its traces land in `default`
  and carry the project only in `tags`/`metadata.project_key`. The product pi-langfuse
  integration (v1.4.3) has a known propagation risk where the intended environment fails
  to attach and traces also land in `default`. **Detection:** a trace whose
  `environment` is `default` (or any value not in the project's expected env set) while
  metadata/tags indicate a real project → emit `wrong_env`. The importer must **not**
  fold `default`-environment traces silently into a project total.
- **Claude Code hook silent-fail (missing/stale).** If the Claude Code Langfuse hook
  fails silently, no generation observations arrive for an agent that pi-observe (or the
  runtime observer) saw active. **Detection:** runtime/expected activity with no matching
  trace in the window → `missing`; if the newest trace predates the threshold → `stale`.
  Either way the gap is surfaced, never treated as zero AI cost.

---

## 6. Import flow: pagination, dedup, cursors (I1, I2, I3)

### 6.1 Query (I1)
`GET /api/public/traces?environment={env}&fromTimestamp={fromISO}&toTimestamp={toISO}&limit={n}&page={p}`,
Basic auth. Environment-scoped + time-windowed. Read-only — the importer **pulls** traces
and never pushes activity (SEC-002).

### 6.2 Pagination + cursor (I2)
Walk `page = 1..meta.totalPages` (or until a short page) accumulating `data[]`. The
**per-environment cursor/checkpoint** is the **maximum observed trace timestamp** in the
completed window (the probe computes `maxTs`). The next import resumes from
`cursor − overlap` (a small safety overlap, e.g. a few minutes) to catch `delayed`
late-arrivals; overlap re-reads are made idempotent by dedup (§6.3). Durable persistence
of the cursor is **deferred to TASK-004 / the MVP** — this spike computes and proves the
position, it does not store it.

### 6.3 Deduplication (I3)
Dedup key is **`(environment, trace_id)`**. A trace id already seen for the environment
is suppressed and counted once; a `duplicate` signal is recorded rather than
double-counting usage/cost. This makes pagination overlaps, re-imports, and the
cursor-overlap window all safe. (Project scoping rides on top of environment: for local
pi-observe traces the project lives in `metadata.project_key`/`tags`, so the practical key
is `(environment, project_key, trace_id)` once mapping is applied — classification itself
is TASK-008.)

**Proof status:** the algorithm is implemented in `langfuse-probe.mjs` (pagination loop,
`seen` set keyed by `environment:trace_id`, `dupes` counter, `maxTs` cursor) and
parse-checked. The live multi-page/duplicate/overlap demonstration is **PENDING** the
running stack (§0, §9).

---

## 7. Project-mapping signal assessment (M1)

| Signal | Availability on 3.63.0 traces | Usefulness for trace→Vire-project mapping |
| --- | --- | --- |
| `environment` | present (defaults to `default` when unset) | **Primary** per DEC-004/DEC-017 **for integrations that set it**. For pi-observe it is always `default`, so for the reference emitter it is a **health signal**, not a mapping key. |
| `tags[]` | `['local', tool, role, project_key, status]` | Strong secondary: carries `project_key` and `tool`/`role` directly. |
| `metadata.project_key` (+`project_confidence`) | present, with confidence label (`explicit`/`env`/`.pi-project`/`path-map`/`git-remote-hash`/`low`) | Strong: the emitter's own resolved project + how confident it was. Low-confidence values must be treated as provisional. |
| `metadata.cwd_basename`, `git_branch` (hashed), `git_remote_hash` (hashed), `command_label` | present | Useful corroborating metadata; git fields are **hashed**, so they correlate but do not reveal repo identity. |
| `sessionId` | present but **hashed before transmission** (`session-<sha256[:24]>`) | **Opaque correlation only.** Can group a session's traces; cannot recover project identity. |
| `userId` | present (`local-janne` default) | Low signal in a single-user local setup. |

**Assessment:** mapping leans on `environment` **first only where the integration sets it
correctly**; otherwise on `tags`/`metadata.project_key` (+confidence), corroborated by
hashed git/cwd metadata. Hashed session ids are opaque handles. **`wrong_env` is
first-class** (§4/§5). Full classification rules are **deferred to TASK-008**; summary
shaping to **TASK-010**. This spike only assesses signal availability/quality.

---

## 8. Proposals to TASK-004 (T4) — shapes only, no migration

These are **input proposals** for TASK-004's durable schema work. **No table, migration,
or durable file is created by this change.** No credentials appear in any shape.

### 8.1 `langfuse_import_runs` (import bookkeeping / cursor / health)

| Column | Type (proposed) | Notes |
| --- | --- | --- |
| `id` | text/uuid PK | import run id |
| `environment` | text | environment scope of this run |
| `project_key` | text null | resolved project (post-mapping; null until classified) |
| `window_from` / `window_to` | timestamp | requested time window |
| `cursor_ts` | timestamp null | per-environment checkpoint = max observed trace ts; null when window empty (≠ 0) |
| `status` | text enum | one of the 9 health states (§4) |
| `pages_walked` / `traces_seen` / `duplicates_suppressed` | integer | pagination/dedup bookkeeping |
| `warnings` | json/text[] | health detail (e.g. unmapped-price, `wrong_env` count); **never** secrets |
| `started_at` / `finished_at` | timestamp | run timing |

**No** `public_key`, `secret_key`, host with embedded creds, or any token column — keys
load from local secure config only (SEC-003).

### 8.2 Normalized AI-evidence shape (trace time/usage/cost/source/health)

| Field | Type (proposed) | Notes |
| --- | --- | --- |
| `trace_id` | text | from Langfuse |
| `environment` | text | env dimension |
| `project_key` | text null | mapped (TASK-008 owns the rule) |
| `source` | text | e.g. `langfuse` (primary AI source per DEC-017) |
| `ai_start_ts` / `ai_end_ts` | timestamp null | from observation/trace time (**not** pi-observe wall-clock) |
| `prompt_tokens` / `completion_tokens` / `total_tokens` | integer null | summed over generation observations; **null ≠ 0** |
| `cost_total` | numeric null | aggregated `totalCost`/`calculatedTotalCost`; **null ≠ 0** (unmapped price) |
| `cost_currency` | text null | when present |
| `health` | text enum | §4 state for this evidence row |
| `import_run_id` | text FK | → `langfuse_import_runs.id` |

Dedup uniqueness: `(environment, trace_id)`. Keep raw trace payloads (if retained at all)
on **short, configurable retention** with no credentials — a note for TASK-004.

---

## 9. TASK-003 host-runtime friction signal (T3)

For TASK-003's Tauri+helper-vs-Swift-first decision, the importer side contributes:

- **The importer is a thin, read-only REST+JSON client** over four documented endpoints
  with HTTP Basic auth, time/page query params, and exponential backoff on 429. There is
  **no Langfuse-specific native dependency** — any host that can do HTTPS + JSON + a small
  amount of date math suffices.
- **Rust/Tauri path:** low friction. `reqwest` + `serde_json` cover the client; the
  loopback/SEC-002 boundary is a single base-URL allowlist; secrets load from local
  secure config (e.g. keychain/`.env`), never into rows/logs. This is the **lower-friction
  option from the importer's perspective.**
- **Swift path:** also feasible (`URLSession` + `Codable`), no blocker, but duplicates the
  REST/health/cursor logic in a second language if other components are Rust.
- **Net signal to TASK-003:** the importer does **not** force the host decision — it is
  comfortable on either runtime. It contributes a mild **"Rust REST client is slightly
  lower friction"** vote, to be weighed against the capture-side signal from TASK-002.
  The decision remains TASK-003's.

---

## 10. Security / boundary review (B1) — SEC-002 / SEC-003

- **SEC-002 network boundary.** The importer's only network path is the **configured
  Langfuse base URL / trace endpoints** (local `http://localhost:3000`). The probe
  **refuses any non-loopback host** and the emitter enforces the same. The importer is
  **read-only** against Langfuse (GET only) — it pulls traces, never pushes macOS
  activity, window titles, prompt/response text, or command bodies.
- **SEC-003 credentials.** Keys load from `observability/langfuse/.env` (chmod 600,
  gitignored) via a **data-only parser** (no shell sourcing). They are used only for the
  `Authorization` header — **never** printed, logged, persisted to rows/exports/fixtures,
  or placed in PR text. Proposed `langfuse_import_runs` has **no** credential column.
  Documented config uses redacted placeholders only
  (e.g. `LANGFUSE_PUBLIC_KEY=...`, `LANGFUSE_SECRET_KEY=...`).
- **Probe data safety.** `langfuse-probe.mjs` emits **shape-only** output (field names,
  types, nullability, counts; strings reduced to length buckets) and writes nothing on its
  own; ephemeral records go to gitignored `*.local.log` with a documented `rm` cleanup. No
  prompt/response text, command bodies, secrets, or environment dumps are persisted.
- **Isolation.** Probe lives under `spikes/task-007-langfuse-importer/`, outside `src/`,
  `src-tauri/src/`, `observability/`; not referenced by any build target. `pi-observe`,
  the local stack, and the legacy manual-tracker surface are **unmodified** and
  reference-only (DEC-017). No new pi/Claude adapter was built.

---

## 11. Exit gate

- **Langfuse can serve as the primary AI time/usage/cost source where traces are valid** —
  established from the pinned 3.63.0 trace+observation model, **conditioned on**
  generation observations carrying usage+cost (pi-observe alone validates time + identity
  envelope, not live cost; see §2.3, §3). The live byte-for-byte confirmation is PENDING
  the running stack (§0/§12), not a design gap.
- **The 9 health states** (`valid`, `missing`, `stale`, `wrong/default environment`,
  `delayed`, `duplicate`, `schema mismatch`, `auth/config failure`, `rate limit`) are
  **defined with detection basis + consequence**, and **absence ≠ zero usage/cost** is an
  enforced invariant (§4).
- **Pagination, dedup `(environment, trace_id)`, and per-environment cursor** are
  **designed and implemented in the probe**; live multi-page/duplicate/overlap demo is
  PENDING the stack.
- **Mapping-signal assessment** delivered (environment-first where set; tags/project_key
  otherwise; hashed sessions opaque); classification deferred to TASK-008, summaries to
  TASK-010.
- **Credentials protected; network boundary preserved** (§10).
- **No durable importer shipped; no host-runtime decision (TASK-003); no durable schema
  (TASK-004).** Shapes proposed only.

**Gate verdict:** design/validation deliverables **met**; the spike's empirical
live-round-trip rows are **PENDING a missing local dependency**, not failed. See §12.

---

## 12. Blockers / missing dependencies (do-not-install policy honored)

The local Langfuse stack **was not brought up** because of missing **project-level
dependencies** in this environment. Per scope, these are **reported, not installed**:

1. **Docker daemon not running.** `docker info` fails (daemon down); no Colima present.
   Starting Docker Desktop/Colima is an interactive host action the repo's own
   `scripts/setup-local-observability.sh` gates behind a confirmation — not silently
   started here.
2. **`docker compose` plugin missing.** `docker compose version` reports
   `unknown command: docker compose`. The stack's `langfuse-up.sh` requires it. Installing
   the compose plugin is forbidden by scope.
3. **`observability/langfuse/.env` not created.** Produced by the interactive
   `setup-local-observability.sh` (and project API keys are created in the UI afterward).
   No `.env` ⇒ no public/secret keys ⇒ read API cannot be exercised.

**No production / non-local credentials were requested or used** — the spike is local-only
by design, so this is **not** a `needs_input` for secrets. To close the PENDING rows
(S1/S2 live round-trip, I1/I2/I3 live demo) a maintainer needs to, on a machine with
Docker available:

```sh
./scripts/setup-local-observability.sh   # interactive; checks Docker/Compose, writes .env
./scripts/langfuse-up.sh                 # pinned stack on 127.0.0.1:3000
# create project API keys in the UI → paste into observability/langfuse/.env
./scripts/langfuse-smoke-test.sh         # emits non-sensitive vire + default traces
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default
```

The probe then confirms S1/S2 field names/units/nullability and demonstrates I1/I2/I3
live; any field drift from §2 surfaces as `schema_mismatch`. None of this changes the
design in this report.

---

## 13. Spike disposal

Delete `spikes/task-007-langfuse-importer/` once the TASK-007 MVP has consumed these
findings, so no probe code lingers near product runtime.
