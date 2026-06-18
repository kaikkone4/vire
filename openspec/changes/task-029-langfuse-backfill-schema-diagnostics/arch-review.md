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

---

# 8. SW-4 escalation redesign — backfill page-limit continuation (DEC-032)

**Context.** SW-4 code review **FAILed twice on the same theme** (`review.md`): the page-limited backfill
continuation **skips unread equal-timestamp traces** and **loops / fails to progress** when the boundary
instant is saturated or carries no parseable timestamp. Per the Pi QV fix-loop rule (two substantive
failures on one theme), this is escalated to the SW Architect for a **redesign decision before any further
backend fix**. This section is that decision. It supersedes the continuation **mechanism** in `design.md`
§4.3 (the "second-oldest distinct instant" / `min_ts2` / exclusive-`toTimestamp` scheme). The
**requirements** (`specs/langfuse-importer/spec.md`) are mechanism-agnostic and do **not** change.

## 8.1 Root cause — the failed mechanism fights the API's own boundary semantics

The current scheme walks each window **newest → oldest** (descending) and, on hitting the page backstop,
resumes the next run with an **exclusive** `toTimestamp` set to a *derived* boundary (the oldest, or
"second-oldest distinct", instant reached). Because `toTimestamp` is exclusive, an exclusive ceiling at an
instant **excludes everything at that instant** — so to avoid skipping the cut instant the code must
finesse an off-by-one-instant via `min_ts2`. That finesse:

- **breaks under equal-timestamp saturation** — when the page-limited window collapses to **one** distinct
  instant, `min_ts2` is `None`, the boundary falls back to that single instant, and the exclusive resume
  scans *strictly below it*, permanently dropping every unread trace at the cut instant (`importer.rs:921`,
  `:471`; the regression at `tests.rs:2432` *documents* this data loss);
- **has no key for a no-parseable-timestamp run** — `note_oldest_instants` ignores such entries, so a
  page-limited run yields no boundary; the driver then either preserves a stale boundary (repeats the same
  window forever) or, with none, restarts from `now` (`importer.rs:810`, `:921`, `:546`).

This is not an implementation slip to patch a third time — it is the **wrong continuation primitive** for
this API. The fix is to stop deriving an exclusive boundary at all.

## 8.2 Verified Langfuse API semantics (the decision rests only on these)

`GET /api/public/traces` — confirmed against the canonical Fern API definition on `langfuse/langfuse@main`
and the API docs (sources below). Each fact is marked **VERIFIED** or **UNVERIFIED**; nothing about cursor
or order is guessed.

| # | Fact | Status | Source |
| --- | --- | --- | --- |
| V1 | `fromTimestamp` filters `trace.timestamp` **on or after** the value — **inclusive (≥)**. | **VERIFIED** | Fern `trace.yml`; Public API docs |
| V2 | `toTimestamp` filters `trace.timestamp` **before** the value — **exclusive (<)**. | **VERIFIED** | Fern `trace.yml`; Public API docs |
| V3 | `orderBy` is supported, format `[field].[asc\|desc]`; fields include `timestamp` **and** `id`; example `timestamp.asc`. | **VERIFIED** | Fern `trace.yml` |
| V4 | Pagination is **offset-based** (`page` from 1, `limit`); response `meta` carries `page/limit/totalItems/totalPages`. **No cursor / page-token exists.** | **VERIFIED** | Fern `trace.yml`; Public API docs |
| V5 | There is **no documented default sort order** when `orderBy` is omitted, and **no documented compound/secondary sort** (single field only) — i.e. **no guaranteed stable tie-breaker** for equal `timestamp` rows. | **VERIFIED (as a documented absence)** | Fern `trace.yml` (no default/secondary documented) |
| V6 | There is **no documented hard cap** on `page`/`limit`; the `limit` doc warns large pages cause "api issues" (deep offsets are slow, ClickHouse-backed). | **VERIFIED (as a documented absence + warning)** | Fern `trace.yml` |
| V7 | A Nov-2025 **advanced `filter`** (JSON) param supports datetime/string/number/array filters; **exact operators and which fields (e.g. `id` greater-than) are filterable were NOT confirmed**, and self-hosted v3 deployments may predate it. | **UNVERIFIED** — must NOT be depended on | Advanced-filtering changelog (operators/fields not in excerpt) |

**Decisive corollaries.**
- From **V1+V2**: the API's *inclusive* bound is `fromTimestamp`; its *exclusive* bound is `toTimestamp`.
  A continuation cursor that moves the **inclusive** bound (`from`) **re-reads the boundary instant for
  free**; one that moves the **exclusive** bound (`to`) must invent the `min_ts2` off-by-one. The failed
  design picked the exclusive bound. **Switch to the inclusive bound.**
