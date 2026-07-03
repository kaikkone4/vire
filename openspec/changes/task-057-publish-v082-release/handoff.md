<!-- handoff.md - compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-057 publish Vire v0.8.2 release

- **Change dir**: `openspec/changes/task-057-publish-v082-release/`
- **Gate**: SW-2 **Part A DONE** (version bump landed on branch). Next = SW-3 QA.
- **Tier**: L2 · **Split**: NO (one task; internal Part A/Part B boundary)
- **Branch**: `feat/task-057-publish-v082-release`, cut from real `origin/main` `df79423` (incl. TASK-056).

## Part A done (SW-2)
Bumped version triple `0.8.1→0.8.2`: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.lock` (`vire` line only — `cargo build --locked` confirms zero dep drift).
Prepended `RELEASE.md` `## v0.8.2` (TASK-056 Settings UI + unsigned Plan-B/xattr notes; `v0.8.1`/`v0.8.0`
intact). `README.md`: `Current version: v0.8.0→v0.8.2` + two DMG examples `0.8.1→0.8.2`. **5 files
changed, nothing else.** `src-tauri/src/update_check/` untouched; no `tauri-plugin-updater`.

## Verify (all green)
`cargo build --locked` OK; triple all `0.8.2`; `cargo test update_check` 11/0; `updateCheckUi.test.mjs`
12/0; `npm run build` OK; `cargo fmt --check` clean; `openspec validate … --strict` valid.

## Blockers / flags
- Draft PR opened by SW-2 (see stdout for #). Gates (SW-3/4/5) + merge pending.
- Env hygiene (non-blocker): parked 3 stashes on entry — task-056 tracked handoff/review edits, and
  task-053/054 untracked gate-doc drift (`git stash list` top entries). Not part of this branch.
- Flag for Janne (non-blocking): bump is patch `0.8.2` per directive; TASK-056 feature implied `0.9.0`
  by strict SemVer. `0.8.2` valid pre-1.0 and meets the goal (any `>0.8.1` triggers UpdateAvailable).
- feedback_to_ba: DEC-031 (release ordering); FB-054-1 (TASK-028 signing) open. (non-blocking)

## Part B (SW-6, AFTER merge — NOT done here)
`git checkout main` (incl. `0.8.2`+`df79423`) → `npm run tauri:build` → `Vire_0.8.2_aarch64.dmg` →
`gh release create v0.8.2 --target <sha> ... <dmg>` (full, server-side tag; unsigned Plan-B notes).
Live verify: 0.8.1 build → UpdateAvailable(0.8.1→0.8.2); 0.8.2 build → UpToDate.

## Next role
**SW-3 QA Engineer** — gate Part A (`qa.md`).
