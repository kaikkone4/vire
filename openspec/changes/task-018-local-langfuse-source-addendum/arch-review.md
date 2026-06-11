# Architecture Addendum / Drift Review — TASK-018

**Local Docker self-hosted Langfuse (DEC-020) supersedes the DEC-018 cloud-first source posture**

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-018-local-langfuse-source-addendum`
- **Branch (current):** `feat/task-018-local-langfuse-source-addendum` (dedicated branch; draft PR
  **#10** open, base `feat/task-003-implementation-path-decision` — stacked on the task-003 branch, so
  task-003 merges first or PR #10 is retargeted to `main` once task-003 lands; see §8)
- **Tier:** L2 · **Gate context:** drift/realignment review before TASK-007 MVP continues. No
  product runtime is built or modified here; this fixes-forward the source posture the TASK-007
  importer inherits.
- **Date:** 2026-06-11
- **Verdict:** **PASS** — TASK-003 / DEC-019 (the implementation-path decision) remains valid in
  full. Only the *default Langfuse source posture* it inherited from DEC-018 ("cloud-first") is
  superseded by **DEC-020** (local Docker self-hosted Langfuse default). The fix is additive
  supersession notes, not a rewrite of the merged decision package. No component boundary is
  crossed; no split; no BA escalation (DEC-020/DEC-022 already exist in the BA decision log).

---

## 1. Trigger and scope

BA artifacts were updated with **DEC-020** (local Docker self-hosted Langfuse is Vire's
canonical/default AI trace source; Langfuse Cloud is an explicit non-default override only) and
**DEC-022** (technical-plan alignment to that posture). The code docs were updated to match
(`README.md`, `docs/langfuse-local-setup.md`, `docs/backup-restore.md`).

A read-only drift search found that the already-merged/gated OpenSpec change
`task-003-implementation-path-decision` still carries the earlier **DEC-018 cloud-first** framing
throughout its proposal/design/spec/arch-review/qa/sec/release artifacts. That package was authored
2026-06-05, before DEC-020 (2026-06-11). The wording is now misleading for a TASK-007 implementer
who reads it for binding importer constraints.

This review establishes, authoritatively:

- what of TASK-003 / DEC-019 **remains valid** (almost all of it);
- what is **superseded** by DEC-020 (only the default-source posture);
- the **authoritative TASK-007 direction** after DEC-020;
- the **supersession notes** added so downstream is not misled, without rewriting the gated history.

## 2. What remains VALID from TASK-003 / DEC-019

DEC-019 is an **implementation-path / host-runtime** decision. It is orthogonal to where Langfuse
runs. DEC-020 changes the *source location and operational shape* of Langfuse; it does **not** touch
any of the following, which stand unchanged and binding for TASK-004/005/006/007-MVP:

| Carried-forward constraint (DEC-019) | Status under DEC-020 |
| --- | --- |
| Reuse the existing **Tauri v2 / Rust / TypeScript shell**; do not wipe; ActivityWatch reference-only; Swift/AppKit-first rejected | **Valid — unchanged** |
| Native **Swift/AppKit capture helper** as a bundled Tauri sidecar (`externalBin`); helper holds the AX/TCC grant, not the renderer | **Valid — unchanged** |
| Helper ↔ Rust core via **Tauri IPC**; raw window titles flow helper → core → normalized/redacted and **never** reach the webview or any network path | **Valid — unchanged** |
| **Langfuse importer is a read-only REST client in the Rust core**, behind the locked webview CSP (`connect-src ipc:`) and a Tauri HTTP **URL allowlist** | **Valid — unchanged** (only the *default URL* changes — §3) |
| **No raw macOS activity / window-title / prompt / command-body / env egress** to Langfuse (SEC-002) | **Valid — strengthened** (default target is now loopback, off-host egress only on explicit Cloud override) |
| Langfuse **credentials kept server-side in the Rust core**; never the renderer, never logs/exports (SEC-003) | **Valid — unchanged** (broadened to the local stack secrets too — §5) |
| **No new pi/Claude adapter** in MVP; Langfuse-first; runtime observer reconciles, never a duplicate ledger (DEC-017) | **Valid — unchanged** |
| Reuse is evidence-driven; legacy generic-tracker CRUD surface (`time_entries`, manual-entry, stopwatch) **not** re-admitted as a capture path; migrate-vs-retire deferred to TASK-004 | **Valid — unchanged** |
| Nested-binary codesign + notarization is an **assumption** with a TASK-005 packaging-spike follow-up | **Valid — unchanged** |

**Architectural point:** the read-only-REST-client-in-the-Rust-core design is *more* consistent
with a local-first source than with cloud-first. A loopback (`127.0.0.1`) read-only pull keeps all
trace traffic on Janne's machine, so DEC-019's privacy boundary is **reinforced**, not weakened, by
DEC-020. Nothing in DEC-019 needs to be reopened.

## 3. What is SUPERSEDED by DEC-020

Exactly one thing: the **default Langfuse source posture**.

| Dimension | DEC-018 (superseded) | DEC-020 (authoritative) |
| --- | --- | --- |
| Canonical/default source | Janne's configured Langfuse **Cloud** API project | **Local Docker self-hosted Langfuse** stack |
| Default importer base URL | a configured Cloud endpoint | **`http://127.0.0.1:3000`** (loopback) |
| Cloud Langfuse | the default | **explicit non-default override only** |
| Operational shape | single remote endpoint | **stateful multi-component local stack** (web/worker + PostgreSQL + ClickHouse + Redis/Valkey + MinIO/S3) |
| "cloud-first" characterization of the read-only pull | correct at the time | **incorrect now** — read "local-first read-only pull, Cloud optional override" |

