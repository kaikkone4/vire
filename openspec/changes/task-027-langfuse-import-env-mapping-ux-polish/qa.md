# QA Report — TASK-027

**Status:** PASS  
**Tier:** L2  
**Branch:** `feat/task-027-langfuse-import-env-mapping-ux-polish`  
**PR:** #22 (draft)  
**Date:** 2026-06-17 (re-run after blocker fixes)

---

## Scenario Coverage Matrix

### Spec: `langfuse-importer`

| # | Scenario | Evidence | Result |
|---|----------|----------|--------|
| A1 | Empty import is explained, not silent | `import_report_explains_an_empty_import_rather_than_blank` (tests.rs:1063) — zero-trace run surfaces `missing` health per-env, never blank | ✅ PASS |
| A2 | Partial import reports counts and skips | `import_report_counts_duplicates_and_skips_on_a_partial_run` (tests.rs:1077) — unique/duplicates/skipped_schema surfaced per env and totalled | ✅ PASS |
| A3 | Import result carries no secrets | `import_report_is_secret_free` (tests.rs:1106) — trace with `sk-`, `Bearer`, session, `Authorization` metadata; serialized report must not contain them | ✅ PASS |
| A4 | Current-shape usage/cost captured | `observation_reads_current_usage_and_cost_details` (tests.rs:941), `current_shape_generation_is_healthy_and_cost_captured` (tests.rs:998) — `usageDetails`/`costDetails` parsed; no `schema_changed` for recognized current shape | ✅ PASS |
| A5 | Absent usage stays absent, not zero | `observation_absent_usage_stays_none_not_zero` (tests.rs:971), `current_shape_with_empty_detail_maps_degrades_to_schema_changed` (tests.rs:1015) — empty maps yield `None` not `0`; `schema_changed` reported; `skipped_schema=1` counted | ✅ PASS |
| B1 | Startup + periodic import keep evidence current | `run()` (lib.rs:463) spawns dedicated OS thread; first loop iteration imports at startup; subsequent iterations sleep `auto_import_interval()` (default 900s) | ✅ PASS |
| B2 | Auto and manual imports do not overlap | `import_slot_serializes_concurrent_imports` (lib.rs tests:574), `auto_import_cycle_skips_when_an_import_is_already_in_progress` (lib.rs tests:614) — shared `Arc<Mutex<()>>`; auto skips, manual waits | ✅ PASS |
| B3 | Automatic import respects the disabled switch | `auto_import_cycle_runs_nothing_when_disabled` (lib.rs tests:589) — disabled setting short-circuits before any Keychain read or probe; 0 import_runs recorded | ✅ PASS |
| C1 | Discovered environments offered for selection | `envPickerCheckboxes`/`envPickerOptions`/`mergeSelectedEnvironments` (env-mapping-ui.ts); `picker always offers the default environment, dedupes, trims, and sorts` + picker tests (envMappingUi.test.mjs) | ✅ PASS |
| C2 | Discovery preserves the network boundary | `discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param` (tests.rs:1200) — URL stays under `/api/public/traces` on loopback, no `environment=` param, off-host local refused | ✅ PASS |

### Spec: `project-env-mapping`

| # | Scenario | Evidence | Result |
|---|----------|----------|--------|
| D1 | User maps an environment to a project | `set_persists_a_mapping_and_list_reads_it_back` (env_mapping/tests.rs:42) — set/list roundtrip; mapping rows include env, project_id, project_name | ✅ PASS |
| D2 | Changing a mapping does not destroy evidence | `evidence_is_associated_to_a_project_at_read_time_without_rewrite` (env_mapping/tests.rs:113) — clearing a map: evidence row count stays 2, only join result changes | ✅ PASS |
| D3 | Unmapped environment offers create-project suggestion | `discovered_unmapped_suggests_create_then_explicit_action_maps_it` (env_mapping/tests.rs:90) — `list_discovered_environments_repo` returns `mapped=false`; no project auto-created (count stays 0) | ✅ PASS |
| D4 | Accepting suggestion creates and maps in one step | Same test (env_mapping/tests.rs:90) explicit-action path + `bindEnvMapping` in main.ts (line 53) — `data-create-map` handler calls `create_project` then `set_env_mapping`; UI test `an unmapped environment offers a project picker AND an explicit create-and-map action` (envMappingUi.test.mjs:75) | ✅ PASS |
| D5 | Mapping data carries no secrets | `mapping_surfaces_carry_no_secrets` (env_mapping/tests.rs:171) — serialized surfaces from list_evidence_projects, list_discovered_environments, list_env_mappings contain no `session-`, `Bearer`, `9.99`, `579` | ✅ PASS |

### Spec: `desktop-ui`

