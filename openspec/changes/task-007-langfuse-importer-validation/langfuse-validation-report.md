# Langfuse importer validation report — TASK-007 (Phase A spike)

- **Change:** `task-007-langfuse-importer-validation`
- **Branch:** `feat/task-007-langfuse-importer-validation`
- **Author:** Integration Engineer (SW-2)
- **Date:** 2026-06-04
- **Tier:** L2 · **Gate context:** APP-005 (SEC-002 network boundary, SEC-003 credentials)
- **Validation target (DEC-018):** the **configured Langfuse API** — a configurable base
  URL with project-scoped Basic-auth credentials, environment/date filtering — validated
  **cloud-first** against Janne's configured project (`https://cloud.langfuse.com`). The
  pinned local Docker stack (`langfuse/langfuse:3.63.0`, `observability/langfuse/`,
  loopback-bound) is an **optional offline/dev contract-test fixture, not a blocking
  dependency**.
- **Reference-only inputs (not modified):** `observability/pi-observe/bin/pi-observe.mjs`
  (emitter v0.1.2), `observability/langfuse/`, legacy manual-tracker surface
- **Spike probe:** `spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs`
  (non-shipping; `--mock` offline-proof mode + live configured-API mode)

> Scope reminder: this is the **Phase A validation spike** only. It ships no durable
> importer, no host-runtime decision (TASK-003), no durable schema/migration (TASK-004),
> no classification (TASK-008), no summaries (TASK-010), and no new pi/Claude adapter
> (DEC-017). It validates the schema, designs+proves the import flow, defines the
> health-state model, assesses mapping signals, and proposes table shapes as inputs to
> TASK-004.

---

## 0. Validation-status matrix (read this first)

Per **DEC-018** the spike validates **cloud-first** against the configured Langfuse API.
Project-scoped credentials were available in **local secure config** (environment), so the
**live cloud round-trip was performed** against `https://cloud.langfuse.com`: auth
succeeded (health `HTTP 200`), the real trace/observation schema was recorded **shape-only**
(field names, types, nullability, counts — never values or secrets), and pagination +
per-environment cursor were exercised over a real multi-page window. The import-flow logic
(pagination / dedup / cursor / 9-state health) is **additionally proven offline** with
synthetic fixtures via `langfuse-probe.mjs --mock` (no network, no credentials, no
container) so the same code path is validated with and without live access.

| # | Deliverable | Method | Status |
| --- | --- | --- | --- |
| S1 | Trace/observation schema (identity, env, timestamps, session, metadata) | **live cloud round-trip** (shape-only) + emitter source + 3.63.0 read-API contract | **Confirmed live** |
| S2 | Usage/cost field shapes | **live cloud observations** (shape-only); emitter confirms pi-observe emits **none** | **Confirmed live** |
| S3 | Time/usage/cost sufficiency as primary source | semantics analysis + live shapes | **Confirmed** (caveat in §3) |
| I1 | REST query by environment + time window | read-API contract + probe (live + mock) | **Proven** |
| I2 | Pagination to window completion + cursor | live multi-page window + mock 3-page proof | **Proven** |
| I3 | Dedup by trace id scoped to env/project | mock cross-page/overlap proof; live cursor computed | **Proven** |
| H1 | 9-state health model w/ detection basis + consequence | design + mock assertion of all 9 states | **Complete** |
| H2 | absence ≠ zero usage/cost invariant | design + detection wiring + mock assertion | **Complete** |
| H3 | Workspace failure modes (wrong/default env, hook silent-fail) | design + **live confirmation** (see §5) | **Complete (live-confirmed)** |
| M1 | Mapping-signal assessment (environment-first, hashed sessions) | emitter source + live field availability | **Complete** |
| T3 | TASK-003 host-runtime friction signal | analysis | **Complete** |
| T4 | TASK-004 normalized-evidence + `langfuse_import_runs` shapes | design proposal | **Complete** |
| B1 | Network/credential boundary (SEC-002/003) | code + probe review | **Complete** |

---

## 1. Validation environment

- **Primary target (DEC-018, cloud-first):** the **configured Langfuse API** at a
  configurable base URL. For this run the configured base was `https://cloud.langfuse.com`,
  with **project-scoped Basic-auth keys loaded from local secure config** (environment
  variables `LANGFUSE_HOST` / `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY`; an
  `observability/langfuse/.env`, chmod 600 + gitignored, is the equivalent file form). The
  keys are used only for the `Authorization` header and are **never** printed, logged, or
  persisted.
