# QA Report — TASK-047 Tauri/GTK RustSec cleanup (SW-3 recheck)

- **Role**: QA Engineer (SW-3) · **Date**: 2026-06-21
- **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup` · **PR**: #35
- **Tier**: L2 security-posture; no runtime change, no compiled artifact delta.
- **Verdict**: **PASS**

> This report supersedes the prior SW-3 PASS (which was voided by the SW-5 SEC-001 finding) and
> the subsequent SW-2 fix. All SW-4 and SW-5 blockers are closed; this recheck confirms the
> corrected implementation against the spec and tasks.

## SW-4 / SW-5 blocker closure

| Blocker | Source | Fix | Verified |
|---|---|---|---|
| Floating scanner (no binary version pinned) | review.md §1 | `tool: cargo-deny@0.19.9` in workflow line 38 | ✅ grep confirmed |
| Real scans not run | review.md §2 | Real `cargo-deny 0.19.9` run (project-local temp install) documented in `ops-review.md` §3 | ✅ evidence accepted |
| SEC-001: 5 `unic-*` advisories undocumented, gate would fail first run | sec.md | Added to `[advisories].ignore` with rationale; scoped gate exits 0 | ✅ `deny.toml` confirmed |
| "12 total / clean by construction" wording false | sec.md | Corrected to **17 = 12 Linux-only deferred + 5 Apple-present accepted** throughout | ✅ `deny.toml`, `ops-review.md`, `RELEASE.md` confirmed |
| RELEASE.md "byte-identical" claim unverifiable | review.md suggestion | Replaced with "zero shipped-crate delta (`git diff` is empty; no source/IPC/capability/schema change)" | ✅ grep: no "byte-identical" in RELEASE.md |

## Scenario coverage matrix

| Scenario (from `specs/dependency-security/spec.md`) | Status | Evidence |
|---|---|---|
| **S1** Scoped advisory check passes on the shipped Apple targets; `Cargo.{toml,lock}` unchanged | **PASS** | Real `cargo-deny 0.19.9` → `advisories ok`, exit 0 (`ops-review.md` §3/§5 V1). `git diff main -- src-tauri/Cargo.toml src-tauri/Cargo.lock` = 0 bytes (verified). 5 `unic-*` accepted via documented `ignore`; gtk3-rs/glib target-scoped out. CI re-runs the pinned version. |
| **S2** Tripwire: adding a Linux triple re-surfaces gtk3-rs/glib cluster, not silently suppressed | **PASS** | Full-target real `cargo-deny 0.19.9` (Apple+`x86_64-unknown-linux-gnu`, `ignore=[]`) errors on 11 Linux unmaintained IDs; `glib` RUSTSEC-2024-0429 via osv-scanner. `cargo tree -i glib --target aarch64-apple-darwin` → "nothing to print"; `--target x86_64-unknown-linux-gnu` → `glib←atk←gtk←{muda,tao}←tauri`. No gtk3-rs ID in `ignore`; enforcement is `[graph].targets` (tripwire intact). |
| **S3** Risk acceptance traceable to a real scan; per-ID rationale; review trigger documented | **PASS** | 17 IDs from real `cargo-deny 0.19.9` + `osv-scanner` cross-check (not hand-copied). `deny.toml` ACCEPTED block (5 Group-B) and DEFERRED block (12 Group-A) each carry crate, version, class, fix, and Apple reachability. Review trigger in `deny.toml` header and `design.md` §7. |

## Implementation conformance

| Spec requirement | Impl artifact | Status |
|---|---|---|
| `[graph].targets` = `{aarch64,x86_64}-apple-darwin` only | `deny.toml` lines 46–50 | ✅ |
| `[advisories].ignore` = exactly 5 Apple-present `unic-*` IDs (RUSTSEC-2025-0075/0080/0081/0098/0100); gtk3-rs/glib IDs NOT in `ignore` | `deny.toml` lines 64–70; grep `-E '^\s+"RUSTSEC-2024'` = 0 matches | ✅ |
| `unmaintained = "all"` (requires cargo-deny ≥ 0.18) | `deny.toml` line 58 | ✅ |
| `yanked = "deny"` | `deny.toml` line 55 | ✅ |
| `version = 2` | `deny.toml` line 52 | ✅ |
| 17 advisory IDs documented (12 Linux-only DEFERRED + 5 Apple-present ACCEPTED) with per-ID rationale | `deny.toml` ACCEPTED + DEFERRED comment blocks | ✅ |
| CI gate: pinned `cargo-deny@0.19.9`, ubuntu runner, `permissions: contents: read`, correct triggers | `.github/workflows/dependency-advisories.yml` lines 18–38 | ✅ |
| Triggers: `pull_request`, `push` to `main`, `workflow_dispatch` | same file lines 12–16 | ✅ |
| `cargo-deny 0.19.9` version recorded in `deny.toml` header, `tasks.md` T2, `ops-review.md` §2 | all three files | ✅ |
| Local one-liner documented for contributors | `RELEASE.md` v0.6.3; `deny.toml` header | ✅ |
| No `tauri-plugin-updater` Rust deps | `Cargo.lock` (grep 0 matches) | ✅ |
| RELEASE.md: narrowed to "zero shipped-crate delta" (no "byte-identical") | `RELEASE.md` lines 9–10, 43–44 | ✅ |

## Scope integrity

| Check | Result |
|---|---|
| `git diff main -- src-tauri/Cargo.toml src-tauri/Cargo.lock` | **0 bytes** (verified) |
| `git diff main -- src-tauri/src/ src/ src-tauri/tauri.conf.json src-tauri/capabilities/` | **0 bytes** (verified) |
| Changed files on branch | `src-tauri/deny.toml`, `.github/workflows/dependency-advisories.yml`, `RELEASE.md`, `openspec/changes/task-047-…/{arch-review,design,handoff,ops-review,proposal,tasks,qa}.md`, `openspec/changes/task-047-…/specs/dependency-security/spec.md` — config/docs/openspec only |
| `openspec validate task-047-tauri-gtk-rustsec-cleanup --strict` | **PASS** |

## Gate verdicts

| Gate | Status | Note |
|---|---|---|
| V1 `cargo deny check advisories` exits 0 (Apple targets) | **PASS** | Real `cargo-deny 0.19.9` evidence (ops-review.md §3/§5). CI enforces same pinned version. `cargo-deny` not installed in QA environment; SW-2 real-binary evidence accepted; CI gate is the enforcement checkpoint. |
| V2 `git diff main -- src-tauri/Cargo.{toml,lock}` empty | **PASS** | 0 bytes verified. |
| V3 Tripwire re-reports on Linux triple | **PASS** | Full-target real `cargo-deny 0.19.9` (Apple+Linux, `ignore=[]`) errors on 11 Linux unmaintained IDs; `glib` unsound via osv-scanner; `cargo tree` corroborates (ops-review.md §3). |
| V4 No compiled regression | **N/A by construction** | No source/dep change; `deny.toml`/workflow not compiled. |
| V5 `sec.md` records deferred IDs | **PASS (SW-5 artifact)** | `sec.md` written by SW-5; documents the full 17-ID inventory, Group A/B split, and risk acceptance basis. |

## Deferred / known gaps

None. All prior gates are closed or N/A by construction. `cargo-deny` binary not re-run by SW-3 (not installed per policy; consistent with SW-5 approach); the CI gate is the reproducible enforcement point.

## Handoff

**→ SW-4 (Code Reviewer) + SW-5 (Security Agent)** in parallel on PASS.
SW-6 release unblocked for this change once advisory gate is green in CI.
