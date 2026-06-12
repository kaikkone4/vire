# Documentation Review — TASK-020 Langfuse Importer Hardening + L2 Release Hygiene

- **Documentation Engineer:** SW-6
- **Change:** `task-020-langfuse-importer-hardening`
- **Branch:** `feat/task-020-langfuse-importer-hardening` · **PR:** #12
- **Tier:** L2 · **Gate context:** SW-6 docs-drift check after SW-4 PASS (`review.md`) and SW-5 PASS (`sec.md`)
- **Date:** 2026-06-12
- **Verdict:** **PASS — no docs drift**

---

## Scope reviewed

| Artifact | Description |
|---|---|
| `README.md` | Intro status line, §Local Langfuse Docker stack, §AI trace import, §Availability and UX, §Privacy status |
| `docs/langfuse-local-setup.md` | Health states table, Vire import endpoint configuration, Docker/Langfuse availability section |
| `docs/backup-restore.md` | Backup scope, schema listing |
| `openspec/changes/task-020-langfuse-importer-hardening/specs/langfuse-importer/spec.md` | TASK-020 spec delta (4 ADDED requirements) |
| `openspec/changes/task-020-langfuse-importer-hardening/design.md` | Architect design: A1 + S-3 through S-6 |
| `openspec/changes/task-020-langfuse-importer-hardening/review.md` | SW-4 code review (PASS) |
| `openspec/changes/task-020-langfuse-importer-hardening/qa.md` | QA gate (PASS, 39/39) |
| `openspec/changes/task-020-langfuse-importer-hardening/sec.md` | Security review (PASS) |
| `src-tauri/src/langfuse/importer.rs` | `persist_run`, `now()`, `PERSIST_FAILURE_MSG` |
| `src-tauri/src/lib.rs` | `import_langfuse_now`, `run_bounded`, `IMPORT_TIMEOUT_MSG`, `IMPORT_TIMEOUT_SECS` |
| `src-tauri/src/langfuse/store.rs` | `persist_import_run` transaction entry point |

---

## 1. Change characterization — why docs drift is not expected

TASK-020 is a **hardening + release-hygiene** change, not a new capability. The proposal and
arch-review both state this explicitly. No new source, classifier, runtime observer, review UI, or
user-configurable surface is introduced. Specifically:

| Change | User/operator-visible? | Docs implication |
|---|---|---|
| **A1** — `Cargo.lock` now committed | No (build infrastructure; `npm run tauri:build` is unchanged) | None |
| **S-3** — atomic persistence per run | No (internal SQLite transaction; same row contract) | None |
| **S-4** — persistence failures surface as `unknown` with secret-free warning | Yes (UI sees `unknown` instead of a silently clean state on DB failure) | Covered — `unknown` state is already documented |
| **S-5** — `now()` emits UTC RFC3339 instead of local-time strings | No (column values change format; no schema or API shape change; no user-visible field affected) | None |
| **S-6** — `import_langfuse_now` bounded at 30 s | Yes (on hang, UI gets a bounded error instead of blocking forever) | Covered — error message encodes absence-≠-zero; `unknown` is already documented |

---

## 2. Per-artifact drift check

### 2.1 README.md

**Status line:** "The local Docker Langfuse trace importer MVP is available (TASK-019); automatic
macOS activity capture is in active development (TASK-005)." — still accurate. TASK-020 adds no new
user-visible capability; it is a correctness/reliability hardening of an already-shipped feature. No
update required.

**§Availability and UX:** States that Vire reports `unavailable`, `stale`, or `unknown` when the
stack is not running. `unknown` is among the listed states. No new state is introduced. **PASS.**

**§Local Langfuse Docker stack, §AI trace import, §Privacy status, §Components table, §Object
storage:** All reference setup/configuration information unchanged by TASK-020 (no new env var, no
new port, no schema change, no new Docker service). **PASS.**

### 2.2 `docs/langfuse-local-setup.md`

**Health states table:** All 10 states are present (populated by the TASK-019 D-3 fix). TASK-020
introduces no new state — the 10-state taxonomy is fixed (BA §7; preserved as constraint #8 in
arch-review §5). The `unknown` row reads:

> `unknown` — Cannot determine whether local Langfuse state is current or complete | AI totals shown
> with unknown-source flag; user prompted to check stack

This covers both new observable outcomes from TASK-020:
- **S-4** persistence failure — degrades to `HealthState::Unknown`, summary warning in snapshot
- **S-6** import timeout — returns `IMPORT_TIMEOUT_MSG` ("AI usage and cost are unknown, not zero"),
  a non-healthy IPC error that the UI surfaces the same way as any other non-healthy response

No additional row needed. **PASS.**

**Vire import endpoint configuration table:** Env vars (`VIRE_LANGFUSE_*`) unchanged — no new
variable, no default change. **PASS.**