- **Optional fixture (not required):** `langfuse/langfuse:3.63.0` (web + worker), pinned in
  `observability/langfuse/docker-compose.yml`, port published **only** to `127.0.0.1`. This
  is retained as an offline/dev contract-test fixture and self-host evaluation harness, **not**
  the blocking validation gate (DEC-018).
- **Emitter (reference-only):** `pi-observe` v0.1.2 — a **metadata-only** wrapper. It does
  **not** use the Langfuse SDK; it POSTs hand-built batches to `POST /api/public/ingestion`
  with HTTP Basic auth, failing open. Its source is authoritative for the *content* of the
  local `vire`/`default` traces it writes.
- **What the spike validates against:** the configured server's **public REST API**
  (live round-trip, shape-only) plus the exact payloads the emitter produces. The live probe
  closed the loop against the real project; the `--mock` mode proves the same import-flow
  logic offline.

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
field** on the trace body. Under the configured Langfuse API, ingestion without an explicit
environment lands traces in the project's **`default`** environment. This is the empirical
root of the "wrong/default environment" workspace risk (§5, §6.3), and the live round-trip
confirmed it directly: the configured **`vire` environment is empty**, while the project's
real pi/Claude traffic lands in **`default`** (§5). The importer therefore **must not** rely
on `environment` alone for local pi-observe traces and **must** treat `default` as a
first-class health signal, not a silent pass.

### 2.2 Read-API envelope — confirmed live against the configured API (S1)

The importer reads via the **public REST API** (Basic auth, `pub:sec`). Confirmed
endpoints and the **observed live `Trace` shape** (recorded shape-only from
`GET /api/public/traces?environment=default`, 50 real traces, 2 pages):

- `GET /api/public/traces` → `{ data: Trace[], meta: { page, limit, totalItems, totalPages } }`.
  Query params used by the importer: `environment`, `fromTimestamp`, `toTimestamp`,
  `page`, `limit`, optional `tags`, `sessionId`, `userId`, `orderBy`.
- `GET /api/public/traces/{traceId}` → single `Trace` including `observations[]`.
- `GET /api/public/observations` (and `/observations/{id}`) → observation/generation
  rows where **token usage and model cost live** (see §2.3).

**Observed `Trace` fields (live, shape-only):** `id` (string), `timestamp` (string),
`createdAt`/`updatedAt` (string), `name` (string; **may be empty**), `environment`
(string), `sessionId` (**nullable** — `null | string`), `userId` (null in this single-user
setup), `tags[]` (len 0–1 observed), `metadata` (object; **key-count varies 0 / 8 / 14**),
`release`/`version`/`externalId` (null), `public`/`bookmarked` (boolean), `projectId`
(string), `htmlPath` (string), `latency` (**number/float — present on the list view**),
`totalCost` (**number/float — aggregate cost present on the trace**), `scores[]`,
`observations[]` (array), and `input`/`output` (`null | object | string`).

**Two observed-vs-assumed nuances the importer must handle:**

1. **`sessionId` is nullable** — session-based correlation cannot be assumed present.
2. **`name` may be empty** and **`metadata` richness varies** (0 to 14 keys) — mapping must
   tolerate sparse traces rather than assume a fixed metadata shape.

The probe records the live key→shape map (`schemaSketch`) so any future drift in names/
units/nullability surfaces as `schema_mismatch` (§4) rather than a wrong total.

> Privacy note: trace `input`/`output` can carry model prompt/response content. The probe
> recorded **only their shape** (object key-count / string length-bucket), never values, and
> the importer **must not** persist raw trace `input`/`output` content (SEC-003, §10).

### 2.3 Where usage and cost actually are — confirmed live (S2)

This is the single most important schema finding for cost correctness, now **confirmed
against the real project**:

- **pi-observe traces carry no token usage and no per-call cost.** The emitter writes no
  `usage`, no token counts, and no generation observations; timing is conveyed as
  `metadata.duration_ms` + `metadata.end_time` (wall-clock of the wrapped command), **not**
  model latency or token cost.
- **Usage/cost come from generation observations**, not from the trace body. The live
  observation schema (shape-only) confirms a generation observation carries:
  `promptTokens` / `completionTokens` / `totalTokens` (int), a nested `usage`
  (object, 4 keys), `usageDetails` (object; **0 or 4 keys** — may be empty),
  `calculatedTotalCost` / `calculatedInputCost` / `calculatedOutputCost`,
  `costDetails` (object; **0 or 3 keys**), `inputPrice` / `outputPrice` / `totalPrice`,
  `unit`, `model` / `modelId` (**nullable**), `startTime` / `endTime` / `latency`,
  `timeToFirstToken` (nullable), `level`, `type`, plus `usagePricingTierId` /
  `usagePricingTierName` (nullable).
