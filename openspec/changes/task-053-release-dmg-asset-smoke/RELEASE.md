# RELEASE gate artifact — TASK-053 / v0.8.1 (asset attach, no version bump)

- **Role**: Release Manager (SW-6) · **Date**: 2026-07-01
- **Branch**: `feat/task-053-release-dmg-asset-smoke` · **PR**: #41 (draft → ready this gate)
- **Version**: v0.8.1 (unchanged — no code/version bump; this task attaches a release asset to
  the existing published `v0.8.1` GitHub Release and adds install docs)
- **Tag (planned)**: `task-053/v0.8.1` — **not yet created**, see §Tag status below
- **Tier**: L2

---

## Gate status

| Gate | Verdict | Evidence |
|---|---|---|
| SW-3 QA | **PASS** | `qa.md` — scenario matrix all PASS; asset/checksum verified live |
| SW-4 Code Review | **PASS** | `review.md` — scope confined to README.md/RELEASE.md/change-dir |
| SW-5 Security | **PASS** | `sec.md` — gitleaks/semgrep/Trivy clean; OSV advisories pre-existing, none reachable ≥7.0 CVSS |
| Docs (SW-6 pre-gate) | **PASS** | `docs.md` — README/RELEASE.md accurate, no false signing/notarization/CI claims |

All prior gates PASS. No open blockers carried into this gate other than the human-Mac UAT
smoke steps (§Smoke checklist disposition) and the signed-tag limitation (§Tag status), both
non-blocking to shipping per established project precedent (see task-052 RELEASE.md).

---

## 1. Deployment size: **patch**

