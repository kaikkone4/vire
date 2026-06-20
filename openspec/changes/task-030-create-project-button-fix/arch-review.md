# Architecture review — TASK-030 (SW-1 triage / design)

**Verdict: pass.** Single-component frontend defect; minimal OpenSpec change created. No component-boundary
split, no BA-architecture divergence, no research mandate.

## Bug

Settings → Environment → project mapping → **"Create project for &lt;env&gt;"** button does nothing.
Reported against the TASK-029 build; Langfuse import in the same view works.

## Diagnosis (root cause)

The env-mapping create affordance is the **only** project-creation path driven by a native
`window.prompt()`:

- `src/main.ts:57` `bindEnvMapping()` → `[data-create-map]` handler:
  `const name = prompt(...); if (name === null) return;` then `create_project` → `set_env_mapping`.
- On macOS Tauri (WKWebView / wry) the native **text-input** panel is not implemented; `prompt()`
  returns `null` with no dialog. The handler early-returns → silent no-op = the reported symptom.
- Corroboration: `window.alert` and `window.confirm` **are** wired in wry and are used for Map / Clear /
  Archive / Clear-credentials (all working). `prompt()` is the lone such call in the frontend
  (`grep "prompt(" src/` → 1 hit). The Projects-view "Create project" uses an in-app `<form>` and works.

**Not a TASK-029 regression.** Introduced in TASK-027 (`6b2d41d`, env-mapping UX). TASK-029's `main.ts`
change (`e4e2945`) was import-diagnostics only. Latent TASK-027 defect, first surfaced in real-app test
of the 029 build.

**Why tests missed it:** `tests/envMappingUi.test.mjs` covers pure HTML builders with no DOM/webview, so
a `prompt()`-returns-`null` no-op is unobservable there.

## Intended UX / data flow (Create Project in settings/project management)

DEC-006 (suggestion-first, human-approved) and the `project-env-mapping` spec require: an unmapped
discovered environment offers an **explicit** "create a project for this environment" action; on accept,
the app creates the project via the **normal creation path** (`create_project`) and then records the
environment → project mapping (`set_env_mapping`) — create-then-map, in one user-initiated step, never
auto-created. The contract is correct; the implementation chose an affordance (native `prompt()`) the
target webview cannot render. The fix realigns the affordance with the contract — input must be collected
**in-app**.

## Owner & boundary

- **Owner: frontend (SW-2).** Work is confined to `desktop-ui`: `src/main.ts` (`bindEnvMapping`) and
  `src/env-mapping-ui.ts` (`mappingRow` markup), plus a builder test.
- **Backend unchanged and sufficient:** `create_project` (`lib.rs:428`) and `set_env_mapping`
  (`lib.rs:630`) exist, are registered in `generate_handler!`, and are already exercised successfully by
  the working Projects-view form. No integration/data/agent-runtime/devops involvement.
- **No boundary crossing** per `artifacts/ba/03_architecture_plan.md` (frontend ⇄ Rust core IPC
  contract is untouched) → no split required.

## NFR / security

- SEC-010 holds: the affordance carries only environment names and project references — no secret
  material. No new IPC surface, no new permission, no data-model change.
- Tauri dialog plugin (already bundled) offers no text-input dialog — a prompt replacement **must** be
  in-app HTML, not a plugin call. Recorded as an implementation constraint.

## Recommendation

Implement **Option A** (inline input in the unmapped mapping row; reuse the Projects `#projectForm`
pattern). Verification requires a **manual UAT in the packaged macOS app** — the headless builder tests
cannot reproduce the WKWebView `prompt()` behavior.

## feedback_to_ba[]

None. BA architecture and the `project-env-mapping` spec intent are correct; this is an
implementation-affordance defect, not an architecture divergence.
