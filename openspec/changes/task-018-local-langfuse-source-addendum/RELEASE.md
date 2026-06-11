# Release — TASK-018 Local Langfuse Source Supersession Addendum

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-018-local-langfuse-source-addendum`
- **Branch:** `feat/task-018-local-langfuse-source-addendum` · **PR:** [kaikkone4/vire#10](https://github.com/kaikkone4/vire/pull/10)
- **Release unit:** architecture addendum realigning TASK-003/DEC-019 to DEC-020 local Docker self-hosted Langfuse default — docs/OpenSpec only
- **Date:** 2026-06-11
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml`) — **unchanged by this release**
- **Verdict:** RELEASE-READY (all three required declarations complete)

---

## Release contents

A single coherent addendum unit confined to:

- `openspec/changes/task-018-local-langfuse-source-addendum/arch-review.md` — SW-1 architecture addendum / drift review (Verdict: PASS)
- `openspec/changes/task-018-local-langfuse-source-addendum/proposal.md` — change rationale and scope
- `openspec/changes/task-018-local-langfuse-source-addendum/specs/langfuse-trace-source/spec.md` — minimal ADDED spec delta (4 scenarios; 1 capability; `openspec validate --strict` passes)
- `openspec/changes/task-018-local-langfuse-source-addendum/qa.md` (SW-3, PASS)
- `openspec/changes/task-018-local-langfuse-source-addendum/review.md` (SW-4, PASS)
- `openspec/changes/task-018-local-langfuse-source-addendum/sec.md` (SW-5, PASS)
- `openspec/changes/task-018-local-langfuse-source-addendum/docs.md` (SW-6 docs gate, PASS — commit `74f8c85`, 2026-06-11)

Plus additive supersession banners (no history rewrite) on two architecture-reference docs a TASK-007
implementer reads for binding constraints:

- `openspec/changes/task-003-implementation-path-decision/arch-review.md` — banner added
- `openspec/changes/task-003-implementation-path-decision/design.md` — banner added

Plus the three code docs updated to the DEC-020 posture:

- `README.md` (+64/-2) — Local Langfuse stack, AI trace import, privacy sections
- `docs/langfuse-local-setup.md` (new, 153 lines)
- `docs/backup-restore.md` (new, 158 lines)

**No product runtime is created or modified.** `git diff --name-only main...HEAD -- src/ src-tauri/src/ observability/` is empty; all changed files are `.md`. No `tauri.conf.json`, `Cargo.toml`, `package.json`, capabilities, or lockfile is touched.

### Summary of change

Before TASK-007 MVP work continues, TASK-018 fixes forward the Langfuse default-source posture that
the TASK-007 importer inherits. The `task-003-implementation-path-decision` package was authored
2026-06-05 under the earlier **DEC-018 cloud-first** posture. BA decision **DEC-020** (with
technical-plan alignment **DEC-022**) subsequently established **local Docker self-hosted Langfuse**
as Vire's canonical default, demoting Langfuse Cloud to an explicit non-default override.

This addendum:
1. Authoritatively establishes what of DEC-019 remains valid (everything except the default-source
   posture — the Tauri + native-helper + Rust-core read-only REST importer path is unchanged).
2. Records exactly what DEC-020 supersedes (only the cloud-first default; loopback `127.0.0.1:3000`
   replaces the configured Cloud endpoint as the default base URL).
3. Provides the binding TASK-007 direction: local Docker default, Cloud explicit override, full
   multi-component stack modeling (web/worker + PostgreSQL + ClickHouse + Redis/Valkey + MinIO/S3),
   Docker-down = `unavailable`/`stale`, not zero cost.
4. Adds forward-pointing supersession banners on the two task-003 docs a TASK-007 implementer reads.
5. Adds a minimal ADDED `specs/langfuse-trace-source/spec.md` delta so `openspec validate --strict`
   passes with the new capability on record.

DEC-019 (implementation path) is not reopened. The addendum is additive, not a history rewrite.

---

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact |
| --- | --- | --- | --- |
| SW-1 | SW Architect | PASS | `arch-review.md` |
| SW-3 | QA Engineer | PASS (final re-run after fix commit `21f5fdd`, 2026-06-11) | `qa.md` |
| SW-4 | Code/Doc Reviewer | PASS (commit `24e8279`, 2026-06-11) | `review.md` |
| SW-5 | Security Agent | PASS (§10 re-check after `21f5fdd` + `dbb9c6f`, commit `96f910b`, 2026-06-11) | `sec.md` |
| SW-6 docs | Documentation Engineer | PASS (no docs drift — all five DEC-020 dimensions verified; commit `74f8c85`, 2026-06-11) | `docs.md` |

