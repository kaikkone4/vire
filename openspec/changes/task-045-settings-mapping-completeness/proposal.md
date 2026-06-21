# TASK-045 — Settings shows every mappable Langfuse environment, not just last-7-day discoveries

## Why

After PR #32 (TASK-044) Janne ran the local desktop app end-to-end. The Settings
**Environment → project mapping** panel showed **only 8 entries**, yet his local Langfuse holds
more environments/trace sources than that. The environments that are missing from the panel can
never be mapped to a Vire project, so their imported AI evidence stays permanently **unmapped → not
trackable → produces no suggestions**. The Suggestions view even displays an "Unmapped AI evidence —
Map it in Settings to get suggestions" notice that points at Settings, but Settings has no row for
those environments. The loop is a dead end.

### Root cause (confirmed by reading the code path)

The mapping panel is rendered from exactly one source:

```
mappingPanel(discoveredEnvs, projects)                    src/main.ts:73
  ← list_discovered_environments  (IPC)                   src-tauri/src/lib.rs:910
  ← env_mapping::list_discovered_environments_repo        src-tauri/src/env_mapping/mod.rs:178
  ← langfuse::store::list_discovered_environments         src-tauri/src/langfuse/store.rs:306
  ← table `langfuse_discovered_environments`              src-tauri/src/langfuse/store.rs:53
```

That table is populated by **one** writer — `discover_and_record` — which scans a **fixed 7-day
window** regardless of the configured import range:

```rust
const DISCOVERY_WINDOW_DAYS: i64 = 7;                      src-tauri/src/langfuse/mod.rs:27
discover_and_record(&api, &conn, &recent_window(DISCOVERY_WINDOW_DAYS));   mod.rs:172
```

The configured import range defaults to **last 30 days** and can be `last_90d`, `all`, or a custom
`since:` floor (`ImportRange`, `mod.rs:33`). A **backfill** imports across that whole range and writes
AI evidence for every environment it sees into `langfuse_ai_evidence`. But discovery only ever looks
at the last **7** days. So any environment whose traces fall entirely outside the last 7 days —
exactly what a 30/90/all backfill surfaces — is **imported but never discovered**, and therefore
**never gets a mapping row**.

Two surfaces consequently disagree about which environments exist:

| Surface | Environment universe | Source |
| --- | --- | --- |
| Suggestions "Unmapped AI evidence" notice | every environment with imported evidence | `langfuse_ai_evidence` (`suggestions/engine.rs:195` `unmapped_summary`) |
| Settings mapping panel | only environments seen in the last 7 days | `langfuse_discovered_environments` (7-day scan) |

So Suggestions can report "N traces in unmapped env X" while Settings offers no way to map `X`.
"Only 8 mapping entries" = the 8 environments active in the last 7 days; the rest of the backfilled
history is invisible and unmappable.

There is **no artificial cap** — no `LIMIT 8`, no truncation in the SQL or the renderer. "8" is just
the count of rows that the 7-day discovery scan happened to write. The defect is **coverage**, not a
limit.

## What changes

A small, two-part bugfix inside the **existing** `project-env-mapping` capability and Langfuse
discovery — no new component, no schema migration, no IPC contract change.

1. **Primary — the mapping surface enumerates every environment that needs mapping (read-time
   completeness).** `list_discovered_environments_repo` returns the **union** of:
   - discovered environments (`langfuse_discovered_environments`),
   - **environments that have imported AI evidence** (`langfuse_ai_evidence`) — the load-bearing
     addition; it guarantees every imported trace source is mappable, and it fixes Janne's current
     database **with no re-import**,
   - environments that already have a mapping (`langfuse_env_project_map`) — so a mapping stays
     visible (with its Clear action) even after its environment ages out of the 7-day scan.

   Each environment keeps its `mapped` / `project_id` / `project_name` join and a best-known
   `last_seen`. Pure read over existing additive tables; the IPC name and `DiscoveredEnvState` shape
   are unchanged — only the row set grows.

2. **Secondary — discovery look-back follows the configured import range (forward completeness).**
   `discover_and_record` scans from the resolved **import-range floor** (the same floor the import
   just used) instead of a hard 7-day window, so a 90d/all backfill discovers environments across the
   whole span. This keeps `langfuse_discovered_environments` accurate going forward and stops the
   two-surface divergence from recurring. Discovery stays read-only, name-only, and `MAX_PAGES`-bounded.

Part 1 alone fully resolves the reported bug (retroactively, no re-import). Part 2 prevents
recurrence and keeps the environment **picker** complete.

## What does NOT change (preserved behavior)

- **TASK-031 scroll preservation** — only the *data* feeding `mappingPanel` changes; the `shell()` /
  `nextScrollTop` rerender path is untouched.
- **TASK-030 create-and-map UX** — the per-row inline name input + "Create & map" button is preserved
  for every newly-visible environment; no `window.prompt()` is reintroduced.
- **TASK-034 suggestions behavior** — the engine, 30-minute clustering, same-minute normalization, and
  AI-cost reporting are unchanged. The fix only lets Settings *act on* what Suggestions already
  reports, closing the "Map in Settings" loop via the existing read-time join (no evidence rewrite).
- **TASK-044 credential storage** — no credential, Keychain, or `resolve_config` path is touched. All
  surfaces stay environment-names-and-project-refs only (SEC-010).

## Scope / impact

- **Type:** bugfix (data-completeness). **Tier:** L1-equivalent — no new backend egress, no new
  dependency, no schema migration, no IPC contract change.
- **Capabilities touched:** `project-env-mapping` (MODIFIED), `langfuse-importer` (MODIFIED, discovery
  look-back).
- **Out of scope:** any change to suggestion clustering/cost, credential storage, the import engine's
  trace/dedup/cursor logic, or the network allowlist.
