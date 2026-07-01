# Security review — TASK-054 fix broken v0.8.1 DMG release (Plan B, docs-only)

**Verdict: PASS** · **Tier: L2** · **Date: 2026-07-01** · **Gate: SW-5 (parallel with SW-4)**

## SW-5 recheck (post SW-2 wrapper-tag cleanup + SW-3 recheck PASS) — 2026-07-01

Re-verified after SW-2 removed the stray `</content>`/`</invoke>` wrapper-tag residue from
`ops-review.md` (commit `e217603`) and SW-3 QA re-passed. **Verdict unchanged: PASS (L2).** The cleanup
was a pure tail-trim of an artifact and introduces **no** new security exposure.

- **Diff surface still docs-only** — `git diff origin/main...HEAD --name-only` = `README.md`,
  `RELEASE.md`, and this change's `openspec/changes/task-054-*/` artifacts (9 files, 490 ins / 6 del).
  Pattern grep for `src-tauri/ .github/ tauri.conf.json package.json Cargo.* *.lock *.yml` → **NONE**.
- **Wrapper-tag residue** — `ops-review.md` now scans **clean** (no `function_calls`/`invoke`/
  `parameter`/`content` residue in the artifact); only benign prose mentions remain in
  `handoff.md`/`review.md` describing the fix. All 9 artifacts end with a single trailing newline.
- **semgrep** (ERROR) on the changed surface → **0 findings** (exit 0). No code changed, so codebase
  SAST posture is identical to the prior PASS.
- **gitleaks** (full tree, 225 commits / 3.92 MB) → **1 finding, same pre-existing false positive**:
  `generic-api-key` on the `"password"` literal in `PROHIBITED_COLS`
  (`src-tauri/src/active_window/tests.rs`, now line **144** — shifted from 134 by unrelated file
  evolution; that file is **not** in TASK-054's diff). Still a security *control test*
  (`no_prohibited_column_in_any_active_window_table`), not a credential. Not an auto-fail.
- **OSV-scanner / Trivy** → still **N/A** (no dependency/lockfile change; no container image).
- **Plan B docs unchanged in substance and still safe** — quarantine remedy is only
  `xattr -dr com.apple.quarantine /Applications/Vire.app` (scoped to the single app path; **no** `sudo`,
  **no** `spctl --master-disable`, **no** broad-tree xattr, **no** blanket Gatekeeper disable); security
  caveat + "trust the official source" condition present; official releases URL
  (`github.com/kaikkone4/vire/releases`) matches `origin`; docs still state "not code-signed or
  notarized" with **no** false signing/notarization claim (the `spctl`/`codesign`/`stapler` references
  are read-only *assessment* steps in the Plan A / TASK-028 real-fix path). No secrets, no auth/
  endpoint/rate-limit surface.

**No new blockers. No security rework needed. Advisories ADV-1/ADV-2 below still stand.**

## Scope reviewed

Docs-only release-packaging / Gatekeeper fix for the unsigned macOS DMG. Diff surface confirmed
docs-only via `git diff origin/main...HEAD --name-only`: only `README.md`, `RELEASE.md`, and this
change's `openspec/changes/task-054-*/` artifacts. **Zero** touches to `src-tauri/`, `.github/`,
`tauri.conf.json`, `package.json`, `Cargo.toml/lock`, or any lockfile/CI/YAML (verified by pattern
grep → NONE). No application code, auth flow, IPC surface, capability, endpoint, or dependency was
changed, so the classic SAST/auth/rate-limit attack surface is not in play for this change.

## Scanner results (Tier 1 / L2)

| Scanner | Scope run | Result | Auto-fail? |
|---|---|---|---|
| **semgrep** (SAST, ERROR) | changed files `README.md`, `RELEASE.md`, 1279 code + 44 multilang rules | **0 findings** | No |
| **gitleaks** (secrets) | full history + working tree, 224 commits / 3.92 MB | 1 finding — **false positive, out of scope** (see below) | No |
| **OSV-scanner** (dep CVEs) | N/A — no dependency/lockfile change in diff | not applicable | — |
| **Trivy** (image vulns) | N/A — no container image; Tauri desktop app, no Dockerfile/image | not applicable | — |

