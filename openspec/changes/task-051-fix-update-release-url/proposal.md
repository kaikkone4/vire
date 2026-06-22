# TASK-051 — Hotfix: correct GitHub repo owner in update-check URLs (`kaikkonen4` → `kaikkone4`)

## Why

TASK-050 shipped the U-lite update check, but the merged code (PR #38, `main` @ `a0c8cff`)
targets the **wrong GitHub owner**: `kaikkonen4/vire` (extra `n`) instead of the canonical
`kaikkone4/vire`. The repository identity is `https://github.com/kaikkone4/vire.git` and the
TASK-050 spec mandates `kaikkone4/vire`
(`openspec/changes/task-050-in-app-update-available/specs/in-app-update-check/spec.md:9,45,53`).
The TASK-050 post-merge code-to-spec verification flagged this as **DIVERGED**
(`openspec/changes/task-050-in-app-update-available/code-to-spec.md`).

User smoke confirms the impact:

- **Open GitHub Releases** opens a 404 at `https://github.com/kaikkonen4/vire/releases`.
- **Check for updates** fail-soft returns `Unknown` against a nonexistent repository.

The spec is correct; the **implementation diverged**. This is a string-correction hotfix, not a
feature change.

## What changes

A tiny, single-component correction in the **Vire desktop app** plus a regression guard so the
typo cannot recur:

- `src-tauri/src/update_check/mod.rs:23` — `RELEASES_URL` → `https://github.com/kaikkone4/vire/releases`.
- `src-tauri/src/update_check/mod.rs:26` — `GITHUB_API_LATEST` → `https://api.github.com/repos/kaikkone4/vire/releases/latest`.
- `src-tauri/capabilities/default.json:12` — `opener:allow-open-url` allowlist URL → `https://github.com/kaikkone4/vire/releases`.
- Test fixtures aligned to the canonical owner: `src-tauri/src/update_check/tests.rs:9,27`,
  `tests/updateCheckUi.test.mjs:41,70`.
- **New regression guard** (static assertion / unit test) asserting the compiled URLs target
  `kaikkone4/vire` and contain no `kaikkonen4` typo — the only behavioural delta this change adds
  to the spec (an invariant, not a feature).

## What does NOT change

- **No feature expansion**, **no `tauri-plugin-updater`**, no download/install/signing — TASK-050
  scope boundaries are untouched.
- **No version bump.** A URL string correction does not change the release/version contract;
  the running-version source (`CARGO_PKG_VERSION` = `0.8.0`, reconciled by TASK-049) is unaffected.
- **No schema, migration, IPC-storage, CSP-`connect-src`, capture, classifier, or Langfuse change.**
- No manual edits to generated files — `src-tauri/gen/schemas/capabilities.json` and
  `src-tauri/target/**` are gitignored and regenerate from `capabilities/default.json` at build.

## Impact

- **Restores the two TASK-050 user-facing behaviours**: Open Releases reaches a real page;
  Check-for-updates resolves against the real repository.
- **No security-boundary change.** The egress host set is identical (one read-only GitHub host);
  only the path's owner segment is corrected. CSP, capability *type*, and client posture are
  unchanged. SEC-002 is unaffected.
- **Component scope:** one component (the desktop app). No cross-boundary work; no split required.
