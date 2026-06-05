# Documentation Review — TASK-007 Langfuse importer validation spike

- **Change:** `task-007-langfuse-importer-validation`
- **Branch:** `feat/task-007-langfuse-importer-validation`
- **PR:** https://github.com/kaikkone4/vire/pull/8
- **Tier:** L2 · **Gate:** SW-6 (Documentation)
- **Reviewer role:** Documentation Engineer (SW-6)
- **Date:** 2026-06-05
- **Verdict:** **No drift — no README/docs/runbook changes required now**

> Scope note: this is the **Phase A validation spike** half of WP-007. It ships nothing,
> changes no product runtime, and decides no implementation path (TASK-003 owns the host
> runtime; TASK-004 owns durable schema; the TASK-007 **MVP** ships the importer). Deliverables
> are the Langfuse validation report, the proven import-flow design, the source-health state
> model, the mapping-signal assessment, and the TASK-004 shape proposals — all under the
> OpenSpec change dir — plus one isolated, non-shipping, parse-validated Node probe. SW-6 here
> verifies documentation drift against **shipped state** and downstream handoff sufficiency; it
> does not document a released feature (none was released).

---

## 1. What was reviewed

| Surface | State |
| --- | --- |
| `README.md` | Describes Vire **v0.1** (local-only Tauri v2 manual time tracker; automatic capture deferred; "Privacy status" affirms no accounts/cloud sync/hosted API/**network feature**). |
| `docs/` | Does not exist. Not required at L2 for a spike that ships nothing. |
| `RELEASE.md` / `CHANGELOG.md` | Do not exist for this change. No release is cut by this spike (Release Manager owns SW-6 release record if one is produced). |
| Probe docs | `spikes/task-007-langfuse-importer/README.md` + the probe header in `probe/langfuse-probe.mjs`. |
| Dev/observability docs | `observability/langfuse/README.md`, `observability/langfuse/.env.example`, `observability/pi-observe/README.md`, `scripts/*.sh` — **reference-only**, unmodified by this change. |
| Spike deliverables | `openspec/changes/task-007-langfuse-importer-validation/` (report, proposal, design, arch-review, spec, tasks, qa/review/sec gates). |

Branch diff vs `main` confirms **zero product-runtime changes**: nothing under `src/`,
`src-tauri/src/`, `observability/`, `index.html`, `package.json`, or `tsconfig.json` is
modified. README setup commands (`npm install`, `tauri:dev`, `tauri:build`, `npm test`,
`test:frontend`) still match `package.json` scripts — no setup drift.

Upstream gates all **PASS**: QA (`qa.md`), Code Review (`review.md`), Security (`sec.md`).

---

## 2. Drift assessment against the four SW-6 questions

### Q1 — Cloud-first Langfuse import model (DEC-018): README now, or downstream only? **Downstream only — do NOT add to README now.**

DEC-018 makes Janne's **configured Langfuse API project the canonical AI usage/cost source**,
validated cloud-first (configurable base URL, project-scoped credentials, environment/date
filtering), with the local Docker stack demoted to an optional dev/contract-test fixture. This
is a **design decision for the future importer**, not shipped behaviour:

1. **Doc-version alignment.** The README documents the **shipped product (v0.1)**, which has no
   importer at all. The durable importer is owned by the **TASK-007 MVP** (Phase B), gated on
   TASK-003 (host runtime) and TASK-004 (durable schema). Documenting a cloud-first import model
   now would describe behaviour that does not exist in the shipped app.
2. **Already captured where it belongs.** The model is fully documented in `design.md` (Decision:
   cloud-first), `proposal.md` (What Changes), `arch-review.md`, and the validation report
   (§0/§2/§6) — and routed downstream to TASK-003/TASK-004/TASK-007-MVP.

### Q2 — Credential setup: README/runbook now, or downstream only? **Downstream only — do NOT add to README now.**

There is **no shipped credential surface**. Credentials (`LANGFUSE_HOST`/`PUBLIC_KEY`/
`SECRET_KEY`) are consumed only by the **non-shipping probe**, which already documents secure
local-config handling in `spikes/task-007-langfuse-importer/README.md` (SEC-002/SEC-003 section)
and `observability/langfuse/.env.example` (redacted placeholders). Adding credential-setup
instructions to the product README would imply a network/credential feature exists in v0.1,
contradicting the shipped "no network feature" privacy promise. The operator-facing credential
runbook is a **TASK-007 MVP release-docs obligation** for when the importer actually ships.

### Q3 — Wrong/default environment finding: README now, or downstream only? **Downstream only — it is a validation finding, not product behaviour.**

The empirically-confirmed wrong-env finding (pi/Claude traffic landed in `default`; the `vire`
environment was empty over the live window) is a **risk-retirement result**, not a user-facing
feature. It is recorded in the validation report (§2/§5/§12) and encoded as the first-class
`wrong_env` health state (design.md health-state table). It feeds TASK-006 (reconciliation
taxonomy) and the TASK-007 MVP. Nothing about it belongs in the v0.1 README.

### Q4 — Local-only boundary: does DEC-018 break the README's "no network feature" promise? **No drift now — but a future release-docs obligation is recorded below.**

The README "Privacy status" states v0.1 has "no accounts, cloud sync, hosted API, or network
feature." This **remains accurate for the shipped product**: the spike adds no product network
path. The only outbound traffic is the **non-shipping, read-only probe** (GET-only, configured
Langfuse base URL only, shape-only output — verified by `sec.md` / report §10), which is not a
member of any build target and is disposable. The local-only product boundary is therefore
intact and the README needs no change.

**Action for a future release (recorded so it is not lost):** when the **TASK-007 MVP** ships
the durable importer, it introduces a genuine outbound network path to the configured Langfuse
API. At that point the README "Privacy status" (and a future installation/maintenance guide +
credential runbook) **must** be updated to disclose: the configured-Langfuse-only import path,
that only existing AI traces are pulled (no raw macOS activity/prompt/command-body/env egress),
and that credentials live in local secure config and never in SQLite/logs/exports. This is a
**TASK-007 MVP release-docs obligation** (SEC-007 transparency), not a Phase-A spike one.

---

## 3. Probe & handoff documentation sufficiency

- **Probe is self-documented (no addition warranted).** `spikes/task-007-langfuse-importer/
  README.md` covers purpose/non-shipping status, the read-API surface exercised, isolation
  guarantees (outside `src/`/`src-tauri/src/`/`observability/`; not in any build graph), the
  SEC-002/SEC-003 privacy rules (shape-only, configured-endpoint-only, credential handling),
  `--mock` vs live run steps, the parse-only check, and the **delete-after-MVP** disposal note.
- **Downstream handoffs are concrete and traceable.** Report §6 (import-flow), §8 (TASK-004
  `langfuse_import_runs` + normalized-evidence shapes), §4 (health-state taxonomy for TASK-006),
  §7 (mapping signals for TASK-008), and the TASK-003 host-runtime friction signal. QA, Code
  Review, and Security gates independently verified traceability with no unsupported checkbox.

---

## 4. Conclusion

No documentation drift exists against the shipped state, and no README, `docs/`, or runbook
changes are appropriate for this Phase-A spike:

- The cloud-first model (Q1), credential setup (Q2), and wrong-env finding (Q3) belong in
  OpenSpec/downstream and are already there; adding them to the v0.1 README would break
  doc-version alignment and imply a network feature the shipped app does not have.
- The local-only boundary (Q4) is intact for v0.1; the spike's only network path is the
  isolated, non-shipping, read-only probe. The future importer's disclosure duty is recorded
  above as a **TASK-007 MVP release-docs obligation**.

The only documentation this spike should produce is the OpenSpec change deliverables (already
present) and this SW-6 record. **No README/PR/doc content changes were made** beyond adding
this report.

**DOCS STATUS: complete.**
