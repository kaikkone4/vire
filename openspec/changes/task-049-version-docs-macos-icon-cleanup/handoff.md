<!-- Compact state; keep <= 2 KB. -->

# Handoff — TASK-049 v0.8.0 version/docs + macOS DMG cleanup

- **Branch**: `feat/task-049-version-docs-macos-icon-cleanup` · PR #37
- **SW-3 QA**: PASS (initial and post-doc-fix recheck, 2026-06-22). See `qa.md`.
- **SW-4 review**: PASS (post-doc-fix recheck, 2026-06-22). See `review.md`.
- **SW-5 security**: PASS (Tier L2); doc-only fixes require no recheck. See `sec.md`.
- **SW-6 Release Manager**: COMPLETE (2026-06-22). See `RELEASE.md`.
  - Deployment size: **patch** (packaging/docs only)
  - Rollback: **automated**
  - Component compatibility matrix: in `RELEASE.md §3`
  - RELEASE.md: `openspec/changes/task-049-version-docs-macos-icon-cleanup/RELEASE.md`
  - Signed tag: **BLOCKED (non-critical)** — `~/.ssh/id_ed25519.pub` not found; SSH key absent.
  - PR #37: promoted draft → **ready-for-review**.
- **Next**: Janne merges PR #37 to release.

## SW-4 recheck result

All prior documentation blockers are resolved:

- `docs/active-window-capture.md:66` accurately states that title columns exist while TASK-048 always persists `window_title = NULL`.
- `docs/active-window-capture.md:5` accurately states default-OFF behavior: the thread exists, but disabled capture calls no native API and writes no evidence.
- `RELEASE.md:17`, `RELEASE.md:25-30`, and `RELEASE.md:59` distinguish TASK-048 runtime-surface stability from TASK-049 version/DMG packaging metadata.
- Optional README current-version split applied at `README.md:5-7`.

## Scope and checks

- No source, schema, IPC, capability, permission, CSP, or third-party dependency delta.
- `Cargo.lock` changes only the `vire` self-version.
- `git diff --check origin/main...HEAD` — PASS.
- `openspec validate task-049-version-docs-macos-icon-cleanup --strict` — PASS.
- Blockers: none. Suggestions: none. Escalations: none.