- **Observed `usage` map keys: `{ input, output, total, unit }`** — **not** the historical
  `{ promptTokens, completionTokens, totalTokens, unit }`. The prompt/completion/total token
  counts appear instead as **top-level observation fields**. The importer must read token
  counts from the observed shape, not assume the nested-`usage` historical key names.
- **Trace-level `totalCost` is also present** as an **aggregate** (§2.2). A trace total can
  be read from `totalCost`, but the authoritative per-call breakdown (tokens, per-model
  cost, nullability) lives on observations.
- Several cost/price fields are **nullable** (e.g. `calculatedInputCost`,
  `calculatedOutputCost`, `model`, `usageDetails`/`costDetails` can be empty) — confirming
  the `schema_mismatch` / unmapped-price condition is real and must degrade visibly, never
  fold to zero (§3).

**Consequence for the MVP / TASK-006:** a valid pi/Claude **cost** total requires the
product integration (pi-langfuse / Claude Code Langfuse hook) to emit **generation
observations with usage+cost** in the project's correct environment. The metadata-only
pi-observe wrapper validates **time** semantics and the **trace identity/environment/
session/metadata** envelope, but not live token cost on its own. The importer's contract
must read usage/cost from **observations** and aggregate to the trace, never assume they sit
on the trace body.

---

## 3. Time / usage / cost semantics (S3)

- **Time.** Two distinct clocks exist and must not be conflated: (a) wrapped-command
  wall-clock from pi-observe (`metadata.duration_ms`, start `timestamp` → `end_time`),
  a **runtime activity** signal; and (b) model/observation `latency` + `startTime`/`endTime`
  from generation observations, the **AI** time signal (both confirmed present live). DEC-017
  makes Langfuse the primary **AI** time/usage/cost source; the runtime/wall-clock from
  pi-observe is reconciliation/health (TASK-006), never a duplicate ledger.
- **Usage.** Token usage is per-observation; a trace total is the sum across its generation
  observations. Null/absent usage on a non-generation observation is normal and is **not**
  zero usage for the trace.
- **Cost.** Per-observation `calculatedTotalCost`/`costDetails`, aggregated to trace
  `totalCost`. Cost/price fields may be **null** when a model/price is unmapped (observed
  live) — a `schema_mismatch`/unmapped-price condition, **not** zero cost.
- **Primary-source verdict (where valid):** the configured Langfuse trace+observation model
  **is sufficient** to serve as the primary AI time/usage/cost source for traces that carry
  generation observations with usage+cost. The hard precondition is the **absence ≠ zero**
  invariant (§4 H2): missing/null usage or cost is surfaced as a health state, never folded
  into a total as zero.

---

## 4. Source-health state model (H1, H2) — the 9-state taxonomy (TASK-006 contract)

This taxonomy is the interface TASK-006's runtime observer reconciles against. Defined
here; implemented for reconciliation there; this change does not depend on the observer. All
nine states are asserted produced-by-their-detection-rule in `langfuse-probe.mjs --mock`.

| State | Detection basis (against the read API / import bookkeeping) | User-visible consequence |
| --- | --- | --- |
| `valid` | recent import; traces align with expected env/session; usage/cost present where a generation exists | Langfuse usable as AI time/usage/cost source for the window |
| `missing` | local runtime/expected activity (pi-observe event, manual marker) but **no matching trace** in window | flagged as missing AI evidence; **never** rendered as zero usage/cost |
| `stale` | latest trace / `langfuse_import_run.latest_trace_ts` older than expected threshold | shown as stale, not treated as current totals |
| `wrong_env` | traces in `default`/unexpected environment vs project mapping (pi-langfuse v1.4.3 propagation risk; pi-observe always `default`; **confirmed live — see §5**) | surfaced for review; never silently trusted as the project's total |
| `delayed` | trace `timestamp` precedes a prior checkpoint but arrives after it (late arrival) | re-imported safely via overlap window; reconciled, not dropped |
| `duplicate` | same trace id across pages / re-imports / overlapping windows | deduped by `(environment, trace_id)`; counted once; `duplicate` signal recorded |
| `schema_mismatch` | expected usage/cost/timestamp field absent or wrong type vs recorded shape (e.g. nullable cost/model, empty `usageDetails`, `usage` key naming §2.3) | importer degrades visibly; the trace is not converted to a (wrong) total |
| `auth_or_config_error` | read API returns 401/403, or keys/host misconfigured | reported **without** exposing any secret material; no total fabricated |
| `rate_limited` | read API returns 429 | backoff/retry; surfaced as health, **not** as zero for the window |

