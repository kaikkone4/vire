# Design — TASK-045 Settings mapping completeness

## 1. Problem, precisely

The set of environments the user **can map** must equal the set of environments that **need mapping**.
Today it does not:

- *Needs mapping* = every environment that has (or could have) imported AI evidence, i.e. every
  distinct `environment` in `langfuse_ai_evidence`, plus any already-mapped environment, plus the
  configured/discovered ones.
- *Can map* = only the rows of `langfuse_discovered_environments`, which is written solely by a
  **7-day** discovery scan (`DISCOVERY_WINDOW_DAYS = 7`, `langfuse/mod.rs:27,172`).

A backfill imports across the configured range (default 30d; up to `all`), so evidence accumulates for
environments far older than 7 days. Those environments are imported but undiscovered, so they get no
mapping row. Result: "only 8 entries"; the rest are untrackable.

This is an architecture-plan realization gap, not a plan change: `03_architecture_plan.md:141` requires
"map traces to Vire projects by environment first," and `:175` models `PROJECT ||--o{ PROJECT_MAPPING`.
The plan mandates that *trace sources* be mappable; the 7-day discovery window (an implementation choice
from TASK-029 C3, justified only for *name enumeration cost*) silently narrowed that to a subset.

## 2. Fix A (primary) — union the mapping surface at read time

`env_mapping::list_discovered_environments_repo` (`env_mapping/mod.rs:178`) becomes a union over three
existing **additive** tables, all already in the same DB, all secret-free:

