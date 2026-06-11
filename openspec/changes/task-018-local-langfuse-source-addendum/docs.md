# Documentation Review — TASK-018 local Langfuse source supersession addendum

- **Documentation Engineer:** SW-6
- **Change:** `task-018-local-langfuse-source-addendum`
- **Branch:** `feat/task-018-local-langfuse-source-addendum` · **PR:** #10
- **Tier:** L2 · **Gate context:** SW-6 docs-drift check after final gate commits (spec delta `1955eb4`, SW-4 blocker-close `21f5fdd`, SW-3 QA re-run `dbb9c6f`, SW-5 final re-check `96f910b`)
- **Date:** 2026-06-11
- **Verdict:** **PASS — no docs drift**

---

## Scope reviewed

10 PR-diff files plus the task-003 supersession banners (both architecturally significant docs a
TASK-007 implementer reads for binding constraints):

- `README.md` — Local Langfuse Docker stack, Availability and UX, Object storage, AI trace import, Privacy status, Setup sections
- `docs/langfuse-local-setup.md` — full setup and operations guide (153 lines)
- `docs/backup-restore.md` — backup scope, consistency, restore procedure, divergence failure modes (158 lines)
- `openspec/changes/task-018-local-langfuse-source-addendum/specs/langfuse-trace-source/spec.md` — ADDED spec delta (4 scenarios)
- `openspec/changes/task-018-local-langfuse-source-addendum/{arch-review,proposal,qa,sec}.md` — gate artifacts
- `openspec/changes/task-003-implementation-path-decision/{design,arch-review}.md` — supersession banners

---

## 1. DEC-020 posture accuracy

Verified against five required dimensions across all three user-facing docs:

| Dimension | README.md | langfuse-local-setup.md | backup-restore.md | Result |
| --- | --- | --- | --- | --- |
| Local Docker self-hosted Langfuse is the default | Intro paragraph; §Setup | Intro sentence (DEC-020 cited) | Intro (primary AI trace source) | **PASS** |
| Cloud is explicit non-default override only | §AI trace import: "supported only as an explicit non-default override" | §Vire import endpoint table: "change only for explicit Cloud override" | n/a (operations scope) | **PASS** |
| Default base URL `http://127.0.0.1:3000` | §AI trace import; §Setup step 3 | §Localhost binding; §Vire import endpoint table | Post-restore checklist: "Langfuse UI reachable at `http://127.0.0.1:3000`" | **PASS** |
| All service ports loopback-bound; internal services not host-published | §Components table: MinIO `internal (not host-published)`; "All service ports must be bound to `127.0.0.1`" | §Localhost binding (long- and short-form examples; "Internal services … and MinIO — not host-published at all"); §MinIO console: "no `127.0.0.1:9001` mapping by default" | n/a (operations scope) | **PASS** |
| Docker/stack down ≠ zero AI usage or cost | §Availability and UX: "Vire will not interpret missing or unavailable traces as zero AI usage or cost" | Health-state table + "Vire never interprets Docker down … as zero AI usage or cost" | n/a (operations scope) | **PASS** |

---

## 2. MinIO/S3 internal/private documentation

| Criterion | Location | Status |
| --- | --- | --- |
| MinIO not host-published (no host port) | README §Components table: `internal (not host-published)`; `langfuse-local-setup.md` component table: `none (internal); API minio:9000, console :9001 inside the container — not host-published` | **PASS** |
| Bucket must not be set to public | `langfuse-local-setup.md` §MinIO/S3 access control: "`langfuse` bucket **must not be set to public**" | **PASS** |
| MinIO access/secret keys must not be committed, logged, exported | README §Object storage; `langfuse-local-setup.md` §MinIO/S3 access control | **PASS** |
| MinIO console not published to host | `langfuse-local-setup.md` §MinIO console access: "not published to the host. There is no `127.0.0.1:9001` mapping by default" | **PASS** |

---

## 3. Backup/restore consistency documentation