- From **V4+V5**: a true **keyset cursor** (resume "after id X" on a stable total order) is **not
  available** — there is no page-token, and `orderBy` gives no guaranteed tie-breaker. So "persist a stable
  tie-breaker / page token" (one option SW-4 floated) is **not implementable on the verified API**. The
  redesign therefore must **not need** a tie-breaker.
- From **V1/V2** again: a backfill always passes **both** `fromTimestamp` and `toTimestamp`, so the server
  only returns traces it can place in the window — i.e. **every windowed trace has a server-comparable
  `timestamp`**. A genuinely timestamp-less trace is *never in a time-windowed result set at all*. The
  "page-limited run with no usable timestamp" case can therefore only arise from an **importer-side parse
  failure of a value the server accepted** (millisecond precision / offset), which is fixed by robust
  parsing — not from a real timestamp-less trace.

## 8.3 Decision (DEC-032): ascending sweep with an inclusive `fromTimestamp` resume-cursor + full-boundary-instant re-read

Replace the descending / exclusive-`toTimestamp` / `min_ts2` continuation with:

1. **Order every trace-import page request explicitly `orderBy=timestamp.asc`** (oldest → newest). Do not
   rely on the undocumented default (V5). This is a fixed literal added to the existing
   `ApiPath::Traces` query builder — same allowlisted `/api/public/traces` path, **no new endpoint, no new
   egress host, GET-only** (SEC-002 untouched). Apply it to the shared `get_traces` call (harmless for the
   small incremental window; load-bearing for backfill).
2. **Continuation key = a single durable inclusive `fromTimestamp` cursor** — reuse the
   `langfuse_backfill_progress` row, reinterpreted as `resume_from` (still one UTC RFC3339 timestamp + the
   fixed marker key; SEC-011 surface **unchanged** — never serialized/rendered/logged, per `sec.md` §2.1).
3. **On a page-limited backfill run**, the new cursor is **`max_reached`** — the chronological **maximum**
   parseable `timestamp` the run actually returned (this is the *existing* `max_ts` the importer already
   computes for the incremental cursor; it does **not** need the oldest-instant tracking). Persist
   `resume_from = max_reached`.
4. **The next run resumes with `fromTimestamp = resume_from` (inclusive ≥, V1).** Because `from` is
   inclusive, the run **re-reads the entire boundary instant** from page 1; durable `(environment,
   trace_id)` dedup (`store.rs:213`) suppresses the already-imported overlap, and pagination then advances
   into strictly-newer history. **No tie-breaker is needed** because the whole boundary instant is
   re-scanned every run rather than resumed mid-instant — equal-timestamp traces at the cut are **fully
   re-read, never skipped.**
5. **A clean run that drains `[resume_from, now)` without hitting the backstop clears the cursor** (backfill
   complete; a later "Backfill now" starts fresh at the range floor). A hard-down stop preserves the cursor
   (unchanged from the Blocker-2 fix). Continuation-store read/write/clear faults still surface in-band via
   `PERSIST_FAILURE_MSG` (Blocker-2 fix retained verbatim).

This **deletes** `note_oldest_instants`, `min_ts`, `min_ts2`, and the `page_limit_floor_ts` second-oldest
derivation — i.e. it removes exactly the code that failed review twice. The chunked atomic-per-window commit
(S-3 / TASK-021) is **kept**, only re-ordered oldest → newest so the inclusive cursor advances forward; a
page-limit hit in any chunk sets `resume_from = max_reached` and stops the run.

### The one genuinely-unreachable corner — single-instant saturation — is detected and surfaced, never skipped or looped

Let reachable depth **D = MAX_PAGES × PAGE_LIMIT = 1000 × 50 = 50 000** traces per ordered window scan.
The only case the inclusive-from cursor cannot drain is a **single `timestamp` instant holding ≥ D traces**:
run 1 fills all D within that instant, so `max_reached == resume_from` and the cursor **cannot advance**.
The redesign handles this as an **explicit terminal state, not a silent skip and not a loop**:

- **Detect:** page backstop hit **AND** `max_reached == resume_from` (cursor did not advance past the
  window-start instant).
- **Do not skip:** never push the cursor past unread data (do **not** advance to `resume_from + ε` /
  exclusive-below — that is the exact data loss the failed design committed).
- **Do not loop:** do **not** present this as ordinary "re-run to continue" progress. Surface a **distinct,
  secret-free terminal diagnostic** (e.g. an `import_report` flag separate from `reached_page_limit`, with
  the **count** reachable — *no timestamp value*, preserving SEC-011) meaning "one instant exceeds the
  page-depth limit of D traces; it cannot be paged through this API". The run stops; the UI presents it as a
  capped/terminal condition, not an infinite continuation.

