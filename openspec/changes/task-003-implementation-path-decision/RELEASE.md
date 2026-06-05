# Release — TASK-003 Implementation-path Decision

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-003-implementation-path-decision`
- **Branch:** `feat/task-003-implementation-path-decision` · **PR:** [kaikkone4/vire#9](https://github.com/kaikkone4/vire/pull/9)
- **Release unit:** the implementation-path decision deliverable — ADR **DEC-019** + options matrix + technology verification register + downstream architecture constraints (documentation only)
- **Date:** 2026-06-05
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`) — **unchanged by this release**
- **Verdict:** RELEASE-READY (all three declarations complete; signed tag deferred — see §5)

## Release contents

A single, coherent decision unit comprising the documentation artifacts confined to
`openspec/changes/task-003-implementation-path-decision/`:

- `proposal.md`, `tasks.md`, `design.md` (ADR DEC-019 + options matrix + technology register),
  `arch-review.md`
- `specs/implementation-path-decision/spec.md` (one ADDED capability, 4 requirements / 6 scenarios)
- `qa.md` (SW-3), `review.md` (SW-4), `sec.md` (SW-5) gate reports

No product source, schema, configuration, container image, or dependency manifest is modified
(`git diff --name-only main...HEAD` → docs only, all under the change dir; none under `src/`,
`src-tauri/src/`, or `observability/`). This is a decision record only: it fixes forward the
architecture constraints TASK-004/005/006/007-MVP inherit. It builds neither the schema, capture
adapter, durable importer, nor runtime observer, and it neither migrates nor wipes the legacy
manual-tracker surface (that migrate-vs-retire call is deferred to TASK-004).

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact |
| --- | --- | --- | --- |
| SW-3 | QA Engineer | PASS | `qa.md` |
| SW-4 | Code/Doc Reviewer | PASS (one non-blocking merge-ordering condition — see §6) | `review.md` |
| SW-5 | Security Agent | PASS | `sec.md` |

No task is being released that did not pass both SW-4 and SW-5.

---

## Required declaration 1 — Deployment size

**`patch` (documentation-only decision record; no runtime/product change).**

This release ships no compiled or interpreted product code. The product manifest stays at `0.1.0`;
no `src/`, `src-tauri/src/`, or `observability/` artifact, schema, config, build target, or
dependency is added, changed, or removed. Under semantic versioning the change carries no MAJOR (no
breaking change), no MINOR (no new product feature), and no PATCH-level code fix — it is the
smallest possible deployment class. Nothing is built, packaged, or pushed to any environment; the
"deployment" is the merge of an ADR (DEC-019) and its spec delta into the repository's OpenSpec
change history.

## Required declaration 2 — Rollback strategy

**`forward-only` (trivially reversible by `git revert`; nothing runtime to roll back).**

There is no deployed runtime artifact, migration, or stateful change to reverse, so a staged
rollback rehearsal is not applicable (L2 "tested rollback in staging" — **N/A**, no runtime
surface). If the decision must be withdrawn, the full and complete reversal is a `git revert` (or
branch deletion) of the documentation commits on `feat/task-003-implementation-path-decision`; it
affects only repository text, never a running system, data store, or downstream consumer. Because
DEC-019 is a forward-fixing constraint (it confirms/operationalizes DEC-008/DEC-009 rather than
diverging), reverting it would re-open the implementation-path question for TASK-004 but break no
shipped behaviour. No forward-fix beyond ordinary doc edits is ever required.

## Required declaration 3 — Component compatibility matrix

This change introduces **no** new upstream/downstream version constraint because it modifies no
runtime, dependency, or interface. The matrix below records the components the **decision rests on**
(from the `design.md` technology verification register and TASK-001 inventory) — all **unchanged**
by this release and documented for traceability, not altered. Versions cited are the minimum the
DEC-019 path assumes; "Max" is left open where the decision pins no upper bound.

