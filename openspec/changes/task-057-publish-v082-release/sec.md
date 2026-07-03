# Security Review — TASK-057 Part A (publish v0.8.2 release: version bump + docs)

- **Gate**: SW-5 Security (parallel with SW-4 Code Review — already PASS per `review.md`)
- **Branch / PR**: `feat/task-057-publish-v082-release` · draft PR #44, base `main`
- **Tier**: **L2** — Secrets + CVE ≥ 7 + Trivy + semgrep ERROR
- **Scope gated**: Part A only — version triple (`0.8.1`→`0.8.2`) + `RELEASE.md`/`README.md` docs.
  Part B (DMG build, `gh release create`, live update-check smoke) is SW-6, out of scope here.

## Verdict: PASS

No auto-fail condition is **introduced by this change**. The product diff against the real base
(`origin/main` = `df79423` = TASK-056) is exactly 5 files — three version strings plus two docs —
with **zero dependency drift** (`Cargo.lock` is a 1-line change: only the local `vire` package
version). All Tier-L2 scanners are clean on the changed surface; the ≥7.0 OSV advisories are
pre-existing baseline debt (byte-identical to `main` and to the already-shipped `v0.8.1`), not
attributable to a version/docs-only bump. Consistent with the TASK-052 (v0.8.1 publish) and TASK-056
(the code this release publishes) gate dispositions.

## Change scope verified (base `df79423`, real `origin/main`)

`df79423` confirmed = `origin/main` and an ancestor of `HEAD`. Product diff excluding this change's
OpenSpec artifacts is exactly:

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | `version 0.8.1 → 0.8.2` |
| `src-tauri/tauri.conf.json` | `version 0.8.1 → 0.8.2` |
| `src-tauri/Cargo.lock` | single line — `vire` package `0.8.1 → 0.8.2`; **numstat `1 1`**, zero dep drift |
| `RELEASE.md` | new `## v0.8.2` entry prepended; older entries intact |
| `README.md` | version header + two DMG-filename examples `0.8.1/0.8.0 → 0.8.2` |

No change under `src-tauri/src/` (update-check comparison logic, IPC commands, bounded fail-soft
client, scoped opener capability, canonical-owner guards all untouched). No `tauri-plugin-updater` /
download / install / auto-updater. No schema / migration / IPC-storage / CSP / capture / capability /
Langfuse / npm change. No release assets built or touched in Part A.

## Scanner results (Tier L2)

| Scanner | Version | Target | Result | Auto-fail? |
|---|---|---|---|---|
| **gitleaks** | 8.30.1 | branch delta `df79423..HEAD` (2 commits, ~34 KB) | **no leaks found** | **No** |
| **semgrep** | 1.168.0 | 4 changed non-lock files, `p/secrets`, ERROR severity | **0 ERROR findings, 0 scan errors** | **No** |
| **Trivy** | 0.72.0 | changed files, `fs` secret+misconfig, HIGH/CRITICAL | **clean (0/0)** — deliverable is a desktop bundle, no container image | **No** |
| **OSV-scanner** | 2.4.0 | `src-tauri/Cargo.lock` (529 pkgs) | 21 advisories (3 High @ 7.5, 1 @ 6.9) — **all pre-existing, none introduced by this change** (see Advisories) | **No** |

## Advisories (OSV) — pre-existing baseline, carried-forward, NOT blocking this gate

Every advisory below is present **byte-identical** in the base `df79423` lockfile (verified:
`quick-xml 0.39.4`, `quinn-proto 0.11.14`, `glib 0.18.5` unchanged). TASK-057 adds/removes/bumps
**zero** dependencies — a version-string + docs change cannot introduce or regress a dependency CVE.
The dependency-vulnerability posture of the published `v0.8.2` is identical to the already-shipped
`v0.8.1`.

