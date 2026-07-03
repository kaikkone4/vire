<!-- handoff.md - compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-057 publish Vire v0.8.2 release

- **Change dir**: `openspec/changes/task-057-publish-v082-release/`
- **Branch / PR**: `feat/task-057-publish-v082-release` · PR #44 **ready for review**, base `main`
- **Phase / gate**: SW-4 **PASS**, SW-5 **PASS**, Docs **complete**, **SW-6 Release Part A DONE**
- **Tier**: L2 · Part A only; Part B belongs to SW-6 after merge

## Last gate result
SW-6 (this gate): task `RELEASE.md` written with all 3 declarations (patch / automated /
compatibility matrix — no dep drift). Tag `task-057/v0.8.2` created and **signed** (SSH format,
verified via `git cat-file -p`). PR #44 promoted draft → ready-for-review. Gate artifacts
(qa/review/sec/docs/handoff) committed. No GitHub Release or DMG built — correctly deferred to
Part B per task scope.

## Active blockers
None. DevOps CI-unblock applied (2026-07-03): required `dependency-advisories` gate was failing on
two new SCORED quick-xml advisories (RUSTSEC-2026-0194/0195, DoS 7.5). No upstream fix exists (plist
1.9.0 hard-pins quick-xml <0.40; latest tauri/plist). Fixed via documented targeted `[advisories].
ignore` in `src-tauri/deny.toml` (group C) — unreachable in vire (own Info.plist only; no untrusted
XML; no first-party quick-xml/plist use). Verified locally: cargo-deny 0.19.9 `advisories ok` exit 0;
control run fails on exactly those 2 IDs. See `ops-review.md`. Pushed to PR #44; awaiting CI re-run.

## Exact next action
Janne merges PR #44 → route to SW-6 Part B: build `Vire_0.8.2_aarch64.dmg`, publish full
non-draft/non-prerelease `v0.8.2` release on merged `main` (steps in this dir's `RELEASE.md`
§Part B), live-smoke update check, backfill DMG name/size/sha256 into root `RELEASE.md`.

## Required files
- `ops-review.md` (this dir) — CI-unblock rationale (quick-xml advisories) + local verification.
- `src-tauri/deny.toml` — targeted `[advisories].ignore` group (C) for RUSTSEC-2026-0194/0195.
- `RELEASE.md` (this dir) — 3 declarations + Part B instructions.
- `review.md`, `qa.md`, `docs.md`, `sec.md` — gate evidence.
- `README.md`, root `RELEASE.md` — install/release note wording.
- `src-tauri/Cargo.toml`, `tauri.conf.json`, `Cargo.lock` — version triple (0.8.2, confirmed).

## Notes carried forward
- Merge base `df79423` (TASK-056) confirmed ancestor of `HEAD`. Product diff (excl. openspec) is
  exactly README, RELEASE, Cargo.toml, tauri.conf.json, Cargo.lock; `Cargo.lock` only bumps local
  `vire` version — no dep drift. No update_check/updater/asset/schema/CSP/capability/npm changes.
