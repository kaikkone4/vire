# QA Report — TASK-018 local Langfuse source supersession addendum

- **QA Engineer:** SW-3
- **Change:** `task-018-local-langfuse-source-addendum`
- **Branch:** `feat/task-018-local-langfuse-source-addendum`
- **Tier:** L2
- **Date (initial pass):** 2026-06-11
- **Date (re-run after fix commit):** 2026-06-11
- **Verdict:** **PASS**

---

## Re-run scope (after SW-2 fix commit `bdada81`)

Fix commit resolves:
- SW-4 blocking finding: stale "current branch is task-003 / move to dedicated branch" text in `arch-review.md` and `proposal.md`.
- SEC-ADV-01: false "no compose file exists in this repo" claim in `README.md` and `docs/langfuse-local-setup.md`.
- SEC-ADV-02: incorrect `127.0.0.1:9090` host-port claim for MinIO in `README.md` and `docs/langfuse-local-setup.md`.
- Adds `sec.md` gate artifact to the change package (8 scope files total, up from 7).

Re-run checks R-01 through R-07 below cover the fixed items specifically. Original checks C-01–C-16 are re-confirmed valid; only C-01 count is updated (7 → 8 files).

---

## Re-run verification matrix

| Check | Criterion | Result | Notes |
| --- | --- | --- | --- |
| R-01 | Docs-only: no runtime files changed (src/, src-tauri/src/, observability/) | PASS | `git diff main...HEAD -- src/ src-tauri/src/ observability/` = 0 lines |
| R-02 | Stale "move to dedicated branch" recommendation gone from `arch-review.md` | PASS | §8 now reads "done. Committed on feat/task-018-local-langfuse-source-addendum… draft PR #10. Merge-order caveat: task-003 merges first." Old "recommend moving" text absent |
| R-03 | Stale "move to dedicated branch" recommendation gone from `proposal.md` | PASS | "Branch recommendation: move…" replaced with "Branch: done. Committed on feat/task-018-local-langfuse-source-addendum… PR #10 stacked on feat/task-003…" |
| R-04 | README §Setup: no longer claims compose file absent; references existing `observability/langfuse/` | PASS | "Existing local stack: A loopback-bound… stack already exists at observability/langfuse/…" present; old "does not exist in this repo yet" gone |
| R-05 | `docs/langfuse-local-setup.md` §Prerequisites: references existing loopback-bound stack | PASS | "Existing stack" callout present pointing to `observability/langfuse/docker-compose.yml`; "does not exist" gone |
| R-06 | README component table: MinIO shows `internal (not host-published)`, not `127.0.0.1:9090` | PASS | Table row updated; `127.0.0.1:9090` absent from README |
| R-07 | `docs/langfuse-local-setup.md` component table: MinIO shows internal-only, no host port | PASS | "none (internal); API `minio:9000`, console `:9001` inside the container — not host-published"; `127.0.0.1:9090` absent |
| R-08 | `docs/langfuse-local-setup.md` §Localhost binding: explicitly states MinIO not host-published | PASS | "Internal services (…and MinIO) are not host-published at all — do not add `ports:` entries for them. MinIO is reachable only on the internal Compose network (`minio:9000`)" |
| R-09 | `docs/langfuse-local-setup.md` §MinIO console: confirms console not published to host | PASS | "is **not** published to the host. There is no `127.0.0.1:9001` mapping by default, which is the correct, stricter posture." |
| R-10 | No credential values or personal paths in any changed file | PASS | Only template placeholders (`${MINIO_ROOT_PASSWORD}`, `<pg-user>`, etc.); `sec.md` advisory text names patterns searched and found absent, not values |
| R-11 | DEC-020 direction preserved: local Docker default, `127.0.0.1:3000`, Cloud non-default override | PASS | README intro + arch-review §5 item 1–2; langfuse-local-setup.md §Vire import endpoint |
| R-12 | DEC-020 direction preserved: environments primary, Docker down never zero cost, MinIO/S3 risks visible | PASS | README §AI trace import, §Availability and UX, §Object storage; langfuse-local-setup.md health table + §MinIO; docs/backup-restore.md |
| R-13 | `sec.md` added to change package; content is a valid L2 security gate artifact | PASS | `sec.md` present and non-empty; covers 4 scanners (gitleaks, semgrep, OSV, Trivy) + 6 manual focus checks; verdict PASS with two non-blocking advisories now resolved by fix commit |