No task is being released that did not pass both SW-4 and SW-5.

---

## Required declaration 1 — Deployment size

**`patch` (documentation-only addendum; no runtime/product change).**

This release ships no compiled or interpreted product code. The product manifest stays at `0.1.0`;
no `src/`, `src-tauri/src/`, or `observability/` artifact, schema, config, build target, or
dependency is added, changed, or removed. Under semantic versioning the change carries no MAJOR (no
breaking change), no MINOR (no new product feature), and no PATCH-level code fix — it is the
smallest possible deployment class. Nothing is built, packaged, or pushed to any runtime environment;
the "deployment" is the merge of a docs/OpenSpec addendum into the repository.

---

## Required declaration 2 — Rollback strategy

**`forward-only` (trivially reversible by `git revert`; nothing runtime to roll back).**

There is no deployed runtime artifact, migration, or stateful change to reverse, so a staged
rollback rehearsal is not applicable. (L2 "tested rollback in staging" — **N/A**, no runtime
surface; recorded as N/A with rationale rather than silently skipped.) If the addendum must be
withdrawn, the complete reversal is a `git revert` (or branch deletion) of the documentation commits
on `feat/task-018-local-langfuse-source-addendum`; it affects only repository text, never a running
system, data store, or downstream consumer. Reverting would re-expose the misleading DEC-018
cloud-first framing to a TASK-007 implementer but would break no shipped behaviour and no data.
No forward-fix beyond ordinary doc edits is required.

---

## Required declaration 3 — Component compatibility matrix

This change introduces **no** new upstream/downstream version constraint because it modifies no
runtime, dependency, or interface. The matrix records:

(A) The components the **TASK-018 addendum rests on** (docs reference these; all unchanged by this
release) and their role under DEC-020.

(B) The **local Langfuse stack components** this addendum surfaces as a new operational reality for
TASK-007 — previously invisible to the OpenSpec layer. These are DEC-020/DEC-022 from the BA layer;
no SW code pins them yet. Min versions are those documented in `docs/langfuse-local-setup.md`; Max
is open (Vire's importer talks only to the Langfuse web REST endpoint, not to the internal services
directly).

### (A) Core product and framework components (unchanged)

| Component | Role under DEC-020 | Min | Max | Status |
| --- | --- | --- | --- | --- |
| Vire app (Tauri shell) | This product (reuse path — DEC-019) | `0.1.0` | `0.1.0` | Unchanged (no code touched) |
| Tauri runtime | App shell — `reuse-as-is` | v2 | open | Unchanged |
| Tauri v2 sidecar (`externalBin`) | Bundle/run native Swift capture helper | v2 | open | Unchanged |
| Tauri v2 IPC | helper↔core / core↔frontend; renderer isolation | v2 | open | Unchanged |
| Tauri v2 HTTP client (`reqwest`, URL-allowlisted) | Langfuse REST read-only pull from Rust core | v2 | open | Unchanged (default target now loopback — see B) |
| `tauri-plugin-shell` | Sidecar stdin/stdout lifecycle | 2.x | open | Unchanged |
| Swift/AppKit capture helper | AX/TCC grant; activity capture; raw titles never reach renderer or network | macOS 13+ | open | Unchanged |
| SQLite (Vire local store) | Raw macOS activity; never mixed into Langfuse traces | — | open | Unchanged |

### (B) Local Langfuse stack — newly surfaced operational components (DEC-020)

Vire's importer connects **only** to `langfuse-web` on `127.0.0.1:3000` (REST/UI endpoint). The
remaining services are Langfuse-internal and are surfaced to Vire only as stack-health state.

