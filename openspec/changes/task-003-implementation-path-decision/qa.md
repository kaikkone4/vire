# QA Gate — TASK-003 implementation-path decision

- **Role:** QA Engineer (SW-3)
- **Change:** `task-003-implementation-path-decision`
- **Branch:** `feat/task-003-implementation-path-decision` · **PR:** #9
- **Tier:** L2 · **Gate:** APP-005 Gate A (validation/decision gate)
- **Date:** 2026-06-05
- **Verdict:** **PASS**

## 0. Coverage method (decision/spike change)

TASK-003 ships **no product runtime** — no source, schema, build target, or config. There is
therefore nothing executable to integration- or load-test, and no NFR throughput/latency target in
scope. For a decision-only change, "observable coverage" means **every spec scenario maps to
verifiable, static evidence in the committed artifacts**, plus the no-runtime / data-safety
invariants are mechanically confirmed against the diff. That is the method used below. No flaky or
mocked tests are introduced. The standing capture/importer test obligations are inherited by
TASK-005 / TASK-007-MVP, which the decision explicitly defers.

`openspec validate task-003-implementation-path-decision --strict` → **valid** (run from `code/`,
2026-06-05).

**Re-run note (post-packaging, 2026-06-05):** re-verified after commit `0c877ff`, which adds only the
SW-4/SW-5/docs/release gate artifacts (`review.md`, `sec.md`, `docs.md`, `RELEASE.md`) under the
change dir. The branch diff grew from 5 to 10 files, all still docs/spec under
`openspec/changes/`; no product-runtime file was introduced. Strict validation still valid; PR #9
OPEN, non-draft, MERGEABLE. Verdict unchanged: **PASS**.

## 1. Scenario coverage matrix

All 6 scenarios across the 4 ADDED requirements in
`specs/implementation-path-decision/spec.md` have observable coverage:

| # | Requirement / Scenario | Observable evidence | Result |
| --- | --- | --- | --- |
| 1 | **Path decided from spike evidence** → *Options compared against spike-derived criteria* | `design.md` §"Options considered" scores A/B/C against reuse value, SEC-002 importer fit, added complexity, privacy boundary, EPIC-006 seams; capture access + permission burden marked non-differentiating gates, sourced to TASK-002 §10 (verified on `origin/feat/task-002-…`: report §10 "Tauri+helper vs Swift-first — signal for TASK-003", capture "Full/Full" both native). | **PASS** |
| 2 | **Path decided from spike evidence** → *Single path recorded with rationale before TASK-004* | `design.md` ADR **DEC-019** records exactly one path (reuse Tauri + native helper + Rust importer; AW reference-only; Swift-first rejected) with rationale traced to TASK-001/002/007 and DEC-008/009/017/018. Recorded on the pre-TASK-004 gate (`05` §4 chain, §5 Gate A confirmed in BA plan). | **PASS** |
| 3 | **Every named technology verified** → *Technology register records verification status* | `design.md` §"Technology verification register" marks Tauri v2 sidecar/IPC/HTTP-client, TCC/AX, Swift/AppKit, ActivityWatch, SQLite as **verified (source URL)**; nested-binary codesign+notarization marked **assumption + TASK-005 packaging-spike follow-up**. No load-bearing claim asserted as fact. | **PASS** |
| 4 | **Downstream constraints fixed** → *Capture and credential boundaries bound to helper and core* | `design.md` DEC-019 constraints 1–3: AX grant on the signed helper (TCC-trusted binary), raw titles flow helper→Rust core only (never webview/network; `connect-src ipc:` stays locked), importer is a read-only Rust-core REST client scoped to the configured Langfuse base URL, no raw-activity egress (SEC-002, DEC-018). | **PASS** |
| 5 | **Downstream constraints fixed** → *Reuse stays evidence-driven and excludes legacy capture surface* | `design.md` DEC-019 constraint 4: carries only `reuse-as-is`/`reuse-with-changes` assets (CSP, capabilities, CSV escaping/formula-neutralization, validation/error patterns — matches TASK-001 inventory: `csv_formula_neutralized` lib.rs:133, `csv_escape` lib.rs:142, adversarial.rs); legacy `time_entries`/manual-entry/stopwatch **not** re-admitted as capture, migrate-vs-retire deferred to TASK-004. | **PASS** |
| 6 | **Ships no product runtime and no MVP** → *No product-runtime change made by the decision* | `git diff --name-only main...HEAD` = 10 files (post-packaging), all under the change dir; none under `src/`, `src-tauri/src/`, `observability/`; no build target / `tauri.conf.json` / `Cargo.toml` / `package.json` / `capabilities/` touched. Schema/capture/importer/observer remain unbuilt; legacy left reference-only. | **PASS** |

