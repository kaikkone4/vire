# Code review — TASK-030 create-project-button-fix

**Gate:** SW-4 Code Review  
**Verdict:** PASS  
**Branch / PR:** `feat/task-030-create-project-button-fix` / PR #25  
**Reviewed delta:** implementation commit `2c86111` against parent `9de66e5`; unrelated TASK-029 history excluded.

## Blocking issues

None.

The change removes the unsupported native prompt, keeps the existing create-then-map IPC sequence, and limits the new UI to unmapped rows. Naming and structure follow the surrounding frontend conventions; no dead code or unnecessary abstraction was introduced.

## Suggestions

1. Update the mapping-panel help text to match the new button label. `src/env-mapping-ui.ts:97` still tells users to use “Create project for …”, while the rendered action is now “Create & map”.
2. Add handler-level coverage when the frontend test harness supports DOM interaction. `tests/envMappingUi.test.mjs:88`–`95` verifies only generated markup; the input lookup, trimming, empty-name rejection, and `create_project` → `set_env_mapping` ordering at `src/main.ts:57` remain covered by inspection rather than an executable regression test.

## Escalations to SW Architect

None.

## Commit / PR quality

- Commit `2c86111` is scoped, descriptive, and records the root cause, implementation, checks, and outstanding packaged-app UAT.
- PR metadata could not be fetched because this review environment could not connect to `api.github.com`; PR description completeness was therefore not independently verified.

## Checks

- `node --import tsx --test tests/envMappingUi.test.mjs`: 11/11 passed.
- `npm run build`: passed.
- `npm test`: passed (142 unit tests plus 3 integration tests).
- `npm run test:frontend`: 70/74 passed; four unrelated `pi-observe.security` tests could not bind `127.0.0.1` in this sandbox (`EPERM`). TASK-030 tests passed.
- No `prompt()` calls found in `src/`.
- `git diff --check`: passed.
- Packaged macOS T6 UAT remains outstanding as documented in `tasks.md`; it is not an SW-4 craft blocker.

## Reviewed paths

- `src/env-mapping-ui.ts`
- `src/main.ts`
- `tests/envMappingUi.test.mjs`
- `openspec/changes/task-030-create-project-button-fix/`

