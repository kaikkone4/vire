# Architecture review — TASK-029 (Langfuse backfill + schema diagnostics + import correctness)

**Reviewer:** SW Architect (SW-1) · **Verdict:** **PASS — no split required, no blocking escalation.**
**Tier:** L2. **Change:** `openspec/changes/task-029-langfuse-backfill-schema-diagnostics/`.

## 1. Summary

The task is a **correctness + completeness** fix to the **existing Langfuse importer component**, not a
new capability or a boundary change. TASK-027 delivered discovery (9 envs, 640 traces seen) but the
import yields 0 useful new traces (611 skipped, `schema_changed`, one warning repeated hundreds of
times). The proposed change: (A) classify *why* traces are skipped, secret-free and aggregated; (B) widen
the v3 parser **only for proven shapes** and decouple trace identification from usage extraction so
identifiable traces are imported (not dropped); (C) add a configurable import range + per-env incremental
cursor + resumable atomic backfill; (D) a grouped, actionable summary. Time-entry suggestion is **fenced
to TASK-030**.

All of this lives inside the component the architecture plan names **"Langfuse Importer — primary AI
evidence source … import trace timestamps, usage/cost fields, metadata/session IDs … compute health
state"** (`03_architecture_plan.md:89`). It operationalizes currency tracking the plan already specified —
*"record trace health: last successful import, latest trace timestamp …"* (`03_architecture_plan.md:144`)
— which the code never realized (the window was hard-coded `recent_window(7)`, cursor unused for `from`).

## 2. Split evaluation (required by the task)

**No split required.** Reasoning:

- **Component cohesion.** A, B, C, D are all inside the importer component boundary
  (`src-tauri/src/langfuse/*`) plus its IPC/settings/report-rendering edges. No workstream crosses into
  capture, the runtime observer, the classifier, env→project mapping, or CSV export. There is no
  `03_architecture_plan.md` boundary that A–D straddle, so the "stop and split on a boundary crossing"
  rule does not trigger.
- **Tight sequencing, not separable scope.** A **gates** B (diagnose before widening — the task's explicit
  intent and the safe order); D **renders** A's output; C **reuses** the same engine and only pays off
  once B stops the parser dropping everything. Splitting them would create artificial hand-offs across a
  single data-flow.
- **Mirrors the proven TASK-027 shape** (one change, sequenced workstreams A→E), which shipped cleanly.

**Heaviest piece = C4 (chunked resumable backfill).** It is genuine engineering (it reshapes *when*
`persist_import_run` commits — see §4). It is still **within** the importer component, so it does not force
a split now. **Contingency, not a required split:** if SW-2 finds C4 disproportionately large, it may be
carved to an immediate follow-up (TASK-029b) **after** A/B/D land — but A/B/D are the correctness core and
must ship together. I am **not** mandating that split; I flag it as the only plausible fault line.

**TASK-030 (time-entry suggestion) is correctly out of scope** and does **not** need to be pulled in:
`langfuse_ai_evidence` already carries the substrate (`session_id`, `ai_start_ts`/`ai_end_ts`, tokens,
`cost_total`, health — `model.rs:276`). TASK-029's job is to make that substrate trustworthy; turning it
into *suggested time entries* is a different competency (suggestion engine + review UI) and a high-risk
attribution surface (DEC-001/006) that must not be coupled to an import-correctness task. **Architect
ruling: not necessary here.**

## 3. Architectural-consistency findings (the design is sound)

- **F-1 (key insight, endorsed): decouple identification from usage parsing.** The root fault is not only
  the `observations` field shape — it is that `serde_json::from_value::<Trace>` (`importer.rs:222`) makes
  the *whole* trace all-or-nothing, so one peripheral field drops a trace we could clearly identify. The
  design's principle — *an identifiable trace (id + ts) is always imported; usage is read separately and
  degrades to `schema_changed` when unreadable* — is the **durable** fix and aligns with the plan's
  "absence is a warning, not zero" (`03_architecture_plan.md:89`). Endorsed as the primary architectural
  move, above a narrow "just fix observations" patch.
- **F-2 (diagnose-before-widen ordering, endorsed).** Workstream A must **confirm** the dominant skip
  reason against the live stack (VF-1) before B changes parsing. The `observations_not_embedded`
  (v3 ID-list list shape) hypothesis has a strong numeric fit (611 with-observations fail; 29
  observation-less parse as duplicates) but is **explicitly held as a hypothesis**, not baked into B. Good
  — this is exactly the forensic posture the task asked for.
- **F-3 (cursor model aligns with the plan).** Per-env incremental `from = cursor − overlap`, non-
  regressing cursor, durable `(env,trace_id)` dedupe — all already half-present in the code
  (`store.rs:161,213`, `importer.rs:277`); the change finishes wiring them to the import window. No new
  data model.
- **F-4 (no taxonomy growth).** Stays within the ten-state health taxonomy (`model.rs:11`); `schema_changed`
  is *narrowed* to genuine cases, not extended. Correct.

## 4. NFR / risk implications addressed

