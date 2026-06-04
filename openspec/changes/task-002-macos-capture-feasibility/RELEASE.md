# RELEASE — TASK-002 macOS capture feasibility spike

- **Change:** `task-002-macos-capture-feasibility` (TASK-002)
- **Branch:** `feat/task-002-macos-capture-feasibility`
- **PR:** [#7](https://github.com/kaikkone4/vire/pull/7) — OPEN (draft), MERGEABLE, mergeStateStatus CLEAN, base `main`
- **Tier:** L2 · **Gate:** SW-6 (Release)
- **Release Manager:** Release Manager (SW-6)
- **Date:** 2026-06-04
- **Verdict:** **RELEASE-READY** (declarations complete; signed tag deferred — see §5)

> Scope: this release unit is a feasibility **spike**. Deliverables are OpenSpec docs
> (proposal/design/spec/tasks + capture-feasibility report + gate artifacts) plus one
> isolated, non-shipping, compile-only Swift probe under `spikes/task-002-macos-capture/`.
> **It ships no product artifact**: no product runtime, schema, configuration, dependency
> manifest, or distributable binary is changed. Deployment/rollback/compatibility are
> declared against that reality below.

---

## 1. Release unit & gate provenance

This release bundles exactly the TASK-002 change. All upstream SW gates passed:

| Gate | Role | Verdict | Artifact |
| --- | --- | --- | --- |
| SW-3 | QA | **PASS** | `qa.md` |
| SW-4 | Code Review | **PASS** | `review.md` |
| SW-5 | Security | **PASS** | `sec.md` |

No task is released that did not pass both SW-4 and SW-5. `openspec validate
task-002-macos-capture-feasibility --strict` is reported valid by SW-3/SW-4; the probe
typechecks (`swiftc -typecheck`, exit 0) against the macOS 15 CLT SDK.

---

## 2. Required declaration 1 — Deployment size

**PATCH (non-deploying / docs + isolated spike).**

- Semantic-version classification: **PATCH** — no public/runtime surface changes, no
  breaking changes, no new product feature. This is the smallest category in the
  patch/minor/major/big-bang scale and is the honest fit for a docs+spike change.
- **No product deployment occurs.** The change adds OpenSpec documentation and a probe that
  is **not a member of any build target** (verified absent from `Cargo.toml`,
  `tauri.conf.json`, `package.json`; no `Package.swift`/`.xcodeproj` exist) and is not
  referenced from `src/`, `src-tauri/src/`, or `observability/`. Nothing is built, packaged,
  shipped, or installed to any environment.
- Blast radius: documentation tree + `spikes/task-002-macos-capture/` only. Zero product
  runtime impact.

---

## 3. Required declaration 2 — Rollback strategy

**FORWARD-ONLY (git revert of docs/spike commits) — no runtime rollback applicable.**

- Because nothing is deployed, there is no deployed state to roll back. Reversal, if ever
  needed, is a documentation-level `git revert` of the TASK-002 commits or closing PR #7
  unmerged — no migration, no data, no service restart, no traffic cutover involved.
- The isolated probe is explicitly disposable: its README documents the
  delete-after-TASK-003 instruction, so removal is a directory delete with no dependents.
- **Staging rollback test (L2 requirement):** **N/A — no deployable artifact and no staging
  surface.** The L2 "rollback tested in staging before prod" rule presupposes a deployed
  artifact; there is none. The forward-only doc-revert path is inherently safe (no state to
  diverge) and requires no staging rehearsal.

---

## 4. Required declaration 3 — Component compatibility matrix

This change introduces **no product dependency** and **no runtime component**. The matrix
below declares the toolchain/platform the spike was *validated against* (reproduction
environment), not shipped or pinned product dependencies.

| Component | Min version | Max version | Notes |
| --- | --- | --- | --- |
| macOS (target/runtime) | 15.0 | 15.x (validated on 15.7.7, build 24G716) | Probe target `arm64-apple-macosx15.0`. TCC/AX/Quartz behavior assessed on macOS 15. |
| Architecture | arm64 (Apple Silicon) | arm64 | Validated on Apple Silicon. |
| Swift toolchain | 6.2.4 | 6.2.x | `swiftc -typecheck` clean. |
| Command Line Tools SDK | macOS 15 CLT SDK | macOS 15 CLT SDK | No full Xcode required for typecheck. |
| System frameworks (probe-only, no third-party) | AppKit, ApplicationServices, CoreGraphics, Foundation (OS-bundled with macOS 15) | OS-bundled | First-party only; **no external/pinned dependency, no CVE surface**. |
| Product upstream/downstream services | — | — | **None.** Spike has no network client, no DB writes, no service integration. |

**No incompatibility found.** No upstream or downstream product service version constraint
is created or violated by this change.

---

## 5. L2 expectations — signed artifacts, SBOM, signed tag

- **Signed artifacts:** **N/A.** No distributable artifact (binary, image, package) is
  produced by this change, so there is nothing to sign or attest. (The probe is
  compile-only and non-shipping.)
- **SBOM:** **N/A.** SBOM enumerates the dependency closure of a shipped artifact. No
  artifact ships and no dependency manifest is added/changed; the probe links only
  OS-bundled first-party frameworks. There is no dependency closure to enumerate.
- **Signed git tag (policy preserved):** **DEFERRED — not created.** The role requires a
  *signed* tag (`git tag -s v<x.y.z>`); the signed-tag policy forbids creating an unsigned
  tag. No signing key is available in this environment (no GPG secret key, no SSH signing
  key, and no `commit.gpgsign`/`tag.gpgsign`/`user.signingkey`/`gpg.format` configured), so
  a signed tag cannot be produced here. **No unsigned tag was created** — policy preserved.
  Additionally, PR #7 is still OPEN (draft) and unmerged; tagging is appropriately applied
  at the release/merge point on `main`, not on the unmerged feature branch.
  - **Proposed tag (to be created, signed, at merge):** `v0.0.1` (first tag in repo; no
    existing tags). To be cut as `git tag -s v0.0.1` once a signing key is provisioned and
    PR #7 is merged to `main`.

---

## 6. Release approval

- **APP-008** — release approval recorded for TASK-002 SW-6 gate (declarations complete).
- Gate criterion satisfied: **RELEASE.md contains all three required declarations**
  (deployment size §2, rollback strategy §3, component compatibility matrix §4).

---

## 7. Outstanding items (non-blocking for the spike gate)

1. Provision a signing key and cut the **signed** tag `v0.0.1` at merge to `main`
   (§5) — required to fully satisfy the role's tag output once signing is available.
2. PR #7 is in **draft**; flip to ready-for-review / merge per Pi-Assistant routing.
3. Downstream handoffs already routed by prior gates: FB-002 permissions-screen
   reconciliation to BA/UX; raw-title redaction to remain first-class in the TASK-005
   adapter (per `sec.md` / report §7.4).

---

## 8. Handoff

- **L2 → Documentation Engineer (`sw-documentation-engineer`):** release triggers doc update.
- **Verification flow (Flow 3, stubbed):** no deployed artifact to environment-test — spike
  produces evidence/docs only.
