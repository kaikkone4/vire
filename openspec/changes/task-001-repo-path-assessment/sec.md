# Security Review (SW-5) — TASK-001 Repo/Path Assessment

- **Change:** `task-001-repo-path-assessment`
- **Branch:** `feat/task-001-repo-path-assessment` · **PR:** kaikkone4/vire#6
- **Tier:** L2 (secrets + CVE ≥ 7 + Trivy + Sonar quality gate)
- **Reviewer role:** Security Agent (SW-5) · **Date:** 2026-06-04
- **Change type:** Read-only documentation/spike. Diff is 642 insertions across 7 Markdown files, all confined to `openspec/changes/task-001-repo-path-assessment/`. No `src/`, `src-tauri/`, `observability/`, schema, config, or dependency changes.

## Verdict: **PASS**

No auto-fail condition hit. Both replacement and reuse remain open for TASK-003; no source or control was mutated.

## Scope confinement (verified)

`git diff --name-only main...HEAD` → all 7 files under the change dir (`arch-review.md`, `design.md`, `proposal.md`, `qa.md`, `salvage-reuse-inventory.md`, `specs/repo-path-assessment/spec.md`, `tasks.md`). Nothing outside docs touched. Confirmed: `clean: all changes confined to change dir`.

## Tier 1 scanner results (L2)

| Scanner | Auto-fail condition | Result | Notes |
| --- | --- | --- | --- |
| **gitleaks** (secrets) | any detected secret | **PASS** | Binary not installed in this env; performed equivalent manual secret-pattern scan over the diff and tracked tree (below). No secret material committed or produced. |
| **OSV-scanner** (dep CVE) | CVE ≥ 7.0 | **N/A → PASS** | No dependency manifest changed (`Cargo.toml`/`package.json` not in diff). No new dependency attack surface. |
| **Trivy** (container) | HIGH/CRITICAL | **N/A → PASS** | No container image or `docker-compose` change in diff. Existing local Langfuse stack (loopback-bound, pinned images) untouched. |
| **SonarQube CE** (quality gate) | gate = ERROR | **N/A → PASS** | Markdown-only change; no compiled/interpreted product code introduced. Quality gate does not apply to documentation. |

> Tier 1 binaries (`gitleaks`, `osv-scanner`, `trivy`, `sonar-scanner`) are not present on this runner. For a docs-only spike that adds no code or dependencies, the only live security surface is secret/data leakage in the produced text, which was checked manually and directly. This is recorded transparently rather than silently skipped.

## Secret & data-leakage scan (manual, no values printed)

- **Credential-pattern scan over added lines** (`github_pat_`/`ghp_`/`gho_`/`ghu_`/`ghr_`/`sk-ant-`/`sk-proj-`/`sk-…`/`xox[abprs]-`/`AKIA…`/`BEGIN … PRIVATE KEY`): **1 match**, on salvage-reuse-inventory.md §6, which *lists the pattern prefixes by name* to describe the scan ("github_pat_/ghp_/sk-ant-…"). These are token-prefix labels in prose, **not credential values**. Benign.
- **Env-dump / `KEY=value` secret assignments in added docs:** 0.
- **Absolute home/user paths (`/Users/<other>`, `/home/`) that could leak private data:** 0.
- **Raw private data / prompt-response text / window-title / terminal-command bodies / env dumps in the produced artifacts:** none. The salvage doc §10 self-attests to this and it was independently confirmed.

## Repo hygiene (SEC-003)

- Tracked DB/secret artifacts: **none** (`*.sqlite`/`*.db`/`.env` not tracked; only `observability/langfuse/.env.example`).
- `.env.example` non-empty secret/password/key assignments: **0** (all template fields blank).
- `.gitignore` continues to exclude `*.sqlite`, `*.db`, `observability/langfuse/.env`, `.env.*` (with `!.env.example`). Unchanged by this diff.

## APP-005 control preservation (L2 set: SEC-001/002/003/005/006/008)

This change modifies no source, so all existing controls are preserved by construction. Spot-verified the primitives remain present and untouched:

| Control | Status | Evidence |
| --- | --- | --- |
| **SEC-001** capture allowlist | Preserved (N/A today — capture deferred) | Non-collection posture documented; no capture path added. |
| **SEC-002** network boundary / no raw egress | Preserved | Webview CSP (IPC-only) and `pi-observe` loopback gating (`isLoopbackLangfuseHost`) present and unmodified. |
| **SEC-003** credential handling | Preserved | `.gitignore` excludes, empty `.env.example`, `pi-observe` `redact()` present and unmodified; no creds committed. |
| **SEC-005** retention/deletion | Preserved (none existed; none removed) | No raw evidence store touched. |
| **SEC-006** CSV safety | Preserved | `csv_formula_neutralized` + `csv_escape` present in `lib.rs` and unmodified. |
| **SEC-008** release integrity | Preserved (no release/distribution action) | Change releases nothing; APP-005 Gate D does not fire. |

The salvage inventory §5 documents downstream control gaps (importer egress control, retention lifecycle, summary-only export default, SBOM/signing) and correctly routes them to TASK-004/007/010/011/012 without resolving them here.

## Findings

- **Auto-fail findings:** none.
- **Advisory findings:** none blocking. The one credential-pattern hit is a documentation-of-the-scan artifact, not a leak. Optional nicety: future revisions could backtick-fence the pattern list in §6 to keep naive scanners quiet, but it is correctly benign as written.
- **Design-level escalations:** none. The DEC-017 vs. `pi-observe`-emitter tension is a legitimate downstream design input and is correctly deferred to TASK-003/006/007 by the artifact — it is not a security defect in this deliverable.

## Gate handoff

PASS → hold for SW-4 (code review) completion, then release to SW-6 (Release Manager).
