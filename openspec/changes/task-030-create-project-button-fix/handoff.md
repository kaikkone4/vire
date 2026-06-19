# Handoff — TASK-030 task-030-create-project-button-fix

## Current phase
SW-2 Frontend implementation — COMPLETE. Ready for SW-3 (QA / integration).

## Last gate result
SW_FRONTEND: complete. Option A (inline in-app input replaces native prompt()). T1–T5 done; T6 (manual
UAT in packaged macOS app) outstanding — human-only. Branch `feat/task-030-create-project-button-fix`
(based on task-029 branch — env-mapping not yet on `main`). Detail in `sw2-impl-notes.md`.
Prior SW-1 SW_ARCHITECT: pass.

## Changes
- `src/env-mapping-ui.ts` `mappingRow()` — unmapped row renders inline `<input data-create-name>`
  (pre-filled w/ env name) + `Create & map`; pure builder, SEC-010 unchanged.
- `src/main.ts` `bindEnvMapping()` — `[data-create-map]` reads the input, validates non-empty via
  `alert(...)`, then create_project → set_env_mapping → rerender. `prompt()` removed.
- `tests/envMappingUi.test.mjs` — in-app input builder test + updated assertions.

## Checks
- frontend `node --test`: 72/74 (2 pre-existing `pi-observe.security` network fails, unrelated).
- cargo: 142/142. build (tsc+vite): clean. `grep -rn "prompt(" src/`: none.

## Bug / root cause (fixed)
Settings → Environment mapping "Create project for <env>" did nothing: the `[data-create-map]` handler
used native `window.prompt()`, which returns `null` (no dialog) in the macOS WKWebView → silent no-op.
Latent TASK-027 defect (`6b2d41d`), not a 029 regression. Full detail in `proposal.md`.

## Next action (SW-3 QA)
Integration test the in-app create-and-map flow. **T6 manual UAT in the packaged macOS app is the
required proof-of-fix** (headless tests cannot reproduce the WKWebView prompt() no-op) — steps in
`sw2-impl-notes.md`. Backend unchanged; no Rust to re-test beyond the existing 142.

## Blockers
None for code. T6 manual UAT (packaged macOS app) is human-only and still outstanding.

## Files
- `proposal.md`, `specs/project-env-mapping/spec.md`, `tasks.md`, `arch-review.md`,
  `sw2-impl-notes.md` (SW-2 impl + UAT steps), `handoff.md`.

## Key refs
- `src/env-mapping-ui.ts` `mappingRow()` (in-app input); `src/main.ts` `bindEnvMapping()` (handler)
- `src-tauri/src/lib.rs:428`/`:630` create_project/set_env_mapping (unchanged). Origin TASK-027 `6b2d41d`.
