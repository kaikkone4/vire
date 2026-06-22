# Code Review — TASK-047 Tauri/GTK RustSec cleanup (SW-4 recheck)

- **Date**: 2026-06-21
- **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup`
- **PR**: #35
- **Verdict**: **PASS**

## Blocking issues

None. Both prior SW-4 blockers are closed.

## Prior blocker verification

1. **Scanner binary is pinned and consistently recorded.** The workflow installs
   `cargo-deny@0.19.9` at `.github/workflows/dependency-advisories.yml:32-38`. The same version is
   recorded in `src-tauri/deny.toml:5-9`, `tasks.md` T2, and `ops-review.md` §2.
2. **Real scoped and full-target evidence is recorded.** `ops-review.md` §3 records real
   `cargo-deny 0.19.9` runs: the shipped Apple-scoped configuration exits 0, while an
   Apple+Linux evidence configuration with `ignore=[]` reports the 5 Apple `unic-*` and 11
   Linux unmaintained advisories. The non-enforced `glib` unsound advisory is separately recorded
   from the `osv-scanner` cross-check.

## Craft and scope verification

- `[advisories].ignore` contains exactly five IDs, all `unic-*`, at
  `src-tauri/deny.toml:64-70`.
- GTK3, `glib`, and `proc-macro-error` advisories remain target-scoped and are not ignored;
  their deferred inventory and tripwire are documented at `src-tauri/deny.toml:95-124`.
- The graph remains scoped only to the two shipped Apple targets at
  `src-tauri/deny.toml:45-50`.
- The corrected posture is consistently stated as 17 advisories: 12 Linux-only deferred and
  5 Apple-present accepted (`src-tauri/deny.toml:12-34`, `RELEASE.md:15-27`,
  `ops-review.md` §4).
- The release claim is narrowed to the verified zero shipped-crate/source delta at
  `RELEASE.md:9-10` and `RELEASE.md:43-46`.
- Commit `3bdda8e` is clearly scoped and describes the fixes and evidence.

## Suggestions

None.

## Escalations to SW Architect

None.

## Checks performed

- `openspec validate task-047-tauri-gtk-rustsec-cleanup --strict` — PASS.
- Workflow YAML parse — PASS.
- `git diff --check main...HEAD` — PASS.
- Cargo manifest/lock, product source, UI, IPC, capability, and Tauri config diff — empty.
- `cargo tree -i glib --target aarch64-apple-darwin` — absent.
- `cargo tree -i glib --target x86_64-unknown-linux-gnu` — GTK3/Tauri chain present.
- `cargo deny check advisories` — not rerun in SW-4 because `cargo-deny` is not installed;
  pinned real-run evidence is recorded in `ops-review.md` §3 and independently accepted by QA.
