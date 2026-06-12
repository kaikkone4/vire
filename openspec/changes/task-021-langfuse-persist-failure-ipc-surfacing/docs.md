# Documentation Review — TASK-021 Surface Persist Failure to Import IPC, Marker-Independent

- **Documentation Engineer:** SW-6
- **Change:** `task-021-langfuse-persist-failure-ipc-surfacing`
- **Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing` · **PR:** #13
- **Tier:** L2 · **Gate context:** SW-6 docs-drift check after SW-4 PASS (`review.md`) and SW-5 PASS (`sec.md`)
- **Commit audited:** `99e1118`
- **Date:** 2026-06-12
- **Verdict:** **PASS — no docs drift**

---

## Scope reviewed

| Artifact | Description |
|---|---|
| `README.md` | §Availability and UX, §AI trace import, §Local Langfuse Docker stack, §Privacy status |
| `docs/langfuse-local-setup.md` | Health states table, §Docker/Langfuse availability, §Vire import endpoint configuration |
| `docs/backup-restore.md` | Backup scope, schema listing, restore procedure |
| `openspec/changes/task-021-langfuse-persist-failure-ipc-surfacing/specs/langfuse-importer/spec.md` | TASK-021 spec delta (MODIFIED surfacing requirement) |
| `openspec/changes/task-021-langfuse-persist-failure-ipc-surfacing/proposal.md` | Problem statement, what changes, impact |
| `openspec/changes/task-021-langfuse-persist-failure-ipc-surfacing/review.md` | SW-4 code review (PASS) |
| `openspec/changes/task-021-langfuse-persist-failure-ipc-surfacing/qa.md` | QA gate (PASS, 40/40) |
| `openspec/changes/task-021-langfuse-persist-failure-ipc-surfacing/sec.md` | Security review (PASS) |
| `src-tauri/src/langfuse/mod.rs` | `run_blocking_import`, `import_result` helper |
| `src-tauri/src/langfuse/importer.rs` | `pub const PERSIST_FAILURE_MSG`, doc comment on best-effort marker |
| `src-tauri/src/langfuse/tests.rs` | `persist_failure_surfaces_in_band_even_when_marker_write_also_fails` |

---

## 1. Change characterization — why docs drift is not expected

TASK-021 is a **backend-only correctness hotfix** to one component (`langfuse` importer + its single IPC command). The proposal and arch-review explicitly state: no new source, no schema change, no new health state, no renderer change, no new IPC command, no new dependency.

| Change | User/operator-visible? | Docs implication |
|---|---|---|
| `PERSIST_FAILURE_MSG` made `pub const` | No (visibility change within the Rust crate) | None |
| `run_blocking_import` now inspects summaries and returns `Err` on persist failure | Yes (in the rare edge case where both the run write and marker write fail, the import command now returns a non-healthy error instead of a stale `healthy` snapshot) | Covered — `unknown` health state already documented; this is a correctness fix, not a new capability |
| `import_result` private helper | No (private to `langfuse` module) | None |
| Regression test added | No | None |
| Marker stays `let _` best-effort | No (no change to marker behavior) | None |

The one user-observable behavioral change is a **correctness fix**: under total DB unwritability during a manual import, the user now sees a non-healthy error (correct) instead of a false-healthy response (incorrect). The `unknown` health state this produces is already documented. The absence-≠-zero invariant was already documented and this fix enforces it in a previously uncovered edge case.

---

## 2. Per-artifact drift check

### 2.1 README.md

**Status line:** "The local Docker Langfuse trace importer MVP is available (TASK-019); automatic macOS activity capture is in active development (TASK-005)." — still accurate. TASK-021 is a hotfix to an existing feature; no new capability is introduced. No update required.

**§Availability and UX:** "If Docker or the local Langfuse stack is not running, Vire reports AI trace health as `unavailable`, `stale`, or `unknown`." — `unknown` is listed. No new state is introduced by TASK-021. **PASS.**

**§AI trace import, §Local Langfuse Docker stack, §Privacy status:** all reference setup and configuration information unchanged by TASK-021 (no new env var, no new port, no schema change, no Docker service change). **PASS.**

### 2.2 `docs/langfuse-local-setup.md`

**Health states table:** All 10 states are present (populated by TASK-019). TASK-021 introduces no new state — the 10-state taxonomy is fixed and preserved. The `unknown` row reads:

> `unknown` — Cannot determine whether local Langfuse state is current or complete | AI totals shown with unknown-source flag; user prompted to check stack

This covers the TASK-021 scenario: when both the run write and the marker write fail, the in-band `Err` from `run_blocking_import` surfaces as a non-healthy response with `health == Unknown` and a persist-failure warning. The user-visible outcome (seeing `unknown` with a prompt to check the stack) is already described. No additional row needed. **PASS.**

**§Docker/Langfuse availability prose:** "Vire never interprets Docker down, a missing stack component, or absent traces as zero AI usage or cost." TASK-021 enforces this invariant on a path that previously violated it (false-healthy on total-DB-unwritability). The prose remains accurate — more so now than before. No update required. **PASS.**

**§Vire import endpoint configuration table:** Env vars (`VIRE_LANGFUSE_*`) unchanged — no new variable, no default change. **PASS.**

### 2.3 `docs/backup-restore.md`

No schema change (no new table, no `ALTER`/`DROP`). The `langfuse_import_runs` table structure is unchanged; the marker insert is unchanged (still `let _` best-effort). Backup scope, volume list, and divergence failure modes are unaffected. **PASS.**

---

## 3. OpenSpec / implementation coherence

| Criterion | Status |
|---|---|
| `specs/langfuse-importer/spec.md` present and non-empty | **PASS** |
| MODIFIED requirement present: surfacing is IPC-reaching and marker-independent (new scenario §"A persistence failure reaches the import command even when the failure-marker write also fails") | **PASS** |
| Requirement tightens TASK-020 S-4; nothing removed; absence-≠-zero contract strengthened | **PASS** |
| `PERSIST_FAILURE_MSG` made `pub`; `import_result` helper inspects summaries via exact sentinel match | **PASS** (SW-4 review §1; QA §Sentinel key correctness) |
| `run_blocking_import` returns `Err(PERSIST_FAILURE_MSG)` when persist-failure sentinel found | **PASS** (SW-4 review §2; QA coverage matrix) |
| `lib.rs` unchanged; existing `?` on `run_bounded` propagates the new `Err` | **PASS** (tasks.md §3; SW-4 review §2; QA coverage matrix) |
| Marker stays `let _` best-effort; no load-bearing role for correctness | **PASS** (SW-4 review §3; QA §Marker stays best-effort) |
| Regression test covers both-writes-fail gap; stale-healthy assertion proves in-band channel is authoritative | **PASS** (SW-4 review §5; QA coverage matrix §1-2) |
| Pre-existing marker-succeeds test unmodified | **PASS** (tasks.md §4; QA §3) |
| No new health state; 10-state taxonomy unchanged | **PASS** (proposal §Unchanged surfaces; QA §5) |
| No schema change | **PASS** (proposal §Unchanged surfaces; QA §6) |
| Secret-free surfacing preserved (SEC-003): `PERSIST_FAILURE_MSG` is a fixed string, no interpolation | **PASS** (SW-5 §manual review #1; QA §8) |
| No TASK-006 scope (no retry/backoff/reconciliation) | **PASS** (SW-5 §manual review #7) |
| `tasks.md` all subtasks checked off; gate criteria met (`cargo test --lib` 40/40, clippy clean on changed code) | **PASS** (tasks.md §5) |
| Closes B1 advisory from TASK-020 RELEASE.md | **PASS** — the total-DB-unwritability visibility gap noted as B1 is addressed by this hotfix |

---

## 4. Non-blocking notes

**NB-01 — SW-4 suggestions S1–S3 from TASK-021 review.**
S1 (`import_result` double-`.any()` — readable as-is), S2 (multi-environment `Err` carries same fixed string — correct and documented in review), S3 (pre-existing clippy warnings in TASK-020 code) have no docs implications.

**NB-02 — Pre-existing `csv_export_neutralizes_formula_like_project_names_and_notes` test failure.**
Out of scope; fails identically on base commit `dd5d3b9`. Not a TASK-021 docs item.

**NB-03 — SW-5 carry-forward advisories A1/A2.**
18 OSV advisories in pre-existing transitive deps (none CVE≥7) and 2 gitleaks false positives in gitignored build artifacts. Carry-forward from TASK-019/020; no docs implications.

---

## 5. Verdict

**PASS — no docs drift.**

TASK-021 is a pure correctness hotfix that makes persist-failure surfacing IPC-reaching and marker-independent. The one user-observable behavioral change (non-healthy error instead of false-healthy on total-DB-unwritability) is covered by the existing `unknown` health state documentation. No new health state, no new env var, no schema change, no new IPC command. `README.md`, `docs/langfuse-local-setup.md`, and `docs/backup-restore.md` are accurate against the released state. OpenSpec spec delta, proposal, and implementation are coherent. No documentation update was required or made; this gate pass is attestation only.
