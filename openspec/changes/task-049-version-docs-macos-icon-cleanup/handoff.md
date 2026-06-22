<!-- Compact state; keep <= 2 KB. -->

# Handoff — TASK-049 v0.8.0 version/docs + macOS DMG cleanup

- **Branch**: `feat/task-049-version-docs-macos-icon-cleanup` · PR #37
- **SW-3 QA**: PASS (V1–V6); see `qa.md`.
- **SW-4 review**: FAIL (2026-06-22) → SW-2 doc-fix applied (2026-06-22); see `review.md`.
- **SW-5 security**: PASS (Tier L2); doc-only fixes need no recheck. See `sec.md`.
- **Next**: route to SW-4 to re-review the 3 doc corrections.

## SW-2 doc-fix — 3 SW-4 blockers resolved

1. `docs/active-window-capture.md` "no title columns" line → states `window_title`/`title_state` columns exist (TASK-046 schema) but TASK-048 always persists `NULL` and collects no titles; keeps the screenshots/keystrokes/URLs/credentials allowlist.
2. `docs/active-window-capture.md:5` default-OFF → startup thread spawns + reads config each tick, but disabled mode calls no native capture API and writes no evidence.
3. `RELEASE.md` v0.8.0 → TASK-048 runtime-surface claim qualified; `tauri.conf.json` row split in component matrix; new "Version and packaging metadata (TASK-049)" subsection documents `0.1.0→0.8.0` (tauri.conf.json + Cargo.toml) and `bundle.macOS.dmg` layout as packaging-only.
- Optional: `README.md` current-version paragraph split into a short "New in v0.8.0" para (content unchanged).
- No source/schema/IPC/CSP/dep/packaging-semantics change. Checks PASS: `git diff --check`, JSON parse, `openspec validate --strict`.

## Passing scope (unchanged)

- `0.8.0` Tauri/Cargo metadata + Cargo self-version consistent.
- DMG block matches approved layout; README install steps clear.
- No source, schema, IPC, capability, CSP, or third-party dependency delta.