| Component | Role under DEC-019 | Min | Max | Verification status (design.md register) |
| --- | --- | --- | --- | --- |
| Vire app (Tauri shell) | This product (reuse path) | `0.1.0` | `0.1.0` | Unchanged (no code touched) |
| Tauri runtime | App shell — `reuse-as-is` | v2 | open | verified |
| Tauri v2 sidecar (`externalBin`) | Bundle/run native Swift capture helper | v2 | open | verified (v2.tauri.app/develop/sidecar) |
| Tauri v2 IPC | helper↔core / core↔frontend; renderer isolation | v2 | open | verified |
| Tauri v2 HTTP client (`reqwest`, URL-allowlisted) | Langfuse REST pull from Rust core | v2 | open | verified (SEC-002 fit) |
| `tauri-plugin-shell` | Sidecar stdin/stdout lifecycle | 2.x | open | verified (sidecar spawn API) |
| macOS TCC / Accessibility | AX grant on the signed helper binary | macOS 13+ (AXUIElement) | open | verified |
| macOS nested-binary codesign + notarization | Sign/notarize embedded sidecar so TCC + Gatekeeper hold | n/a | n/a | **assumption — TASK-005 packaging spike** |
| Swift / AppKit | Native capture helper language/framework | Swift 5 / current SDK | open | verified |
| SQLite (`rusqlite`, bundled) | Local store on the reused path | 3.x (bundled) | bundled | verified |
| ActivityWatch | Reference/benchmark only — **not adopted** | n/a | n/a | verified (as reference) |
| Langfuse (configured source) | Read-only cloud-first import target (DEC-018) | per deployment config | open | boundary fixed; instance version owned by TASK-007-MVP |

**No new upstream or downstream service dependency** is created or shipped by this release. The
single decision-load-bearing item not yet verified — **nested-binary codesign/notarization of the
sidecar** — is explicitly carried as an assumption with a named **TASK-005 packaging-spike**
follow-up, and is the only signal that would reopen DEC-019 (toward Option B). It is not asserted as
fact.

---

## L2 release-integrity expectations (SEC-008)

| Expectation | Applicability | Rationale |
| --- | --- | --- |
| Signed git tag | **Applies** (see §5) | Marks the release unit; signing-key provisioning pending in this runner. |
| Signed binary / notarized artifact | **N/A** | No compiled or distributable artifact is produced by a docs-only decision change. |
| SBOM (Software Bill of Materials) | **N/A** | No build output and no dependency-graph change to inventory; nothing to enumerate beyond the unchanged, already-pinned components above. |
| Tested rollback in staging | **N/A** | No runtime/deployable surface; reversal is `git revert` of documentation (see Declaration 2). |
| Audit trail | **Present** | SW-3/4/5 gate reports (`qa.md`, `review.md`, `sec.md`) + this RELEASE.md form the evidence chain. |

Per `sec.md`, APP-005 Gate D (release/distribution integrity) does not fire because this change
releases no distributable artifact. The signed-artifact/SBOM expectations are recorded as **N/A with
rationale** rather than silently skipped, consistent with the TASK-001/002/007 sibling releases.

## 5. Tag status

- **Intended tag:** `v0.1.0` (signed), annotated as the TASK-003 implementation-path decision
  release unit. (Product baseline is unchanged at `0.1.0`; this docs-only decision adds no product
  version increment.)
- **Intended command:** `git tag -s v0.1.0 -m "TASK-003 implementation-path decision DEC-019 (docs-only)"`
- **Status: DEFERRED — signing key not provisioned in this runner.** No `user.signingkey` is set and
  no GPG secret key is available, so `git tag -s` cannot produce a *signed* tag here. Per the L2
  signed-tag requirement, an **unsigned** fallback tag was deliberately **not** created (the
  signed-tag policy forbids substituting an unsigned tag). The tag is left as a pending action for a
  signing-capable runner. This does not block the gate criterion (RELEASE.md with all three
  declarations), but it is an open release-integrity item — identical disposition to the
  TASK-001/002/007 releases.

## 6. Merge-ordering condition (SW-4 §5) — RESOLVED

SW-4 carried one non-blocking condition: PR #9's cross-references to TASK-001/002/007 resolve only
on the feeder branches, which were **not yet merged to `main`** at review time, so PR #9 had to
merge after/with the three feeder PRs for citations to resolve on `main`.

**Status at release (verified 2026-06-05, post-`git fetch`):** all three feeders are now **merged
into `origin/main`** — `git merge-base --is-ancestor origin/feat/task-00{1,2,7}-… origin/main` → all
MERGED; the referenced files (`salvage-reuse-inventory.md`, `capture-feasibility-report.md`,
`arch-review.md`) are present on `origin/main`. Their PRs (#6/#7/#8) are MERGED. The cross-reference
dependency is therefore **satisfied**: merging PR #9 now lands DEC-019 on a `main` where every
citation resolves. **No merge-ordering blocker remains.**

## 7. PR ready-for-review

Per role v3.1.2, the draft PR is promoted to ready-for-review as the final SW-6 step (all of QA,
code review, and security have passed; RELEASE.md is complete). See the STATUS report for the
`gh pr ready #9` result and final URL.

## Handoff

- **Verification flow (Flow 3, stubbed):** N/A — no deployed artifact to environment-test.
- **Documentation Engineer (`sw-documentation-engineer`, required at L2):** this release triggers
  the doc-update step. RELEASE.md and the SW-3/4/5 gate chain are ready for handoff.