**H2 — absence ≠ zero (validated as a design invariant and asserted in mock):** in every
row above, the *absence* of data (`missing`, null usage, null cost, `stale`, `rate_limited`,
`auth_or_config_error`) resolves to a **health flag**, never a numeric zero contributed to
an AI total. The probe encodes this literally: an empty environment yields cursor `null`
(state `missing`), not `0` — asserted in `--mock` and observed live for the empty `vire`
environment.

---

## 5. Workspace-specific failure modes (H3) — live-confirmed

- **Wrong/`default` environment — confirmed live.** The live round-trip showed the
  configured **`vire` environment returns no traces**, while the project's real pi/Claude
  traffic lands in **`default`** (50 traces in a 30-day window). This matches the emitter
  finding (§2.1): pi-observe sets **no `environment`**, so its traces land in `default` and
  carry the project only in `tags`/`metadata.project_key`. The product pi-langfuse
  integration (v1.4.3) has the same propagation risk. **Detection:** a trace whose
  `environment` is `default` (or any value not in the project's expected env set) while
  metadata/tags indicate a real project → emit `wrong_env`. The importer must **not** fold
  `default`-environment traces silently into a project total, and must **not** read the empty
  `vire` environment as zero cost (it read as `missing`/empty-cursor live).
- **Claude Code hook silent-fail (missing/stale).** If the Claude Code Langfuse hook fails
  silently, no generation observations arrive for an agent that pi-observe (or the runtime
  observer) saw active. **Detection:** runtime/expected activity with no matching trace in
  the window → `missing`; if the newest trace predates the threshold → `stale`. Either way
  the gap is surfaced, never treated as zero AI cost.

---

## 6. Import flow: pagination, dedup, cursors (I1, I2, I3)

### 6.1 Query (I1)
`GET /api/public/traces?environment={env}&fromTimestamp={fromISO}&toTimestamp={toISO}&limit={n}&page={p}`,
Basic auth. Environment-scoped + time-windowed. Read-only — the importer **pulls** traces
and never pushes activity (SEC-002). **Exercised live** against the configured API.

### 6.2 Pagination + cursor (I2)
Walk `page = 1..meta.totalPages` (or until a short page) accumulating `data[]`. The
**per-environment cursor/checkpoint** is the **maximum observed trace timestamp** in the
completed window (`maxTs`). The next import resumes from `cursor − overlap` (a small safety
overlap) to catch `delayed` late-arrivals; overlap re-reads are made idempotent by dedup
(§6.3). Durable persistence of the cursor is **deferred to TASK-004 / the MVP** — this spike
computes and proves the position, it does not store it. **Live:** the probe walked a real
2-page `default` window (`meta.totalPages=2`) and computed the cursor. **Offline:** `--mock`
walks a synthetic 3-page window and computes the cursor deterministically.

### 6.3 Deduplication (I3)
Dedup key is **`(environment, trace_id)`**. A trace id already seen for the environment is
suppressed and counted once; a `duplicate` signal is recorded rather than double-counting
usage/cost. This makes pagination overlaps, re-imports, and the cursor-overlap window all
safe. (Project scoping rides on top of environment: for local pi-observe traces the project
lives in `metadata.project_key`/`tags`, so the practical key is
`(environment, project_key, trace_id)` once mapping is applied — classification itself is
TASK-008.)

**Proof status:** the algorithm is implemented in `langfuse-probe.mjs` (shared `paginate()`
with a `seen` set keyed by `environment:trace_id`, `dupes` counter, `maxTs` cursor) and is
exercised **identically** offline and live. `--mock` deterministically proves: 3-page window
→ 4 unique / 5 raw rows / 1 cross-page duplicate suppressed / 3 pages walked, plus an
overlapping re-import that is fully suppressed against the prior cursor (idempotent). The
live run exercised the same loop over the real `default` window.

---

## 7. Project-mapping signal assessment (M1)

| Signal | Availability on configured-API traces | Usefulness for trace→Vire-project mapping |
| --- | --- | --- |
| `environment` | present (defaults to `default` when unset; live `vire` env empty) | **Primary** per DEC-004/DEC-017 **for integrations that set it**. For pi-observe it is always `default`, so for the reference emitter it is a **health signal**, not a mapping key. |
| `tags[]` | `['local', tool, role, project_key, status]` (live: 0–1 tags on `default`-env traffic) | Strong secondary where present: carries `project_key` and `tool`/`role` directly. |
| `metadata.project_key` (+`project_confidence`) | present on pi-observe traces, with confidence label (`explicit`/`env`/`.pi-project`/`path-map`/`git-remote-hash`/`low`); **live metadata key-count varies 0–14** | Strong where populated: the emitter's own resolved project + confidence. Sparse-metadata traces must be tolerated. |
| `metadata.cwd_basename`, `git_branch` (hashed), `git_remote_hash` (hashed), `command_label` | present on pi-observe traces | Useful corroborating metadata; git fields are **hashed**, so they correlate but do not reveal repo identity. |
| `sessionId` | present but **hashed before transmission** (`session-<sha256[:24]>`); **nullable** on the API (live) | **Opaque correlation only**, and not always present. Can group a session's traces; cannot recover project identity. |
| `userId` | present (`local-janne` default; null on other traffic) | Low signal in a single-user local setup. |

**Assessment:** mapping leans on `environment` **first only where the integration sets it
correctly**; otherwise on `tags`/`metadata.project_key` (+confidence), corroborated by
hashed git/cwd metadata, and must **tolerate sparse/nullable** session and metadata fields
(confirmed live). Hashed session ids are opaque handles. **`wrong_env` is first-class**
(§4/§5). Full classification rules are **deferred to TASK-008**; summary shaping to
**TASK-010**. This spike only assesses signal availability/quality.

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
| `prompt_tokens` / `completion_tokens` / `total_tokens` | integer null | summed over generation observations (top-level obs fields / `usage{input,output,total}`); **null ≠ 0** |
| `cost_total` | numeric null | aggregated trace `totalCost` / observation `calculatedTotalCost`; **null ≠ 0** (unmapped price, observed live) |
| `cost_currency` | text null | when present |
| `health` | text enum | §4 state for this evidence row |
| `import_run_id` | text FK | → `langfuse_import_runs.id` |

Dedup uniqueness: `(environment, trace_id)`. Raw trace `input`/`output` content **must not**
be persisted (privacy, §10); if any raw payload is retained at all, keep it on **short,
configurable retention** with no credentials — a note for TASK-004.

---

## 9. TASK-003 host-runtime friction signal (T3)

For TASK-003's Tauri+helper-vs-Swift-first decision, the importer side contributes:

- **The importer is a thin, read-only REST+JSON client** over four documented endpoints
  with HTTP Basic auth, time/page query params, and exponential backoff on 429. There is
  **no Langfuse-specific native dependency** — any host that can do HTTPS + JSON + a small
  amount of date math suffices. (The live probe is a ~5 KB Node script using only `fetch`.)
- **Rust/Tauri path:** low friction. `reqwest` + `serde_json` cover the client; the
  SEC-002 boundary is a single base-URL allowlist; secrets load from local secure config
  (e.g. keychain/`.env`), never into rows/logs. This is the **lower-friction option from the
  importer's perspective.**
- **Swift path:** also feasible (`URLSession` + `Codable`), no blocker, but duplicates the
  REST/health/cursor logic in a second language if other components are Rust.
- **Net signal to TASK-003:** the importer does **not** force the host decision — it is
  comfortable on either runtime. It contributes a mild **"Rust REST client is slightly lower
  friction"** vote, to be weighed against the capture-side signal from TASK-002. The decision
  remains TASK-003's.

---

## 10. Security / boundary review (B1) — SEC-002 / SEC-003

- **SEC-002 network boundary.** The importer's only network path is the **configured
  Langfuse base URL / trace endpoints** (cloud or optional local). The probe builds every
  request as **base + path** and **never follows an absolute URL from response data**
  (`apiGet`); `normalizeBaseUrl` accepts only a syntactically valid `http(s)` origin. The
  importer is **read-only** against Langfuse (GET only) — it pulls traces, never pushes
  macOS activity, window titles, prompt/response text, or command bodies. No raw macOS
  activity is egressed.
- **SEC-003 credentials.** Keys load from local secure config (`observability/langfuse/.env`,
  chmod 600, gitignored, via a **data-only parser** with no shell sourcing; or environment
  variables). They are used only for the `Authorization` header — **never** printed, logged,
  persisted to rows/exports/fixtures, or placed in PR text. Proposed `langfuse_import_runs`
  has **no** credential column. Documented config uses redacted placeholders only
  (e.g. `LANGFUSE_HOST=https://cloud.langfuse.com`, `LANGFUSE_PUBLIC_KEY=...`,
  `LANGFUSE_SECRET_KEY=...`).
- **Probe data safety.** `langfuse-probe.mjs` emits **shape-only** output (field names,
  types, nullability, counts; strings reduced to length buckets) and writes nothing on its
  own; ephemeral records go to gitignored `*.local.log` with a documented `rm` cleanup. The
  live run printed **no values** — no prompt/response text, command bodies, secrets, real
  usage/cost numbers, or environment dumps. Trace `input`/`output` (which can hold model
  content) was recorded shape-only and **must not** be persisted by the importer.
- **Isolation.** Probe lives under `spikes/task-007-langfuse-importer/`, outside `src/`,
  `src-tauri/src/`, `observability/`; not referenced by any build target. `pi-observe`,
  the local stack, and the legacy manual-tracker surface are **unmodified** and
  reference-only (DEC-017). No new pi/Claude adapter was built.

---

## 11. Exit gate

- **Langfuse can serve as the primary AI time/usage/cost source where traces are valid** —
  **confirmed live** against the configured project's trace+observation model, **conditioned
  on** generation observations carrying usage+cost (pi-observe alone validates time +
  identity envelope, not live cost; see §2.3, §3).
- **The 9 health states** (`valid`, `missing`, `stale`, `wrong/default environment`,
  `delayed`, `duplicate`, `schema mismatch`, `auth/config failure`, `rate limit`) are
  **defined with detection basis + consequence** and **asserted in `--mock`**, and
  **absence ≠ zero usage/cost** is an enforced invariant (§4), confirmed live for the empty
  `vire` environment.
- **Pagination, dedup `(environment, trace_id)`, and per-environment cursor** are
  **designed, implemented, and proven** both offline (`--mock`) and against a real multi-page
  live window.
- **Mapping-signal assessment** delivered (environment-first where set; tags/project_key
  otherwise; hashed/nullable sessions opaque); classification deferred to TASK-008, summaries
  to TASK-010.
- **Credentials protected; network boundary preserved** (§10).
- **No durable importer shipped; no host-runtime decision (TASK-003); no durable schema
  (TASK-004).** Shapes proposed only.

**Gate verdict:** design/validation deliverables **met**, and the cloud-first live round-trip
**was performed** (schema confirmed shape-only; pagination/cursor exercised; wrong-env
empirically confirmed). See §12 for residual notes.

---

## 12. Residual notes (DEC-018 cloud-first; not blockers)

The live cloud round-trip succeeded, so the prior Docker-blocked status is **withdrawn**
(consistent with the arch-review §0 DEC-018 addendum). Remaining notes:

1. **`vire` environment is currently empty.** Real pi/Claude traffic lands in `default`
   (§5). This is a **finding, not a gap**: it is exactly the `wrong_env` workspace failure
   mode the importer must surface. A future pi-langfuse/Claude-hook configuration that sets
   `environment=vire` correctly would populate the `vire` env; until then the importer reads
   `default` + maps via `tags`/`metadata.project_key`.
2. **Live cost values were not recorded** — by design. The probe is shape-only; it confirmed
   usage/cost **fields exist and their nullability**, but did not read or persist any cost
   value (SEC-003). Numeric reconciliation of real totals is TASK-006's runtime concern.
3. **Optional offline fixture** remains available for contributors without configured cloud
   credentials:

```sh
# Offline logic proof (no network, no credentials, no container):
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --mock

# Live shape probe against the configured API (keys from local secure config):
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default

# Optional local Docker fixture (NOT required per DEC-018):
./scripts/setup-local-observability.sh   # interactive; writes observability/langfuse/.env
./scripts/langfuse-up.sh                 # pinned stack on 127.0.0.1:3000
./scripts/langfuse-smoke-test.sh         # emits non-sensitive vire + default traces
```

If a configured base URL / project keys are ever **absent**, the live probe prints secure
local-config instructions (redacted placeholders) and exits `needs_input` (exit code 2)
rather than probing or printing any secret.

---

## 13. Spike disposal

Delete `spikes/task-007-langfuse-importer/` once the TASK-007 MVP has consumed these
findings, so no probe code lingers near product runtime.