What does **not** change because of this: the importer is still read-only, still in the Rust core,
still behind the locked CSP + URL allowlist, still Langfuse-only outbound, still no raw-activity
egress. Only the *target the allowlist points at by default* moves from a cloud host to loopback,
and a new local operational surface appears that TASK-003 never modeled.

Langfuse **environments remain the primary Vire→project mapping mechanism** — that flow is
unchanged and explicitly preserved by DEC-020/DEC-022.

## 4. Drift inventory — task-003 package (cite-and-correct)

The merged `task-003-implementation-path-decision` package contains the superseded framing at these
locations. None of these were *wrong when written*; they are now stale and must be read through the
DEC-020 correction below. (Verified 2026-06-11.)

| File | Line(s) | Drifted text | Corrected reading under DEC-020 |
| --- | --- | --- | --- |
| `proposal.md` | 25–27 | "Under DEC-018 the importer is a **read-only, cloud-first REST pull**…" | Read-only REST pull whose **default target is local Docker Langfuse on `127.0.0.1`**; Cloud is explicit override only. |
| `proposal.md` | 80–81 | guardrail "DEC-018 (cloud-first import)" | Guardrail is now "DEC-020 local-Docker-default import; Cloud explicit override". |
| `design.md` | 15–16, 75, 80 | "keep the decision inside DEC-017/DEC-018"; trace cites DEC-018 | Decision sits inside DEC-017 + **DEC-020/DEC-022**; DEC-018 is historical/superseded. |
| `design.md` | 64–65 | constraint 3: "read-only pull (**DEC-018 cloud-first**), scoped to the configured Langfuse base URL via the Tauri HTTP allowlist" | Same constraint; **default base URL is `http://127.0.0.1:3000`** (loopback). Cloud only on explicit override. |
| `arch-review.md` | 30, 39–41, 142, 150–151 | "read-only, **cloud-first** REST pull (DEC-018)"; "DEC-017/DEC-018 fidelity" | Read-only **local-first** REST pull; fidelity now tracks DEC-017 + **DEC-020**. |
| `spec.md` | 25, 46, 55 | "DEC-008/DEC-009/DEC-017/DEC-018"; "(SEC-002, DEC-018)" | Substitute **DEC-020** for the DEC-018 default-source citation; the *requirement text itself* (read-only Rust-core REST client, base-URL-scoped, no raw-activity egress) is still correct. |
| `qa.md` | 39, 49, 63, 70–71 | "read-only **cloud-first** pull" in coverage/fidelity rows | Gate evidence stands; substitute local-first/DEC-020 for the cloud-first/DEC-018 phrasing. |
| `sec.md` | 26, 54, 76, 85, 127–128 | DEC-018 "cloud-first configured Langfuse source" boundary row | SEC-002/SEC-003 preservation stands and is **stronger** (loopback default); DEC-018 row reads as superseded-by-DEC-020. |
| `RELEASE.md` | 86 | component matrix: "Read-only **cloud-first** import target (DEC-018)" | "Read-only **local-first** import target (DEC-020); Cloud explicit override". Product baseline `0.1.0` still correct. |
| `tasks.md` | 17–18 | "cloud-first import" in the constraints the decision observed | Historical task log; the observed constraint is now DEC-020 local-default. |

**Disposition:** these are a **single semantic correction** (cloud-first default → local-Docker
default) applied across one merged package. Rewriting the gate-history files (qa/sec/review/release/
spec/proposal/tasks) would invalidate their audit trail for no benefit. The correct, proportional
fix is a **forward-pointing supersession banner** on the two architecture-reference docs a TASK-007
implementer actually reads for binding constraints — `design.md` and `arch-review.md` — plus this
addendum as the comprehensive record. See §6.

