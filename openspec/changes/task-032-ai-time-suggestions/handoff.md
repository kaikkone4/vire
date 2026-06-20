<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-032 AI time-entry suggestions

- **Change dir**: openspec/changes/task-032-ai-time-suggestions/
- **Branch / PR**: `feat/task-032-ai-time-suggestions` · PR #27 (ready-for-review)
- **Phase / gate**: SW-6 Release (PASS, 2026-06-20)
- **Tier**: L2

## Last gate result
SW-6 PASS, 2026-06-20. RELEASE.md written (task + root). PR #27 promoted draft→ready.
Tag `task-032/v0.4.0` dry-run only — SSH private key absent (same constraint as prior releases).

## Active blockers
- SSH signing key absent: Janne must run `git tag -s task-032/v0.4.0 ...` + push tag (see RELEASE.md §Tag signing).
- Manual macOS UAT M1–M4 outstanding (human-only, packaged `.app`).

## Exact next action
Janne: (1) create signed tag on commit `fd5cf12`, push tag; (2) merge PR #27; (3) run UAT M1–M4 on packaged app.
Optionally: route to sw-documentation-engineer (L2 doc update trigger).

## Required files (read these, not the whole tree)
- `RELEASE.md` — SW-6 output; three required declarations + tag dry-run + gate checklist
- `sec.md` — SW-5 PASS + advisories (A1 vite, A2 GTK-RUSTSEC, A3 cross-midnight test)
- `review.md` — SW-4 PASS

## Notes carried forward
- v0.4.0: minor bump (new feature set A+B+C); base v0.3.2.
- Two additive schema changes: `time_entry_suggestions` table + `time_entries.origin` column.
- Zero dep delta vs `main` (Cargo.toml + package.json unchanged).
- Pre-existing advisories: vite CVSS 8.2 dev-only → separate dep-bump task; GTK RUSTSEC → Tauri version bump task.
- Non-blocking: cross-midnight policy test (review.md §Suggestions).
