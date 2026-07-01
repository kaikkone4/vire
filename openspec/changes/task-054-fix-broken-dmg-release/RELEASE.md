# RELEASE gate artifact — TASK-054 / v0.8.1 (docs fix, no version bump, no asset change)

- **Role**: Release Manager (SW-6) · **Date**: 2026-07-01
- **Branch**: `feat/task-054-fix-broken-dmg-release` · **PR**: #42 (draft → ready this gate)
- **Version**: v0.8.1 (unchanged — no code/version bump; this task corrects install docs for the
  already-published `v0.8.1` GitHub Release)
- **Tag (planned)**: `task-054/v0.8.1` — **not created this gate**, see §Tag status below
- **Tier**: L2
- **Plan**: B (unsigned + honest quarantine-removal docs; no signing cert introduced)

---

## Gate status

| Gate | Verdict | Evidence |
|---|---|---|
| SW-3 QA | **PASS** (rechecked post wrapper-tag cleanup) | `qa.md` — diff-surface, scenario matrix, trailing-newline checks all pass |
| SW-4 Code Review | **PASS** (recheck) | `review.md` — no blocking issues; scope confined to README.md/RELEASE.md/change-dir |
| SW-5 Security | **PASS** (rechecked) | `sec.md` — semgrep 0 ERROR, gitleaks 1 pre-existing false positive (unrelated file), OSV/Trivy N/A |
| Docs (SW-6 pre-gate) | **PASS** | `docs.md` — README/RELEASE.md accurate, no false signing/notarization/CI/auto-update claims |

All four prior gates PASS. No open blockers other than the signed-tag limitation (§Tag status)
and the standing physical-Mac GUI smoke UAT carried from TASK-053 — both non-blocking to shipping,
consistent with `task-052`/`task-053` precedent.

---

## 1. Deployment size: **patch**

Zero application code delta. `git diff origin/main...HEAD` touches only `README.md` (30 lines),
root `RELEASE.md` (14 lines), and this change's `openspec/changes/task-054-fix-broken-dmg-release/`
artifacts (9 files) — 490 insertions / 6 deletions total, confirmed by `qa.md`/`review.md`/`sec.md`
independently. No `src-tauri/`, `tauri.conf.json`, `.github/`, `package.json`, or lockfile change.

The shipped GitHub Release asset (`Vire_0.8.1_aarch64.dmg`, sha256 `e77d15cf2066a24ee344ea5ab65…`)
is **unchanged** — not rebuilt, not re-uploaded, not yanked. This is a docs-only correction to the
first-launch Gatekeeper remedy that TASK-053 published: the previous guidance (right-click → Open)
does not clear the "damaged" quarantine dialog on Apple Silicon for a browser-downloaded unsigned
app; the corrected remedy (`xattr -dr com.apple.quarantine /Applications/Vire.app`) does. No
breaking change, no schema/IPC/capability change, no dependency change — smallest deployment-size
category available (patch).

---

## 2. Rollback strategy: **automated**

Single rollback surface, fully scripted, no running-install impact either direction:

```bash
git revert <task-054-merge-sha>
```

Verify: `git diff origin/main...HEAD -- README.md RELEASE.md` returns to the TASK-053 baseline
(right-click → Open guidance restored). Deterministic — the diff is confined to two doc files plus
the change dir; no data migration, no schema, no IPC, no running-app impact. A user who already
followed the corrected `xattr` remedy and has the app installed and running is unaffected by a
docs revert in either direction. The GitHub Release asset is never touched by this rollback path
(no `gh release` command involved) — **automated**.

---

## 3. Component compatibility matrix

Inherited unchanged from TASK-052/053 (v0.8.1) — this task adds no dependency, no lockfile change,
no runtime surface change. Confirmed via `git diff` on all `package*.json`, `Cargo.*`, CI, and
capability files → empty (`qa.md`, `sec.md`, `review.md`).

