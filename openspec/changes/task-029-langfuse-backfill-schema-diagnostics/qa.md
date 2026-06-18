# QA Report — TASK-029 Langfuse Backfill + Schema Diagnostics

**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
**PR:** #23 (draft, base main)
**Tier:** L2
**Date:** 2026-06-18
**Verdict:** PASS

---

## Test Suite Results

### Rust (`cargo test --lib`)

**133 passed · 0 failed · 0 ignored** (finished in ~0.14 s)

Includes 60 langfuse tests (13 new for TASK-029) and 73 tests covering settings, runtime_observer,
and core app logic — all green.

### Frontend unit tests (`node --import tsx --test tests/*.test.mjs`)

**69 passed · 2 failed · 0 cancelled**

| File | Pass | Fail | Notes |
|------|------|------|-------|
| `importReport.test.mjs` | 11 | 0 | All D4 report tests — new |
| `langfuseSettings.test.mjs` | 10 | 0 | All range/settings tests — new |
| `envMappingUi.test.mjs` | — | — | passing (pre-existing) |
| `htmlEscape.test.mjs` | — | — | passing (pre-existing) |
| `pi-observe.security.test.mjs` | 8 | **2** | **PRE-EXISTING on `main`; unrelated to TASK-029** |
| Other (`date`, `forms`, `pi-observe.*`, `shellChrome`, etc.) | — | — | passing (pre-existing) |

**The 2 pi-observe.security failures are pre-existing on `main`** — verified by running the same
test file on `main` directly (0-byte diff, same 2 failures). They are in scope for a separate fix
but do not gate this task.

### Frontend build (`tsc && vite build`)

**PASS** — tsc emits 0 errors; vite outputs `dist/assets/index-*.js` 23.78 kB (gzip 8.24 kB).

### OpenSpec validate

`openspec validate --strict` returns "Unknown item" — the change is not registered as an active
openspec item in the CLI state. The spec artefacts (`design.md`, `tasks.md`, `proposal.md`,
`arch-review.md`, `specs/`) are present on disk. This is a tooling state issue only; it does not
affect the implementation.

---

## Scenario Coverage Matrix

### Workstream A — Secret-free skip taxonomy + grouped diagnostics

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| A1 | `SkipReason` enum (5 variants), `SkipReasonCount`, `SkipSample` types in `model.rs` | `skip_reasons_aggregate_per_env_and_in_total` | PASS |
| A2 | `SkipClassifier` replaces per-trace warning spam; `MAX_SAMPLES_PER_REASON=3` cap | `skip_samples_are_bounded_per_reason` | PASS |
| A3 | `skip_reasons`/`skip_samples` threaded onto `ImportSummary`/`EnvImportLine`; `total_skip_reasons` aggregated | `skip_reasons_aggregate_per_env_and_in_total` | PASS |
| A4 | VF-1 confirmed via deterministic test (live stack not reachable from agent) | `identifiable_trace_with_idlist_observations_is_imported` | PASS |
| A5 | SEC-011: serialised reasons/samples carry no `sk-`/`pk-`/`Bearer`/`Authorization`/session/`oat01`/raw field value | `skip_diagnostics_are_secret_free` | PASS |

### Workstream B — Tolerant v3 parser + identification decoupled from usage

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| B1 | Identification-first: `trace_id(value)` + fallback to `from_value_tolerant`; only `missing_trace_id` still drops | `peripheral_field_type_mismatch_still_imports_identifiable_trace`, `trace_with_no_id_is_missing_trace_id_counted_not_crashed` | PASS |
| B2 | `deserialize_tolerant_observations`: keeps embedded objects, silently drops ID strings → fetch fallback | `identifiable_trace_with_idlist_observations_is_imported` | PASS |
| B3 | Usage/cost from fetched observations for v3 list shape; `schema_changed` reserved for genuinely unreadable; absence ≠ 0 | `generation_with_no_usage_anywhere_is_schema_changed_absence_preserved` | PASS |
| B4 | VF-2 code-complete (live count is a Janne/QA step — see §VF-2 smoke checklist below) | Mock-proven via B5 tests | PASS (code) |
| B5 | Full B-workstream mock suite | `identifiable_trace_with_idlist_observations_is_imported`, `generation_with_no_usage_anywhere_is_schema_changed_absence_preserved`, `peripheral_field_type_mismatch_still_imports_identifiable_trace`, `trace_with_no_id_is_missing_trace_id_counted_not_crashed` | PASS |