**Reachability in practice:** Langfuse `timestamp` is millisecond-precision ISO-8601 and the cursor stores
the source value verbatim (no second-truncation), so saturation needs **≥ 50 000 traces at one millisecond**
— physically unreachable for this single-user prototype (Janne's entire stack is 640 traces across 9
environments). The terminal-surface exists for **honesty and the invariant**, not because it will fire.

## 8.4 Invariant (state it precisely; the regressions must assert it)

> **Backfill completeness.** Every trace whose `timestamp`-instant is shared by **fewer than D = 50 000**
> traces is **eventually imported exactly once** (durable `(environment, trace_id)` dedup ⇒ exactly once).
> A single instant holding **≥ D** traces is surfaced via a **distinct, named, secret-free terminal
> diagnostic** — **never silently skipped and never infinitely re-run as if progressing.** Because no
> instant in this product approaches D, the operative guarantee is unconditional: **every trace is
> eventually imported exactly once.**

**Convergence proof (for the PR / test rationale).** With `from = F`, an ascending run imports the oldest
≤ D traces of `[F, now)`. If `|[F, now)| ≤ D` it drains fully and clears the cursor. Otherwise it imports
the oldest D, sets `from' = max_reached = M`. If `M > F` (the first instant has `< D` traces — always true
here), the un-imported remainder `[M, now)` is strictly smaller than `[F, now)` by `≥ D − |instant at M|`
new traces, so the process strictly decreases and terminates. If `M == F` (single instant `≥ D`) it is the
saturation terminal above. ∎

## 8.5 Required tests (replace the C4b / C7a saturation tests that assert data loss)

The current `page_limited_backfill_at_a_saturated_single_instant_advances_then_clears_without_looping` and
`page_limited_backfill_with_no_usable_timestamp_preserves_boundary_never_clears` **encode the defect** (they
assert the cut instant is skipped / the boundary merely preserved). **Delete them** and replace with:

1. `backfill_page_limited_resumes_forward_by_inclusive_from_cursor` — window > D across many instants; run 1
   hits the backstop and sets `resume_from = max_reached`; run 2 resumes inclusive and imports strictly-newer
   history; **assert the union over runs equals the full source set and every trace is imported exactly once
   (no skip, no duplicate row).**
