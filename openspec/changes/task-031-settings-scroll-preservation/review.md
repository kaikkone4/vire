# Code Review — TASK-031

**Gate**: SW-4 · **Verdict**: PASS  
**Branch / PR**: `feat/task-031-settings-scroll-preservation` / #26  
**Date**: 2026-06-20

## Blocking issues

None.

## Review

- `src/main.ts:26-28,43` keeps the render state narrowly scoped and performs capture, decision,
  replacement, restoration, and state update at the existing `shell()` chokepoint. It does not duplicate
  scroll logic in handlers or view renderers.
- `src/scroll.ts:1-11` is a small, side-effect-free helper with no unused branches or exported dead code.
  Its separate module follows the repository's existing pure-helper test pattern.
- `tests/scroll.test.mjs:8-16` covers same-view preservation, navigation reset, and zero-position cases.
  Live DOM behavior remains correctly assigned to human macOS UAT rather than simulated by an unrelated
  test abstraction.
- `src/env-mapping-ui.ts:93-97` changes only the stale help copy and retains the explicit, non-automatic
  mapping meaning. `tests/envMappingUi.test.mjs:112-117` locks the displayed label to the live affordance.
- Complexity is constant and local: one state variable, one pure conditional, and one chokepoint update.
  No dead code, unused imports, commented-out code, convention drift, or architecture concern found.

## Suggestions

- PR #26 is recorded as draft in `handoff.md`. Before marking it ready, confirm its description includes
  the implementation summary, automated checks, the known unrelated frontend-suite failures, and the
  outstanding human-only macOS UAT. GitHub metadata could not be fetched in this sandbox because
  `api.github.com` was unreachable; the local commit subject/body are complete and well scoped.

## PR / commit quality

The single commit uses a scoped conventional subject and a detailed body covering root cause, chosen
chokepoint, behavior, copy cleanup, and tests. The changed file set matches the OpenSpec scope.

## Checks

- `git diff --check main...HEAD`: PASS
- `npm run build`: PASS
- `node --import tsx --test tests/scroll.test.mjs tests/envMappingUi.test.mjs`: PASS (14/14)
- `npm run test:frontend`: TASK-031 tests PASS; aggregate 71/75 in this sandbox. Four unrelated
  `tests/pi-observe.security.test.mjs` cases could not bind `127.0.0.1` (`EPERM`). QA separately recorded
  73/75 with two pre-existing Langfuse-dependent failures.

## Escalations to SW Architect

None.
