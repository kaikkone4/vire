# Architecture Review (SW-1) — TASK-022 pi/Claude runtime reconciliation observer (TASK-006 MVP slice)

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-022-runtime-reconciliation-observer-mvp`
- **Branch (proposed):** `feat/task-022-runtime-reconciliation-observer-mvp` · **base:** `main`
  (origin/main `bc064e5`, post-PR #13)
- **Tier:** L2 · **Gate context:** SW-1 task-design review before developer roles implement the
  TASK-006 (WP-006) runtime-reconciliation MVP.
- **Date:** 2026-06-12
- **Verdict:** **PASS** — single component (the BA "AI Runtime Observer"), one cohesive OpenSpec
  change. No component boundary is crossed; **not split-required**. No BA escalation required
  (DEC-003/DEC-017 already establish the posture). Three non-blocking `feedback_to_ba[]` items
  carried (§7).

---

## 1. Inputs read

- BA architecture: `artifacts/ba/03_architecture_plan.md` §3 (component model — AI Runtime Observer),
  §4.2 (pi/Claude AI evidence, reconciliation states), §5 (`AI_RUNTIME_SESSION` data model), §7
  (privacy/security), §9 (NFR), §10 (DEC-003/DEC-017).
- BA project plan: `05_project_plan_epics.md` EPIC-003, TASK-006 row + acceptance, risks (§ runtime
  reconciliation weak; absence ≠ zero); `07_decision_log.md` DEC-003/DEC-017/DEC-020;
  `10_traceability_matrix.md` FR-012/FR-013/FR-014/NFR-008.
- BA technical plan: `04_technical_plan.md` §6 (runtime/session reconciliation spike plan, allowed
  fields, anti-goals, health states), §8 (`ai_runtime_sessions` table).
- Wiki: `decisions/vire-uses-langfuse-first-ai-evidence-with-runtime.md` (DEC-017).
- TASK-001 salvage inventory: `task-001-repo-path-assessment/salvage-reuse-inventory.md` (pi-observe
  `events.jsonl` as the strong TASK-006 reconciliation signal source; redaction/loopback patterns;
  DEC-017 emitter-vs-importer tension carried to TASK-003/006/007).
- pi-observe reference: `observability/pi-observe/bin/pi-observe.mjs` (events.jsonl lifecycle,
  `runs.json`, idle/orphan reconciliation, redaction, session-id hashing, loopback gating).
- Current importer runtime (TASK-019/020/021): `src-tauri/src/langfuse/{model,store,importer,
  config,mod}.rs` — 10-state health taxonomy, `langfuse_import_runs` / `langfuse_raw_traces` /
  `langfuse_ai_evidence`, `get_langfuse_source_health` + `import_langfuse_now` IPC, the absence-≠-zero
  and persist-failure-surfacing invariants. Renderer surface `src/main.ts` (`SourceHealth` panel).
- OpenSpec contracts: `task-019.../specs/langfuse-importer/spec.md` (health taxonomy + no-egress
  contract), TASK-020/021 spec deltas (hardening, persist-failure IPC surfacing).

## 2. Architecture-consistency findings

The slice is consistent with the BA architecture and DEC-003/DEC-017. Four findings shape the design
and must be honored by developers:

1. **Read a coarse session log; do not scan processes.** BA §4.2/§6 calls for *coarse* pi/Claude
   observation and explicitly forbids command bodies. Process scanning re-collects what `pi-observe`
   already emits **and** exposes process argv (= prohibited command bodies). The guardrail-minimal
   realization is to ingest the already-redacted, metadata-only `events.jsonl` (TASK-001's identified
   signal source) read-only. This is an *implementation* choice inside the AI Runtime Observer
   component — it does not reopen any BA decision. (See `design.md` §2.)
2. **Runtime evidence must never become a cost/time authority.** DEC-003/DEC-017 are categorical. The
   design enforces this *structurally*: `ai_runtime_sessions` has **no token/cost column**, so it
   physically cannot duplicate the Langfuse ledger. Cost/time stay sourced from `langfuse_ai_evidence`.
   (See `design.md` §5, spec R3.)
3. **`observed_no_trace` must be gated on import health — this is the absence-≠-zero rule for the
   observer.** A session with no matching trace under a `unavailable`/`unknown`/`auth_or_network_error`
   import is **not** a gap; it is `reconciliation_unknown`. Only a `healthy` import licenses
   `observed_no_trace`. This mirrors the importer's own NFR-008 discipline and is the single
   load-bearing correctness rule of the change. (See `design.md` §4.1, spec R5.)
4. **Match against the *normalized* evidence row, not raw prompt-bearing payloads.** `model.rs` already
   parses `Trace.session_id` but `langfuse_ai_evidence` does not persist it. Surfacing it as a nullable
   evidence column (additive) keeps the observer out of `langfuse_raw_traces.payload` (which may carry
   prompts/metadata) — a privacy-positive interface extension, not a boundary crossing. (See
   `design.md` §3.)

## 3. Split analysis — one task, not split-required

Per the role rule, "split-required" means scope crosses component boundaries in
`03_architecture_plan.md` §4. It does not. Every piece is a sub-part of the single **AI Runtime
Observer** component:

| Listed piece | Component | Boundary crossing? |
| --- | --- | --- |
| Coarse runtime-log ingest + allowlist | AI Runtime Observer | No |
| `ai_runtime_sessions` persistence (additive slice of TASK-004) | AI Runtime Observer | No |
| Reconciliation engine + import-health gating | AI Runtime Observer | No |
| Reads `langfuse_import_runs` / `langfuse_ai_evidence` | Read-only contract on the Langfuse importer | No (consume, not modify) |
| `session_id` column on `langfuse_ai_evidence` | Langfuse importer (additive interface extension) | No (same component, additive) |
| Read-only `get_runtime_reconciliation` IPC + thin banner | Observer status surface (thin) | No (full UI = TASK-009) |
| No prohibited-field persistence / no egress | Cross-cutting constraint (SEC-001/002/004), not a component | No |

The macOS capture adapter (TASK-005), classification engine (TASK-008), review UI (TASK-009), and
exporter (TASK-010) are untouched. No pi/Claude emitter/adapter is built (DEC-017). **Verdict: one
cohesive change** with an internal implementation sequence (`tasks.md`), not a split.

### MVP boundary (explicit, per task ask)

- **Captured/stored:** coarse session boundaries only — `project_key`, `tool`, hashed `session_id`,
  `run_id`, start/end timestamps, coarse status, resolved environment, reconciliation state, matched
  trace id. **Never** prompts/responses, command bodies, shell history, env dumps, secrets, free-text
  summaries, or repo/cwd identifiers beyond the safe project token.
- **Reconciles with Langfuse import:** read-only against `langfuse_ai_evidence` (session_id first,
  then environment + time-window overlap) and `langfuse_import_runs` (health gating). References the
  importer's trace-side health; does not re-derive it; does not duplicate cost/time.
- **UI/API surface:** one read-only IPC (`get_runtime_reconciliation`) returning coarse counts/states
  + one thin Settings line. No review/approval UI.
- **Out of scope:** process scanning, native capture (TASK-005), emitter/adapter (DEC-017), runtime
  cost/time ledger, classification (TASK-008), full review UI (TASK-009), CSV export (TASK-010),
  retention automation.

## 4. Boundary touchpoints (contracts/dependencies, not splits)

1. **Langfuse importer (TASK-019/020/021).** Read-only consumer of run health + evidence. The one
   recommended importer edit (`session_id` on `langfuse_ai_evidence`) is additive and consumed
   read-only — analogous to how TASK-019 *published* the health taxonomy for this observer to consume.
2. **SQLite store / TASK-004 schema.** TASK-022 owns the narrow additive `ai_runtime_sessions` slice
   (its own table + migration through the single `init_db` entry point; no `projects`/`time_entries`/
   `langfuse_*` write change) — the same precedent TASK-019 set for the importer tables. Flagged to BA.
3. **Review UI (TASK-009) / Summary model (TASK-010).** TASK-022 produces the reconciliation state +
   a thin read-only surface only; it does not fold runtime into approved summaries or build the review
   UI.

## 5. Security/privacy controls preserved (L2 / SEC-001/002/003/004, DEC-017)

| # | Constraint | Where enforced in this change |
| --- | --- | --- |
| 1 | No prompts/responses, no command bodies, no shell history, no env dumps, no secrets persisted | Strict ingest allowlist (`design.md` §6, spec R2); adversarial tests (`tasks.md` §7) |
| 2 | No raw local-activity egress; renderer off-network; no CSP change | Observer makes zero network calls; no new capability (`design.md` §1, spec R1) |
| 3 | Runtime is reconciliation/health only — never a cost/time ledger | `ai_runtime_sessions` has no token/cost column (`design.md` §5, spec R3) |
| 4 | Docker/Langfuse down or log absent ⇒ unknown, never zero | Import-health gating → `reconciliation_unknown` (`design.md` §4.1, spec R1/R5) |
| 5 | Credentials/secrets never read or surfaced | Observer reads no credentials; surface exposes counts/states only (spec R6) |
| 6 | Path safety on the local log | Reject symlinks, cap bytes, tolerate absence (mirror pi-observe guards) (spec R2) |
| 7 | Observer avoids prompt-bearing raw payloads | Match on normalized `langfuse_ai_evidence.session_id` (`design.md` §3) |

**Privacy note:** the MVP-local relaxation (prompt/session/metadata allowed *inside* the local
Langfuse boundary) does **not** propagate to the observer — `ai_runtime_sessions` is strictly coarse,
and the recommended interface extension keeps the observer out of raw payloads entirely. This is a
*tighter* posture than the importer's raw-trace table, which is correct for a reconciliation/health
component.

## 6. Empirical facts the developer must honor

- `pi-observe` writes `events.jsonl` (lifecycle: `tool_started`/`tool_finished`/`manual_active`/
  `manual_inactive`/`idle_*`/`tool_orphaned`) **and** POSTs a Langfuse trace with the **same** hashed
  `sessionId` — so `session_id` matching is exact when pi-observe is the emitter, and the emit can
  still fail/timeout/be remote-blocked, which is exactly the `observed_no_trace` signal.
- `session_id` is `session-<hash>` (opaque, not content). `project_key` is already safe-tokenized.
  Neither is a secret; both are safe to store.
- `langfuse_ai_evidence` does **not** currently store `session_id`; `model.rs` already parses it on
  `Trace`. The raw payload (`langfuse_raw_traces.payload`) carries it but also prompts/metadata —
  prefer surfacing the field over parsing raw payloads.
- Reconciliation is bi-directional: `observed_no_trace` (session, no trace, healthy import),
  `unmatched_trace` (trace, no session), `matched` (both). Absence of the log ⇒ no session can be
  `observed_no_trace`; never an AI-usage conclusion.
- **Absence ≠ zero** asserted literally: down/uncertain import or absent log ⇒
  `reconciliation_unknown`, never `0`.

## 7. Open items / `feedback_to_ba[]`

- **(Carried from TASK-001/003) pi-observe tooling role still undecided.** TASK-022 treats
  `events.jsonl` as a read-only **optional** runtime-signal source (configurable path; absence is a
  state), **not** a committed dependency and **not** an adoption of pi-observe as the emitter. Request
  a DEC ratifying that the runtime observer MAY ingest a local coarse session log without fixing
  pi-observe's permanent role.
- **`ai_runtime_sessions` schema slice.** Named in BA §5/§8 under TASK-004. Confirm TASK-022 owning
  the narrow additive slice is accepted (same precedent as TASK-019's importer tables), or assign a
  decision id.
- **`session_id` on `langfuse_ai_evidence`.** Confirm the additive, privacy-positive importer
  interface extension is acceptable (keeps the observer out of prompt-bearing raw payloads). Fallback
  documented if declined.

None block developer start; all three route to ba-architect via Pi-Assistant.

## 8. Recommendation — next roles and branch

- **Change name:** `task-022-runtime-reconciliation-observer-mvp` (this dir).
  `openspec validate --strict` passes (verified 2026-06-12).
- **Branch:** `feat/task-022-runtime-reconciliation-observer-mvp`, base `main` (origin/main `bc064e5`,
  post-PR #13; no merge-order dependency).
- **Next role (primary):** **backend-developer (Rust/Tauri)** — `runtime_observer` module, coarse
  `events.jsonl` ingest + allowlist, `ai_runtime_sessions` migration, the reconciliation engine with
  import-health gating, the additive `session_id` evidence-column extension, and the read-only
  `get_runtime_reconciliation` IPC.
- **Supporting role:** **integration-engineer** — pi-observe `events.jsonl` runtime fit (state-dir/
  path discovery, lifecycle/orphan semantics), reconciliation against real seeded local traces, and
  the thin Settings line in `src/main.ts`.
- **Then:** SW-3 (QA per `design.md` §9 / `04_technical_plan.md` §12), SW-4 (code review), SW-5
  (security: SEC-001/002/004 — allowlist, no-egress, no cost/time authority), SW-6 (release).

## 9. Verdict

**PASS.** TASK-022 is one cohesive change inside the single AI Runtime Observer component boundary —
not split-required. It faithfully implements DEC-003/DEC-017 and the BA architecture (coarse
read-only runtime observation for reconciliation/health only; no cost/time ledger; no pi/Claude
adapter; absence ≠ zero via import-health gating; strict no-prohibited-field allowlist; no egress;
renderer off-network). Deliverables for developer handoff (`proposal.md`, `design.md`, `tasks.md`,
`specs/runtime-reconciliation/spec.md`, this review) are in place and validate strictly. Three
non-blocking `feedback_to_ba[]` items carried; route TASK-022 to backend-developer (Rust/Tauri) +
integration-engineer on `feat/task-022-runtime-reconciliation-observer-mvp`.
