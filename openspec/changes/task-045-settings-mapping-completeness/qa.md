# QA Report — TASK-045 settings-mapping-completeness

**Gate**: SW-3  **Tier**: L1  **Result**: PASS  **Date**: 2026-06-21

---

## Run results

| Check | Result | Notes |
|---|---|---|
| `cargo test --lib` | ✓ 182 passed, 0 failed | |
| `cargo fmt --check` | ✓ | |
| `cargo clippy --lib` | ✓ no new findings | 3 pre-existing warnings in `importer.rs`/`lib.rs` (untouched files) |
| `npm run build` | ✓ tsc + vite clean | |
| `openspec validate --changes` | ✓ task-045 passes | 5 other changes fail for unrelated pre-existing reasons |
| UI/IPC contract diff | ✓ no change | `src/env-mapping-ui.ts`, `src/main.ts` unchanged |

---

## Scenario coverage matrix

### Spec: project-env-mapping

| Scenario | Test | Status |
|---|---|---|
| User maps an environment to a project | `set_persists_a_mapping_and_list_reads_it_back`, `evidence_is_associated_to_a_project_at_read_time_without_rewrite` | PASS |
| An imported environment can be mapped even if not recently discovered | `evidence_only_environment_appears_unmapped_without_rediscovery` (C1) | PASS |
| A mapped environment stays visible after it ages out of discovery | `mapped_environment_stays_visible_after_aging_out_of_discovery` (C2) | PASS |
| Each environment appears once | `environment_in_all_three_sources_yields_exactly_one_row_with_correct_join` (C3) | PASS |
| Changing a mapping does not destroy evidence | `evidence_is_associated_to_a_project_at_read_time_without_rewrite` | PASS |
| Unmapped environment offers a create-project suggestion | `discovered_unmapped_suggests_create_then_explicit_action_maps_it` | PASS |
| Accepting the suggestion creates and maps in one step | `discovered_unmapped_suggests_create_then_explicit_action_maps_it` | PASS |
| Mapping data carries no secrets (SEC-010) | `mapping_surfaces_carry_no_secrets` | PASS |

### Spec: langfuse-importer

| Scenario | Test | Status |
|---|---|---|
| Discovery covers the configured import range | `discovery_window_floor_equals_the_resolved_import_range_floor` (C5-A) | PASS |
| Discovered environments are offered for selection | `discovery_collects_distinct_non_empty_environments_across_pages`, `discovered_environments_persist_additively_with_last_seen` | PASS |
| Discovery stays bounded for a wide range | `discovery_is_bounded_by_max_pages_so_an_all_floor_cannot_spin` (C5-B) | PASS |
| Discovery preserves the network boundary | `discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param` | PASS |

---

## Additional verifications

| Check | Result |
|---|---|
| Evidence-only env surfaces without re-import (Janne's bug) | PASS — C1 asserts unmapped env appears with evidence `last_seen`, no re-import |
| Mapped-only env stays in list (no drop after aging out) | PASS — C2 asserts row present with empty `last_seen` |
| No duplicate rows when env in all three sources | PASS — C3 asserts `len() == 1`, correct join |
| `last_seen` precedence: discovered > evidence end > evidence start > empty | PASS — C4 (`last_seen_fallback_chain_is_sorted_and_drops_no_row`) |
| Discovery window floor == import range floor for all range variants | PASS — C5-A covers `Last7d`, `Last30d`, `Last90d`, `All`, `Since` |
| `all` range discovery bounded by `MAX_PAGES`, no spin | PASS — C5-B with infinite-source mock |
| No credential/keychain drift (`resolve_config` path untouched) | PASS — `import_run_table_has_no_credential_columns`, `secret_and_credentials_never_render_their_values` (pre-existing) |
| Suggestions engine untouched | PASS — `suggestions::tests::*` (12 tests, all pass) |
| Project join/mapped status preserved | PASS — `set_persists_a_mapping_and_list_reads_it_back`, C3 |
| `DiscoveredEnvState` JSON shape unchanged (IPC contract) | PASS — struct fields identical, no renderer change |
| `discovery.rs` change is visibility-only (`MAX_PAGES` pub) | PASS — diff confirmed no behavioral change |

---

## Changed paths

- `src-tauri/src/env_mapping/mod.rs` — Fix A: union surface (discovered ∪ evidence ∪ mapped)
- `src-tauri/src/env_mapping/tests.rs` — C1–C4 new tests + SEC-010 UUID false-positive hardening
- `src-tauri/src/langfuse/mod.rs` — Fix B: `discovery_window` replaces `recent_window`; `DISCOVERY_WINDOW_DAYS` removed
- `src-tauri/src/langfuse/discovery.rs` — `MAX_PAGES` visibility `pub` only
- `src-tauri/src/langfuse/tests.rs` — C5-A, C5-B new tests

---

## Blockers

None.
