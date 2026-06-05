# RELEASE — TASK-007 Langfuse importer validation (Phase A spike)

- **Change:** `task-007-langfuse-importer-validation` (TASK-007)
- **Branch:** `feat/task-007-langfuse-importer-validation`
- **PR:** [#8](https://github.com/kaikkone4/vire/pull/8) — OPEN (draft), MERGEABLE, mergeStateStatus CLEAN, base `main`
- **Tier:** L2 · **Gate:** SW-6 (Release)
- **Release Manager:** Release Manager (SW-6)
- **Date:** 2026-06-05
- **Verdict:** **RELEASE-READY** (all three declarations complete; signed tag deferred — see §5)

> Scope: this release unit is the **Phase A validation spike** half of WP-007. Deliverables are
> OpenSpec docs (proposal/design/spec/tasks + `langfuse-validation-report.md` + gate artifacts)
> plus one isolated, non-shipping Node probe under `spikes/task-007-langfuse-importer/`.
> **It ships no product artifact:** no durable importer, no product schema/migration, no product
> runtime, no dependency manifest, no `pi-observe`/emitter change, and no distributable binary is
> created or modified. Per **DEC-018** the spike validated the importer **cloud-first against the
> configured Langfuse API** (`https://cloud.langfuse.com`, project-scoped Basic auth); the local
> Docker stack (`langfuse/langfuse:3.63.0`) and the `pi-observe` emitter are reference-only,
> optional offline/dev contract-test fixtures and are **unchanged** by this diff.
> Deployment/rollback/compatibility are declared against that reality below.

---

## 1. Release unit & gate provenance

This release bundles exactly the TASK-007 change. All upstream SW gates passed:

| Gate | Role | Verdict | Artifact |
| --- | --- | --- | --- |
| SW-3 | QA | **PASS** | `qa.md` |
| SW-4 | Code Review | **PASS** | `review.md` |
| SW-5 | Security | **PASS** | `sec.md` |

No task is released that did not pass both SW-4 and SW-5. `openspec validate
task-007-langfuse-importer-validation --strict` is reported valid by SW-3/SW-4; the probe parses
clean (`node --check`, exit 0) and its offline proof (`node langfuse-probe.mjs --mock`) reports
ALL CHECKS PASS (pagination/dedup/cursor, idempotent overlap, all 9 health states, absence→`null`/
`missing`). Live auth + base-URL + health independently confirmed (`HTTP 200`, shape-only).

**Release-unit boundary:** the durable TASK-007 **MVP importer** is *not* in this release unit —
it ships product code and is gated downstream on TASK-003 (host runtime) and TASK-004 (durable
SQLite import/health tables). See §7.

---

## 2. Required declaration 1 — Deployment size

**PATCH (non-deploying / docs + isolated spike).**

- Semantic-version classification: **PATCH** — no public/runtime surface change, no breaking
  change, no new product feature. The validated schema, import-flow design, and health-state
  model are *design inputs* (to TASK-004 and the MVP), not shipped behavior. PATCH is the honest
  fit on the patch/minor/major/big-bang scale for a docs+spike change.
- **No product deployment occurs.** The change adds OpenSpec documentation and a probe that is
  **not a member of any build target** (verified absent from `package.json`, `Cargo.toml`,
  `tauri.conf.json`) and is not referenced from `src/`, `src-tauri/src/`, or `observability/`.
  Nothing is built, packaged, shipped, or installed to any environment.
- Blast radius: documentation tree (`openspec/changes/task-007-…/`) + `spikes/task-007-langfuse-importer/`
  only. Zero product-runtime impact; no change to the configured Langfuse project (read-only probe).

---

## 3. Required declaration 2 — Rollback strategy

**FORWARD-ONLY (git revert of docs/spike commits) — no runtime rollback applicable.**

- Because nothing is deployed, there is no deployed state to roll back. Reversal, if ever needed,
  is a documentation-level `git revert` of the TASK-007 commits or closing PR #8 unmerged — no
  migration, no data, no service restart, no traffic cutover involved.
- The probe is **read-only** (GET-only, shape-only output) and **disposable**: its README documents
  the delete-after-MVP/TASK-003 instruction, so removal is a directory delete with no dependents.
- **No external side effects to undo:** the live validation only *read* from the configured Langfuse
  API; it wrote nothing, persisted no real trace content, and committed no secrets — so there is no
  remote state to reconcile on rollback. No credential rotation is required (nothing leaked).
- **Staging rollback test (L2 requirement): N/A — no deployable artifact and no staging surface.**
  The L2 "rollback tested in staging before prod" rule presupposes a deployed artifact; there is
  none. The forward-only doc-revert path is inherently safe (no state to diverge) and requires no
  staging rehearsal. The rule re-arms for the **MVP importer** release, which ships a real network
  client and durable tables.

---

## 4. Required declaration 3 — Component compatibility matrix

This change introduces **no product dependency** and **no runtime component**. The matrix declares
the toolchain/platform/services the spike was *validated against* (reproduction + read-only probe
target), not shipped or pinned product dependencies.

| Component | Min version | Max version | Notes |
| --- | --- | --- | --- |
| Node.js (probe runtime) | 18 (global `fetch`/`AbortController`) | current LTS | Single-file ESM; `node:` builtins only, **zero third-party deps** → no CVE surface, no lockfile. |
| Langfuse API (validated source, DEC-018) | public API as served by `cloud.langfuse.com` (2026-06) | — | Cloud-first; read-only GET against `/api/public/*`. Schema observed, not assumed (report §2). Probe is resilient to schema drift via shape-only sketch + `schema_mismatch` health state. |
| Langfuse stack (optional offline fixture) | `langfuse/langfuse:3.63.0` (pinned) | `3.63.0` | **Reference-only, unchanged.** Optional dev/contract-test fixture per DEC-018; not a blocking validation dependency. |
| `pi-observe` emitter | as in repo (`observability/pi-observe`) | as in repo | **Reference-only, unmodified.** Cited for trace/ingestion shape; no adapter built (DEC-017). |
| Product upstream/downstream services | — | — | **None.** Spike commits no network client, no DB writes, no service integration into product. |

**No incompatibility found.** No upstream or downstream **product** service version constraint is
created or violated by this change. The Langfuse API/version constraints above are **forward
inputs** the MVP importer must pin and re-declare when it ships a real client (see §7).

---

## 5. L2 expectations — signed artifacts, SBOM, signed tag

- **Signed artifacts: N/A.** No distributable artifact (binary, image, package) is produced, so
  there is nothing to sign or attest. The probe is non-shipping and links only Node builtins.
- **SBOM: N/A.** An SBOM enumerates the dependency closure of a shipped artifact. No artifact ships
  and no dependency manifest is added/changed; the probe has **zero third-party dependencies**.
  There is no dependency closure to enumerate. (When the MVP importer ships a real network client,
  an SBOM becomes in-scope for its SW-6.)
- **Signed git tag (policy preserved): DEFERRED — not created.** The role requires a *signed* tag
  (`git tag -s v<x.y.z>`), and the signed-tag policy forbids creating an unsigned tag. This
  environment has **no signing key** (no GPG secret key; no `commit.gpgsign`/`tag.gpgsign`/
  `user.signingkey`/`gpg.format` configured), so a signed tag **cannot** be produced here.
  **No unsigned tag was created — policy preserved.** Additionally, PR #8 is still OPEN (draft) and
  unmerged; tagging is correctly applied at the release/merge point on `main`, not on the unmerged
  feature branch.
  - **Proposed tag (to be created, signed, at merge):** `v0.0.3` — third spike release unit
    (TASK-001 → `v0.0.1`, TASK-002 → `v0.0.2` were both proposed but never cut, as no signing key
    has yet been provisioned; the repo currently has **zero tags**). To be cut as
    `git tag -s v0.0.3` once a signing key is provisioned and PR #8 is merged to `main`.

---

## 6. Release approval

- **APP-008** — release approval recorded for TASK-007 SW-6 gate (declarations complete).
- Gate criterion satisfied: **RELEASE.md contains all three required declarations** — deployment
  size (§2), rollback strategy (§3), component compatibility matrix (§4).
- DEC-018 cloud-first context recorded (§0 scope-note, §4): validated against the configured
  Langfuse API; local Docker stack/`pi-observe` reference-only and unchanged.

---

## 7. Outstanding items & MVP follow-ups (non-blocking for the spike gate)

1. **Provision a signing key** and cut the **signed** tag `v0.0.3` at merge to `main` (§5) —
   required to fully satisfy the role's tag output once signing is available.
2. **PR #8 is in draft;** flip to ready-for-review / merge per Pi-Assistant routing.
3. **TASK-007 MVP importer** (durable, ships product code) is split out and gated on **TASK-003**
   (host runtime: Tauri+helper vs Swift-first) and **TASK-004** (durable `langfuse_import_runs` /
   normalized AI-evidence tables). Its SW-6 will re-arm signed-artifact + SBOM + staging-tested
   rollback expectations that are N/A here.
4. **Wire the real Tier-1 scanner stack** (gitleaks / OSV-scanner / Trivy / Sonar) into CI before
   the MVP importer reaches SW-5 — substitute scans were used here per `sec.md`.
5. **MVP advisories carried forward** (from `sec.md`): A1 — confirm persisted import-run logs may
   carry the cursor timestamp/aggregate counts; A2 — consider `redirect: 'manual'` (or same-origin
   assertion) on the real client for defense-in-depth.
6. **Downstream design inputs already routed:** TASK-004 (proposed `langfuse_import_runs` +
   normalized AI-evidence shapes), TASK-006 (validated 9-state health taxonomy), TASK-003 (importer
   host-runtime friction signal), TASK-008/TASK-010 (project-mapping signal assessment only; full
   classification deferred).

---

## 8. Handoff

- **L2 → Documentation Engineer (`sw-documentation-engineer`):** release triggers doc update.
- **Verification flow (Flow 3, stubbed):** no deployed artifact to environment-test — the spike
  produces validated evidence/design + a read-only probe only.
