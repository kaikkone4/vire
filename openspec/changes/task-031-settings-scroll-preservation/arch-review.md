# Architecture review — TASK-031 settings-scroll-preservation

**Verdict: PASS** — design is ready for SW-2. Single-chokepoint frontend fix, no component-boundary
crossing, no schema/IPC/NFR impact, no BA escalation. Not split-required (one render-chokepoint change
plus one in-Settings copy line, one PR).

## 1. Problem (confirmed from code, not reproduced in a webview)

Symptom: pressing controls in Settings makes the panel "appear to reload" and scroll back to the top.

Mechanism, end to end:
- The scroll container is `<main>` — `src/style.css`: `main{overflow:auto;padding:24px}`; the parent
  `.window` is `overflow:hidden`, so `<main>` is the only scroller.
- Every view is rebuilt by one function: `shell(content)` does `app.innerHTML = "<div class='window'>…
  <main>${content}</main>…"` (`src/main.ts:39`). This **destroys and recreates** `<main>`.
- Every Settings handler ends in `rerender()` → `render()` → `renderSettings()` → `shell()`
  (`src/main.ts:33,42,49`): Save settings / Save secret / Clear / Test (`:56`), Import / Backfill /
  Save range (`:53,:54`), Map / Clear mapping / Create & map (`:57`).
- A newly created `<main>` has `scrollTop = 0`, so the viewport jumps to the top; the full-tree
  innerHTML swap is the "reload" flash.
- No scroll-preservation exists today (`grep -n "scrollTop\|scrollTo" src/` → none).

This is inherent to the full-innerHTML render pattern (present since v0.1), not a TASK-029/030 regression.
It affects all views but is felt in Settings because Settings is the longest and most control-dense
(Langfuse panel + credentials + test + source panel + range form + env-mapping table), so the controls
sit below the fold.

## 2. Recommended fix (design — SW-2 implements; do not implement here)

Preserve `<main>` scroll across **same-view** re-renders, reset on **view change**, at the single
`shell()` chokepoint:

- Module-level `let lastRenderedView: View | null = null;`.
- In `shell()` before the innerHTML assignment: capture `prevScroll = app.querySelector('main')?.scrollTop
  ?? 0` and `sameView = lastRenderedView === current`.
- After the assignment: `const m = app.querySelector('main'); if (m) m.scrollTop = sameView ? prevScroll
  : 0;` then `lastRenderedView = current;`.
- Factor the decision into a pure `nextScrollTop(sameView, prevScroll)` helper so it is unit-testable
  (the live-DOM wiring around it is not).

Why this placement:
- `shell()` is the ONE place that replaces `app.innerHTML`; every render funnels through it. Fixing it
  there is the least code and is correct for all views.
- The `sameView` guard preserves the desired UX: navigating between views still opens at the top (set in
  the `.nav` click handler `current=…; rerender()`), while in-view state updates keep the user's place.

Scope note (not creep): because the fix lives at the shared chokepoint, it benefits every view, not only
Settings. A Settings-only special case would be MORE code and more fragile. User-facing scope and
acceptance remain "Settings"; the global behaviour is a free, lower-risk side effect.

## 3. Alternatives considered and rejected

- **Partial / targeted DOM updates or a diffing framework** (re-render only the changed panel): correctly
  eliminates the rebuild, but is a disproportionate rewrite of the render pipeline for a polish bug, adds
  surface area / possibly a dependency, and carries real regression risk. Rejected for this change; if the
  full-innerHTML pattern later becomes a performance problem it can be revisited as its own change.
- **`scrollIntoView` on the clicked control after re-render**: fragile (form submits report the form, not
  a position; the clicked node is destroyed by the rebuild), restores an approximate not exact position.
  Rejected.
- **Persisting scroll to/from storage**: unnecessary for in-session re-renders; over-engineered. Rejected.

## 4. Edge cases SW-2 must honour

- First render: no prior `<main>` → `prevScroll = 0`, `lastRenderedView = null` → `sameView = false` →
  top. Correct.
- Shorter content after an action (e.g. a panel collapses): the browser clamps `scrollTop` to the new max
  — lands near the bottom of the shorter content, acceptable and far better than 0.
- Error path (`render()` catch → `shell()`): same-view restore is acceptable; an error banner is prepended
  so the restore is best-effort. No special handling required.
- Focus of the clicked control is NOT restored (the node is destroyed on rebuild). Pre-existing behaviour;
  explicitly out of scope — note, do not fix here.

## 5. Testability

- The current frontend suite tests **pure HTML builders only** (no DOM/webview) — the same gap that hid
  TASK-030's `prompt()` no-op. The scroll fix is DOM-bound, so:
  - Add a unit test for the pure `nextScrollTop` helper (cheap, deterministic).
  - Primary acceptance is **manual UAT in the macOS app** (consistent with TASK-030), per `tasks.md §4`.
- Copy fix: `tests/envMappingUi.test.mjs:82` asserts only the button label (`/Create &amp; map/`), not the
  help text — so aligning the help text breaks no test. An optional assertion can lock it in (`tasks.md
  §3.2`).

## 6. Architecture decisions / contracts

- No new ADR required. No data-model change vs the architecture plan; no IPC/command added or changed; no
  credential or egress surface touched (the env-mapping markup remains pure HTML, SEC-010 unchanged).
- No `feedback_to_ba[]` entry: this is a frontend rendering defect, not a divergence from BA architecture.

## 7. Copy cleanup decision (folded in, bounded)

The mapping panel help text (`src/env-mapping-ui.ts:97`) says *use "Create project for …"* while the
control is **Create & map** (`:87`) — a TASK-030 leftover. It is one line in the same Settings view, so it
is naturally part of this Settings UX polish and is included. It does not broaden scope (no behaviour, no
new surface). If it had required touching another view or any logic, it would have been split out.