2. `backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped` — a block of N (`< D`) traces
   sharing the boundary instant, only part of which fits in run 1's first D; run 2 (`from = boundary`,
   inclusive) **re-reads the whole instant**; assert **every** equal-timestamp trace is imported exactly once
   (this is the direct regression for SW-4's "skips unread equal-timestamp traces").
3. `backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping` — a mock instant with
   ≥ D traces at one timestamp; assert (a) the cursor is **not** advanced past unread data (no skip), (b) a
   **distinct** saturation diagnostic is surfaced (separate from ordinary `reached_page_limit`), (c) the
   run is terminal — re-running does **not** falsely report convergence/progress and the harness proves
   **no infinite loop** (bounded iteration count).
4. `backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor` — a trace with a
   millisecond/offset `timestamp` the server accepted parses and advances the cursor; a genuinely garbage
   value is still **imported** (identification decoupled, B1) but **excluded from the cursor**, and never
   triggers a restart-from-`now` (the all-unparseable degenerate, unreachable for a time-windowed scan, is
   surfaced rather than silently restarting).
5. Retain VF-3 convergence (`interrupt → re-run → no duplicate rows, cursor monotonic`) adapted to the
   inclusive-from cursor, and the SEC-011 negative test (cursor/diagnostic carry no timestamp value or
   secret).

## 8.6 OpenSpec / design amendments

- **`specs/langfuse-importer/spec.md` — NO CHANGE.** Its requirements are mechanism-agnostic ("persist
  progress in bounded atomically-committed units", "converging via env+trace-id de-duplication and a
  non-regressing cursor", "report bounded rather than silently truncating", "no trace imported twice"). The
  redesign satisfies every scenario; the saturation terminal is the spec's "reports it was bounded rather
  than silently truncating" specialized to the unreachable-instant case.
- **`design.md` §4.3 — AMEND** (done in this change): replace the "Page-limit continuation (SW-4 fix)"
  paragraph (descending / `min_ts2` / exclusive `toTimestamp` / second-oldest) with the DEC-032 ascending
  inclusive-`fromTimestamp` cursor + full-boundary-instant re-read + saturation-terminal. §4.2/§4.3 chunk
  ordering note flips newest→oldest to **oldest→newest**; §7 boundary-row wording updates `resume_to` →
  `resume_from` (inclusive) semantics; §8 VF-3 wording aligns to the inclusive cursor.
- **`tasks.md` C4a / C7a — AMEND** (done in this change): re-point from the `min_ts2` second-oldest-instant
  mechanism to DEC-032; list the replacement tests in §8.5.
- **DEC-032** is a **refinement of DEC-030's** continuation mechanism, fully **inside** the importer
  component — **no split, no `escalate-to-ba`** (it contradicts no *locked* BA decision; DEC-030/031 stand).
  Routed to the BA-flow Architect for the canonical log via `feedback_to_ba[]` **F4** below.

## 8.7 Considered & rejected

1. **Patch `min_ts2` a third time** (keep descending/exclusive-`to`) — rejected: re-touches the exact
   mechanism that failed review twice and keeps the harder-to-verify off-by-one-instant logic; the inclusive
   bound removes the need for it entirely.
2. **Stable keyset cursor on `(timestamp, id)` / a page-token** (SW-4's suggested "stable tie-breaker / page
   token") — rejected as **not implementable on the verified API**: there is no page-token (V4) and
   `orderBy` gives no guaranteed secondary sort (V5). The inclusive-from + full-instant-re-read + dedup
   design **obviates** the tie-breaker rather than requiring one.
3. **`orderBy=id.asc` keyset via an `id >` filter** — rejected/deferred: needs a verified `id` greater-than
   operator, which only the **UNVERIFIED** advanced `filter` (V7) might offer, and which self-hosted v3 may
   not support; it would also forfeit time-window chunking. See the optional spike (§8.8).
4. **Raise `limit` / `MAX_PAGES` to drain a saturated instant** — rejected as a *guarantee* (raises D but
   never makes it infinite; V6 warns large pages fail) — acceptable only as an optional mitigation, not the
   correctness mechanism.

## 8.8 Spike — not required to implement DEC-032; optional and bounded

DEC-032 rests entirely on **VERIFIED** facts (V1–V6); **no spike is needed to proceed**. A spike is
warranted **only if** single-instant saturation ever becomes real on a real deployment (it will not here):
a **time-boxed (≤ ½ day) spike** to confirm whether the advanced `filter` param (V7) exposes an `id`
greater-than operator with `orderBy=id.asc` on the *target* Langfuse version — if so, a true keyset cursor
makes even a saturated instant fully drainable, as a future optimization. Bounded, optional, post-TASK-029.

## 8.9 `feedback_to_ba[]` addition

- **F4 — DEC-032 (proposed):** backfill page-limit continuation uses an ascending `orderBy=timestamp.asc`
  sweep with an **inclusive `fromTimestamp` resume-cursor** and full-boundary-instant re-read (dedup ⇒
  exactly once), replacing the exclusive-`toTimestamp` second-oldest-instant scheme. Single-instant
  saturation (≥ 50 000 traces at one instant) is a **surfaced terminal** state, never a silent skip/loop.
  Refines DEC-030's mechanism; importer stays sole AI cost/time authority (DEC-003/017 unchanged); SEC-011
  surface unchanged. Routed for the canonical decision log.

## 8.10 Handoff (this escalation)

Back to the **Backend Developer (SW-2)** to implement DEC-032 in `src-tauri/src/langfuse/{importer.rs,
api.rs,config.rs,model.rs,store.rs,tests.rs}` (add `orderBy` to `ApiPath::Traces` + the `get_traces`
signature/mock; replace the `min_ts2` continuation with the inclusive-from cursor; add the saturation
terminal flag; swap the §8.5 tests). `design.md` §4.3/§4.2/§7/§8 and `tasks.md` C4a/C7a are amended in this
change to match. Re-run SW-3 (QA) → SW-4 (Code Review) ∥ SW-5 (Security — SEC-011 surface is unchanged, so
expect no new finding). **No split. No BA escalation.**

## 8.11 Sources

- Langfuse Fern API definition (canonical, `langfuse/langfuse@main`):
  `https://github.com/langfuse/langfuse/blob/main/fern/apis/server/definition/trace.yml` — `orderBy` field
  set + format, `fromTimestamp`/`toTimestamp` inclusive/exclusive descriptions, `page`/`limit` pagination,
  no cursor/page-token, no documented default/secondary sort.
- Langfuse Public API docs: `https://langfuse.com/docs/api-and-data-platform/features/public-api`.
- Langfuse API reference: `https://api.reference.langfuse.com/`.
- Advanced filtering changelog (V7, UNVERIFIED operators/fields):
  `https://langfuse.com/changelog/2025-11-03-advanced-filtering-traces-and-observations-api`.
