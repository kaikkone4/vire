# Architecture review — TASK-045 Settings mapping completeness

**Verdict: PASS.** Single bugfix slice inside two existing components (`project-env-mapping`,
Langfuse discovery) within the architecture plan's `PROJECT_MAPPING` boundary. No new component, no
schema migration, no IPC contract change, no boundary crossing, no BA escalation. **No split required.**

## 1. Diagnosis (root cause, evidence)

The Settings mapping panel and the Suggestions view disagree about which environments exist, because
they read from two different tables:

- **Mapping panel** ← `langfuse_discovered_environments`, written **only** by `discover_and_record`
  over a **fixed 7-day window** (`langfuse/mod.rs:27` `DISCOVERY_WINDOW_DAYS = 7`; `mod.rs:172`).
- **Suggestions "Unmapped AI evidence"** ← `langfuse_ai_evidence`, every imported environment
  (`suggestions/engine.rs:195` `unmapped_summary`).

The configured import range defaults to 30 days and can be 90/all/`since:` (`ImportRange`, `mod.rs:33`).
A backfill imports evidence across that whole range, but discovery only looks back 7 days. So an
environment whose traces are all older than 7 days is **imported but never discovered** → has evidence
but **no mapping row** → cannot be mapped → its evidence is permanently untrackable/unsuggestable.
"Only 8 mapping entries" = the environments active in the last 7 days. There is **no `LIMIT`/cap** in
the SQL (`store.rs:306`) or the renderer (`env-mapping-ui.ts:93`) — the defect is coverage.

The Suggestions notice literally says "Map it in Settings to get suggestions"
(`suggestions-ui.ts:208`), but Settings has no row to map — a closed dead-end loop. Confirmed by tracing
`mappingPanel` (`main.ts:73`) → `list_discovered_environments` (`lib.rs:910`) →
`list_discovered_environments_repo` (`env_mapping/mod.rs:178`) → `store::list_discovered_environments`
(`store.rs:306`).

## 2. Architectural consistency check

`03_architecture_plan.md` requires environment-first project mapping for trace sources:

- `:141` — "map traces to Vire projects by environment first, then metadata/session/manual correction";
- `:175` — `PROJECT ||--o{ PROJECT_MAPPING : has`;
- `:137` — "Langfuse environments remain the primary project mapping mechanism";
- DEC-004 — import by environment as the primary AI time/usage/cost path.

The plan therefore mandates that **every** environment carrying traces be mappable. The 7-day discovery
window was an implementation cost optimization (TASK-029 C3, justified for *name-enumeration cost only*)
that silently narrowed the mappable set to a subset. **This change realizes the plan more completely; it
does not diverge from it.** → no `feedback_to_ba[]`, no FAIL-DESIGN.

## 3. Component boundaries (no crossing, no split)

- **`project-env-mapping`** (`env_mapping/mod.rs`) — read-time surface change only (union of three
  additive tables it already reads). Stays Vire-authoritative (DEC-001), suggestion-first (DEC-006),
  read-time-join for evidence association (D3).
- **Langfuse discovery** (`langfuse/mod.rs` + `discovery.rs`) — widens the look-back window to the
  already-resolved import-range floor; same allowlist, loopback gate, name-only output, `MAX_PAGES`
  bound.

Both live inside the existing `PROJECT_MAPPING` capability and the Langfuse-importer component. The fix
touches two files but **one** architectural capability and **one** user-visible behavior — normal for a
bugfix, not a boundary crossing. No new egress/host/endpoint/dependency/capability. No schema migration
(reads existing additive tables). The IPC command `list_discovered_environments` and the
`DiscoveredEnvState` shape are unchanged — only the row set grows.

## 4. NFR / security implications

- **SEC-010 (no secrets on these surfaces):** every value remains an environment name, a project ref,
  or mapping state. `langfuse_ai_evidence.environment` is a non-secret label; the raw payload
  (potential prompt/session content) lives in `langfuse_raw_traces` and is never read by this path.
- **Performance:** the union is over small local tables (one row per environment, not per trace);
  bounded and cheap. Fix B issues the same allowlisted reads discovery already makes, over a wider date
  window, still `MAX_PAGES`-bounded.
- **Correctness/absence-≠-zero:** unaffected — the fix changes which environments are *listed*, not any
  token/cost/health value.

## 5. Preserved prior-task contracts

- **TASK-031** scroll preservation — data-only change; `shell()`/`nextScrollTop` rerender untouched.
- **TASK-030** create-and-map — `mappingRow` inline input + "Create & map" preserved for every new row;
  no `window.prompt`.
- **TASK-034** suggestions — engine/clustering/cost/normalization unchanged; fix lets Settings act on
  the unmapped evidence the engine already reports, via the existing read-time join (no evidence
  rewrite).
- **TASK-044** credential storage — no credential/Keychain/`resolve_config` path touched.

## 6. Decision to record — DEC-038 (proposed; BA-owned entry)

The mapping/discovery surface contract is clarified from *"environments discovered by the recent scan"*
to *"every environment that needs a project mapping = discovered ∪ has-evidence ∪ already-mapped,"* with
discovery's look-back aligned to the configured import range. Next free number is **DEC-038** (repo max
is DEC-037). This is a realization of DEC-004 and `03_architecture_plan.md:141,175`, traceable to
DEC-001/DEC-006/DEC-028. SW cannot write `artifacts/ba/07_decision_log.md`; Pi-Assistant should route
the DEC-038 entry to BA. **Non-blocking** for implementation.

## 7. Recommended routing

Backend developer (Rust) — the change is entirely in `src-tauri/src/env_mapping/mod.rs` and
`src-tauri/src/langfuse/mod.rs` plus their unit tests. No frontend change is required (the renderer and
IPC contract are unchanged); frontend verification is regression-only (TASK-030/031 affordances).