| Component | Min version | Notes |
|---|---|---|
| Vire (shipped binary) | 0.8.1 | identical bytes to TASK-052/053's build; not rebuilt by this task |
| macOS | Ventura 13+ (Tauri 2.x platform minimum) | `aarch64` (Apple Silicon) only — the Gatekeeper "damaged" verdict this task documents is Apple-Silicon-specific quarantine policy |
| GitHub Releases API | `/repos/kaikkone4/vire/releases/latest` | unchanged; polled by in-app update-check, asset-independent |
| Shipped artifact | `Vire_0.8.1_aarch64.dmg`, 5,787,708 B, sha256 `e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f` | unchanged since TASK-053; re-verified structurally identical via fresh `tauri:build` comparison (`ops-review.md`) |
| Signing state | ad-hoc / linker-signed, **not** Developer-ID-signed or notarized | unchanged; real fix tracked as TASK-028 (FB-054-1) |

No upstream/downstream service version constraint changed. No new Rust crate or npm package
(confirmed empty diff on `Cargo.lock`/`package-lock.json`).

---

## Tag status: **not created this gate — passphrase required, deferred to Janne**

Attempted the required signed, task-scoped tag:

```bash
git tag -s task-054/v0.8.1 -m "release(task-054): v0.8.1 patch — corrected unsigned first-launch Gatekeeper docs"
```

Blocked: the configured signing key (`user.signingkey = ~/.ssh/id_ed25519.pub`, `gpg.format=ssh`)
is passphrase-protected; this non-interactive session has no way to supply it. No tag object was
created. This is the same recurring, already-documented environment limitation recorded in
`task-053-release-dmg-asset-smoke/RELEASE.md` (§Tag status) and `task-052-publish-v081-release`
(`handoff.md`, `tasks.md`) — `task-053/v0.8.1` and `task-052`'s local tag are likewise not present
locally (`git tag -l "task-05*"` → empty); `gh release create` minted the v0.8.1 tag server-side
for TASK-052 instead.

Per established project precedent (task-052/053), this limitation does not block PR promotion or
merge.

**Deferred action for Janne** (run locally, where the SSH key passphrase is available):

```bash
git tag -s task-054/v0.8.1 -m "release(task-054): v0.8.1 patch — corrected unsigned first-launch Gatekeeper docs"
git push origin task-054/v0.8.1
```

---

## Changed paths (this gate)

| Path | Change |
|---|---|
| `openspec/changes/task-054-fix-broken-dmg-release/RELEASE.md` | Added (this file — 3 declarations) |
| `openspec/changes/task-054-fix-broken-dmg-release/qa.md` | Committed (was untracked; SW-3 evidence) |
| `openspec/changes/task-054-fix-broken-dmg-release/review.md` | Committed (was untracked; SW-4 evidence) |
| `openspec/changes/task-054-fix-broken-dmg-release/sec.md` | Committed (was untracked; SW-5 evidence) |
| `openspec/changes/task-054-fix-broken-dmg-release/docs.md` | Committed (was untracked; Docs pre-gate evidence) |
| `openspec/changes/task-054-fix-broken-dmg-release/handoff.md` | Updated (SW-6 final state) |
| `README.md`, root `RELEASE.md` | No new edits this gate (already landed by SW-2, independently re-verified) |

No GitHub Release asset was touched, re-uploaded, or yanked as part of this gate.

---

## Non-blocking follow-ups (carried forward, not this task's scope)

1. Signed `task-054/v0.8.1` tag — create locally when Janne has the SSH key passphrase available
   (command above).
2. **FB-054-1**: TASK-028 (signed + notarized build) remains the real fix for the unsigned-app
   Gatekeeper friction this task documents around.
3. Root `RELEASE.md` smoke-checklist steps 2–5 (physical-Mac GUI launch) — standing human UAT gate
   carried from TASK-053, not required to merge this PR.
4. README "Current version: v0.8.0" header staleness — pre-existing, out of scope (flagged by
   every gate in this chain: `qa.md`, `sec.md`, `docs.md`, `review.md`).
