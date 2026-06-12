# Architecture Review (SW-1) — TASK-020 Langfuse importer hardening + L2 release hygiene

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-020-langfuse-importer-hardening`
- **Branch (proposed):** `feat/task-020-langfuse-importer-hardening` · **base:** `main`
- **Tier:** L2 · **Gate context:** SW-1 task-design/scoping review before the developer role
  implements the TASK-019 carryover hardening.
- **Date:** 2026-06-12
- **Verdict:** **PASS** — one cohesive change inside the single **Langfuse importer** component
  (plus one repo-level lock file). No component boundary is crossed; **not split-required**. No BA
  escalation (no architecture decision is reopened; DEC-020/DEC-022 posture is preserved and
  reinforced). No new `feedback_to_ba[]` item.

---

## 1. Inputs read

- TASK-019 `review.md` (SW-4): §9 non-blocking suggestions **S-3, S-4, S-5, S-6** and §10 carryover;
  carryover **A1** (Cargo.lock).
- TASK-019 `design.md` §5 (importer-owned persistence), §6 (10-state taxonomy), §7 (privacy posture);
  `proposal.md`, `specs/langfuse-importer/spec.md` (the requirements being hardened).
- Current `main` runtime: `src-tauri/src/langfuse/importer.rs` (`persist_run` 384–420, `now()` 45–46),
  `store.rs` (persistence helpers + `latest_run*` ordering), `mod.rs` (`run_blocking_import`),
  `src-tauri/src/lib.rs` (`import_langfuse_now` 181–191).
- Build state: `src-tauri/Cargo.lock` present (~125 KB) but **untracked**; root `.gitignore` ignores
  `src-tauri/target/`, **not** `Cargo.lock`. No CI workflow / SBOM tooling exists yet (SBOM appears
  only as a RELEASE.md concept) — A1 is the prerequisite input for any future SBOM/audit path.

## 2. Findings the developer must honor

1. **Atomicity unit is one import run (S-3).** Persistence must wrap a single run's writes (raw +
   evidence + run record) in one transaction. Keep the SQL in `store.rs` behind a transactional entry
   point (`conn.unchecked_transaction()`), reusing the existing `upsert_*`/`insert_*` helpers on the
   `&tx`. Do **not** widen the unit to cross-environment or cross-run atomicity — that would fight the
   per-environment cursor model.
2. **Surfacing failure protects absence-≠-zero (S-4).** The current `let _ = …` swallow is not just
   untidy — a discarded DB error lets a failed/partial write present as a clean state, which is a
   latent **absence-≠-zero** violation (a failed write reading as a healthy zero). Propagate the error
   (preferred; the IPC already maps `run_blocking_import` errors to a secret-free string) **or** record
   a secret-free warning + resolve `status = unknown`. Either way the failure must be observable and
   must never read `healthy`/zero. Error text stays secret-free (SEC-003).
3. **Timestamp normalization has an ordering-key implication (S-5).** `now()` feeds `finished_at`,
   the `ORDER BY finished_at DESC` "latest run" key. Switching to
   `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` is safe across the format transition: `'T'`
   (0x54) sorts after `' '` (0x20), so every new RFC3339 row out-sorts every legacy local-format row →
   "latest run" stays correct with **no data migration**. The developer must keep the change a
   string-value change only (columns are already `TEXT`; no `ALTER`/`DROP`).
4. **Bound the IPC without growing the taxonomy (S-6).** Keep the dedicated-thread blocking model
   (the blocking `reqwest` client must stay off the Tauri async runtime), but replace the unbounded
   `.join()` with `mpsc` + `recv_timeout` (~30 s, above the reqwest 15 s/5 s ceilings). A timeout maps
   to an existing non-healthy signal or a surfaced IPC error — **no 11th health state**; the BA §7
   ten-state taxonomy is fixed.
5. **A1 is build hygiene, not capability.** Commit `Cargo.lock` with **no** `cargo update` — pin the
   exact closure TASK-019 validated (incl. the `reqwest`/`rustls` TLS stack). It is the SBOM/audit
   input the release path needs; it gets a tasks.md entry and a release note, **no** spec delta.

## 3. Split analysis — one task, not split-required

Per role working rules, "split-required" means scope would cross component boundaries defined in
`03_architecture_plan.md`. It does not. Every code change is internal to the single **Langfuse
importer** component; the one non-code change is a repo-root lock file.

| Listed piece | Component / locus | Boundary crossing? |
| --- | --- | --- |
| Commit `Cargo.lock` (A1) | Repo build hygiene (`src-tauri/Cargo.lock`) | No (not a component) |
| Transaction-wrap persistence (S-3) | Langfuse importer (`store.rs`/`importer.rs`) | No |
| Surface DB errors (S-4) | Langfuse importer (`importer.rs` → `mod.rs` → IPC) | No |
| UTC RFC3339 timestamps (S-5) | Langfuse importer (`importer.rs::now`) | No |
| Bounded import IPC (S-6) | Langfuse importer status surface (`lib.rs` command) | No |
| Optional mocked/local smoke | Langfuse importer tests | No |

The macOS capture adapter, AI runtime observer, classifier, review UI, and CSV exporter are untouched.
**Verdict: one cohesive change** with an internal implementation sequence (`tasks.md`), not a split.

## 4. Boundary touchpoints (contracts/dependencies, not splits)

1. **SQLite store (importer-owned tables).** S-3/S-4 change *how* the importer writes (transaction +
   error handling) and S-5 changes the *value* of three `TEXT` timestamp columns. **No schema change**
   — no new table, no `ALTER`/`DROP`. The `finished_at` ordering key tolerates the legacy→RFC3339
   transition (§2.3); no backfill.
2. **`import_langfuse_now` IPC (status surface).** S-6 bounds the existing read-only command's wait;
   the `SourceHealthSnapshot` DTO contract is unchanged (still secret-free). The thin frontend banner
   is untouched (S-7 banner tuning is deferred to TASK-009).
3. **TASK-006 AI runtime observer (downstream consumer).** TASK-006 reconciles against the importer's
   health states and run records. TASK-020 makes those records **more** trustworthy (atomic, failures
   surfaced, uniform timestamps) — a strengthening of the interface TASK-006 will consume, with no new
   surface. TASK-020 must **not** build the observer or any reconciliation/ledger (DEC-017).

## 5. Constraints explicitly preserved (DEC-020 / TASK-019 §5)

| # | Constraint | Status under TASK-020 |
| --- | --- | --- |
| 1 | Local Docker self-hosted Langfuse is the default source | Unchanged (no source/config change) |
| 2 | Cloud is explicit non-default override only (sole off-host egress) | Unchanged |
| 3 | Renderer off-network (CSP/capabilities untouched) | Unchanged (no webview/capability change) |
| 4 | Loopback `127.0.0.1:3000` default; no LAN binding | Unchanged |
| 5 | No credential/secret in logs/evidence/exports | **Reinforced** — S-4 error text must stay secret-free |
| 6 | No raw macOS-activity egress | Unchanged (no activity path touched) |
| 7 | **Absence is never zero** | **Reinforced** — S-3/S-4 remove a silent-failure→false-zero path |
| 8 | 10-state health taxonomy (BA §7) is fixed | **Preserved** — S-6 adds no 11th state |
| 9 | Local prompt/session/metadata allowed for MVP; redaction deferred | Unchanged (no payload-handling change) |

No decision is reopened. DEC-019/DEC-020/DEC-022 stand; this change is implementation hardening only.

## 6. Adjacent item deliberately left out of scope

- **S-2** (`importer.rs` compares Langfuse-**emitted** trace timestamps lexicographically). It is
  adjacent to S-5 but **distinct**: S-5 normalizes the importer's *own* timestamps; S-2 concerns
  *externally-supplied* values, correct today because Langfuse emits `…Z` UTC (empirically validated,
  TASK-019 `design.md` §4). It is **not** in TASK-020's stated scope. Noted as an optional near-zero
  cost win (reuse `parse_ts()`) **only if** the developer is already in that file; it must not expand
  the change. (See `design.md` §6.)
- **S-1** (dead `_config` param), **S-7** (`degradedHealth` banner set → TASK-009), **S-8** (per-env
  snapshot selection) — out of scope; deferred polish.
- Pre-existing `csv_export_*` adversarial failure — separate time-tracker-core `csv_escape` defect, a
  distinct follow-up, not importer scope.

## 7. `feedback_to_ba[]`

None new. The two TASK-019 items (DEC-019 ratification; TASK-004 schema-slice ownership) remain open
with BA and are unaffected by this hardening change — TASK-020 neither resolves nor depends on them.

## 8. Recommendation — next role and branch

- **Change name:** `task-020-langfuse-importer-hardening` (this dir). `openspec validate --strict`
  passes (verified 2026-06-12).
- **Branch:** `feat/task-020-langfuse-importer-hardening`, base `main` (TASK-019 PR #11 merged at
  `64d5f9f`; no merge-order dependency).
- **Next role (primary):** **backend-developer (Rust/Tauri)** — owns A1 (commit lock), S-3 transaction
  wrap in `store.rs`, S-4 error propagation/recording, S-5 `now()` normalization, S-6 bounded IPC in
  `lib.rs`, and the mock-based regression/smoke tests. All work is Rust-core + one repo-root file.
- **Supporting role (conditional):** **integration-engineer** only if the **optional real** local
  Docker Langfuse smoke is attempted (stack availability, loopback read-only run). The default mocked
  smoke is backend-developer's; a real stack is not a gate.
- **Then:** SW-3 (QA, per `design.md` §7), SW-4 (code review — verify S-3…S-6 closed and absence-≠-zero
  preserved), SW-5 (security — confirm S-4 error text stays secret-free, no new egress), SW-6 (release
  — `Cargo.lock` now SBOM-auditable; reproducible build).

## 9. Verdict

**PASS.** TASK-020 is one cohesive hardening + release-hygiene change inside the single
Langfuse-importer component boundary — **not** split-required and **not** a BA escalation. It closes
TASK-019 carryovers A1/S-3/S-4/S-5/S-6, materially reinforces the absence-≠-zero and no-secret-exposure
invariants, preserves the DEC-020 posture and the fixed 10-state taxonomy, and adds no new source,
classifier, runtime observer, or review UI. Deliverables for developer handoff (`proposal.md`,
`design.md`, `tasks.md`, `specs/langfuse-importer/spec.md`, this review) are in place and validate
strict. Route to **backend-developer (Rust/Tauri)** on `feat/task-020-langfuse-importer-hardening`.