**§Docker/Langfuse availability prose:** Reads "Vire will not interpret missing or unavailable
traces as zero AI usage or cost." — TASK-020 reinforces this invariant (S-3/S-4 close a silent
partial-failure path). Prose remains accurate; no update needed. **PASS.**

### 2.3 `docs/backup-restore.md`

No schema change (no new table, no `ALTER`/`DROP`). S-5 is a string-value change to three existing
`TEXT` columns, not a structural schema change. Backup scope, volume list, and divergence failure
modes are unchanged. **PASS.**

### 2.4 Cargo.lock / build documentation

`README.md §Build` instructs operators to run `npm install && npm run tauri:build`, which invokes
`cargo build` internally. Now that `Cargo.lock` is committed, that internal invocation uses a pinned
closure (reproducible build) automatically. Operators do not invoke `cargo` directly; no README
change is needed. **PASS.**

---

## 3. OpenSpec / implementation coherence

| Criterion | Status |
|---|---|
| `specs/langfuse-importer/spec.md` present and non-empty | **PASS** |
| All 4 ADDED requirements present: atomic persistence (S-3), surfaced failures (S-4), UTC RFC3339 timestamps (S-5), bounded import IPC (S-6) | **PASS** |
| S-3 implemented: `store::persist_import_run` wraps one transaction per run (`store.rs:102–118`) | **PASS** (SW-4 review §S-3; QA §S-3) |
| S-4 implemented: `PERSIST_FAILURE_MSG` const; `summary.health = Unknown`; marker run inserted (`importer.rs:392–436`) | **PASS** (SW-4 review §S-4; QA §S-4; SW-5 §3.2) |
| S-4 error text secret-free (SEC-003): fixed string, no driver/config material interpolated | **PASS** (SW-5 §3.2) |
| S-5 implemented: `now()` → `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` (`importer.rs:50–52`) | **PASS** (SW-4 review §S-5; QA §S-5) |
| S-5 ordering invariant: `'T'` > `' '` → no backfill needed | **PASS** (design.md §5; QA §S-5 ordering test) |
| S-6 implemented: `run_bounded` with 30 s ceiling; `IMPORT_TIMEOUT_MSG` fixed const (`lib.rs:184–216`) | **PASS** (SW-4 review §S-6; QA §S-6) |
| No 11th health state introduced | **PASS** (confirmed by QA §S-6 and SW-4 §S-6) |
| A1: `Cargo.lock` committed (`f8fd591`); no `cargo update`; OSV/Trivy clean | **PASS** (SW-4 §A1; SW-5 §3.1) |
| Absence-≠-zero invariant preserved and reinforced | **PASS** — S-3/S-4 remove silent-failure→false-zero path; IMPORT_TIMEOUT_MSG explicitly says "unknown, not zero" |
| No credential material in any new surfaced string | **PASS** (SW-5 §3.2, §3.4) |
| 10-state taxonomy unchanged; DEC-020 posture preserved | **PASS** (SW-5 §3.5; arch-review §5) |
| `tasks.md` all required subtasks checked off; optional real-stack smoke skipped per design.md §7 | **PASS** |

---

## 4. Non-blocking notes

**NB-01 — Timeout ceiling not exposed as a configuration option.**
`IMPORT_TIMEOUT_SECS = 30` is a hardcoded const. At L2 this is correct — no documentation surface
needed. If the value becomes configurable (e.g. for slow network environments), `docs/langfuse-local-setup.md`
§Vire import endpoint configuration should gain a new row for it at that time.

**NB-02 — SW-4 non-blocking suggestions S1–S3 from TASK-020 review.**
S1 (marker-insert double-discard annotation), S2 (`started_at` == `finished_at` semantic drift),
S3 (`run_import` return value unmarked discard) have no docs implications; they are implementation
polish items for a follow-up pass.

**NB-03 — Pre-existing `csv_export_neutralizes_formula_like_project_names_and_notes` test failure.**
Out of scope; classified as pre-existing in SW-3 QA, SW-4 review, and design.md §8. Not a TASK-020
docs item.

**NB-04 — SW-5 carry-forward advisories A2/A3.**
A2 (pre-existing RUSTSEC transitive notices, max CVSS 6.9) and A3 (raw trace payload at-rest) carry
forward from TASK-019. Neither has docs implications at this tier.

---

## 5. Verdict

**PASS — no docs drift.**

TASK-020 is a pure internal hardening. All four spec requirements (S-3, S-4, S-5, S-6) are
implemented, tested (39/39 pass), and reviewed. No new user-visible capability, no new env var, no
new health state, no schema change. `README.md`, `docs/langfuse-local-setup.md`, and
`docs/backup-restore.md` are accurate against the released state. OpenSpec spec delta, design, and
implementation are coherent. No documentation update was required or made; this gate pass is
attestation only.