### Workstream C — Configurable range + incremental cursor + resumable backfill

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| C1 | `ImportRange` enum: `last_7d/last_30d/last_90d/all/since:<RFC3339>`; default `Last30d`; IPC `get/set_langfuse_import_range`; malformed `since:` falls back to default | `import_range_parses_validates_and_floors`, `import_range_setting_persists_validates_and_defaults` | PASS |
| C2 | Per-env incremental: `from = max(range_floor, cursor_ts − OVERLAP_SECS=3600)`; cursor never regresses | `incremental_window_resumes_from_cursor_floored_by_range`, `first_import_uses_range_floor_then_resumes_from_cursor` | PASS |
| C3 | Discovery bounded to `DISCOVERY_WINDOW_DAYS=7` (no full-history scan) | Code review confirmed | PASS |
| C4 | Chunked resumable atomic backfill: ordered newest→oldest sub-windows (≥30d, ≤24 chunks); `reached_page_limit` surfaced | `backfill_imports_history_in_chunks_and_is_resumable` (multi-chunk, dedup, cursor non-regressing), `backfill_reports_bounded_run_rather_than_truncating_silently` | PASS |
| C5 | `backfill_langfuse_now` IPC: `import_lock` serialised, dedicated off-UI thread, SEC-002 loopback, `BACKFILL_TIMEOUT_SECS=300` | Code review: `backfill_langfuse_now` in `lib.rs:813` confirmed | PASS |
| C6 | N+1 fetch cost documented and bounded by chunked design (deferred optimisation, acknowledged) | `// N+1 cost` comment at `normalize_trace` fetch site | ACKNOWLEDGED |
| C7 | Full C-workstream test suite | 6 tests listed above | PASS |

**Deduplication:** `store::seen_trace_ids` loads persisted trace IDs per env before each run; `INSERT OR IGNORE` on `(environment, trace_id)` PK; backfill re-run confirmed idempotent (VF-3 via `backfill_imports_history_in_chunks_and_is_resumable`).

### Workstream D — Grouped import summary + range/backfill UI

| Task | Criterion | Test(s) | Result |
|------|-----------|---------|--------|
| D1 | `ImportReport`/`EnvImportLine` extended; `import-report.ts` decoupled from `main.ts` | `null report renders nothing`, `headline reports new traces` | PASS |
| D2 | `renderImportReport`: per-env seen/new/dup/skipped; grouped reason breakdown (one line/reason); bounded `<details>` sample; backfill vs incremental headline; `reached_page_limit` note; no repeated per-trace string; `aria-live="polite"`; all text escaped | `reasons are GROUPED`, `each environment shows seen/new/duplicate/skipped`, `structural samples show key/type NAMES only`, `backfill headline is distinguished`, `a page-limit run says so`, `environment names/reasons/sample keys are escaped`, `SEC-011` | PASS |
| D3 | Range control: preset picker + custom date → `set_langfuse_import_range`; "Backfill now" button → `backfill_langfuse_now`; both disable while in-flight, honour disabled-source short-circuit; no CSP/capability change | `the default range is last_30d`, `keyword presets canonicalize`, `a custom date canonicalizes`, `canonical → control → canonical round-trips`, `Test connection is blocked while disabled` | PASS |
| D4 | 21 focused frontend tests: 11 `importReport.test.mjs` + 10 `langfuseSettings.test.mjs` (7 new range); `tsc --noEmit` + `vite build` green | All 21 pass | PASS |

### Cross-cutting