- **Atomicity vs. backfill (the real design risk).** `persist_import_run` commits one run atomically at the
  end (`store.rs:135`). A naïve full-history backfill would hold all evidence in memory, hold a long write,
  and **lose everything** on interruption/timeout. The design's **chunked atomic-per-window backfill**
  resolves this: each chunk is its own atomic run advancing the cursor, so progress is durable and a re-run
  resumes. This **preserves** the S-3 invariant and the TASK-021 in-band persist-failure surfacing rather
  than weakening them. This is the load-bearing NFR decision and the design handles it correctly.
- **Performance / N+1.** Per-trace `get_observations` (`importer.rs:321`) is N round-trips; at backfill
  scale that is the dominant cost. Design recommends a windowed observations scan joined by `traceId`
  (C6), and — critically — requires the N+1 cost be **acknowledged/logged** if deferred, never hidden
  (no-silent-caps). Acceptable for a single-user local prototype; flagged, not ignored.
- **Bounded wait.** Backfill uses a larger `run_bounded_result` ceiling (`lib.rs:764`); because backfill is
  chunked-durable-resumable a timeout is non-destructive. UI never freezes (off-thread, serialized via
  `import_lock`). Consistent with the existing manual-import posture.
- **Storage growth.** Backfilling 30/90/all days grows `langfuse_raw_traces` (which may hold
  prompt/session/metadata under the DEC-020 MVP relaxation). See `feedback_to_ba[]` F2 — this is the one
  reality-vs-architecture divergence worth BA's attention (retention is specified but unrealized). It does
  **not** block TASK-029 (these payloads are local-only and already stored today; backfill stores more).

## 5. L2 threshold sign-off

| L2 threshold | Verdict | Where enforced |
| --- | --- | --- |
| Secret-free diagnostics | ✅ | SEC-011: fixed reason labels + counts; structural samples = key names + JSON type names only; **no** `serde` error pass-through (it can embed values). `design.md` §3.2; spec scenarios "Diagnostics and samples carry no secrets". |
| No raw payload/body/prompt/session content in UI/logs/reports | ✅ | Classifier inspects `Value` structurally, never serializes content; report excludes per-trace `evidence`; D renderer shows grouped labels. Extends SEC-003/010. |
| Loopback / explicit-cloud boundary unchanged | ✅ | Backfill/incremental/discovery all via `ImporterConfig::build_url` under `/api/public/*`; `local`→loopback, `cloud`= only off-host path; GET-only; **no new egress host**. SEC-002 untouched. |
| Compat / rollback documented | ✅ | Additive `settings` row; no schema change; default range 7d→30d behaviour note; inert-to-older-build rollback. `design.md` §7, tasks X4 (RELEASE.md/SW-6). |
| Absence ≠ zero | ✅ | Widening never coerces missing→0; unreadable usage → `schema_changed`, not a zero total. |
| No new health state | ✅ | Ten-state taxonomy unchanged; `schema_changed` narrowed. |

## 6. `feedback_to_ba[]` (routed via Pi-Assistant to BA-flow Architect)

- **F1 — new ADRs for the canonical decision log (proposed):**
  - **DEC-030** — configurable import range + per-env incremental cursor + resumable chunked backfill
    (replaces fixed 7-day window; importer stays sole AI cost/time authority, DEC-003/017 unchanged).
  - **DEC-031** — forensic secret-free schema diagnostics + decoupled trace identification (identifiable
    traces imported as `schema_changed` when usage unreadable, never silently dropped).
  - **SEC-011** — schema-diagnostics secret-free invariant (reason labels + JSON key/type names + counts
    only; no payload bytes / values / `serde` strings / prompt/session/metadata). Extends SEC-003/SEC-010.
- **F2 — retention divergence (non-blocking, recommend BA schedule a task):** DEC-007 specifies "short
  configurable retention" and the data model has `retention_until` (`03_architecture_plan.md:196`), but
  `langfuse_raw_traces` is **never pruned** today. Backfill **amplifies** this unrealized gap (more raw
  payloads, which may contain prompt/session content, accumulate locally). Recommend a follow-up
  raw-trace-retention task. Not in TASK-029 scope; surfaced so BA owns the retention boundary.
- **F3 — TASK-030 readiness note:** TASK-029 makes `langfuse_ai_evidence` correct/complete; TASK-030 will
  turn it into suggested time entries under DEC-006 (suggestion-first) / DEC-003 (Vire authoritative). No
  time-entry logic is built in TASK-029.

These are **proposed** decisions routed for canonicalization (same pattern as TASK-027 DEC-027/028/029);
they do **not** make this an `escalate-to-ba` — nothing in the change contradicts a *locked* BA decision.
The fixed 7-day window is an implementation default, not a BA decision; widening it realizes the plan's
currency model rather than opposing it.

## 7. Handoff

Routes to the **Backend Developer** for Workstreams A → B → C, then the **Frontend Developer** for
Workstream D, in that order (A gates B; D renders A/D output). Verification flags VF-1..VF-4
(`design.md` §8) are SW-2 acceptance gates — VF-1 in particular **must** confirm the dominant skip reason
against the live stack before B widens any parsing. SW-4 (Code Reviewer) escalates here on any design-level
concern, especially around the chunked-backfill atomicity (§4) and the SEC-011 secret-free guarantee.
