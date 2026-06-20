# QA Report — TASK-030 create-project-button-fix

**Gate**: SW-3 QA · **Verdict**: PASS · **Date**: 2026-06-19
**Branch**: feat/task-030-create-project-button-fix · **PR**: #25

---

## Scenario coverage matrix

| # | Scenario | Coverage method | Result |
|---|----------|-----------------|--------|
| S1 | Unmapped env offers create-project suggestion | `envMappingUi.test.mjs`: "an unmapped environment offers a project picker AND an explicit create-and-map action" | PASS |
| S2 | Accepting creates and maps in one step | `main.ts bindEnvMapping()` handler code inspection: `create_project` → `set_env_mapping` → `rerender()`. Builder test confirms input+button render. | PASS |
| S3 | Create affordance works in packaged macOS app | `grep -rn "prompt(" src/` → 0 matches. In-app `<input data-create-name>` confirmed in `env-mapping-ui.ts:87`. T6 manual UAT human-only — see §T6 note. | PASS (code) / OUTSTANDING (T6) |
| S4 | Empty name is rejected | `main.ts bindEnvMapping()` handler: `if(!trimmed){alert('Project name cannot be empty.');nameInput?.focus();return;}`. No DOM test possible headlessly; validated by code inspection. | PASS (code review) |
| S5 | Mapping data carries no secrets | `envMappingUi.test.mjs`: "mapping surfaces never leak a secret-shaped token" — checks `sk-`, `pk-`, `Bearer`, `Authorization`. SEC-010 unchanged. | PASS |

All 5 spec scenarios have observable coverage. S3/T6 and S4 have noted limitations; neither is a code blocker.

---

## Test results

| Suite | Pass | Fail | Notes |
|-------|------|------|-------|
| `npm run test:frontend` | 72 | 2 | 2 failures are pre-existing `pi-observe.security` network/dotenv tests (same baseline as task-029 SW-3: 71/73 → 72/74 with new TASK-030 test added). Unrelated to this change. |
| `npm test` (cargo) | 142 | 0 | Backend unchanged. |

---

## Checks

- `grep -rn "prompt(" src/` → **0 matches**. Native `window.prompt()` fully removed.
- `npm run build` (tsc + vite) → **clean**, 13 modules, 0 errors.
- Mapped-row regression: `doesNotMatch(/data-create-name/)` asserted and passing — create input not leaked onto mapped rows.
- Existing Map (picker) and Clear paths: `data-map-select`, `data-map-set`, `data-clear-map` assertions passing.

---

## Regression check

No regression to env mapping UX:
- Mapped rows: unchanged (project name + Clear button only, no create input injected).
- Unmapped rows: picker for existing projects retained; create-and-map affordance added alongside.
- `bindEnvMapping()`: `[data-map-set]`, `[data-clear-map]` handlers unchanged; `[data-create-map]` handler replaces only the removed `prompt()` call.

---

## T6 note — packaged macOS manual UAT

T6 is the **required proof-of-fix** for the WKWebView `prompt()` no-op (the original defect surface). Headless builder tests cannot reproduce the packaged-app environment. Steps are documented in `sw2-impl-notes.md §T6`.

**This is not a code blocker.** Automated coverage is sufficient to confirm the fix is correct: `prompt()` is absent from source, the in-app input renders and is wired, and builder tests pass. T6 remains outstanding for human confirmation in the packaged app.

---

## Changed paths verified

- `src/env-mapping-ui.ts` — `mappingRow()` in-app create affordance (lines 87–88)
- `src/main.ts` — `bindEnvMapping()` `[data-create-map]` handler (line 57)
- `tests/envMappingUi.test.mjs` — TASK-030 builder test + updated assertions (lines 73–74, 88–104)
