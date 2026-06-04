# QA Report — TASK-001 Repo/Path Assessment

- **Change:** `task-001-repo-path-assessment` · **Tier:** L2 · **Gate:** SW-3 (QA)
- **Branch:** `feat/task-001-repo-path-assessment` · **PR:** kaikkone4/vire#6
- **Deliverable under test:** `salvage-reuse-inventory.md` (+ proposal, tasks, spec delta)
- **Type:** Read-only spike/assessment — expected changed artifacts are OpenSpec docs only.
- **Date:** 2026-06-04 · **Verdict: PASS**

## Verdict

**PASS** → route to SW-4 (Code Reviewer) ∥ SW-5 (Security Agent).

All spec scenarios have observable coverage, OpenSpec strict validation passes, the diff is
documentation-only (no source/schema/config touched), the exit gate is satisfied, and the produced
artifacts contain no credentials or sensitive raw data. The inventory's evidentiary claims were
independently spot-verified against the working tree and matched exactly.

## Scenario coverage matrix

Scenarios are drawn from `specs/repo-path-assessment/spec.md`. Coverage is by documented evidence in
the deliverable plus independent QA verification (this is an assessment task; "test" = observable
artifact evidence + verification check, not executable integration tests).

| # | Requirement / Scenario | Coverage evidence | QA verification | Result |
| --- | --- | --- | --- | --- |
| 1 | Inventory deliverable — Every inventoried area is classified | `salvage-reuse-inventory.md` §2.1–2.8: shell, frontend, Rust backend, SQLite, summary/export, tests, observability, privacy. Each asset carries exactly one of `reuse-as-is`/`reuse-with-changes`/`reference-only`/`retire/replace` + BA/EPIC/SEC ref | Read full artifact; every row has one classification + a reference | PASS |
| 2 | Data model coverage is mapped | §3 maps all 13 BA entities (04 §8) as present/partial/absent (1 partial `projects`, 12 absent) | Table complete; covers all 13 entities | PASS |
| 3 | No assumption of wipe or reuse — path decision left open | §Exit-gate (top), §4, §9: both replacement and reuse explicitly open, deferred to TASK-003 | Statements present and unambiguous | PASS |
| 4 | No source or schema is modified | §6, §9 assert no `src/`/`src-tauri/`/`observability/` change | `git diff --name-only main...HEAD` → only `openspec/changes/...` files; tree clean | PASS |
| 5 | No secrets in produced artifacts | §10 self-attestation | Secret-pattern scan of all produced files: only match is §6 naming the scan's own pattern strings — no credential/raw-title/prompt/response/command-body/env-dump | PASS |
| 6 | APP-005 control coverage is mapped | §5 maps SEC-001/002/003/005/006/008 (coverage + gap); §7 records no outbound client + no SBOM/signing/notarization | All six L2 controls present with gaps; both required known-gaps recorded | PASS |
| 7 | DEC-017 tension recorded without resolution | §4.1, tasks 5.3: `pi-observe` = emitter vs. DEC-017 import-first; routed to TASK-003/006/007, not resolved | Present and explicitly unresolved | PASS |

## Required-area checklist (task brief)

| Area | Location | Result |
| --- | --- | --- |
| Tauri/Rust/TS shell, UI, backend | §2.1, §2.2, §2.3 | PASS |
| SQLite schema | §2.4, §3 | PASS |
| project / manual-entry divergence | §2.2, §2.4 (`time_entries` → TASK-004 input), §4.2 | PASS |
| summary / export | §2.5 (incl. raw-rows vs. summary-only default divergence) | PASS |
| tests | §2.6, §8 | PASS |
| privacy posture | §2.8 | PASS |
| observability / pi-observe | §2.7, §4.1 | PASS |
| local Langfuse tooling | §2.7 (pinned stack, loopback-bound, dev-only) | PASS |
| capture gaps | §2.8, §5 (SEC-001 N/A — deferred), §7 | PASS |

## Independent verification performed (read-only)

- **OpenSpec strict validation:** `openspec validate task-001-repo-path-assessment --strict` → valid.
- **Diff scope:** `git diff --name-only main...HEAD` → 6 files, all under
  `openspec/changes/task-001-repo-path-assessment/`. No product source/schema/config. Tree clean.
- **tasks.md ↔ inventory:** every checked box (1.1–7.3) maps to a corresponding inventory section;
  no checked task lacks artifact backing.
- **Claim spot-checks (matched exactly):** CSP string in `tauri.conf.json`; `.gitignore` exclusions
  (`*.sqlite`, `*.db`, `observability/langfuse/.env`); no tracked `*.sqlite|*.db|.env` (only
  `.env.example`); `.env.example` secret fields empty; `lib.rs` anchors `init_db:36`,
  `summary_repo:124`, `csv_formula_neutralized:133`, `csv_escape:142`, `export_csv_repo:143`;
  `pi-observe.mjs` `redact:32`, `isLoopbackLangfuseHost:205`.
- **Secret-scan reproduction:** source matches for secret patterns are exactly the three redaction
  pattern/fixture files (`pi-observe.mjs`, `pi-observe.security.test.mjs`, `pi-observe.test.mjs`) plus
  the inventory doc itself naming the pattern strings — no live credential material. Nothing to rotate.

## Notes for downstream (non-blocking)

- Test suites were inventoried, not executed (no `node_modules/`/`target/` in this read-only checkout;
  spike must not mutate the tree). This is correct for a non-mutating assessment — executable
  integration/perf tests are out of scope for TASK-001 and are deferred to code-changing tasks.
- L2 performance/load testing: no NFR throughput/latency target applies to a documentation deliverable,
  so none was run (correct per QA tier rules).

## Handoff

PASS → SW-4 `sw-code-reviewer` ∥ SW-5 `sw-security-agent`. Branch/PR suitable to proceed.