| Source table | Why it belongs in the surface |
| --- | --- |
| `langfuse_discovered_environments` | discovered names (today's only source) |
| `langfuse_ai_evidence` (distinct `environment`) | **the fix** — any imported trace source must be mappable; covers all backfilled history with no re-import |
| `langfuse_env_project_map` (distinct `environment`) | a mapped environment must stay visible (with its Clear action) even after it ages out of the 7-day scan; today a mapped-but-undiscovered env silently drops off the panel |

### 2.1 Shape and join (unchanged contract)

Output stays `Vec<DiscoveredEnvState>` — `{ environment, last_seen, mapped, project_id, project_name }`
(`env_mapping/mod.rs:57`). The IPC command `list_discovered_environments` and the TS type
`DiscoveredEnvState` (`src/env-mapping-ui.ts:10`) are **unchanged**; only the row count grows. For each
environment in the union:

- `mapped` / `project_id` / `project_name` — the same `langfuse_env_project_map ⋈ projects` lookup the
  current code already does per environment (`mod.rs:182`).
- `last_seen` — best-known timestamp: the `langfuse_discovered_environments.last_seen` when present;
  otherwise derived from evidence (e.g. `MAX(ai_end_ts, ai_start_ts)` for that environment); otherwise
  an empty string. `last_seen` is display-only metadata (`mappingRow` does not render it today), so an
  empty value is safe and never blocks a row from rendering.

Implementation note (non-binding): a single SQL `SELECT environment, MAX(last_seen) FROM ( … UNION ALL
… ) GROUP BY environment ORDER BY environment`, then the existing per-env mapping lookup — or one joined
query. De-duplicated and sorted by environment for a deterministic render (matches today's `ORDER BY
environment`).

### 2.2 Why this fixes Janne's box with no re-import

His evidence rows already exist in `langfuse_ai_evidence` from the backfill. The union reads them
directly, so every previously-imported environment gets a row the next time Settings renders — no
re-import, no migration, no data change.

## 3. Fix B (secondary) — discovery look-back follows the import range

In `run_blocking` (`langfuse/mod.rs:138`) the import already resolves the range floor:

```rust
let range = crate::settings::resolve_import_range(&conn);
let range_floor = range.floor(now_dt);             // mod.rs:151–152
```

Replace `discover_and_record(&api, &conn, &recent_window(DISCOVERY_WINDOW_DAYS))` with a window
`{ from: range_floor, to: now }` — the same floor the import just used. Discovery still:

- enumerates **names only** (`Trace.environment`), never trace content;
- stays under the `/api/public/` allowlist + loopback gate (`discovery.rs:6`);
- is bounded by `MAX_PAGES` (`discovery.rs:18`) so an `all` floor cannot spin forever — it degrades to
  "discovered as many as the backstop allowed," never wrong data.

This keeps `langfuse_discovered_environments` accurate for the environment **picker**
(`envPickerCheckboxes`, `src/main.ts:79`) and prevents the divergence from recurring. `recent_window`
and `DISCOVERY_WINDOW_DAYS` are removed only if no longer referenced; otherwise left intact.

### 3.1 Why both A and B (not just one)

- **A alone** fixes the reported symptom and the existing DB, but `langfuse_discovered_environments`
  (which also feeds the import **picker**) would stay 7-day-limited — a never-imported but
  recently-active environment 8–30 days old still wouldn't appear as a *tickable import target* until
  imported. B closes that.
- **B alone** fixes discovery going forward but does **not** retroactively surface Janne's already-
  imported environments without a fresh backfill, and still leaves the panel blind to an environment
  whose mapping exists but whose traces aged past the range floor. A closes that.

Together: A guarantees completeness at read time (load-bearing for the bug); B keeps the discovered
table — and thus the picker — honest going forward.

## 4. Interaction with the environment picker (intended, benign)

The picker is seeded from the same list: `envPickerCheckboxes(discoveredEnvs.map(d => d.environment),
s.environments)` (`src/main.ts:79`). After Fix A the picker will also list evidence-only and
already-mapped environments. This is correct and desirable: those are real environments in the source,
and `envPickerOptions` already unions discovered + selected + default and de-duplicates
(`env-mapping-ui.ts:27`). A wrong-env `default` that received traces (imported as `WrongEnv`,
`importer.rs:1000`) will now also be offered — which is the right behavior (the user can map or import
it deliberately). No picker logic changes.

## 5. Preserved contracts (regression guards)

| Prior task | Contract | Why it holds |
| --- | --- | --- |
| TASK-031 | Settings scroll position survives a same-view rerender | Only the data behind `mappingPanel` changes; `shell()` + `nextScrollTop` (`src/scroll.ts`) and the `rerender()` path are untouched. |
| TASK-030 | Create-and-map uses an in-app inline input, never `window.prompt` | `mappingRow` (`env-mapping-ui.ts:79`) is unchanged; every new row gets the same inline `<input>` + "Create & map". |
| TASK-034 | Suggestion clustering / same-minute normalization / AI-cost reporting | Suggestions engine untouched; the fix only lets Settings act on the unmapped-evidence the engine already reports. Mapping a previously-unmapped env then associates evidence via the existing read-time join (D3) — no evidence row rewritten. |
| TASK-044 | Public key in SQLite, secret in Keychain; pair-level fallback | No credential, Keychain, or `resolve_config` path touched. All TASK-045 surfaces are names + project refs only (SEC-010). |

## 6. Security & privacy (SEC-010)

Every value crossing the IPC boundary remains an environment **name**, a project reference, or mapping
state. `langfuse_ai_evidence.environment` is a non-secret label (the raw payload — which may carry
prompt/session content — lives in `langfuse_raw_traces` and is never read here). No new egress, host,
endpoint, dependency, or capability. Fix B issues the same allowlisted, loopback-gated read requests
discovery already makes, only over a wider date window.

## 7. Decision to record (DEC-038)

The mapping/discovery surface's contract shifts from *"environments discovered by the recent scan"* to
*"every environment that needs a project mapping = discovered ∪ has-evidence ∪ already-mapped."*
Proposed **DEC-038** (next free number; BA owns the canonical entry in `07_decision_log.md`). This is a
realization of DEC-004 / `03_architecture_plan.md:141,175`, not a departure — see `arch-review.md`.

## 8. Test focus (for the implementer / QA)

- Union returns evidence-only environments that are absent from `langfuse_discovered_environments`.
- Union returns already-mapped environments absent from both discovery and evidence (mapping persists
  in the panel).
- De-dup: an environment present in all three sources yields exactly one row, mapped state correct.
- `last_seen` falls back gracefully (discovered → evidence → empty) and never drops a row.
- Sorted, deterministic order.
- Fix B: discovery window floor equals the resolved import-range floor; `all` is bounded by `MAX_PAGES`.
- Regression: `DiscoveredEnvState` JSON shape unchanged; secret-free (no payload/credential leakage).
