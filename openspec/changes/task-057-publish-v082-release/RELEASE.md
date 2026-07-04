# Release — TASK-057 · Vire v0.8.2 (Part A)

**Gate:** SW-6 Release · **Tier:** L2 · **Date:** 2026-07-03
**Branch / PR:** `feat/task-057-publish-v082-release` · PR #44 (base `main`)
**Commit:** `aa4ec09` (merge base `origin/main` = `df79423`, the TASK-056 merge)
**Tag:** `task-057/v0.8.2` (signed, SSH format)

---

## Summary

Metadata-only patch: version strings reconciled `0.8.1 → 0.8.2` across `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`, and `src-tauri/Cargo.lock` (`vire` package entry only — zero dep
drift). `RELEASE.md` (repo root) prepended with a `## v0.8.2` entry; `README.md` version header
and DMG-filename examples updated. No source, schema, CSP, IPC, dependency, or capability change.
Publishes the version bump the TASK-056 Settings UI needed so an already-distributed `0.8.1`
build's "Check for updates" resolves to **UpdateAvailable** instead of **UpToDate**, once the
Release itself is published in Part B.

**Part A** (this PR): land `0.8.2` metadata + docs on `main`.
**Part B** (SW-6, post-merge): build `Vire_0.8.2_aarch64.dmg`, publish full non-draft/non-prerelease
GitHub Release `v0.8.2` on merged `main`, live-smoke the update check. Not performed in this gate —
explicitly out of scope per task instruction.

---

## Gate results

| Gate | Verdict | Evidence |
|------|---------|----------|
| SW-3 QA | **PASS** | `qa.md` — change scope 1:1 with `proposal.md`/`tasks.md`; `cargo test update_check` 11/11, `updateCheckUi.test.mjs` 12/12, `cargo fmt` clean, `npm run build` clean, `openspec validate --strict` passes |
| SW-4 Code Review | **PASS** | `review.md` — version triple consistent at `0.8.2`; `Cargo.lock` numstat `1 1`; no blocking issues, no suggestions |
| SW-5 Security | **PASS** | `sec.md` — gitleaks 0 leaks; semgrep 0 ERROR; Trivy 0/0; OSV advisories all pre-existing byte-identical to base, zero dep drift introduced |
| Documentation | **PASS (complete)** | `docs.md` — README/RELEASE independently re-verified: version, TASK-056 inclusion, update-check wording, manual-install (no auto-updater claim), Plan B xattr remedy + trust caveat + TASK-028 pointer all accurate; no doc edits required |

---

## Declaration 1 — Deployment size: **PATCH**

Semantic version change: `0.8.1 → 0.8.2` — no breaking change, no new user-facing feature in this
Part A diff itself (the underlying TASK-056 Settings UI was already merged to `main` at `df79423`
without a version bump). The only code delta here is three version-string lines, one `RELEASE.md`
changelog entry, and two `README.md` doc edits. This corrects a metadata inconsistency (the
TASK-056 UI shipped on `main` but `main` still reported `0.8.1`, so a distributed `0.8.1` build's
update check resolved `UpToDate` and the new UI was undiscoverable) and prepares the version for
the Part B Release publish. Patch-tier by SemVer, consistent with the TASK-052/v0.8.1 precedent.

---

## Declaration 2 — Rollback strategy: **automated**

Single-surface rollback for this Part A change: `git revert <merged-sha>` on `main`, reviewed and
merged as a normal PR. The diff is fully deterministic (three version-string lines + one changelog
block + two doc lines) — no data migration, no schema/IPC/capability change to reconcile, no
credential or Keychain state to restore. Verify: `src-tauri/Cargo.toml`/`tauri.conf.json` read
`0.8.1` again and `cargo test update_check` still passes (comparison logic itself is untouched by
either the forward change or the revert).

For an end user who already installed the `v0.8.2` DMG (Part B), rollback is simpler still:
reinstall the prior `Vire_0.8.1_aarch64.dmg` — no cleanup step, no data migration, matching the
root `RELEASE.md` v0.8.2 entry's declared end-user rollback.

**L2 note:** rollback path validated by inspection only (deterministic single-file-class diff);
no staging rebuild was performed this gate since Part A ships no runtime behavior change to
regress — the identical `update_check` test suite (11/11) passes both before and after the
version-string diff.

---

## Declaration 3 — Component compatibility matrix

No dependency, toolchain, or platform-minimum change from the `0.8.1` baseline (`task-052`). This
change is version-string + docs only.

### Runtime and toolchain