## 5. Authoritative TASK-007 direction after DEC-020

This is the binding direction the TASK-007 importer spike/MVP inherits. It supersedes any
cloud-first reading of TASK-003 and of the `task-007-langfuse-importer-validation` feeder spike.

1. **Local Docker self-hosted Langfuse is the default source.** Default importer/setup base URL is
   `http://127.0.0.1:3000` (or an equivalent loopback-bound URL). Validation targets the local stack
   first.
2. **Langfuse Cloud is an explicit non-default override only.** It must never be reintroduced as a
   default or assumed posture, and choosing it is the *only* path that produces off-host egress.
3. **Langfuse environments remain the primary Vire→project mapping** (configure allowed environments
   per project, starting with `vire`). Map by environment first, then metadata/session/manual
   correction. This flow is unchanged and working.
4. **Model Langfuse as a stateful multi-component local stack**, not one container: `langfuse-web`
   (REST/UI, `127.0.0.1:3000`), `langfuse-worker`, PostgreSQL, ClickHouse, Redis/Valkey queue/cache,
   and MinIO/S3-compatible object storage. **Vire's importer talks only to the `langfuse-web` REST
   endpoint** on loopback — it does **not** connect to Postgres/ClickHouse/MinIO directly. The rest
   of the stack is Langfuse-internal and is surfaced to Vire only as **stack-health**.
5. **Keep MinIO/S3 and stack risks visible.** Bucket name (`langfuse`) and prefixes
   (`events/`, `media/`, `exports/`), Docker volumes (`langfuse_postgres_data`,
   `langfuse_clickhouse_data`, `langfuse_clickhouse_logs`, `langfuse_minio_data`,
   `langfuse_redis_data`), **private/non-public bucket** posture, local credentials kept out of
   repo/logs/evidence/exports, and **backup/restore divergence** failure modes across
   PostgreSQL/ClickHouse/MinIO must remain documented and surfaced. (Already covered in
   `docs/langfuse-local-setup.md` and `docs/backup-restore.md`; TASK-007 must keep these accurate as
   the importer lands.)
6. **All host/service ports bound to `127.0.0.1` by default.** No LAN binding; any non-loopback
   binding or Cloud endpoint is an explicit, documented override. The Tauri HTTP URL allowlist scopes
   strictly to the configured base URL and denies everything else; the locked webview CSP
   (`connect-src ipc:`) is untouched (importer is in the Rust core, not the renderer).
7. **Docker/Langfuse down ⇒ `unavailable` / `stale` / `unknown`, never zero cost.** Vire checks local
   stack availability before each import run; absent/down/partial-stack traces are an evidence gap,
   never a zero-AI-usage/cost signal. Vire may offer to open/start Docker where safe, or refuse to
   display AI totals, but must always surface the health state. (Health taxonomy: `healthy`,
   `missing`, `stale`, `wrong_env`, `delayed`, `duplicate`, `schema_changed`, `auth_or_network_error`,
   `unavailable`, `unknown` — per `04_technical_plan.md` §7.)
8. **Trace payload boundary for MVP:** local self-hosted traces may include prompt/session/metadata.
   This is accepted **within the local Langfuse boundary** to make the import flow work first.
   Stricter redaction/retention (prompt-text exclusion, metadata scrubbing, retention windows) is an
   explicit **L2 follow-up** and must not block the local import spike. Raw macOS activity stays in
   Vire's local SQLite and is never mixed into Langfuse traces.

These items are fully consistent with the already-updated BA `03_architecture_plan.md` §4.3/§4.4,
`04_technical_plan.md` §7, and the code docs — this addendum simply makes them the binding TASK-007
direction at the SW layer and supersedes the cloud-first wording.

## 6. Supersession notes applied (what this change touches)

To prevent a TASK-007 implementer being misled while preserving the gated decision record:

- **New:** `openspec/changes/task-018-local-langfuse-source-addendum/arch-review.md` (this file) +
  `proposal.md` — the comprehensive supersession record.
- **Additive banner** at the top of `task-003-implementation-path-decision/design.md` — points to
  DEC-020/DEC-022 and this addendum; flags that DEC-019 constraint 3's "cloud-first" default is
  superseded (default base URL now `127.0.0.1:3000`).
- **Additive banner** at the top of `task-003-implementation-path-decision/arch-review.md` — same
  forward pointer for the architecture-review reader.