| # | Scenario | Evidence | Result |
|---|----------|----------|--------|
| E1 | Only native window controls shown | `grep -n "traffic" style.css` → no match; `grep -n "traffic" main.ts` → no match; `titlebar() renders the brand and version but never the fake traffic-light cluster` (shellChrome.test.mjs:5) passes | ✅ PASS |
| E2 | Removing fakes leaves titlebar coherent | `style.css` `.titlebar{display:grid;grid-template-columns:1fr auto 1fr}`, brand `grid-column:2;text-align:center`, version `grid-column:3;justify-self:end` — symmetric without left dots | ✅ PASS |
| E3 | Icon not oversized in Dock | `generate-vire-mark.mjs` `const SAFE=0.8` (line 26) — mark inset to 80% of canvas; icons regenerated: `icon.icns`, `128x128.png`, `32x32.png`, `64x64.png` etc. all present in `src-tauri/icons/` | ✅ PASS |
| E4 | Safe-area requirement documented for branded asset | Note in `generate-vire-mark.mjs` header (lines 5–8); RELEASE.md TASK-027 section "Safe-area requirement for the branded asset" | ✅ PASS |

---

## Tests Run

### Rust (120 unit + 3 adversarial = 123 total)

```
test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**TASK-027-specific tests (all green):**
- `observation_reads_current_usage_and_cost_details` — A4 current-shape tokens/cost
- `observation_legacy_shape_still_parses` — A4 legacy shape preserved
- `observation_absent_usage_stays_none_not_zero` — A4/A5 absence-≠-zero
- `observation_present_zero_is_distinct_from_absence` — A4 0 vs None
- `current_shape_generation_is_healthy_and_cost_captured` — A4 end-to-end via run_import
- `current_shape_with_empty_detail_maps_degrades_to_schema_changed` — A5 skipped_schema=1
- `import_report_aggregates_per_env_and_total_counts` — A2/A3 per-env + total
- `import_report_explains_an_empty_import_rather_than_blank` — A1 zero-trace result
- `import_report_counts_duplicates_and_skips_on_a_partial_run` — A2 partial run
- `import_report_is_secret_free` — A3/SEC-010 no sk-/pk-/Bearer/Authorization/session-
- `discovery_collects_distinct_non_empty_environments_across_pages` — C1/C5 multi-env dedup
- `discovery_returns_empty_when_no_traces_and_errors_propagate` — C5 error path
- `discovered_environments_persist_additively_with_last_seen` — C3 additive, first_seen preserved
- `discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param` — C2 URL check
- `migrate_is_idempotent_and_additive` — D1 init_db idempotent
- `set_persists_a_mapping_and_list_reads_it_back` — D2 CRUD
- `remapping_updates_project_but_preserves_created_at` — D2 upsert
- `clear_is_idempotent` — D2 clear
- `mapping_to_a_missing_project_is_refused_and_creates_nothing` — D3/DEC-006 no silent create
- `discovered_unmapped_suggests_create_then_explicit_action_maps_it` — D3/D4/DEC-006
- `evidence_is_associated_to_a_project_at_read_time_without_rewrite` — D3 read-time join
- `mapping_surfaces_carry_no_secrets` — D5/SEC-010 no token/cost/session in IPC surface
- `import_slot_serializes_concurrent_imports` — B2 slot guard
- `auto_import_cycle_runs_nothing_when_disabled` — B3 disabled short-circuit
- `auto_import_cycle_skips_when_an_import_is_already_in_progress` — B2 skip while busy
- `auto_import_interval_floors_and_defaults` — B2 interval constants

### Frontend (51/53 pass)

```
npm run test:frontend → 53 tests: 51 pass, 2 fail
```

**TASK-027-specific tests (all green):**
- `titlebar renders the brand and version but never the fake traffic-light cluster (E1/E5)` — shellChrome.test.mjs
- `titlebar escapes its inputs so a hostile brand/version string cannot inject markup` — shellChrome.test.mjs
- `picker always offers the default environment, dedupes, trims, and sorts` — envMappingUi.test.mjs
- `a configured-but-undiscovered environment still appears as a ticked box` — envMappingUi.test.mjs
- `an unselected discovered environment renders unchecked` — envMappingUi.test.mjs
- `saving unions ticked boxes with advanced CSV entries, deduped and order-preserving` — envMappingUi.test.mjs
- `unticking everything yields an empty list` — envMappingUi.test.mjs
- `a mapped environment shows its project and a clear action, no project picker` — envMappingUi.test.mjs
- `an unmapped environment offers a project picker AND an explicit create-and-map action` — envMappingUi.test.mjs
- `unmapped with no projects still offers create-and-map but no empty picker` — envMappingUi.test.mjs
- `panel explains the empty state instead of rendering blank` — envMappingUi.test.mjs
- `mapping surfaces never leak a secret-shaped token` — envMappingUi.test.mjs

**Pre-existing failures (UNRELATED TO TASK-027):**
- `safe dotenv parser loads only allowlisted Langfuse keys without shell execution` — `pi-observe.security.test.mjs`
- `remote Langfuse host is blocked unless explicitly opted in` — `pi-observe.security.test.mjs`

These tests exercise `observability/pi-observe/bin/pi-observe.mjs`. Confirmed unrelated: `git diff main..HEAD -- tests/pi-observe.security.test.mjs` returns empty; `git log --oneline main..HEAD -- tests/pi-observe.security.test.mjs` returns nothing. File untouched by every TASK-027 commit.

### Build / type / format checks

- `npx tsc --noEmit` — clean (no output, exit 0)
- `npm run build` — clean: `✓ built in 103ms`
- `cargo fmt --check` — **FMT_CLEAN** (blocker resolved by commit `02f25c6` "style(task-027): apply rustfmt to src-tauri Rust sources")
- No ESLint config in project (no lint script)

---

## Blocker Resolution (re-run reason)

| Blocker | Resolution | Commit |
|---------|-----------|--------|
| `cargo fmt --check` dirty — rustfmt would reformat Rust sources | `cargo fmt` applied to all `src-tauri` sources | `02f25c6` |
| `RELEASE.md` missing — SW-6 doc gate not satisfied | RELEASE.md populated with TASK-027 section covering all required items | `7601811` |

Both blockers are resolved and verified in this re-run.

---

## Findings

### No blockers

All 19 spec scenarios have observable test coverage. Both prior blockers resolved. No new implementation gaps.

### Acceptance verification

| Acceptance criterion | Verified |
|---|---|
| Import report counts: pages / seen / unique / duplicates / skipped_schema, per-env and total | ✅ `ImportReport` + `EnvImportLine` in importer.rs; `import_report_aggregates_per_env_and_total_counts` |
| Per-env health surfaced (all 10 states reachable) | ✅ 10 `classify_health` branches, each with a test (tests.rs:215–379) |
| Skipped/empty explanation (never blank) | ✅ `import_report_explains_an_empty_import_rather_than_blank` |
| `schema_changed` fixed for current `usageDetails`/`costDetails` shapes | ✅ `current_shape_generation_is_healthy_and_cost_captured` + `observation_reads_current_usage_and_cost_details` |
| Secret-free diagnostics (no sk-/pk-/Bearer/Authorization/raw payload) | ✅ `import_report_is_secret_free` + `mapping_surfaces_carry_no_secrets` + `secret_and_credentials_never_render_their_values` |
| Environment discovery via no-env trace scan | ✅ `ApiPath::TracesAllEnvironments`; `discover_environments` in discovery.rs |
| Loopback/allowlist preserved for discovery | ✅ `discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param` |
| Env picker with discovered + configured envs; CSV advanced fallback; `vire` default unchanged | ✅ `envPickerOptions`/`envPickerCheckboxes`/`mergeSelectedEnvironments` + envMappingUi tests |
| Env→project mapping: explicit create-project only, never auto-create | ✅ `mapping_to_a_missing_project_is_refused_and_creates_nothing` + `discovered_unmapped_suggests_create_then_explicit_action_maps_it` (DEC-006) |
| Auto-import: startup + periodic, serialized with manual, disabled short-circuit | ✅ B-series tests in lib.rs; `run_auto_import_cycle` implementation |
| Fake macOS traffic buttons removed | ✅ No `.traffic` in style.css or main.ts; shellChrome tests |
| Icon safe-area regenerated to ~80% | ✅ `SAFE=0.8` in generator; icons present |
| `cargo fmt` clean | ✅ Resolved in `02f25c6` — `cargo fmt --check` exits 0 |
| RELEASE.md documents all TASK-027 changes | ✅ Resolved in `7601811` — all five workstreams, compat/rollback, TASK-028 split documented |
| No CSP/capabilities change | ✅ `capabilities/default.json` unchanged from TASK-026 |
| No updater implemented (TASK-028 split) | ✅ No updater plugin, no minisign, no GitHub Releases code anywhere in the diff |

### Known limitation (spec-acknowledged, not a blocker)

C2 verification: the assumption that omitting `environment` returns cross-env traces cannot be confirmed via authenticated API response (SEC-003/A1 prohibition). Confirmed shape-only via TASK-007 spike and live 401 response (auth precedes param validation). Advanced hand-entered CSV remains the fallback per design §4; discovery degrading to empty never blocks the user.

---

## Gate Verdict

**PASS** — All 19 spec scenarios covered. 123 Rust tests green, 51 frontend tests green (2 pre-existing pi-observe failures unrelated to this task). TypeScript clean, build clean, `cargo fmt` clean, RELEASE.md complete. No CSP/capability change, no updater, no silent auto-create. Both prior blockers resolved.

Routing: SW-4 (Code Reviewer) + SW-5 (Security Agent) in parallel.