**gitleaks finding (triaged, not an auto-fail):** `src-tauri/src/active_window/tests.rs` (line ~134,
now ~144 after unrelated file evolution — `generic-api-key` on the string `"password"`). This is **not a credential** — it is one entry in a
`PROHIBITED_COLS` array used by a security *control test* (`no_prohibited_column_in_any_active_window_table`)
that asserts no active-window DB table exposes a column named `password`/`secret`/`token`/etc. It is
pre-existing (commit `3aa716e…`, 2026-06-21), lives in a test file, and is **not part of TASK-054's
docs-only diff**. False positive; no action required for this gate. (Advisory ADV-2 below.)

## Manual review — targeted concerns from the gate brief

- **No secrets in the change** — the docs diff introduces no tokens/keys/passwords; only a shell
  command and prose. ✅
- **No auth bypass / no exposed endpoints / rate-limit** — not applicable; docs-only, no code, no
  network surface changed. ✅
- **No unsafe *blanket* Gatekeeper-disabling guidance** — the only command introduced is
  `xattr -dr com.apple.quarantine /Applications/Vire.app`, **scoped to the single app path**. No
  `sudo`, no operation on `/` or a broad tree, no privilege escalation. ✅
- **No `spctl --master-disable` (or any `--master-disable`)** — confirmed absent from the diff and
  the docs. Gatekeeper is left enabled system-wide; only the one app's download-quarantine xattr is
  removed. ✅
- **No hidden quarantine-bypass beyond the documented per-app xattr** — the per-app `xattr` step is
  the sole bypass, and it ships **with an explicit security caveat** ("bypasses a Gatekeeper safety
  check *because Vire is not code-signed or notarized*") and is **conditioned on trust** ("only run
  this if you downloaded the DMG from the official releases page and trust it"). ✅
- **Release-URL integrity** — the caveat's "official Vire releases page" link
  (`github.com/kaikkone4/vire/releases`) **matches the repo's `origin` remote** — users are not
  steered to a third-party/attacker-controlled download source before being told to strip
  quarantine. ✅
- **No false signing/notarization claims** — both docs continue to state the build is "not
  code-signed or notarized," consistent with the confirmed `adhoc, linker-signed` /
  `TeamIdentifier=not set` state in `ops-review.md`. No Developer-ID / notarization / stapling claim
  was introduced; TASK-028 is correctly named as the real signed+notarized fix. ✅
- **No source/config/CI/lockfile changes** — confirmed (diff-surface check above). ✅
- **Public release-asset state acceptable under Plan B** — the shipped asset (`e77d15cf…`) is
  unchanged: pristine `tauri:build` output, not yanked, not re-uploaded. This does not introduce a
  *new* exposure — the unsigned ad-hoc posture is the pre-existing v0.8.x distribution contract
  (DEC-029 / deferred TASK-028), and Janne selected Plan B. The change makes the interim posture
  **honest** (accurate remedy + caveat) rather than expanding attack surface. Acceptable for L2. ✅

## Advisory (documented, non-blocking)

1. **ADV-1 — residual risk inherent to unsigned distribution.** Plan B, by design, instructs users to
   strip a Gatekeeper quarantine check. This is minimized correctly (per-app scope, security caveat,
   trust-the-official-source condition, no system-wide disable), but it remains a real weakening of
   the download-integrity guarantee versus a signed+notarized build. **Real remediation = TASK-028
   (Developer ID sign + notarize).** Already escalated as **FB-054-1** (BA/Architect to promote
   TASK-028 from deferred to required, or formally accept the unsigned interim). No new escalation
   needed from this gate — recorded for continuity.
2. **ADV-2 — gitleaks false positive hygiene.** Optional: add a gitleaks allowlist entry for
   `src-tauri/src/active_window/tests.rs` `PROHIBITED_COLS` so the scanner runs clean in future
   gates. Not this task's scope; docs-only change must not touch source/config.

## Escalations

- **Design-level:** none new. The only design-level security consideration (unsigned distribution)
  is a pre-existing, already-accepted product decision with an existing tracked fix (TASK-028) and an
  open feedback item (FB-054-1). No wrong trust boundary or missing auth layer is introduced by this
  change.

## Triage against L2 auto-fail rubric

No auto-fail condition hit for TASK-054's change surface: semgrep ERROR = 0; gitleaks = 1 false
positive outside the diff (not a real secret); OSV CVE≥7 = N/A (no dep change); Trivy HIGH/CRITICAL =
N/A (no image). → **PASS.**

## Blockers

None.
