<!-- Compact state; keep <= 2 KB. Reference paths, never paste content. -->

# Handoff — TASK-051 fix update-check repo owner

- **Tier:** L2 · **Component:** Vire desktop app (single) · **Base:** `main` @ `a0c8cff`
- **Branch:** `feat/task-051-fix-update-release-url` · **PR:** draft (see PR link in report)
- **Phase:** SW-2 implementation — **COMPLETE**. Next: SW-3 QA → SW-4 review → post-merge code-to-spec recheck.

## What shipped

Replaced wrong owner `kaikkonen4/vire` → canonical `kaikkone4/vire` (404'd Releases page +
fail-soft `Unknown`). Root cause: `../task-050-in-app-update-available/code-to-spec.md`.

Changed (code/):
- `src-tauri/src/update_check/mod.rs:23,26` RELEASES_URL + GITHUB_API_LATEST
- `src-tauri/capabilities/default.json:12` opener allowlist url (== RELEASES_URL)
- `src-tauri/src/update_check/tests.rs` fixtures (9,27) + 2 NEW guards
- `tests/updateCheckUi.test.mjs` fixtures (41,70)

Regression guards (tests.rs): `endpoints_target_canonical_repo_owner` (canonical present, typo
absent — typo built at runtime so grep stays clean) +
`opener_allowlist_url_equals_releases_url` (capability == RELEASES_URL via include_str!).

## Verify (all pass)

`cargo test update_check` 11 ok (9+2) · `node --import tsx --test tests/updateCheckUi.test.mjs`
12 ok · `npm run build` ok · `cargo fmt --all -- --check` ok ·
`grep -rn kaikkonen4 src-tauri/src src-tauri/capabilities src tests` empty ·
`openspec validate task-051-fix-update-release-url --strict` valid.
Smoke: releases page 200, repo API 200 (typo would 404); `/releases/latest` 404 = no release
published yet (legit fail-soft `Unknown`, not the owner). Resolves once a release exists.

## Guardrails honored

No version bump (0.8.0). No updater/download/install. No CSP/schema/Langfuse. Did not touch
generated `gen/schemas/*` or `target/**`. Build dead-code warnings pre-existing, unrelated.

## Artifacts (this dir)

proposal.md · tasks.md (all boxes checked) · specs/in-app-update-check/spec.md · arch-review.md
