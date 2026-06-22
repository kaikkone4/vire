# Security Review — TASK-047 Tauri/GTK RustSec cleanup (SW-5 recheck)

- **Role**: Security Agent (SW-5) · **Date**: 2026-06-21
- **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup` · **PR**: #35
- **Tier**: L2 (secrets + CVE≥7 + Trivy + semgrep ERROR)
- **Verdict**: **PASS**

> Recheck after SW-2 remediation of SEC-001 and SW-3 QA PASS. The prior SW-5 verdict was
> **FAIL (code)** because the 5 Apple-present `unic-*` advisories were undocumented and would
> error the gate on its first run. This pass confirms all three SEC-001 remediation items are
> closed.

## SEC-001 closure — verified

| Remediation item | Verified | Evidence |
|---|---|---|
| 5 Apple-present `unic-*` IDs accepted in a documented, scoped `ignore` with rationale | ✅ | `deny.toml` lines 64–70: exactly RUSTSEC-2025-0075/0080/0081/0098/0100, each with per-line rationale; ACCEPTED block (lines 72–92) carries crate/version/class/fix/Apple-reachability + review trigger |
| Scoped advisory gate exits 0 | ✅ | osv cross-check: the only advisories in the Apple graph are these 5 (all now ignored); gtk3-rs/glib absent. SW-2 documented a real `cargo-deny 0.19.9` run → `advisories ok`, exit 0 (`ops-review.md` §3) |
| gtk3-rs/glib/proc-macro-error NOT in `ignore`; stay target-scoped (tripwire intact) | ✅ | `awk` over the `ignore` block → **0** RUSTSEC-2024-* IDs present; `cargo tree -i glib --target aarch64-apple-darwin` = "nothing to print", `--target x86_64-unknown-linux-gnu` resurfaces `glib←atk←gtk←{muda,tao}←tauri` |
| "clean by construction / 12 IDs" wording corrected to 17 = 12 Linux-only deferred + 5 Apple-present accepted | ✅ | `deny.toml` header lines 11–43, `ops-review.md` §4, `qa.md` S3 all state 17-total split |

## Scanner results (Tier-1 stack, run locally — cargo-deny not installed per no-system-install policy)

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** (git) | `main..HEAD` (2 commits) | **0 leaks**, exit 0 | No |
| **gitleaks** (files) | `deny.toml`, `dependency-advisories.yml` (working tree) | **0 leaks** | No |
| **semgrep** (`--config=auto --severity=ERROR`) | `deny.toml`, `dependency-advisories.yml` | **0 findings**, exit 0 | No |
| **Trivy config** (HIGH/CRITICAL) | `dependency-advisories.yml` | **0 misconfigurations**, exit 0 | No |
| **osv-scanner** | `src-tauri/Cargo.lock` (same RustSec data cargo-deny consumes) | 17 advisories: `glib` **CVSS 6.9** (< 7.0) + 16 unscored/unmaintained; 0 Critical, 0 High | No CVE≥7 |

No secrets, no CVE≥7, no semgrep ERROR, no Trivy HIGH/CRITICAL. **No auto-fail rubric condition is hit.**

## Manual review — target-scoped risk acceptance (verified independently)

- **cargo-deny pinned `0.19.9`** — workflow line 38 (`tool: cargo-deny@0.19.9`) and `deny.toml` header
  agree. Reproducible advisory schema/behavior, not just a floating installer action.
- **`ignore` is exactly the 5 Apple-present `unic-*` IDs** — RUSTSEC-2025-0075/0080/0081/0098/0100,
  matching osv-scanner's `unic-*` set on `Cargo.lock` 1:1. No over- or under-suppression.
- **GTK3/glib/proc-macro-error excluded from the shipped graph, NOT ID-suppressed** — 0 RUSTSEC-2024-*
  IDs in `ignore`; `cargo tree` confirms the gtk3-rs/glib chain is absent on `aarch64-apple-darwin`
  and resurfaces only when a Linux triple is added. Tripwire (`[graph].targets` + `unmaintained = "all"`)
  is real and armed.
- **`unic-*` genuinely Apple-present** — `cargo tree -i unic-ucd-ident --target aarch64-apple-darwin`
  → `unic-ucd-ident → urlpattern → tauri-utils → {tauri-build [build-dep], tauri} → vire`. Accepting
  (not deferring) them is correct; it is what makes the scoped gate exit 0.
- **No deps/source/UI/IPC/network/capability changes** — `git diff main` for
  `src-tauri/Cargo.{toml,lock}`, `src-tauri/src/`, `src/`, `src-tauri/tauri.conf.json`,
  `src-tauri/capabilities/` all = **0 bytes**. Diff is config/docs only (`deny.toml`, workflow,
  `RELEASE.md`, openspec docs).
- **No updater Rust deps** — `tauri-plugin-updater` absent from `Cargo.lock`; no `updater` config.
- **CI gate least-privilege** — `permissions: contents: read`; pinned `actions/checkout@v4`,
  `dtolnay/rust-toolchain@stable`, `taiki-e/install-action@v2`; no secrets; triggers `pull_request` /
  `push:main` / `workflow_dispatch`.

## Advisory inventory (17 total)

- **Group A — 12 Linux-only (DEFERRED, target-scoped out):** RUSTSEC-2024-0429 (`glib`, unsound,
  CVSS 6.9, not enforced by cargo-deny v2 — osv-scanner only) + RUSTSEC-2024-0370/0411–0420
  (`proc-macro-error` + 10 gtk3-rs, unmaintained). Absent from the Apple graph; re-surface on a
  Linux triple (tripwire).
- **Group B — 5 Apple-present (ACCEPTED via documented `ignore`):** RUSTSEC-2025-0075/0080/0081/
  0098/0100 (`unic-*`, unmaintained, unscored, rust-unic EOL, no safe upgrade). Pulled transitively
  via `urlpattern → tauri-utils`; no first-party vire code calls `unic-*`. Low risk.

## Escalation

None to BA-flow Architect — the target-scoping design is sound and the SEC-001 config/docs defect is
fully remediated by SW-2.

## Verdict

**PASS.** All Tier-1 (L2) scanners clean; no auto-fail condition; SEC-001 closed; scope integrity
intact. Per role handoff, SW-6 release proceeds once SW-4 (Code Review) also passes and the CI
advisory gate is green.
