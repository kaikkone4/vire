# Documentation Review — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

- **Documentation Engineer:** SW-6 (pre-release gate)
- **Change:** `task-053-release-dmg-asset-smoke`
- **Branch:** `feat/task-053-release-dmg-asset-smoke` · **PR:** #41 (draft)
- **Tier:** L2 · **Gate context:** release distribution/docs only
- **Date:** 2026-07-01
- **Verdict:** **PASS — docs accurate, no edits needed**

---

## Scope produced

No file edits made this pass — verification-only. `README.md` and `RELEASE.md` were already
landed by SW-2/reviewed by SW-3 (qa.md)/SW-4 (review.md)/SW-5 (sec.md); this gate independently
re-checks the same delta for release-doc accuracy ahead of SW-6 human UAT.

| File | Action |
|---|---|
| `openspec/changes/task-053-release-dmg-asset-smoke/docs.md` | Created (this file) |
| `openspec/changes/task-053-release-dmg-asset-smoke/handoff.md` | Updated |

---

## 1. README "Download & install (prebuilt)" — accurate for v0.8.1 aarch64 unsigned DMG

`README.md:63-81`: names `Vire_0.8.1_aarch64.dmg` explicitly, states Apple Silicon/`aarch64`
only, links `releases/latest`, instructs dragging only `Vire.app` (cross-refs the existing
drag-only warning at `#install-and-run`, anchor resolves — `README.md:103`), and states no dev
server is required at runtime. Artifact-location example bumped to `Vire_0.8.1_aarch64.dmg`
(`README.md:101`). **Accurate.**

## 2. Gatekeeper guidance — safe

`README.md:74-78` and `RELEASE.md:39`: both instruct **right-click (Control-click) → Open** (or
*System Settings → Privacy & Security → Open Anyway*) and explicitly state **"Do not disable
Gatekeeper or strip quarantine."** No `xattr -d com.apple.quarantine`, `spctl
--master-disable`, or other bypass instruction anywhere in either file. **Safe.**

## 3. RELEASE.md smoke checklist / checksum — clear

`RELEASE.md:19-43` "DMG asset attached + release smoke checklist (TASK-053)": artifact table
(name, size, sha256, embedded `CFBundleShortVersionString`, download URL) is present and
internally consistent; the 5-step checklist separates the in-pipeline-verified step 1 from the
four explicitly human/physical-Mac steps 2-5. Checksum
(`e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`) matches qa.md's recorded
three-way match (local build ↔ RELEASE.md ↔ live GitHub asset digest). **Clear.**

## 4. No false signing/notarization/auto-update/CI claims

Grepped both files for signing/notarization/updater/CI language:

- `README.md:74`, `README.md:162-164`: correctly states the app is **not** code-signed or
  notarized.
- `RELEASE.md:21`: "still **no** auto-updater, signing/notarization, or release-feed automation."
- `RELEASE.md:41`: "Check for updates" note correctly scopes the check to reading the release
  `tag_name` only (asset-independent) — does not imply an in-app downloader/installer exists.
- No `.github/` workflow or CI claim added by this change (confirmed empty diff on `.github` in
  qa.md/review.md).

No overclaim found. **Accurate.**

## 5. Docs drift

No new drift introduced by this change's diff (`README.md`, `RELEASE.md` only; scope-excluded
paths empty per qa.md/review.md).

One **pre-existing, out-of-scope** item, unchanged by this task and already flagged three times
in this task's own gate chain (tasks.md task 3, qa.md, sec.md, handoff.md "Notes carried
forward"): `README.md:5` "Current version: v0.8.0" header is stale against the shipped `v0.8.1`.
This gate does not fix it — tasks.md task 3 explicitly scoped the doc change to the new
"Download & install" subsection and the §Artifact-location example bump only, and SW-2 through
SW-5 all independently confirmed and preserved that scope decision. Re-opening it here would
invalidate the SW-4/SW-5 PASS verdicts (their reviewed diff would no longer match) without a
scope authorization to do so. **Recorded as a non-blocking follow-up, not a gate failure.**

---

## Checks run

- Full read of `README.md` and `RELEASE.md`.
- `grep -n "0.8.0\|0.8.1\|Current version"` across both files.
- `grep`-equivalent manual scan for Gatekeeper-bypass patterns (`xattr`, `spctl`, `quarantine`)
  and signing/notarization/auto-update/CI claims — none unsafe or false.
- Anchor resolution check: `#install-and-run` → `README.md:103` exists.
- `openspec validate task-053-release-dmg-asset-smoke --strict` → valid.
- Cross-checked findings against `qa.md`, `review.md`, `sec.md`, `handoff.md` — no contradictions.

## Verdict

**PASS.** README and RELEASE.md are accurate for the v0.8.1 aarch64 unsigned DMG, Gatekeeper
guidance is safe, the smoke checklist/checksum are clear, and no doc overclaims signing,
notarization, auto-update, or CI. No edits required. One pre-existing, already-tracked,
out-of-scope staleness (README "Current version: v0.8.0" header) is carried forward as a
non-blocking follow-up — same disposition as every prior gate in this task's chain.