**Not touched (deliberately):** the task-003 gate-history files — `qa.md`, `sec.md`, `review.md`,
`RELEASE.md`, `spec.md`, `proposal.md`, `tasks.md`. Editing a merged gate's audit trail to swap a
superseded decision reference would be rewriting history for no benefit; the §4 inventory + the two
banners + this addendum are sufficient to keep downstream correct. DEC-019 itself is **not**
reopened — its decision is unchanged.

## 7. Security / boundary check (addendum scope)

| Boundary | DEC-020 effect | Result |
| --- | --- | --- |
| **SEC-002** network boundary | Default target moves cloud → loopback; importer still read-only, base-URL-allowlisted, Langfuse-only, no raw-activity egress. Off-host egress now occurs **only** on explicit Cloud override. | **Preserved / strengthened** |
| **SEC-003** credentials | Now also covers local stack secrets (Postgres/ClickHouse/MinIO/Redis passwords, `NEXTAUTH_SECRET`, `SALT`, `ENCRYPTION_KEY`, Langfuse API keys). All kept out of repo/logs/evidence/exports/support output; stored in local `.env`/Keychain. | **Preserved / broadened** |
| **DEC-017** Langfuse-first, no duplicate ledger, no new adapter | Unchanged. | **Preserved** |
| New: MinIO/S3 object-storage surface | Private bucket, loopback-bound, credentials protected, backup/restore consistency across PG/CH/MinIO; divergence failure modes documented. | **Visible & documented** (README + docs/) |
| New: Docker/stack-down handling | `unavailable`/`stale`/`unknown`, never zero cost. | **Required & specified** |

No new attack surface is *introduced* by this addendum (it is docs-only). The DEC-020 posture is
net-safer for egress because the default source is on-host loopback.

## 8. Open items and handoff

- **DEC-019 BA ratification still pending.** FB-003 (task-003 arch-review §5) asked BA to record
  DEC-019 in `07_decision_log.md`; it is **still absent** there (verified 2026-06-11). SW cannot
  write BA artifacts. **`feedback_to_ba[]`:** record **DEC-019** (implementation-path: reuse Tauri +
  native helper + Rust-core importer) and note that its importer posture now reads against
  **DEC-020** (local-Docker default), not DEC-018.
- **task-007 feeder spike also carries cloud-first framing.** `task-007-langfuse-importer-validation`
  (on `feat/task-007-langfuse-importer-validation`, not in this working tree) validated the importer
  under DEC-018. Its conclusions on REST fields/pagination/dedup/health remain useful, but its
  default-source posture must be read through this addendum (local-Docker default). The TASK-007 MVP
  should re-target validation at the local stack first. Flag for whoever owns the TASK-007 MVP branch.
- **No spec delta.** TASK-018 adds no new requirement — DEC-020/DEC-022 already capture the posture in
  the BA layer and the code docs already implement the documentation side. This addendum is a SW
  architecture realignment + supersession record, so no `specs/**/spec.md` delta and no
  `openspec validate` gate is required for it.

### Recommendation (next role / branch / change)

- **Next role:** route TASK-007 MVP to the **integration-engineer** (Langfuse importer host-runtime
  fit) with **backend-developer (Rust/Tauri)** owning the read-only REST client in the Rust core.
- **Branch:** done. The TASK-018 addendum, the two task-003 banners, and the DEC-020 doc work
  (`README.md`, `docs/langfuse-local-setup.md`, `docs/backup-restore.md`) are committed on the
  dedicated **`feat/task-018-local-langfuse-source-addendum`** branch, surfaced as draft **PR #10**.
  The realignment lands as its own reviewable unit and is no longer mixed into the task-003 branch. **Merge-order caveat:** PR #10's base is `feat/task-003-implementation-path-decision`,
  so it is stacked on task-003 — task-003 must merge to `main` first, then PR #10 merges (or PR #10 is
  retargeted to `main` once task-003 has landed) to avoid pulling task-003's unmerged diff into this
  review.
- **Change:** `task-018-local-langfuse-source-addendum` (this dir). TASK-007 MVP is the next
  implementation change; it inherits §5 as binding direction.

## 9. Verdict

**PASS.** TASK-003 / DEC-019 stands in full; only its inherited DEC-018 cloud-first default-source
posture is superseded by DEC-020 (local Docker self-hosted Langfuse default, Cloud explicit override
only). Supersession is handled additively (this addendum + two forward-pointing banners) without
rewriting the gated decision history. The authoritative TASK-007 direction (§5) is fixed. No split;
no BA escalation (DEC-020/DEC-022 already exist); one non-blocking `feedback_to_ba[]` item (DEC-019
ratification) carried forward.
