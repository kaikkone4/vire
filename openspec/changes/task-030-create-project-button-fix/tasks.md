# Tasks — TASK-030 create-project-button-fix

Owner: **SW-2 frontend developer**. Scope is `desktop-ui` frontend only — no Rust/backend change.
Recommended approach: **Option A — inline input in the unmapped mapping row** (see `proposal.md`).

## Implementation (SW-2, frontend)

- [x] T1 — `src/env-mapping-ui.ts` `mappingRow()`: for an **unmapped** environment, render an in-app
  create affordance instead of (or alongside) the existing `[data-create-map]` button — a text `<input>`
  pre-filled with the environment name plus a "Create &amp; map" button. Keep `mappingRow` a pure
  builder (no DOM, no IPC) so it stays unit-testable. Carry only env name + project references (SEC-010).
  **Done:** inline `<input class="env-create-name" data-create-name="<env>" value="<env>" maxlength="120">`
  + `<button data-create-map>Create &amp; map</button>`; mapped branch untouched.
- [x] T2 — `src/main.ts` `bindEnvMapping()`: replace the `prompt()`-based `[data-create-map]` handler
  with one that reads the new in-app input, trims it, validates non-empty (reuse the existing
  `alert(...)`/error path for the empty case), then calls `create_project` → `set_env_mapping` →
  `rerender()` — the same two-call sequence as today, minus `prompt()`. Remove the now-dead
  `prompt()` call. **Done:** reads `[data-create-name]` via `CSS.escape`, same two-call flow, `prompt()` gone.
- [x] T3 — Confirm no other native `window.prompt()` remains in `src/` (`grep -n "prompt(" src/` must
  return nothing after the change). `alert`/`confirm` usages stay as-is (they work in the webview).
  **Done:** `grep -rn "prompt(" src/` → no matches; `alert`/`confirm` retained.

## Tests / checks (SW-2, then SW-3)

- [x] T4 — `tests/envMappingUi.test.mjs`: add a builder assertion that an unmapped `mappingRow(...)`
  emits the in-app create input + button (and a mapped row does not). Builder-level only; this guards the
  markup, not the click behavior. **Done:** added TASK-030 input test; updated unmapped/mapped/no-projects
  assertions. envMappingUi suite 11/11 green.
- [x] T5 — `npm run test:frontend` (node --test) and `npm test` (cargo) both green; `npm run build`
  (tsc + vite) clean. Backend tests should be unaffected (no Rust change). **Done:** frontend 72/74
  (2 pre-existing `pi-observe.security` network failures, unrelated); cargo 142/142; build clean.
- [ ] T6 — **Manual UAT in the packaged macOS app** (required — the failing surface): import to discover
  environments, then in Settings → Environment → project mapping click "Create project for &lt;env&gt;",
  enter a name in the in-app input, confirm. Expect: project created, environment mapped, row flips to
  "Mapped → &lt;name&gt;". Also verify empty-name is rejected with a message. The headless builder tests
  cannot reproduce the WKWebView `prompt()` behavior, so this step is the real proof of fix.

## Notes / constraints

- Backend is untouched: `create_project` (`src-tauri/src/lib.rs:428`) and `set_env_mapping`
  (`src-tauri/src/lib.rs:630`) already exist and are registered in `generate_handler!`. The Projects-view
  create form already calls `create_project` successfully via an in-app `<form>` — reuse that pattern.
- Do **not** reach for the Tauri dialog plugin: it has no text-input dialog (message/ask/confirm/open/
  save only). The replacement must be in-app HTML.
- Keep the change minimal and within `desktop-ui`; no scope creep into import/range/credential flows.
