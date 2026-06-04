# Architecture Review — TASK-001 Repo/path assessment

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-001-repo-path-assessment`
- **Branch (implementation):** `feat/task-001-repo-path-assessment`
- **Date:** 2026-06-04
- **Verdict:** **PASS** — scope is architecturally coherent as a single read-only spike; no split
  required; design tensions are recorded as downstream decision inputs (one `feedback_to_ba[]`
  entry below), none of which blocks the assessment.

## 1. Scope validation against BA architecture

TASK-001 is a **spike/assessment** (`05_project_plan_epics.md` §3, row TASK-001; `04_technical_plan.md`
§13 WP-001). Its mandate is to inventory the existing repo and produce a salvage/reuse map with the
exit gate *"clear salvage/reuse inventory; no assumption of wipe or reuse."*

- **Scope is self-contained and read-only.** The deliverable is documentation; it crosses no
  component boundary that would require splitting. The single OpenSpec change is the right unit.
- **Sequencing is correct.** TASK-001 precedes TASK-002 (capture), TASK-006/007 (runtime/Langfuse),
  and the TASK-003 path decision (`05` §4 dependency chain). The change defers all path/data/adapter
  decisions downstream, matching the BA "evidence before commitment" posture.
- **No scope creep.** The change implements nothing under `src/`, `src-tauri/`, or `observability/`;
  the spec delta encodes the no-wipe/no-reuse guardrail as an enforceable requirement.

Conclusion: the task design is consistent with `03/04/05` and APP-005. **PASS.**

## 2. Salvage/reuse inventory (architect's read)

| Area | Asset | Classification | Architectural note |
| --- | --- | --- | --- |
| Shell | Tauri v2 config, capabilities, plugin-dialog | reuse-as-is | Locked CSP (`connect-src ipc:` only) is privacy-aligned; importer needs a backend-side outbound path. |
| Frontend | TS SPA (5 views), `escapeHtml` | reuse-with-changes | Viable Review/export shell (§4); review/approval + summary-only UX absent. |
| Backend | `lib.rs` repo fns + Tauri commands | reuse-with-changes | Clean repo pattern, parameterized SQL; needs evidence/review/summary/retention surface. |
| Data | `projects`, `time_entries`, `settings` | partial / decision | `projects` ≈ BA `projects`; `time_entries` (manual) is **not** in the BA evidence model; ~11 BA entities absent; no migration framework. |
| Export | `export_csv_repo`, `csv_escape`, `csv_formula_neutralized` | reuse-with-changes | Escaping/neutralization strong + tested (SEC-006); default emits **raw rows**, BA needs summary-only default. |
| Tests | Rust unit + `adversarial.rs`; FE `*.test.mjs`; `pi-observe` suites | reuse-as-is (patterns) | Good adversarial/privacy posture; fixtures avoid real secrets. |
| Tooling | `pi-observe` runtime wrapper | reference / reclassify | Strong TASK-006 signal source (sessions, idle/orphan reconcile, redaction, loopback-only gating). See §4 tension. |
| Tooling | Local Langfuse stack + scripts | reuse-as-is (dev infra) | Pinned `langfuse:3.63.0`, local-only; TASK-007 validation environment. |
| Privacy | README/Settings non-collection, `.gitignore` (sqlite/db/.env) | reuse-as-is | Posture documented + enforced; no credentials committed. |

## 3. APP-005 control coverage (assessment scope)

| Control | Existing coverage | Gap for downstream |
| --- | --- | --- |
| SEC-001 capture allowlist | N/A — capture deferred ("Manual Mode") | Greenfield in TASK-002/005. |
| SEC-002 network boundary / no raw egress | CSP allows no outbound HTTP; no network client exists | Importer must add a controlled, backend-side Langfuse-only path (TASK-007/012). |
| SEC-003 credential handling | `pi-observe` redaction + `.gitignore` exclusion of `.env`; no creds committed | App-side credential storage/redaction is net-new (TASK-007/012). |
| SEC-005 retention/deletion | None — no raw evidence stored yet | Net-new lifecycle in TASK-004/013. |
| SEC-006 CSV safety | Escaping + formula neutralization implemented and tested | Switch default to summary-only export (TASK-011/013). |
| SEC-008 release integrity | None — no SBOM/signing/notarization tooling | Net-new in TASK-015. |

The assessment change itself releases nothing, so Gate D does not apply; coverage is recorded so the
inventory is control-aware.

## 4. Design-level concern → `feedback_to_ba[]`

**FB-001 — DEC-017 vs. existing `pi-observe` emitter role.**
- **Observation:** DEC-017 makes Langfuse **import** (REST pull, pagination, dedup, health states)
  the primary AI time/usage/cost evidence path, limits local runtime observation to
  reconciliation/health (no duplicate time/cost ledger), and rules a new Vire-specific pi/Claude
  adapter **out of MVP**. The repo's `pi-observe` is a trace **emitter** (POSTs to Langfuse
  ingestion) — architecturally close to the deferred "adapter." No Langfuse importer exists; TASK-007
  is greenfield.
- **Impact:** Reusing `pi-observe` as-is for AI evidence would risk the duplicate-ledger pattern
  DEC-017 forbids and could be read as shipping the deferred adapter.
- **Recommendation (non-blocking):** Reclassify `pi-observe` as a **TASK-006 runtime-reconciliation/
  health signal source** only; route its emission role to TASK-003 for an explicit keep/retire/clarify
  decision. Requesting a BA-side DEC clarification on whether the existing `pi-observe` emitter counts
  as the "no new adapter in MVP" exclusion.
- **Disposition:** Recorded as a downstream decision input; does **not** block TASK-001.

This is the only escalation-grade item, and it is informational. It does not warrant
`escalate-to-ba` status because surfacing exactly this kind of tension is TASK-001's purpose, and the
decision properly lives in TASK-003/006/007.

## 5. Other architectural notes for downstream

- **TASK-004:** decide `time_entries` migrate-vs-retire; introduce a migration framework (none today).
- **TASK-007:** keep the Langfuse importer in the Rust backend so the CSP stays locked and the
  outbound allowlist is enforced server-side (SEC-002).
- **Anti-pattern watch:** do not let manual-tracker reuse momentum re-import the "generic time
  tracker" scope the BA narrowed away from; keep reuse evidence-driven.

## 6. Handoff

- **SW-2 implementer:** **backend-developer** (primary — Rust/Tauri/SQLite + domain/test inventory),
  with **devops** consulted for the observability/build/release tooling inventory and the SEC-008
  gap (SBOM/signing/notarization) and Langfuse local-stack assessment.
- **Mandatory QA/security checks for the assessment deliverable:**
  1. Secret-scan baseline over the repo; confirm no committed credentials and that `.gitignore`
     excludes `*.sqlite`, `*.db`, `observability/langfuse/.env` (SEC-003).
  2. Verify the produced inventory artifact contains no secrets, raw titles, prompt/response text,
     or command bodies (SEC-002/003).
  3. Confirm the exit gate text: inventory present, both reuse and replacement left open (no
     wipe/reuse assumption).
- **OpenSpec status:** `openspec validate task-001-repo-path-assessment --strict` → valid.