| Component | Min version | Tested on | Notes |
|-----------|-------------|-----------|-------|
| Vire | 0.8.2 | 0.8.2 | this release |
| macOS | 10.13 (High Sierra) | — (Part B live smoke pending, physical-Mac step) | Tauri 2.x platform minimum; macOS-only binary |
| Rust toolchain | 1.56 (edition 2021) | per CI/session toolchain | edition 2021 minimum, unchanged |
| Cargo | ≥1.56 | matches Rust toolchain | unchanged |

### Tauri framework (Cargo) — unchanged, zero lockfile diff beyond `vire` self-version

| Crate | Spec version | Resolved version | Notes |
|-------|-------------|-----------------|-------|
| `tauri` | `2.2` | `2.11.2` | no feature flags |
| `tauri-build` | `2.0` | resolved in lock | build-only |

### Frontend / tooling — unchanged

| Package | Spec version | Notes |
|---------|-------------|-------|
| `@tauri-apps/api` | `^2.2.0` | Tauri JS bridge |
| `@tauri-apps/plugin-dialog` | `^2.2.0` | dialog plugin |
| `@tauri-apps/cli` | `^2.2.0` | dev CLI, build-time only |
| TypeScript | `^5.7.2` | strict mode |
| Vite | `^6.4.3` | bundler |
| tsx | `^4.22.0` | test runner |

### Upstream/downstream services

| Service | Version / endpoint | Notes |
|---------|--------------------|-------|
| GitHub Releases API | `/repos/kaikkone4/vire/releases/latest` | polled by in-app update-check; published in Part B |
| GitHub Releases UI | `github.com/kaikkone4/vire/releases` | "Open GitHub Releases" target; no version lock |

**Carry-forward advisories (pre-existing baseline, not introduced by this change, byte-identical
to base `df79423`):** `quick-xml 0.39.4` (RUSTSEC-2026-0194/0195, 7.5 HIGH each — reachable only via
`plist`/Tauri's own build-time config parsing, no attacker-controlled input); `quinn-proto 0.11.14`
(RUSTSEC-2026-0185, 7.5 HIGH — non-enabled `reqwest` QUIC feature, not compiled/linked on macOS
target); `glib 0.18.5` (RUSTSEC-2024-0429, 6.9, below the ≥7 line) plus the Linux GTK3 cluster — not
compiled on this macOS-only app. Governed by the `task-047` dependency-hygiene lineage, independent
of this release gate.

---

## Part B — Post-merge instruction (SW-6 handoff)

**Precondition:** PR #44 merged to `main`; confirm with:

```bash
git fetch origin
git show origin/main:src-tauri/Cargo.toml | grep '^version'  # must read 0.8.2
```

**Steps:**
1. Build `Vire_0.8.2_aarch64.dmg` from the merged `main` head (`npm run tauri:build`).
2. `gh release create v0.8.2 --repo kaikkone4/vire --target <merged-sha> --title "Vire v0.8.2" --notes-file <notes>` — **no** `--draft`, **no** `--prerelease`. Attach the DMG asset.
3. Backfill DMG name/size/sha256 into the root `RELEASE.md` v0.8.2 entry (v0.8.1 precedent).
4. Live-smoke: `0.8.1` build → **UpdateAvailable**; rebuilt `0.8.2` build → **UpToDate**; DMG asset downloadable; `/releases/latest` returns `v0.8.2`.

---

## Changed paths (this gate)

```
openspec/changes/task-057-publish-v082-release/RELEASE.md   (new, this gate)
openspec/changes/task-057-publish-v082-release/qa.md        (SW-3 gate artifact, committed this gate)
openspec/changes/task-057-publish-v082-release/review.md    (SW-4 gate artifact, committed this gate)
openspec/changes/task-057-publish-v082-release/sec.md       (SW-5 gate artifact, committed this gate)
openspec/changes/task-057-publish-v082-release/docs.md      (Documentation gate artifact, committed this gate)
openspec/changes/task-057-publish-v082-release/handoff.md   (updated, this gate)
```

Full PR #44 diff vs `main` (product surface, unchanged from prior gates):
`README.md`, `RELEASE.md`, `src-tauri/Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`,
plus the `task-057-publish-v082-release` change directory.

---

## Gate checklist

- [x] SW-3 QA PASS
- [x] SW-4 Code Review PASS
- [x] SW-5 Security PASS
- [x] Documentation gate PASS
- [x] RELEASE.md written with all three required declarations
- [x] Tag `task-057/v0.8.2` — signed (SSH format), created this gate
- [x] PR #44 promoted from draft to ready-for-review (this gate)

## Non-blocking follow-ups

1. Part B: build DMG, publish full `v0.8.2` GitHub Release, live-smoke update check, backfill
   asset name/size/sha256 into root `RELEASE.md` — after Janne merges PR #44.
2. `quick-xml`/`quinn-proto`/`glib` pre-existing advisories — continue in the dependency-hygiene
   lane (`task-047` lineage), independent of this release gate.
