<!-- handoff.md — compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-047 Tauri/GTK RustSec cleanup

- **Branch/PR**: `feat/task-047-tauri-gtk-rustsec-cleanup` / #35.
- **Phase**: SW-1 PASS → SW-3 PASS(stale) → SW-4 FAIL + SW-5 FAIL → **SW-2 fix done** → re-review.
- **Decision**: deferred risk acceptance via Apple-target-scoped `cargo-deny`; no code/Cargo/UI/IPC/
  capability/config delta (`git diff main` empty).

## SW-2 fix (this pass) — all SW-4 + SW-5 blockers closed

- **Pinned `cargo-deny 0.19.9`** in workflow (`tool: cargo-deny@0.19.9`), `deny.toml`, `tasks.md` T2,
  `ops-review.md` §2. (`unmaintained="all"` needs ≥ 0.18; 0.16.4 rejects it.)
- **Real scans** (pinned binary, project-local temp install, not system):
  - Scoped (Apple): `advisories ok`, **exit 0**. Pre-fix ignore=[] errored on exactly the 5 `unic-*`
    → confirms SW-5 SEC-001.
  - Full-target (Apple+Linux, ignore=[]): 16 unmaintained = 5 `unic-*` + 11 Linux (proc-macro-error
    -0370 + gtk3-rs -0411–0420). `glib` -0429 unsound → not enforced by cargo-deny v2; via osv-scanner
    (CVSS 6.9). Inventory total = **17** (osv-scanner cross-check).
- **SEC-001 fixed**: 5 `unic-*` (RUSTSEC-2025-0075/0080/0081/0098/0100) → documented scoped
  `[advisories].ignore` → gate exits 0. 12 gtk3-rs/glib/proc-macro-error kept OUT (tripwire intact).
- **Wording**: "Apple clean / 12 total" → **17 = 12 Linux-only deferred + 5 Apple-present accepted** in
  `deny.toml`, `ops-review.md` §4, `qa.md` (banner; verdict SUPERSEDED, SW-3 must re-issue). RELEASE
  byte-identical → narrowed to no source/manifest/lock/dep-graph delta.

## Files changed

`src-tauri/deny.toml`, `.github/workflows/dependency-advisories.yml`, `RELEASE.md`,
`openspec/changes/task-047-…/{ops-review,tasks,qa,handoff}.md`. openspec validate --strict = PASS.

## Route

Re-review **SW-4 + SW-5** in parallel; then **SW-3** re-issues QA verdict. SW-6 blocked until the
advisory gate is green in CI.