| Criterion | Location | Status |
| --- | --- | --- |
| Three-store consistency required (PG + ClickHouse + MinIO) | `backup-restore.md` §What must be backed up: "A valid Langfuse backup requires **all three persistent stores to be consistent with each other**" | **PASS** |
| Volume names consistent across all docs | `langfuse_postgres_data`, `langfuse_clickhouse_data`, `langfuse_clickhouse_logs`, `langfuse_minio_data`, `langfuse_redis_data` — identical across README §Components, `langfuse-local-setup.md` volumes table, and `backup-restore.md` §What must be backed up | **PASS** |
| MinIO bucket/prefix layout documented | `langfuse-local-setup.md` §Bucket and prefix layout; `backup-restore.md` §What must be backed up (bucket `langfuse`, prefixes `events/`, `media/`, `exports/`) — consistent | **PASS** |
| Divergence failure modes documented | `backup-restore.md` §Divergence failure modes table (6 scenarios) | **PASS** |
| Post-restore verification checklist | `backup-restore.md` §Post-restore verification checklist — 8 items including MinIO bucket existence and privacy check | **PASS** |
| README cross-references backup-restore.md | README §Object storage: "see [docs/backup-restore.md]" | **PASS** |

---

## 4. OpenSpec spec delta coherence

| Criterion | Status |
| --- | --- |
| `specs/langfuse-trace-source/spec.md` present and non-empty | **PASS** |
| Covers all 4 required scenarios: local default, loopback base URL, down-stack ≠ zero cost, MinIO internal/private + backup risks | **PASS** |
| Scenarios are additive ADDED-only — no existing requirement modified or removed | **PASS** |
| Spec delta posture matches README and docs/ posture (no internal contradiction) | **PASS** |
| `openspec validate task-018-local-langfuse-source-addendum --strict` passes | **PASS** (QA final re-run F-01, 2026-06-11) |

---

## 5. Task-003 supersession banners

| File | Banner present at top | Correct DEC-020/DEC-022 reference | Correct pointer to task-018 arch-review.md | Status |
| --- | --- | --- | --- | --- |
| `task-003-implementation-path-decision/design.md` | Lines 3–14 | Yes — "DEC-020 (local Docker self-hosted Langfuse); Cloud explicit non-default override only" | Yes — `openspec/changes/task-018-local-langfuse-source-addendum/arch-review.md` | **PASS** |
| `task-003-implementation-path-decision/arch-review.md` | Lines 3–12 | Yes — same DEC-020/DEC-022 framing | Yes — same pointer | **PASS** |
| Gate-history files not touched (qa/sec/review/RELEASE/spec/proposal/tasks) | Confirmed by QA C-14 | — | — | **PASS** |

---

## 6. Cross-document internal consistency

No drift was found between documents on any of the following dimensions:

- Default base URL (`http://127.0.0.1:3000`) — consistent in README, `langfuse-local-setup.md`, `backup-restore.md` post-restore checklist, and spec delta.
- Cloud non-default posture — consistent in README, `langfuse-local-setup.md`, spec delta, arch-review §5, proposal.
- MinIO volume name (`langfuse_minio_data`) — consistent in README §Components, `langfuse-local-setup.md` volumes table, and `backup-restore.md` backup/restore steps.
- Docker-down ≠ zero cost — stated in README §Availability and UX and `langfuse-local-setup.md` health-state table; reinforced in spec delta down-stack scenario.
- Trace content boundary (prompt/session/metadata accepted in local boundary for MVP; stricter redaction as L2 follow-up) — consistent in `langfuse-local-setup.md` §Trace content boundary and README §Privacy status.

---

## 7. Non-blocking notes

**NB-01 — DEC-019 BA ratification still pending.**
`07_decision_log.md` does not record DEC-019. Carried as `feedback_to_ba[]` in `arch-review.md` §8. Not an SW blocker and not a docs-drift issue for this change.

**NB-02 — TASK-007 feeder spike carries DEC-018 framing.**
`feat/task-007-langfuse-importer-validation` not in this working tree; flagged in `arch-review.md` §8 for the TASK-007 MVP owner. Not a drift issue within this change's scope.

**NB-03 — Signed git tag pending.**
Consistent disposition with prior TASK-001/002/003/007 releases. Not a docs-drift issue.

---

## 8. Verdict

**PASS — no docs drift.** All three user-facing documentation files (`README.md`,
`docs/langfuse-local-setup.md`, `docs/backup-restore.md`) accurately reflect the DEC-020 posture
after the final gate commits. The OpenSpec spec delta is coherent with the docs and with the
arch-review TASK-007 direction. The task-003 supersession banners are present, correctly formed, and
point to the right authoritative record. No active inconsistency was found across any document or
between the docs layer and the OpenSpec layer. All prior SW-3/SW-4/SW-5 PASS verdicts are
unaffected by this check.
