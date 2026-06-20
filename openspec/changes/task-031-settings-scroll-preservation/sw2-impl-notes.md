# SW-2 implementation notes — TASK-031 settings-scroll-preservation

Owner: sw-frontend-developer. Design: `arch-review.md`. Checklist: `tasks.md`.

## What changed

| File | Change |
|------|--------|
| `src/scroll.ts` (new) | Pure `nextScrollTop(sameView, prevScroll)` helper — returns `prevScroll` on a same-view re-render, `0` otherwise. Lives in its own module so it is importable by the Node test runner (`main.ts` runs `rerender()` and imports `@tauri-apps/api/core` at load, so it cannot be imported in a test). |
| `src/main.ts` | (1) `import { nextScrollTop } from './scroll'`. (2) Module-level `let lastRenderedView:View|null=null;`. (3) In `shell()`: before the `app.innerHTML=…` swap, capture `prevScroll = app.querySelector('main')?.scrollTop ?? 0` and `sameView = lastRenderedView===current`; after the swap, `m.scrollTop = nextScrollTop(sameView, prevScroll)` then `lastRenderedView = current`. No handler / `rerender()` / per-view render function touched. |
| `src/env-mapping-ui.ts` | `mappingPanel()` help text (line 97): old TASK-030 leftover `use “Create project for …”` → `use “Create &amp; map”` (matches the button label at line 87). One sentence; no markup/behaviour change; "explicit, never auto-create" meaning preserved. |
| `tests/scroll.test.mjs` (new) | Unit test for `nextScrollTop`: same-view → returns prevScroll; view-change → returns 0 (both with 0 and non-zero prevScroll). |
| `tests/envMappingUi.test.mjs` | Added one assertion (task 3.2, optional): `mappingPanel([], projects)` (empty state renders the help `<p>` but no mapping-row buttons, isolating the help copy) contains `/Create &amp; map/` and not `/Create project for/`. |

## Why the helper is a separate module (`src/scroll.ts`)

`arch-review.md §2` / `tasks.md §1.4` ask for an exported pure helper for unit testing. `main.ts` is a side-effecting entry module (it calls `rerender()` at load and imports the Tauri API), so importing a symbol from it in a Node test would execute the whole app shell. Every other pure builder in this codebase already lives in its own kebab-case module (`html.ts`, `forms.ts`, `shell-chrome.ts`, `env-mapping-ui.ts`, …), so `scroll.ts` follows the established pattern.

## Edge cases (per arch-review §4)

- First render: no prior `<main>` → `prevScroll=0`; `lastRenderedView=null` → `sameView=false` → top. ✓
- Shorter content after an action: browser clamps `scrollTop` to the new max — lands near the bottom, far better than 0. ✓ (no special handling needed)
- Error path (`render()` catch → `shell()`): same-view best-effort restore; error banner is prepended. ✓
- Focus of the clicked control is NOT restored (node destroyed on rebuild) — pre-existing, explicitly out of scope.

## Tests / checks

- `npm run test:frontend`: 73 pass / 2 fail. The 2 failures are in `tests/pi-observe.security.test.mjs` (Langfuse telemetry ingestion — needs a live local Langfuse + network; they 401 / produce no ingestion request offline). **Pre-existing and unrelated**: verified by stashing all my changes and running that file against pristine `main` HEAD → still 8 pass / 2 fail. The file does not reference scroll/`nextScrollTop`/`lastRenderedView`.
- Affected files in isolation (`tests/scroll.test.mjs` + `tests/envMappingUi.test.mjs`): 14/14 pass.
- `npm run build` (`tsc && vite build`): clean — tsc no errors, vite transformed 14 modules (includes new `scroll.ts`).

## Not done here (correctly out of scope)

- No conversion to partial/diff rendering (rejected in arch-review §3).
- No focus restoration of the clicked control (arch-review §4, §tasks notes).
- Manual UAT (`tasks.md §4`) is human-only in the packaged/dev macOS app — the scroll fix is DOM/webview-bound and the frontend suite tests pure builders only. Routed to SW-3 QA for that.