| Component | Role under DEC-020 | Default bind | Min | Max | Notes |
| --- | --- | --- | --- | --- | --- |
| `langfuse-web` (Langfuse Docker image) | REST API + UI; **Vire's only direct connection target** | `127.0.0.1:3000` | `3.x` | open | Loopback-bound; Cloud only on explicit override |
| `langfuse-worker` (Langfuse Docker image) | Async background processing; Langfuse-internal | internal | `3.x` | open | Vire does not connect directly |
| PostgreSQL (Langfuse-bundled) | Langfuse relational store; Langfuse-internal | `127.0.0.1:5432` | 14 | open | Vire does not connect directly; backup divergence risk documented |
| ClickHouse (Langfuse-bundled) | Event/analytics store; Langfuse-internal | `127.0.0.1:8123` | 23.x | open | Vire does not connect directly; backup divergence risk documented |
| Redis/Valkey (Langfuse-bundled) | Queue/cache; Langfuse-internal | `127.0.0.1:6379` | 7.x | open | Vire does not connect directly |
| MinIO / S3-compatible storage (Langfuse-bundled) | Object storage (`langfuse` bucket, private); Langfuse-internal | `127.0.0.1:9000` (API), `127.0.0.1:9001` (console) | — | open | Private bucket; no LAN exposure; credentials out of repo/logs; backup divergence risk documented |
| Docker Engine | Runs the entire local Langfuse stack | — | 24.x | open | Stack-down → `unavailable`/`stale`, never zero cost |

**Compatibility constraints introduced by this release:** none. The version ranges above describe the
pre-existing local stack committed in `observability/langfuse/docker-compose.yml` (already on
`main`); this addendum documents them in OpenSpec for the first time, it does not pin them.

---

## L2 release-integrity checklist

| Expectation | Applicability | Rationale |
| --- | --- | --- |
| Signed git tag | **Applies** (see §5) | Marks the release unit |
| Signed binary / notarized artifact | **N/A** | No compiled or distributable artifact produced |
| SBOM (Software Bill of Materials) | **N/A** | No build output and no dependency-graph change to inventory |
| Tested rollback in staging | **N/A** | No runtime/deployable surface; reversal is `git revert` of docs |
| Audit trail | **Present** | SW-1/3/4/5 gate reports (`arch-review.md`, `qa.md`, `review.md`, `sec.md`) + this RELEASE.md |

Per `sec.md`, scanner stack (gitleaks, semgrep, OSV-scanner, Trivy) all passed at Tier 1/L2;
zero findings across all four scanners (2026-06-11). APP-005 Gate D (release/distribution integrity)
does not fire — no distributable artifact is released. `docs.md` (SW-6 Documentation Engineer gate,
commit `74f8c85`) confirms no docs drift: all five DEC-020 posture dimensions verified across
`README.md`, `docs/langfuse-local-setup.md`, `docs/backup-restore.md`, the spec delta, and the
task-003 supersession banners — audit trail complete.

---

## 5. Tag status

- **Intended tag:** `task-018/v0.1.0` (signed, SSH), annotated as the TASK-018 local Langfuse
  source supersession addendum release unit. (Product baseline `0.1.0` unchanged; docs-only addendum
  carries no product version increment.)
- **Signing config:** `gpg.format=ssh`, `user.signingkey=/Users/kaikkonen/.ssh/id_ed25519.pub`,
  `tag.gpgsign=true` (verified in runner).
- **Status: DEFERRED — signing key not provisioned in this runner.** `user.signingkey` is set to
  `/Users/kaikkonen/.ssh/id_ed25519.pub` but the file is not present; `git tag -s` cannot produce a
  signed tag. Per L2 policy an unsigned fallback tag was deliberately **not** created. The tag is a
  pending action for a signing-capable runner. This does not block the gate criterion (RELEASE.md
  with all three declarations complete), but it is an open release-integrity item — identical
  disposition to the TASK-003 release.

---

## 6. Non-blocking carryover items

These items are recorded from the SW gate chain. None block the release criterion (RELEASE.md with
all three declarations complete). None block PR readiness.

- **DEC-019 BA ratification pending.** `07_decision_log.md` still does not record DEC-019
  (implementation path). `feedback_to_ba[]`: record DEC-019 and note its importer posture reads
  against DEC-020, not DEC-018. (SW-1 `arch-review.md` §8; SW-4 `review.md` §3.)
- **task-007 feeder spike carries DEC-018 framing.** `feat/task-007-langfuse-importer-validation`
  validated the importer under DEC-018. Its REST field/pagination/dedup/health conclusions remain
  useful; the TASK-007 MVP should re-target validation at the local stack first. (SW-1 §8.)

---

## 7. PR ready-for-review

Per role v3.1.2, draft PR #10 is promoted to ready-for-review as the final SW-6 step (all gates
PASS; RELEASE.md complete). See the STATUS report for the `gh pr ready 10` result and final PR URL.

## Handoff

- **Verification flow (Flow 3, stubbed):** N/A — no deployed artifact to environment-test.
- **Documentation Engineer (`sw-documentation-engineer`, required at L2):** this release triggers
  the doc-update step. RELEASE.md and the SW-1/3/4/5 gate chain are ready for handoff.
