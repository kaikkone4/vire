# Release — TASK-001 Repo/Path Assessment

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-001-repo-path-assessment`
- **Branch:** `feat/task-001-repo-path-assessment` · **PR:** [kaikkone4/vire#6](https://github.com/kaikkone4/vire/pull/6) (OPEN, MERGEABLE/CLEAN)
- **Release unit:** the repo/path salvage-reuse assessment deliverable (documentation only)
- **Date:** 2026-06-04
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`) — **unchanged by this release**
- **Verdict:** RELEASE-READY (gate criterion met; tag pending signing-key provisioning, see §5)

## Release contents

A single, coherent assessment unit comprising 7 documentation artifacts, all confined to
`openspec/changes/task-001-repo-path-assessment/`:

- `proposal.md`, `tasks.md`, `design.md`, `arch-review.md`
- `salvage-reuse-inventory.md` (primary deliverable)
- `specs/repo-path-assessment/spec.md` (one ADDED capability)
- `qa.md` (SW-3 gate report)

No product source, schema, configuration, container image, or dependency manifest is modified
(`git diff --name-only main...HEAD` → docs only). This is a read-only spike: it delivers an
evidence base for the downstream implementation-path decision (TASK-003) and assumes neither a
repo wipe nor a reuse.

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact |
| --- | --- | --- | --- |
| SW-3 | QA | PASS | `qa.md` |
| SW-4 | Code/Doc Reviewer | PASS | `review.md` |
| SW-5 | Security Agent | PASS | `sec.md` |

No task is being released that did not pass both SW-4 and SW-5.

---

## Required declaration 1 — Deployment size

**`patch` (documentation-only; no runtime/product change).**

This release ships no compiled or interpreted product code. The product manifest stays at `0.1.0`;
no `src/`, `src-tauri/`, or `observability/` artifact, schema, config, or dependency is added,
changed, or removed. Under semantic versioning the change carries no MAJOR (no breaking change),
no MINOR (no new product feature), and no PATCH-level code fix — it is the smallest possible
deployment class. Nothing is built, packaged, or pushed to any environment; the "deployment" is
the merge of documentation into the repository's OpenSpec change history.

## Required declaration 2 — Rollback strategy

**`forward-only` (trivially reversible by `git revert`; nothing runtime to roll back).**

There is no deployed runtime artifact, migration, or stateful change to reverse, so a staged
rollback rehearsal is not applicable (L2 "tested rollback in staging" — **N/A**, no runtime
surface). If the assessment must be withdrawn, the full and complete reversal is a `git revert`
(or branch deletion) of the documentation commits on `feat/task-001-repo-path-assessment`; it
affects only repository text, never a running system, data store, or downstream consumer. No
forward-fix beyond ordinary doc edits is ever required.

## Required declaration 3 — Component compatibility matrix

This change introduces **no** upstream/downstream version constraints because it modifies no
runtime, dependency, or interface. The matrix below records the existing pinned components named
in the inventory (`salvage-reuse-inventory.md` §2.1, §2.7) — all **unchanged** by this release and
documented for traceability, not altered.

| Component | Role | Min | Max | Status under this release |
| --- | --- | --- | --- | --- |
| Vire app (Tauri) | This product | `0.1.0` | `0.1.0` | Unchanged (no code touched) |
| Tauri runtime | App shell | v2 | v2 | Unchanged; `reuse-as-is` |
| `tauri-plugin-dialog` | Save/dialog IPC | 2.2 | 2.2 | Unchanged |
| Langfuse (dev infra only) | TASK-007 validation env | 3.63.0 | 3.63.0 | Pinned; not shipped with app; untouched |
| PostgreSQL (Langfuse dep) | dev infra | 16 | 16 | Pinned; loopback-bound; untouched |
| Redis (Langfuse dep) | dev infra | 7 | 7 | Pinned; loopback-bound; untouched |
| ClickHouse (Langfuse dep) | dev infra | 24.8 | 24.8 | Pinned; loopback-bound; untouched |
| MinIO (Langfuse dep) | dev infra | bundled | bundled | Pinned; loopback-bound; untouched |

**No new upstream or downstream service dependency** is created by this release. The local Langfuse
stack is dev-only (binds `127.0.0.1`, pinned images) and is not distributed with the application.

---

## L2 release-integrity expectations (SEC-008)

| Expectation | Applicability | Rationale |
| --- | --- | --- |
| Signed git tag | **Applies** (see §5) | Marks the release unit; signing-key provisioning pending in this runner. |
| Signed binary / notarized artifact | **N/A** | No compiled or distributable artifact is produced by a docs-only spike. |
| SBOM (Software Bill of Materials) | **N/A** | No build output and no dependency-graph change to inventory; nothing to enumerate beyond the unchanged, already-pinned dev infra above. |
| Tested rollback in staging | **N/A** | No runtime/deployable surface; reversal is `git revert` of documentation (see Declaration 2). |
| Audit trail | **Present** | SW-3/4/5 gate reports (`qa.md`, `review.md`, `sec.md`) + this RELEASE.md form the evidence chain. |

Per `sec.md` §SEC-008, APP-005 Gate D (release/distribution integrity) does not fire because this
change releases no distributable artifact. The signed-artifact/SBOM expectations are recorded as
**N/A with rationale** rather than silently skipped.

## 5. Tag status

- **Intended tag:** `v0.1.0` (signed), annotated as the TASK-001 assessment release unit.
- **Intended command:** `git tag -s v0.1.0 -m "TASK-001 repo/path assessment (docs-only spike)"`
- **Status: BLOCKED — signing key not provisioned in this runner.** No `user.signingkey` is set and
  no GPG secret key is available, so `git tag -s` cannot produce a signed tag here. Per the L2
  signed-tag requirement, an **unsigned** fallback tag was deliberately **not** created. The tag is
  left as a pending action to be completed once a signing key is available (or by a signing-capable
  runner). This does not block the gate criterion (RELEASE.md with all three declarations), but it
  is an open release-integrity item.
- **Merge:** PR #6 left **OPEN** (not merged). The framework routes next to SW-6 handoff /
  documentation; merge is not performed by this gate.

## Handoff

- **Verification flow (Flow 3, stubbed):** N/A — no deployed artifact to environment-test.
- **Documentation Engineer (`sw-documentation-engineer`, required at L2):** this release triggers
  the doc-update step. RELEASE.md and the gate chain are ready for handoff.
