<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste. -->

# Handoff — TASK-045 settings-mapping-completeness

- **Change dir**: `openspec/changes/task-045-settings-mapping-completeness/`
- **Branch / PR**: `feat/task-045-settings-mapping-completeness` → draft PR (link in SW-2 report)
- **Phase / gate**: SW-2 Implementation **DONE** → next **SW-3 QA**
- **Tier**: L1-equivalent — no schema migration, no IPC contract change, no new egress/deps

## What changed (A+B + tests, backend-only)

- **A** `env_mapping/mod.rs::list_discovered_environments_repo` — universe is the **union** of
  discovered ∪ distinct `langfuse_ai_evidence.environment` ∪ distinct
  `langfuse_env_project_map.environment` (BTreeMap → sorted, de-duped). `last_seen` precedence
  discovered → evidence(`MAX(ai_end_ts)`→`MAX(ai_start_ts)`) → empty; never drops a row. Shape
  `Vec<DiscoveredEnvState>` unchanged. Fixes Janne's DB with **no re-import**.
- **B** `langfuse/mod.rs::run_blocking` — discovery look-back uses `discovery_window(range_floor, now)`
  not `recent_window(7)`. Removed unused `recent_window`/`DISCOVERY_WINDOW_DAYS`; `discovery::MAX_PAGES`
  made `pub` (visibility only) for the bound test.
- **Tests**: `env_mapping/tests.rs` C1–C4; `langfuse/tests.rs` C5 (window floor == range floor; `all`
  bounded by `MAX_PAGES`). Hardened the SEC-010 test vs a UUID/`579` false positive.

## Gates

- `cargo test --lib` 182 ✓ · `fmt --check` ✓ · clippy **no new findings** ✓ · `cargo check` ✓
- `npm run build` ✓ · `openspec validate --strict` ✓ · `git diff --check` ✓
- `test:frontend`: 2 **pre-existing network failures** in `tests/pi-observe.security.test.mjs`
  (reproduced on clean `main` — NOT this change).

## Next / notes

- **SW-3 QA**: re-run D1–D4; focus C1–C5. Renderer/IPC untouched.
- **DEC-038** BA-owned (surface = discovered ∪ has-evidence ∪ mapped) → `07_decision_log.md`;
  SW can't write BA artifacts. Non-blocking. See `design.md` §7.
