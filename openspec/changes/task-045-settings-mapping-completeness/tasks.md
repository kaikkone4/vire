# Tasks — TASK-045 Settings mapping completeness

> Bugfix slice. No schema migration, no IPC contract change, no new egress/deps. Plan only — code is
> implemented by the routed developer role.

## A — Backend: union the mapping surface (PRIMARY)

- [x] **A1.** In `src-tauri/src/env_mapping/mod.rs`, change `list_discovered_environments_repo` so its
      environment universe is the **union** of:
      `langfuse_discovered_environments` ∪ distinct `langfuse_ai_evidence.environment`
      ∪ distinct `langfuse_env_project_map.environment`.
- [x] **A2.** For each environment in the union, keep the existing `mapped` / `project_id` /
      `project_name` join against `langfuse_env_project_map ⋈ projects`.
- [x] **A3.** Resolve `last_seen` as best-known: `langfuse_discovered_environments.last_seen` when
      present, else derived from evidence (`MAX(ai_end_ts, ai_start_ts)`), else empty string. Never drop
      a row for a missing `last_seen`.
- [x] **A4.** De-duplicate by environment and sort by environment (preserve today's deterministic order).
- [x] **A5.** Keep the return type `Vec<DiscoveredEnvState>` and the field set unchanged — no IPC/TS
      contract change.

## B — Backend: discovery look-back follows the import range (SECONDARY)

- [x] **B1.** In `src-tauri/src/langfuse/mod.rs` `run_blocking`, replace
      `discover_and_record(&api, &conn, &recent_window(DISCOVERY_WINDOW_DAYS))` with a window
      `{ from: range_floor, to: now }` using the already-resolved `range_floor`/`now`.
- [x] **B2.** Confirm discovery stays read-only, name-only, allowlist + loopback-gated, and
      `MAX_PAGES`-bounded (so an `all` floor cannot spin). No change to `discovery.rs` logic required.
- [x] **B3.** Remove `recent_window` / `DISCOVERY_WINDOW_DAYS` only if no longer referenced after B1;
      otherwise leave intact (no gratuitous edits).
      _(Both were unreferenced after B1 → removed; `recent_window` replaced by the `discovery_window`
      helper that takes the resolved range floor.)_

## C — Tests (Rust)

- [x] **C1.** `env_mapping` test: evidence-only environment (no discovery row, no mapping) appears in
      `list_discovered_environments_repo` with `mapped == false`.
- [x] **C2.** `env_mapping` test: already-mapped environment absent from discovery + evidence still
      appears, `mapped == true` with correct project ref.
- [x] **C3.** `env_mapping` test: environment present in all three sources yields exactly one row.
- [x] **C4.** `env_mapping` test: `last_seen` fallback chain (discovered → evidence → empty) and row is
      never dropped.
- [x] **C5.** Importer/discovery test (or `mod.rs` test): discovery window floor equals the resolved
      import-range floor; an `all` range is bounded by `MAX_PAGES`.

## D — Regression verification (no behavior change in preserved areas)

- [x] **D1.** TASK-031: changing/clearing/creating a mapping rerenders Settings without resetting scroll
      (`shell()`/`nextScrollTop` path unchanged) — no frontend/scroll file touched by the diff.
- [x] **D2.** TASK-030: every newly-visible row exposes the inline name input + "Create & map"; no
      `window.prompt` introduced (`grep -n "prompt(" src/` stays empty).
- [x] **D3.** TASK-034: suggestion clustering/cost/same-minute normalization unchanged; after mapping a
      previously-unmapped (evidence-only) environment, the next suggestion recompute associates its
      evidence via the read-time join — no evidence row rewritten (`list_evidence_projects_repo`
      untouched).
- [x] **D4.** TASK-044: no credential/Keychain/`resolve_config` path touched; all surfaces secret-free
      (SEC-010).

## E — Build & gates

- [x] **E1.** `cargo test --lib` green (182 passed); `cargo fmt --check` clean; Clippy introduces no new
      findings (the only remaining warnings predate this change, in `importer.rs`/`lib.rs`/pre-existing
      `MockApi::default()` reassignments).
- [x] **E2.** `npm run build` green; focused frontend tests pass except two **pre-existing,
      network-dependent** failures in `tests/pi-observe.security.test.mjs` (reproduced on clean `main`,
      unrelated to this backend change).
- [x] **E3.** `openspec validate task-045-settings-mapping-completeness --strict` passes.
- [x] **E4.** `git diff --check` clean.

## F — Decision log (BA-owned; non-blocking)

- [ ] **F1.** Pi-Assistant routes the proposed **DEC-038** (mapping surface = discovered ∪ has-evidence
      ∪ mapped) to BA for entry in `artifacts/ba/07_decision_log.md`. SW does not write BA artifacts.
