# SW-2 implementation notes — TASK-030 create-project-button-fix

Owner: frontend developer. Branch: `feat/task-030-create-project-button-fix` (based on the
task-029 branch, since the env-mapping feature TASK-027/029 is not yet merged to `main` —
`src/env-mapping-ui.ts` does not exist on `main`, so basing off `main` would lose the bug surface).

## What changed (Option A — inline in-app input)

### `src/env-mapping-ui.ts` — `mappingRow()` (T1)
- Unmapped row's third `<td>` no longer renders a lone `Create project for <env>` button.
- It now renders an in-app create affordance: a text `<input class="env-create-name"
  data-create-name="<env>" value="<env>" maxlength="120">` (pre-filled with the environment name,
  matching the Projects-view name limit) plus a `<button data-create-map="<env>">Create &amp; map</button>`.
- `mappingRow` stays a pure HTML builder (no DOM, no IPC) — markup remains unit-testable. Carries
  only env name + project references; no secret material (SEC-010 unchanged).
- The mapped-row branch is untouched (still Clear-mapping only; no create input).

### `src/main.ts` — `bindEnvMapping()` (T2)
- The `[data-create-map]` handler no longer calls `window.prompt()`. It reads the paired
  `[data-create-name="<env>"]` input (via `CSS.escape`, consistent with the existing
  `data-map-select` query), trims it, and validates non-empty through the existing `alert(...)`
  error path (now also focuses the input on the empty case).
- On a valid name it runs the same two-call sequence as before — `create_project` →
  `set_env_mapping` → `rerender()`. Backend untouched.
- The lone `window.prompt()` call (the macOS WKWebView no-op, root cause) is removed.

### T3 — no native prompt remains
- `grep -rn "prompt(" src/` → no matches (including comments; the doc comment was reworded to
  "native window.prompt dialog" to keep the gate grep clean). `alert`/`confirm` retained (they work
  in wry's WKWebView).

## Tests / checks (T4, T5)

- `tests/envMappingUi.test.mjs` (T4):
  - Updated the unmapped-row test: button label assertion `Create project for staging` →
    `Create &amp; map`.
  - Added a TASK-030 test: an unmapped row emits `<input ... data-create-name="staging" ...
    value="staging">` pre-filled with the env name, paired with `data-create-map`.
  - Mapped-row test now also asserts `doesNotMatch(/data-create-name/)`.
  - No-projects unmapped test now asserts both the create input and button render.
- `npm run test:frontend`: 72/74 pass. The 2 failures are `tests/pi-observe.security.test.mjs`
  (network/dotenv-dependent, do not import any file changed here) — the same pre-existing failures
  recorded as the task-029 SW-3 baseline ("71/73"); adding one new test moves it to 72/74.
- `npm run build` (tsc + vite): clean, 13 modules transformed.
- `npm test` (cargo): 142 passed, 0 failed — backend unaffected (no Rust change).

## T6 — manual UAT (REQUIRED, human-only)

Per handoff/tasks, the failing surface is the packaged macOS WKWebView, which headless builder
tests cannot reproduce. This step requires a human running the packaged app and is the real
proof-of-fix. Steps:

1. Build/run the packaged macOS app (`npm run tauri:build` or an installed build).
2. Configure Langfuse + import to discover environments.
3. Settings → Environment → project mapping: for an unmapped env, type a name in the inline input
   (pre-filled with the env name) and click **Create & map**.
4. Expect: project created, environment mapped, row flips to `Mapped → <name>` — no silent no-op.
5. Clear the name and click **Create & map**: expect a validation alert ("Project name cannot be
   empty."), no project created.

## Scope / boundary

Frontend-only, within `desktop-ui`. No Rust/backend, no Tauri-dialog-plugin call (it has no
text-input dialog). No data-model, IPC-contract, or NFR change.