Zero application code delta. `git diff origin/main...HEAD -- src-tauri tauri.conf.json .github`
is empty (confirmed independently by qa.md and review.md). The only changes are:
- one GitHub Release asset upload (`Vire_0.8.1_aarch64.dmg`, already-built binary, already
  QA/Sec-verified in TASK-052's lineage — no rebuild, no new binary),
- one new README.md subsection ("Download & install (prebuilt)") + one example-artifact-name
  bump,
- one new RELEASE.md subsection (smoke checklist + artifact table) at repo root.

No breaking change, no schema/IPC/capability change, no dependency change. This is the smallest
deployment-size category available (patch) — arguably smaller in effect than a typical patch
release since the running application binary is byte-for-byte identical to the already-shipped
v0.8.1 build; only the distribution surface (what a new user downloads) changes.

---

## 2. Rollback strategy: **automated**

Two independent, fully-scripted rollback actions, neither touching a running install:

**Remove the asset** (if it needs to be pulled):
```bash
gh release delete-asset v0.8.1 Vire_0.8.1_aarch64.dmg --repo kaikkone4/vire --yes
```
Verify: `gh release view v0.8.1 --json assets` → `"assets":[]`. The release itself, its
`tag_name`, and the in-app update-check (`tag_name`-only comparison, asset-independent) are
unaffected — reverts to the exact TASK-052 post-merge state.

**Revert the docs**:
```bash
git revert <task-053-merge-sha>
```
Deterministic — the diff is one README subsection, one RELEASE.md subsection, and the change
dir. No data migration, no schema, no IPC, no running-app impact either direction (a user who
already downloaded/installed the DMG keeps a working app; rollback only affects future
downloads/doc content).

**No manual step required in either direction** → **automated**.

---

## 3. Component compatibility matrix

Inherited unchanged from TASK-052 (v0.8.1) — this task adds no dependency, no lockfile change,
no runtime surface change. Confirmed via `git diff` on all `package*.json`, `Cargo.*`, CI, and
capability files → empty (qa.md, sec.md).

| Component | Min version | Notes |
|---|---|---|
| Vire (shipped binary) | 0.8.1 | identical bytes to TASK-052's build; not rebuilt by this task |
| macOS | Ventura 13+ (Tauri 2.x platform minimum) | `aarch64` (Apple Silicon) only — no `x86_64`/universal build in scope |
| GitHub Releases API | `/repos/kaikkone4/vire/releases/latest` | unchanged; polled by in-app update-check, asset-independent |
| GitHub Release-assets CDN | `release-assets.githubusercontent.com` | **newly exercised by this task** — the download path now resolves through GitHub's asset-redirect CDN (verified live, HTTP 200 → 200 redirect chain) where previously no asset existed to serve |
| Shipped artifact | `Vire_0.8.1_aarch64.dmg`, 5,787,708 B, sha256 `e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`, embedded `CFBundleShortVersionString=0.8.1` | three-way match: local build ↔ RELEASE.md ↔ live GitHub asset digest (qa.md, this gate re-verified live below) |

No upstream/downstream service version constraint changed. No new Rust crate or npm package
(confirmed empty diff on `Cargo.lock`/`package-lock.json`).

---

## Live re-verification (this gate, 2026-07-01)

```
gh release view v0.8.1 --repo kaikkone4/vire --json assets,tagName,isDraft,isPrerelease
```
→ `tagName=v0.8.1`, `isDraft=false`, `isPrerelease=false`, one asset `Vire_0.8.1_aarch64.dmg`,
`digest=sha256:e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`, `size=5787708`.

```
curl -sI -L https://github.com/kaikkone4/vire/releases/download/v0.8.1/Vire_0.8.1_aarch64.dmg
```
→ `302` redirect to `release-assets.githubusercontent.com` → `200`. Matches qa.md's prior finding
byte-for-byte (same digest, same size). No drift since SW-3/4/5/Docs gates ran.

---

## Smoke checklist disposition (RELEASE.md root, steps 2–5)

Root `RELEASE.md` "Release smoke checklist (v0.8.1)" step 1 (asset present & downloadable) is
verified in-pipeline (above). Steps 2–5 (mount + drag, first-launch Gatekeeper, run without dev
server, in-app "Check for updates") require driving the packaged app's GUI on a physical Mac.

This gate does **not** execute those steps: doing so from this non-interactive agent session
would mean mounting a DMG and launching a GUI app on Janne's live desktop session without being
asked to — out of proportion for a docs/asset-distribution release, and already explicitly
scoped as human-only UAT by QA (qa.md) and tasks.md task 4. Disposition carried forward
unchanged, not re-litigated: **outstanding, non-blocking, human-only**, same as every prior gate
in this task's chain recorded it.

**Recommended 2-minute manual pass for Janne before wide distribution announcement** (not
required to merge this PR):
1. Download `Vire_0.8.1_aarch64.dmg` from the release page, double-click to mount.
2. Drag only `Vire.app` to `/Applications`.
3. Right-click `Vire.app` → Open (confirm Gatekeeper dialog, click Open).
4. Confirm the app runs with no `npm run tauri:dev`/Vite server active.
5. Settings → Check for updates → confirm "up to date" on v0.8.1.

---

## Tag status: **not created this gate — passphrase required, deferred to Janne**

Attempted the required signed, task-scoped tag:
```bash
git tag -s task-053/v0.8.1 -m "release(task-053): v0.8.1 patch — attach DMG asset + release smoke checklist"
```
Blocked: the configured signing key (`user.signingkey = ~/.ssh/id_ed25519.pub`, `gpg.format=ssh`)
is passphrase-protected; this non-interactive session has no way to supply it (`git tag -s`
fails with "unable to sign the tag" against `/dev/null` stdin). No tag object was created (the
failed attempt was cleanly rolled back — `git tag -l` confirms no dangling/partial tag).

Per this role's working rule, an unsigned tag is **not** created as a fallback. This is a
recurring, already-documented environment limitation in this repo (see
`openspec/changes/task-052-publish-v081-release/handoff.md:15` and `tasks.md:43`, where the same
passphrase blocker was sidestepped for the release's own GitHub tag by letting `gh release
create` mint it server-side; `task-045/v0.6.3` is the only locally-signed-intent tag in this repo
and its signature also did not apply — `git tag -v` reports "no signature found").

**Deferred action for Janne** (run locally, where the SSH key passphrase is available):
```bash
git tag -s task-053/v0.8.1 -m "release(task-053): v0.8.1 patch — attach DMG asset + release smoke checklist"
git push origin task-053/v0.8.1
```
This does not block PR #41 promotion or merge — consistent with TASK-045/047/052, where the
same local-signing limitation did not hold up shipping.

---

## Changed paths (this gate, TASK-053 change dir + root docs)

| Path | Change |
|---|---|
| `openspec/changes/task-053-release-dmg-asset-smoke/RELEASE.md` | Added (this file — 3 declarations) |
| `openspec/changes/task-053-release-dmg-asset-smoke/qa.md` | Committed (was untracked; SW-3 evidence) |
| `openspec/changes/task-053-release-dmg-asset-smoke/review.md` | Committed (was untracked; SW-4 evidence) |
| `openspec/changes/task-053-release-dmg-asset-smoke/sec.md` | Committed (was untracked; SW-5 evidence) |
| `openspec/changes/task-053-release-dmg-asset-smoke/docs.md` | Committed (was untracked; Docs pre-gate evidence) |
| `openspec/changes/task-053-release-dmg-asset-smoke/handoff.md` | Updated (SW-6 final state) |
| `README.md`, root `RELEASE.md` | No new edits this gate (already landed by SW-2, independently re-verified live, unchanged) |

---

## Non-blocking follow-ups (carried forward, not this task's scope)

1. Signed `task-053/v0.8.1` tag — create locally when Janne has the SSH key passphrase available
   (command above).
2. Root `RELEASE.md` smoke-checklist steps 2–5 — recommended manual pass before wide
   announcement (not required to merge).
3. README "Current version: v0.8.0" header staleness — pre-existing, out of scope for this
   task (flagged by every gate in this chain: tasks.md, qa.md, sec.md, docs.md).
4. `quinn-proto` 0.11.14→0.11.15 (CVSS 7.5, optional/unreachable) and `glib` 0.18.5 (CVSS 6.9,
   Linux-only) — future dependency-hygiene task (pattern: TASK-043/047), not TASK-053 scope.