## 2. Mandatory QA checks (arch-review §7)

| Check | Finding | Result |
| --- | --- | --- |
| **Verification completeness** | Every named technology carries verified(URL) or assumption+follow-up; the nested-binary codesign gap is flagged as an assumption, not asserted. | **PASS** |
| **Boundary correctness** | AX grant bound to the helper; raw titles kept off renderer/network; importer scoped to a Langfuse-only Rust-core outbound allowlist (SEC-002). | **PASS** |
| **DEC-017/DEC-018 fidelity** | No new pi/Claude adapter; importer stays a read-only cloud-first pull. Matches BA `07_decision_log.md` DEC-017/DEC-018. | **PASS** |
| **No-build check** | Confirmed mechanically (§1 row 6). Legacy surface not migrated, reused-as-capture, or wiped. | **PASS** |
| **Artifact data safety** | Leak-pattern scan (PAT/ghp/sk-ant/AKIA/xox/PRIVATE KEY) over change dir → clean. No real window/app titles, prompt/response text, command bodies, or env dumps. | **PASS** |
| **Exit-gate text** | "Implementation path selected and documented before TASK-004"; MVPs unbuilt; legacy fate deferred to TASK-004. | **PASS** |

## 3. Consistency with BA + feeder spikes

- **EPIC-001 / EPIC-006:** BA `05` §3 row TASK-003 maps to exactly EPIC-001, EPIC-006 with acceptance
  "Decision recorded before schema/feature build starts" — the change satisfies this verbatim.
- **DEC-008/DEC-009:** BA log DEC-009 = "reuse current Tauri shell unless capture spike proves
  replacement is better". DEC-019 confirms it — the spike switch condition is **not met** (capture
  equal per TASK-002 §10; reuse value high). Confirmation, not divergence. **DEC-019 is correctly
  absent from the BA log** and routed via `feedback_to_ba[]` (FB-003) for ratification, since SW does
  not edit BA artifacts directly.
- **DEC-017/DEC-018:** preserved (read-only, cloud-first, Langfuse-only outbound, no new adapter).
- **TASK-001:** salvage classifications, locked `connect-src ipc:` CSP (no outbound HTTP), CSV-safety
  primitives, and backend-side importer placement all cited faithfully against
  `salvage-reuse-inventory.md`.
- **TASK-002 §10:** "capture equally reachable from native helper or Swift-first; trade is helper
  packaging/IPC/TCC-trust vs. rebuild cost" reproduced accurately; capture treated as a
  non-differentiating gate.
- **TASK-007 arch-review:** importer host-runtime explicitly owned by TASK-003; read-only cloud-first
  REST pull behind SEC-002 (DEC-018) reproduced accurately.

## 4. Downstream-obligation audit

DEC-019 introduces **no untracked implementation obligation**. Every obligation routes to an
existing BA task: capture helper / AX grant / idle thresholds → **TASK-005**; helper↔core IPC and
reuse host → **TASK-004/005**; durable Langfuse importer → **TASK-007 MVP**; legacy `time_entries`
migrate-vs-retire and importer table shapes → **TASK-004**; runtime observer → **TASK-006**. The
single net-new packaging concern — **nested-binary codesign + notarization of the sidecar** — is the
one explicitly-flagged exception, recorded as an assumption with a named **TASK-005 packaging-spike**
follow-up and identified as the only signal that would reopen DEC-019. This matches the allowed
scope exactly.

## 5. Verdict

**PASS.** All 6 spec scenarios and all 6 mandatory arch-review checks have observable coverage; the
no-runtime, data-safety, BA-consistency, and downstream-obligation invariants are confirmed.
`openspec --strict` is valid. Route to **SW-4 (Code Reviewer)** ∥ **SW-5 (Security Agent)**.
