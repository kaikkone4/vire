<!-- Compact state; keep <= 2 KB. Reference paths, never paste content. -->

# Handoff — TASK-052 publish v0.8.1 release

- **Change dir**: openspec/changes/task-052-publish-v081-release/
- **Branch / PR**: feat/task-052-publish-v081-release · PR #40 (draft)
- **Phase / gate**: SW-2 backend (Part A code change COMPLETE) → next SW-3 QA
- **Tier**: L2 · **Component**: Vire desktop app + release-ops

## Last gate result
SW-1 architecture review PASS (2026-06-29, `arch-review.md`). SW-2 Part A landed: version
metadata `0.8.0 → 0.8.1` + RELEASE.md v0.8.1 entry. All Part-A verify checks green (see below).

## Active blockers
- none. (TASK-051 signed-tag passphrase blocker is sidestepped: SW-6 creates the tag server-side
  via `gh release create --target <sha>` — Part B, after merge.)

## Exact next action
sw-qa-engineer (SW-3): QA the draft PR. Scope is metadata-only — confirm version triple agrees
(`0.8.1` in Cargo.toml, tauri.conf.json, Cargo.lock `vire`), RELEASE.md v0.8.1 entry present and
v0.8.0 intact, and `src-tauri/src/update_check/` is UNCHANGED. Then SW-4 review, SW-5 security,
merge to `origin/main`. Part B (SW-6 publish) runs only AFTER merge.

## Part A verify results (all PASS, on branch off origin/main@a3bd398)
- `cargo build` OK; `Cargo.lock` `vire` = 0.8.1 (only that line changed — no dep drift).
- `cargo test update_check` = 11 passed. `node --import tsx --test tests/updateCheckUi.test.mjs` = 12 passed.
- `npm run build` OK. `cargo fmt --all -- --check` clean.
- Version triple all `0.8.1`. `openspec validate task-052-publish-v081-release --strict` valid.
- Changed files: src-tauri/{Cargo.toml,tauri.conf.json,Cargo.lock}, RELEASE.md (+ this change dir).

## Required files (read these, not the whole tree)
- arch-review.md — verdict, ownership/ordering, semver, known limitation
- tasks.md — Part A (done) and Part B (SW-6 publish) checklists
- proposal.md — why/what/scope boundaries

## Notes carried forward
- Two ordered parts; hard boundary: Part A = code (SW-2 → gates → merge); Part B = public GitHub
  Release by **SW-6 only after merge**. Do NOT publish before merge.
- Release MUST be a **full** release (NOT draft/prerelease), tag `v0.8.1` on merged `main` SHA —
  `/releases/latest` skips draft/prerelease.
- **Assets out of scope**: notes-only Release; no installer build/sign/upload, no updater plugin.
- Verify both arms post-publish: `0.8.0` build → UpdateAvailable; `0.8.1` build → UpToDate.
- update_check logic, opener capability, CSP, schema, Langfuse, dependencies: untouched.