---

## Original scenario coverage matrix

| Check | Criterion | Result | Notes |
| --- | --- | --- | --- |
| C-01 | All 8 scope files present and non-empty (updated from 7; `sec.md` added in fix commit) | PASS | arch-review.md, proposal.md, qa.md, sec.md in task-018 dir; README.md, docs/langfuse-local-setup.md, docs/backup-restore.md; task-003/design.md and task-003/arch-review.md banners — all confirmed |
| C-02 | Task-018 commits touch only declared scope files | PASS | `7c7a663` (package) + `bdada81` (fix) combined touch: README.md, docs/langfuse-local-setup.md, openspec/changes/task-018-* (arch-review, proposal, qa, sec), and task-003 banners only |
| C-03 | No product runtime source changed (`src/`, `src-tauri/src/`, `observability/`) | PASS | `git diff main...HEAD -- src/ src-tauri/src/ observability/` = 0 lines (re-verified) |
| C-04 | TASK-003 / DEC-019 remains valid in full | PASS | Unchanged |
| C-05 | DEC-018 cloud-first default-source posture clearly superseded by DEC-020 | PASS | Unchanged |
| C-06 | TASK-007 local Docker self-hosted Langfuse as default | PASS | Unchanged |
| C-07 | TASK-007 localhost (`127.0.0.1`) binding requirement | PASS | Unchanged |
| C-08 | TASK-007 Cloud = explicit non-default override only | PASS | Unchanged |
| C-09 | TASK-007 environments = primary project mapping | PASS | Unchanged |
| C-10 | TASK-007 MinIO/S3 and backup/restore risks visible | PASS | Unchanged |
| C-11 | TASK-007 Docker/Langfuse down → unavailable/stale/unknown, never zero cost | PASS | Unchanged |
| C-12 | No secret values, actual credentials, or credential file paths in any changed file | PASS | Re-verified; sec.md adds no credential values |
| C-13 | Markdown links resolve within repo context | PASS | New links to `observability/langfuse/` and `observability/langfuse/docker-compose.yml` are structurally valid (path exists in main) |
| C-14 | Gate-history files (task-003 qa/sec/review/RELEASE/spec/proposal/tasks) not modified | PASS | Unchanged |
| C-15 | Supersession banners present at top of both task-003 architecture-reference docs | PASS | Unchanged |
| C-16 | Authoritative TASK-007 direction path in banners points to existing file | PASS | Unchanged |

---

## Blockers

None.

---

## Non-blocking findings

**NB-01 — design.md constraint 3 body still references "DEC-018 cloud-first"**
- Intentional gate-history preservation. Documented in `task-018/arch-review.md` §4. No action required.

**NB-02 — task-003/arch-review.md handoff item 3 still reads "DEC-017/DEC-018 fidelity"**
- Same intentional pattern as NB-01.

**NB-03 — DEC-019 BA ratification still pending**
- Carried forward; non-blocking for SW. `feedback_to_ba[]` item in `task-018/arch-review.md` §8.

**NB-04 — task-007 feeder spike on `feat/task-007-langfuse-importer-validation` carries DEC-018 framing**
- Documented in `task-018/arch-review.md` §8. Non-blocking; flagged for TASK-007 MVP owner.

---

## Scope confirmation

- **No spec delta** — no `specs/**/spec.md` modified; no `openspec validate` gate required.
- **No new requirement introduced** — DEC-020/DEC-022 already captured the posture at the BA layer.
- **No build artifact, schema, or migration changed.**

---

## Verdict

**PASS.** All re-run checks (R-01–R-13) pass. All original scenario checks (C-01–C-16) hold. Fix commit `bdada81` correctly resolves the SW-4 blocking finding (stale branch/move-recommendation text in `arch-review.md` and `proposal.md`) and both SEC advisories (SEC-ADV-01: existing compose now referenced; SEC-ADV-02: MinIO documented as internal/unpublished). DEC-020 direction is fully preserved. No blockers. Routes to SW-4 (Code Reviewer) and SW-5 (Security Agent) in parallel.
