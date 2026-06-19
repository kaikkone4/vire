# Handoff — TASK-030 task-030-create-project-button-fix

- **Change dir:** `openspec/changes/task-030-create-project-button-fix/`
- **Branch / PR:** `feat/task-030-create-project-button-fix` / PR #25
- **Phase / gate:** SW-6 Docs COMPLETE (all gates done)
- **Tier:** L2 project (this task L1-equivalent: no new backend/egress/deps)

## Last gate result

SW-6 Docs COMPLETE. README.md updated: button label reference corrected from "Create project for
`<env>`" (old `window.prompt()` single-button label) to describe the new inline input + **Create &
map** button. One line changed (`README.md:54`). `docs/langfuse-local-setup.md` verified clean —
no button label reference, no change needed.

## Tag status — BLOCKED (SSH private key absent)

`task-030/v0.3.1` cannot be signed in this environment (`~/.ssh/id_ed25519` absent). Dry-run
documented in `RELEASE.md §Tag signing`. Manual action required before finalizing release:

```
git tag -s task-030/v0.3.1 -m "release(task-030): v0.3.1 patch — in-app input for env create-and-map" db29bae
git push origin task-030/v0.3.1
```

## Active blockers

- SSH private key absent — signed tag `task-030/v0.3.1` pending manual action (see above).
- Packaged macOS T6 manual UAT remains outstanding and human-only.

## Non-blocking (repo-level, NOT TASK-030)

- OSV inherited dev-dep advisory: bump `vite` ≥ 6.4.3 (GHSA-fx2h-pf6j-xcff, CVSS 8.2, dev-only)
  and `esbuild` ≥ 0.28.1. Track at repo level; not introduced by this change.

## Merge ordering

PR #24 already merged to `main` (2026-06-19). PR #25 base updated to `main`; no ordering
dependency remains. Janne merges PR #25 at will. Branch carries some TASK-029 gate-artifact
commits as diff noise (see `RELEASE.md §Merge ordering`); a rebase onto origin/main would
produce a cleaner diff but is not strictly required.

## Checks

- Focused env-mapping tests: 11/11 passed.
- Production build: passed.
- Cargo: 142 unit + 3 integration tests passed.
- Full frontend: 72/74 passed; two unrelated socket-based tests hit sandbox `EPERM`.
- No `prompt()` calls in `src/`; `git diff --check` passed.
- Docs gate: README.md button label corrected; `docs/langfuse-local-setup.md` verified clean.

## Required files

- `RELEASE.md` — SW-6 gate artifact (this change dir)
- `review.md` — SW-4 Code Review
- `sec.md` — SW-5 Security
- `qa.md` — SW-3 QA
- Root `RELEASE.md` — v0.3.1 entry added
- `README.md` — docs gate: button label updated (line 54)