| Task | Criterion | Result |
|------|-----------|--------|
| X1 | `cargo test --lib`: 133 passed; `cargo fmt` applied; `cargo clippy` zero net new warnings | PASS |
| X2 | `openspec validate --strict` | TOOLING STATE ISSUE (see above; code artefacts present) |
| X3 | No new `HealthState` variant; no new egress host; diff in scope (`langfuse/*`, `settings/mod.rs`, `lib.rs`, `src/*.ts`); no change to capture/runtime_observer/classifier/env_mapping/CSV | PASS |
| X4 | RELEASE.md (SW-6 gate scope — not SW-3) | OUT OF SCOPE |

**Security checks:**
- No raw payload, prompt content, or credential values in `SkipSample` (carries only `field_type`/`element_type`/`key_name`).
- `tauri.conf.json` diff: icon paths added only; CSP line unchanged (`connect-src ipc: http://ipc.localhost` — no new egress host).
- No new Tauri capability files.
- No `network`/`fetch`/`XMLHttpRequest` in renderer source; no renderer-side network calls added.
- No time-entry suggestion or updater code found.

---

## Known Pre-existing Failures (Unrelated to TASK-029)

| Test | File | Classification |
|------|------|----------------|
| `safe dotenv parser loads only allowlisted Langfuse keys without shell execution` | `pi-observe.security.test.mjs:50` | Pre-existing on `main` — same code, same failure on base branch. Out of scope. |
| `remote Langfuse host is blocked unless explicitly opted in` | `pi-observe.security.test.mjs:82` | Pre-existing on `main` — same code, same failure on base branch. Out of scope. |

---

## VF-2 Manual Smoke Checklist (Janne's Real 9-env / 640-trace Stack)

Run these against the live Langfuse stack after deploying/building from `feat/task-029-langfuse-backfill-schema-diagnostics`.

**Setup**
- [ ] Build and launch the desktop app from this branch.
- [ ] Navigate to **Settings → AI evidence source (Langfuse)**.

**Import range**
- [ ] Range picker shows **"Last 30 days"** as the selected default.
- [ ] Change to **"Last 90 days"**, save — range label updates to "Last 90 days".
- [ ] Change back to **"Last 30 days"**, save.

**Incremental import (VF-2 core)**
- [ ] Click **"Import from Langfuse now"**.
- [ ] Verify `new_traces > 0` across environments in the report (previously was 0/640).
- [ ] Report shows **grouped reasons** (e.g. "611 observations not embedded"), NOT one warning line per trace.
- [ ] `seen / new / duplicate / skipped` counts shown per environment.
- [ ] No `sk-`/`pk-`/`Bearer`/session/credential material visible in the report UI.

**Tolerant import verification**
- [ ] Traces previously skipped with `observations_not_embedded` now appear as `healthy` or `schema_changed` (not silently dropped).
- [ ] Structural sample `<details>` block (if present) shows only key/type names, not observation IDs or values.

**Backfill**
- [ ] Click **"Backfill now"** — button disables and shows "Backfilling…" while in progress.
- [ ] Backfill completes without error; report headline says "Backfill run".
- [ ] Re-run "Import from Langfuse now" — no duplicate traces (new count = 0 for already-imported range, duplicates count ≥ 0 and stable).

**Page-limit check (optional — if a wide range with many traces)**
- [ ] If a run hits the page limit, report shows "re-run to continue, nothing truncated" note rather than silent truncation.

**Negative paths**
- [ ] With Langfuse integration **disabled**: both "Import now" and "Backfill now" buttons appear as `disabled`.
- [ ] Malformed custom `since:` date (e.g. "not-a-date") in range picker falls back to default rather than crashing.

---

## Gate Verdict

**PASS → route to SW-4 (Code Reviewer) ∥ SW-5 (Security Agent)**

All 13 new Rust tests and 21 new frontend tests pass. Build is clean. Pre-existing pi-observe failures
are isolated to `main` and unrelated to TASK-029 scope. VF-2 live-stack count confirmation is a
Janne/QA step (see smoke checklist above).