- **`quick-xml` 0.39.4 — RUSTSEC-2026-0194 & RUSTSEC-2026-0195, CVSS 7.5 HIGH each** (fix 0.41.0).
  *New* RustSec DB entries against a pre-existing, unchanged crate — not a regression from this
  change. Reachability (`cargo tree -i quick-xml`, aarch64-apple-darwin): pulled **only** transitively
  via `plist v1.9.0` → `tauri-codegen`/`tauri-build`/`tauri-utils`. `plist`/`quick-xml` parse the
  app's **own** `Info.plist` / Tauri config — trusted, local, developer-controlled, build-time input.
  No attacker-controlled / network-supplied XML reaches this parser in Vire. Advisory, not a blocker.
- **`quinn-proto` 0.11.14 — RUSTSEC-2026-0185, CVSS 7.5 HIGH** (fix 0.11.15). Optional, non-enabled
  `reqwest` QUIC feature: inverted dep tree on the macOS target is **empty** → **not compiled or
  linked** into the build. Availability-only, unreachable. Same disposition as TASK-052/056.
- **`glib` 0.18.5 — RUSTSEC-2024-0429, CVSS 6.9** (below the ≥7 line) plus the GTK3 cluster
  (`atk`/`gdk`/`gtk*`/`gdkx11` RUSTSEC-2024-04xx, unscored) — Linux GTK3 bindings / unmaintained-crate
  notices, **not compiled on this macOS-only aarch64 app**. Governed by the TASK-047 GTK/RustSec
  posture, not this release gate.
- **`anyhow` 1.0.102 (RUSTSEC-2026-0190)**, `unic-*` / `proc-macro-error` — unscored / low,
  pre-existing.

## Manual security review (change-focused)

- **Secrets**: none in the diff (gitleaks + semgrep p/secrets + Trivy secret scan all clean). The only
  credential-adjacent token is the unchanged `keyring` dependency context line in `Cargo.lock`.
- **Auth / trust boundary**: N/A — local single-user Tauri desktop app, no auth layer, no server
  endpoints. This change touches no code, so no auth path is altered.
- **Endpoint / rate-limit**: no network code changed. The `/releases/latest` fetch and bounded
  fail-soft client are untouched; the bump only changes the `CARGO_PKG_VERSION` string used for local
  semver comparison — adds no endpoint, alters no request behavior. No new egress.
- **Gatekeeper / xattr guidance** (`RELEASE.md` / `README.md`): the unsigned-first-launch remedy is
  `xattr -dr com.apple.quarantine /Applications/Vire.app` — **scoped to the single installed app**
  (not a recursive global `/` sweep), gated on an explicit "only for a DMG from the official releases
  page that you trust" caveat, with the true fix (Developer-ID signature + notarization) pointed at
  TASK-028. The notes **explicitly state they never instruct `sudo` or a global Gatekeeper disable**;
  grep confirms no `sudo`, no `spctl --master-disable`, no global-disable instruction. The quarantine
  attribute is stamped by Gatekeeper on the user's Mac at download — not embedded in the artifact, so
  no quarantine-bypass is (or can be) shipped. Safe.
- **Schema / CSP / capture / capability**: unchanged (no such paths in the diff).

## Escalations

None. No design-level security issue. No code-level FAIL.

## Recommendation (non-blocking, dependency-lane)

Continue addressing the pre-existing HIGH advisories in the dependency-maintenance lane
(TASK-047 lineage), not this release gate: a `tauri` bump to pull `quick-xml ≥ 0.41.0` via `plist`,
and either a `deny.toml` documented ignore for the unreachable `quinn-proto` (non-enabled `reqwest`
QUIC feature on the build target) or a `reqwest` bump once an advisory-clear `quinn-proto` resolves.

## Gate verdict

**SEC STATUS: pass.** No auto-fail condition introduced by this change: no secrets, no reachable
new/regressed CVE, no source/IPC/CSP/capability/network change, safe unsigned-launch guidance.
Hold for SW-4 (Code Review — already PASS); both gates green → SW-6 publishes `v0.8.2` and performs
Part B live verification.
