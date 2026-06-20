# TASK-031 — Settings controls scroll the panel back to the top on every action (full-innerHTML re-render)

## Why

Janne tested the post-PR-#25 build in the real app. In **Settings**, pressing almost any control —
Save settings, Save/Replace credentials, Test connection, Save range, Import now, Backfill now, Map,
Clear mapping, Create & map — makes the whole Settings panel **appear to reload and scroll back to the
top**. Because Settings is the longest, most control-dense view, the controls a user reaches for sit
below the fold; clicking one bounces the viewport to the top every time. Reported as "very irritating".

### Root cause (diagnosed, not yet fixed)

The frontend is a vanilla-TS single-window app whose entire view tree is rebuilt by one chokepoint:

```
src/main.ts:39  shell(content)  →  app.innerHTML = `<div class="window">…<main>${content}</main>…`
```

Every state change funnels through `rerender()` → `render()` → `renderSettings()` → `shell()`
(`src/main.ts:33,42,49`). Every Settings handler ends in `rerender()`:

- `bindLangfuse()` — settings save, secret save, clear, test (`src/main.ts:56`)
- `bindImportControls()` / `runImport()` — import, backfill, save range (`src/main.ts:53,54`)
- `bindEnvMapping()` — map, clear, create-and-map (`src/main.ts:57`)

The scroll container is `<main>` (`src/style.css`: `main{overflow:auto;padding:24px}`). Re-assigning
`app.innerHTML` **destroys and recreates** that `<main>` element, so the freshly created node starts at
`scrollTop = 0`. The visible full-tree rebuild is the "appears to reload" flash; the new `<main>` at the
top is the "scrolls back to the top" symptom. There is no scroll-preservation logic anywhere today
(`grep -n "scrollTop\|scrollTo" src/` → no hits).

This is not a regression introduced by TASK-029/030 — it is inherent to the full-innerHTML render pattern
that has existed since the app's first version. It only became painful as Settings grew (TASK-027/029
added the Langfuse panel, source panel, range form, and env-mapping table), pushing controls below the
fold. All views share the pattern, but Settings is where it is felt.

## What changes

Preserve the `<main>` scroll position across **in-view** re-renders at the single `shell()` chokepoint,
and reset to the top only when the user navigates to a **different** view (the existing, desired
behaviour). No architecture change, no DOM-diffing library, no new dependencies. See
`arch-review.md` for the recommended mechanism and rejected alternatives.

Folded-in copy cleanup (same Settings view, one line, does not broaden scope): the env→project mapping
panel's help text still reads *use "Create project for …"* while the actual control is **Create & map**
(`src/env-mapping-ui.ts:97` vs `:87`, leftover from TASK-030). Align the help text to the button label.

## Scope

In scope:
- Scroll-position preservation across same-view re-renders; reset on view change (`src/main.ts` `shell()`).
- One-line copy alignment in the mapping panel help text (`src/env-mapping-ui.ts`).

Out of scope (explicitly not this change):
- Converting the render pipeline to targeted/partial DOM updates or a diffing framework (disproportionate
  to a UX-polish bug; higher regression risk). Recorded as a rejected alternative, not deferred work.
- Focus restoration of the clicked control after re-render (existing limitation; note, do not fix here).
- Any backend, schema, IPC, credential, or import-behaviour change.

## Owner

SW-2 (frontend developer) implements; SW Architect (this doc) owns the design.

## Impact

- Affected code: `src/main.ts` (shell render chokepoint), `src/env-mapping-ui.ts` (one help string).
- Affected specs: new capability `settings-scroll-preservation` (this change).
- Tier: L1-equivalent — frontend-only, no new backend/egress/deps, no data-model change.
- Risk: low. The change is confined to one render chokepoint plus one string; the same-view guard keeps
  cross-view navigation behaviour identical.
