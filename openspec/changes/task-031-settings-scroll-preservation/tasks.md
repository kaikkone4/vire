# Tasks — TASK-031 settings-scroll-preservation

Owner: SW-2 (frontend). Design + constraints: `arch-review.md`. Do the minimum below; no extra refactors.

## 1. Preserve scroll across same-view re-renders (`src/main.ts`)

- [x] 1.1 Add module-level `let lastRenderedView: View | null = null;` near the other render state.
- [x] 1.2 In `shell(content)`, BEFORE `app.innerHTML = …`: read the outgoing scroll position
  (`app.querySelector('main')?.scrollTop ?? 0`) and whether the view is unchanged
  (`lastRenderedView === current`).
- [x] 1.3 AFTER assigning `app.innerHTML`: set the new `<main>` scrollTop to the captured value when the
  view is unchanged, else `0`; then set `lastRenderedView = current`.
- [x] 1.4 Keep the decision in a tiny pure helper (e.g. `nextScrollTop(sameView, prevScroll)`), exported
  for unit testing (see task 3). Do NOT change any handler, `rerender()`, or per-view render function.

## 2. Align mapping-panel help copy (`src/env-mapping-ui.ts`)

- [x] 2.1 In `mappingPanel(...)` (line ~97), change the help text so it refers to the create affordance as
  **“Create & map”** (matching the button at line ~87), keeping the “explicit, never auto-create”
  meaning. One sentence; no markup or behaviour change.

## 3. Tests

- [x] 3.1 Add a unit test for the pure `nextScrollTop` helper in `tests/` (mirrors the existing
  `tests/*.test.mjs` pure-builder style): same-view → returns prevScroll; view-change → returns 0.
- [x] 3.2 (Optional) Extend `tests/envMappingUi.test.mjs` to assert the panel help text contains
  “Create & map” (the existing `/Create &amp; map/` assertion covers the button only).
- [x] 3.3 Run `npm run test:frontend` and `npm run build` (tsc + vite). Both must pass.

## 4. Manual acceptance (packaged/dev macOS app — human-only)

- [ ] 4.1 In Settings, scroll down, press Test connection (and Save range / Import now / Map): viewport
  stays put, does not jump to top.
- [ ] 4.2 Switch to another view and back: the other view opens at its top.
- [ ] 4.3 Mapping panel help text reads “Create & map”.

## Notes

- The scroll-restore lives in `shell()` (the single `app.innerHTML` chokepoint), so the fix applies to all
  views, not only Settings — this is the lowest-code, lowest-risk option, not scope creep. Acceptance is
  defined and verified in Settings.
- Focus restoration of the clicked control after re-render is a known, pre-existing limitation and is NOT
  in scope for this change.
