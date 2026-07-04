# Ops Review — TASK-057: unblock required CI (quick-xml advisories on PR #44)

**Role:** DevOps Engineer · **Tier:** L2 · **Branch:** `feat/task-057-publish-v082-release` · **PR:** #44
**Scope of this review:** CI-unblock only. No version change (stays `0.8.2`), no release assets,
no `v0.8.2` GitHub Release. Single policy file touched: `src-tauri/deny.toml`.

## 1. Blocker

Required workflow **`dependency-advisories` / cargo-deny (advisories, Apple-target-scoped)** failed
on PR #44 (run 28681953325, job 85067075469). Two new **scored** RustSec advisories landed against
`quick-xml 0.39.4`:

| ID | Class | CVSS | Fixed in | Title |
|----|-------|------|----------|-------|
| RUSTSEC-2026-0194 | DoS (algorithmic complexity) | 7.5 HIGH | quick-xml ≥ 0.41.0 | O(N²) duplicate-attribute scan in `BytesStart::attributes()` |
| RUSTSEC-2026-0195 | DoS (memory exhaustion) | 7.5 HIGH | quick-xml ≥ 0.41.0 | Unbounded namespace-declaration allocation in `NsReader` |

cargo-deny 0.19.9 (the pinned scanner) reports both as `error[vulnerability]` — reproduced locally
(see §5). These are **not** the unmaintained/unscored class of the existing accepted advisories.

## 2. Remediation options evaluated (upgrade preferred over suppression)

Dependency path (verified `cargo tree -i quick-xml --target aarch64-apple-darwin`): `quick-xml` has
a **single** parent, `plist 1.9.0`, consumed only by Tauri's `tauri-utils` / `tauri-codegen` /
`tauri-build`.

1. **Bump quick-xml to ≥ 0.41.0** — REJECTED by resolver. `plist 1.9.0` hard-pins
   `quick-xml = "^0.39.2"` (`< 0.40`). `cargo update -p quick-xml --precise 0.41.0` fails:
   `candidate versions found which didn't match: 0.41.0 … required by plist v1.9.0`.
2. **Bump plist to a release built on quick-xml ≥ 0.41** — NOT AVAILABLE. `plist 1.9.0` is the
   latest published plist; `cargo update -p plist` = "0 packages to update"; probes for 1.9.1 /
   1.10.0 / 1.11.0 return `no matching package`.
3. **Bump Tauri** — NO EFFECT. `tauri 2.11.2` is already the latest 2.x (`cargo update -p tauri` =
   "0 packages"), and any Tauri still pulls `plist 1.9.0 → quick-xml 0.39.x`. Even bumping the
   `^2.2` constraint cannot clear this while plist has no fixed release.

**Conclusion:** there is no dependency/lockfile move that resolves these advisories today. This is a
genuine "no safe upgrade" situation, mirroring the repo's existing group-(B) acceptance.

## 3. Chosen fix — documented targeted ignore (repo-sanctioned mechanism)

Added the two IDs to the pre-existing `[advisories].ignore` list in `src-tauri/deny.toml` (the same
mechanism already used for the 5 unic-* advisories), under a new **group (C)** block. The fix is
policy-only: no code, no dependency, no version, no lockfile change.

The block explicitly flags that — unlike group (B) — these are **scored 7.5 HIGH** and are accepted
on a **documented unreachability basis**, not on low severity.

## 4. Reachability / trust-boundary rationale

- **Sole entry point is `plist`**, used only for Apple **Info.plist** handling (build/codegen time
  plus `tauri-utils` metadata). The XML parsed is the application's **own** Info.plist —
  developer-authored and sealed inside the signed `.app` at build time. It is never network-,
  user-, or attacker-supplied.
- **Both advisories are remote DoS against services that ingest untrusted XML at scale** (the 0195
  PoC OOM-killed NLnet Labs Routinator's concurrent RRDP workers). vire is a **single-user local
  macOS desktop app** with no untrusted-XML ingestion path — the CVSS-7.5 network-attacker vector
  is not reachable.
- **vire first-party code calls neither quick-xml nor plist** (grep-verified across `src/` and
  `src-tauri/src/`). Foreground-app bundle IDs in `active_window` come from AppKit APIs, not XML
  parsing.
- **No upstream fix exists** (§2), so acceptance — not upgrade — is the correct action.

**Residual risk:** low. Accepting a scored advisory is a real posture decision, recorded here and in
`deny.toml`. Review triggers (any of): (i) plist ships a release on quick-xml ≥ 0.41 → bump and drop
these IDs; (ii) vire begins parsing untrusted/remote XML → reachability changes, re-evaluate
immediately; (iii) either advisory is re-scored or superseded.

## 5. Verification (local equivalent of the required CI gate)

Ran the **exact** pinned scanner and command the workflow runs
(`cargo deny check advisories`, cargo-deny **0.19.9**, from `src-tauri/`):

- `cargo-deny --version` → `cargo-deny 0.19.9` (matches `deny.toml` header + workflow pin).
- **With fix** (patched `deny.toml`): `advisories ok` — **exit 0**.
- **Without the two new IDs** (control, `grep -v` copy of the same config): fails on **exactly**
  `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` (`error[vulnerability]` ×2, `advisories FAILED`) — no
  other advisory surfaces. Confirms the ignore is complete **and** minimal (no over-suppression;
  the target-scoping tripwire for the gtk3/glib group-(A) IDs is untouched).

## 6. Scope guard

- Version stays `0.8.2`; no `Cargo.toml` / `tauri.conf.json` / `Cargo.lock` edits.
- No DMG, no `gh release create`, no tag changes — Part B remains SW-6's post-merge responsibility.
- Only `src-tauri/deny.toml` (+ this artifact and `handoff.md`) changed.
