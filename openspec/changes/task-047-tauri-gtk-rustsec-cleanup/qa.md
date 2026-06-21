# QA Report — TASK-047 Tauri/GTK RustSec cleanup (SW-3)

> ⚠️ **SUPERSEDED (2026-06-21, by SW-2) — SW-3 recheck pending.** This report's PASS predates SW-5
> **SEC-001**, which proved the shipped Apple graph is **not** advisory-free: 5 Apple-present `unic-*`
> advisories were missing from the risk acceptance, so the scoped gate would have failed its first run.
> SW-2 has since (a) pinned the scanner to **cargo-deny 0.19.9**, (b) run the **real** scoped + full-target
> scans, (c) added the 5 `unic-*` to a documented `ignore` (scoped check now exits 0), and (d) corrected the
> "12 total / clean by construction" wording to **17 total = 12 Linux-only deferred + 5 Apple-present
> accepted**. The factual claims below are annotated inline; the overall verdict must be **re-issued by
> SW-3** against the corrected `deny.toml` / `ops-review.md` §4 / `sec.md`. Do not treat the PASS as current.

- **Role**: QA Engineer (SW-3) · **Date**: 2026-06-21
- **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup` · **PR**: #35 (draft)
- **Tier**: L2 security-posture; no runtime change, no compiled artifact delta.
- **Verdict**: **PASS** *(SUPERSEDED — see banner; SW-3 recheck pending after SW-2 SEC-001 fix)*

## Scenario coverage matrix

| Scenario (from `specs/dependency-security/spec.md`) | Status | Evidence |
|---|---|---|
| **S1** Scoped advisory check passes on the shipped Apple targets (exits 0, Cargo.{toml,lock} unchanged) | **PASS** *(corrected by SW-2: now verified with real `cargo-deny 0.19.9` → exit 0)* | `git diff main -- src-tauri/Cargo.{toml,lock}` = 0 bytes (V2 PASS). **Correction:** the Apple graph is *not* gtk3-rs-free of all advisories — gtk3-rs/glib are absent (`cargo tree`), but the 5 `unic-*` ARE present and are now accepted via documented `ignore`. Real `cargo deny check advisories` (0.19.9) exits 0 (`ops-review.md` §3). CI re-runs the pinned version. |
| **S2** Tripwire: adding a Linux triple re-surfaces the gtk3-rs/glib cluster, not silently suppressed | **PASS** *(corrected by SW-2: real-binary evidence)* | Real full-target `cargo-deny 0.19.9` (Apple+Linux, `ignore=[]`) errors on the 11 Linux unmaintained IDs; `glib` -0429 unsound via osv-scanner; corroborated by `cargo tree -i glib --target x86_64-unknown-linux-gnu` → `glib←atk←gtk←{muda,tao}←tauri` (`ops-review.md` §3). `[graph].targets` scoping (not an `ignore` of the gtk3-rs IDs) is the enforcement mechanism. **Correction:** `ignore` is **not** empty — it holds the 5 Apple-present `unic-*` IDs only; the 12 gtk3-rs/glib IDs are deliberately kept OUT of `ignore` so the tripwire is intact. |
| **S3** Risk acceptance traceable to scan: advisory IDs present with per-ID rationale, generated from real scan, review trigger documented | **PASS** *(corrected by SW-2: 17 total, not 12)* | **17 IDs** across two groups in `deny.toml` (Group A: 12 Linux-only DEFERRED, target-scoped out incl. glib RUSTSEC-2024-0429 + proc-macro-error -0370 + gtk3-rs -0411–0420; Group B: 5 Apple-present `unic-*` RUSTSEC-2025-0075/0080/0081/0098/0100 ACCEPTED via documented `ignore`). Generated from a **real** `cargo-deny 0.19.9` scan + `osv-scanner` cross-check against exact `Cargo.lock` versions (`ops-review.md` §3, §4). Review trigger explicit in `deny.toml` header and `design.md` §7. |

## File-scope verification

| Check | Result |
|---|---|
| Changed files on branch vs main | `src-tauri/deny.toml`, `.github/workflows/dependency-advisories.yml`, `RELEASE.md`, `openspec/changes/task-047-…/{arch-review,design,handoff,ops-review,proposal,tasks}.md`, `openspec/changes/task-047-…/specs/dependency-security/spec.md` |
| Zero Cargo.toml / Cargo.lock delta | **PASS** — 0 bytes (git diff) |
| Zero source / IPC / schema / capability / tauri.conf.json changes | **PASS** — no `src-tauri/src/**`, `src/**`, `tauri.conf.json`, or capability files in the diff |
| Zero product-code / feature / UI changes | **PASS** |
| `tauri-plugin-updater` absent from Cargo.lock | **PASS** — grep returns 0 matches |

## Implementation conformance

| Spec requirement | Impl artifact | Conforms? |
|---|---|---|
| `[graph].targets` scoped to `aarch64-apple-darwin` + `x86_64-apple-darwin` only | `deny.toml` lines 21–24 | ✅ |
| `[advisories].ignore` holds ONLY the 5 Apple-present `unic-*` IDs (gtk3-rs/glib OUT — no blanket suppression) | `deny.toml` `[advisories].ignore` block | ✅ *(corrected by SW-2)* |
| `unmaintained = "all"` — unmaintained advisories armed for tripwire (requires cargo-deny ≥ 0.18) | `deny.toml` `[advisories]` | ✅ |
| `yanked = "deny"` | `deny.toml` `[advisories]` | ✅ |
| 17 advisory IDs documented (12 Linux-only DEFERRED + 5 Apple-present ACCEPTED) with per-ID rationale | `deny.toml` ACCEPTED + DEFERRED blocks | ✅ *(corrected by SW-2)* |
| CI gate: pinned `cargo deny check advisories` (cargo-deny **0.19.9**), ubuntu runner, pinned actions, `contents: read` | `.github/workflows/dependency-advisories.yml` | ✅ *(version now pinned)* |
| Triggers: `pull_request`, `push` to `main`, `workflow_dispatch` | same file | ✅ |
| Tauri v2 correction documented | `design.md` §1, `RELEASE.md` v0.6.3 | ✅ |
| No `tauri-plugin-updater` Rust deps | `RELEASE.md` v0.6.3, Cargo.lock verified | ✅ |
| Local one-liner documented for contributors | `RELEASE.md` v0.6.3 | ✅ |

## Gate verdicts

| Gate | Status | Note |
|---|---|---|
| V1 `cargo deny check advisories` exits 0 (Apple targets) | **PASS (real binary)** *(corrected by SW-2)* | `cargo-deny 0.19.9` → `advisories ok`, exit 0. 5 `unic-*` accepted via documented `ignore`; gtk3-rs/glib target-scoped out. CI re-runs the pinned version on every PR/push. |
| V2 `git diff main -- src-tauri/Cargo.{toml,lock}` empty | **PASS** | 0 bytes verified locally. |
| V3 Tripwire re-reports on Linux triple | **PASS (real binary)** *(corrected by SW-2)* | Full-target `cargo-deny 0.19.9` (Apple+Linux, `ignore=[]`) errors on the 11 Linux unmaintained IDs; `glib` -0429 unsound via osv-scanner; `cargo tree` corroborates. |
| V4 No compiled regression | **N/A by construction** | No source/dep change; deny.toml/workflow are not compiled. |
| V5 `sec.md` records deferred IDs | **HANDOFF TO SW-5** | All inputs in `ops-review.md` §4 + `deny.toml`; not SW-3 owned. |

## Deferred / known gaps

- **V1/V3 local binary run**: *(corrected by SW-2)* now executed with the **real** pinned `cargo-deny 0.19.9`
  (project-local temp install, not system) — no longer deferred. CI re-runs the same pinned version as the
  enforcement point.
- **V5 `sec.md`**: SW-5 artifact; inputs fully provided in `ops-review.md` §4 (both advisory groups).
- **Pending**: SW-3 must re-issue the overall verdict against the corrected config (see top banner).

## Handoff

**→ SW-4 (Code Reviewer) + SW-5 (Security Agent)** in parallel.
SW-5 should write `sec.md` from `ops-review.md` §4; the deferred IDs, reachability proof, and risk acceptance are all documented there and in `deny.toml`.
