# TASK-030 — "Create project for <env>" button in Settings does nothing (native prompt() no-op)

## Why

Janne installed the TASK-029 build and tested it in the real app. The Langfuse import path now works
(environments discovered, traces imported), but in **Settings → Environment → project mapping**, the
**"Create project for &lt;env&gt;"** button **does nothing** — clicking it produces no dialog, no
project, no error.

### Root cause (diagnosed, not yet fixed)

The create-project affordance in Settings is the only one that drives project creation through a native
`window.prompt()`:

```
src/main.ts:57  bindEnvMapping()  →  [data-create-map] handler
  const name = prompt(`Create a new Vire project for environment "${env}"…`, env);
  if (name === null) return;   // <-- silent early-return
  …create_project → set_env_mapping…
```

On macOS Tauri (WKWebView via wry), the native **text-input** panel (`window.prompt`) is not wired up,
so `prompt()` returns `null` immediately with no dialog shown. The handler then hits
`if (name === null) return;` and exits — the exact "button does nothing" symptom. This is consistent
with the app's other native dialogs: `window.alert` and `window.confirm` (used for Map / Clear / Archive
/ Clear-credentials, all reported working) **are** wired up in wry; only `prompt` is not. `prompt()` is
the single such call in the entire frontend (`grep "prompt(" src/` → one hit, line 57).

This is **not a TASK-029 regression**. The `prompt()`-based create handler was introduced in TASK-027
(commit `6b2d41d`, env-mapping UX polish). TASK-029's only `main.ts` change (`e4e2945`) was Workstream D
import diagnostics and did not touch the create path. The defect is a **latent TASK-027 bug first
surfaced during real-app testing of the TASK-029 build**.

### Why it slipped through

The frontend suite (`tests/envMappingUi.test.mjs`) exercises the **pure HTML builders** (`mappingRow`,
`mappingPanel`) with no DOM and no webview, so it cannot observe `prompt()` returning `null` inside a
WKWebView. The Projects view's "Create project" uses an in-app `<form>` (not `prompt`) and works — which
is why only the Settings affordance is affected.

## What changes

Replace the native-`prompt()` accept-flow for the env-mapping create-project suggestion with an
**in-app input affordance** (Vire already owns this pattern — the Projects view's `#projectForm`). The
backend is untouched: `create_project` and `set_env_mapping` are existing, registered, working Tauri
commands (`src-tauri/src/lib.rs:428,630`; registered in `generate_handler!`). This is presentation/wiring
only, entirely within the `desktop-ui` frontend boundary.

Recommended approach (**Option A**): an **inline input** in each unmapped mapping row — a text `<input>`
(pre-filled with the environment name) plus a "Create &amp; map" button rendered by `mappingRow()`
(`src/env-mapping-ui.ts`) and wired in `bindEnvMapping()` (`src/main.ts`). This preserves the
DEC-006 "single explicit action: create then map" UX, surfaces an empty-name validation message via the
existing error path, and keeps `mappingRow` a pure builder so the markup stays unit-testable.

> Constraint for the implementer: the bundled Tauri dialog plugin (`@tauri-apps/plugin-dialog`,
> already a dependency, used for CSV save) provides `message` / `ask` / `confirm` / `open` / `save`
> only — **there is no text-input dialog**. A prompt replacement must be in-app HTML, not a plugin call.

## Scope

- **In scope:** the env-mapping "Create project for &lt;env&gt;" affordance in Settings — `src/main.ts`
  (`bindEnvMapping`) and `src/env-mapping-ui.ts` (`mappingRow` markup), plus a builder test for the new
  markup.
- **Out of scope:** any backend / Rust change; the Projects-view create form (already in-app, works);
  the Langfuse import, range, backfill, and credential flows (all working); the env-picker and Map /
  Clear-mapping actions (use checkboxes / `confirm`, both working).

## Owner

**Frontend (SW-2 frontend developer).** No backend, integration, data-model, or external-API work.

## Research mandate

**None.** The defect and fix are purely local UI behavior in Vire's own webview; no external Langfuse /
platform API behavior is in question. The macOS WKWebView `prompt()` limitation is established
desktop-platform behavior, not something to research against a live service.

## Impact

- Capability touched: `project-env-mapping` (the create-and-map suggestion affordance).
- Files: `src/main.ts`, `src/env-mapping-ui.ts`, `tests/envMappingUi.test.mjs` (add markup test).
- No data-model, IPC-contract, security, or NFR change. SEC-010 unchanged: the affordance still carries
  only environment names and project references — no secret material.
- Verification requires a manual UAT pass in the **packaged macOS app** (the failing surface), because
  the WKWebView behavior cannot be reproduced in the headless builder tests.
