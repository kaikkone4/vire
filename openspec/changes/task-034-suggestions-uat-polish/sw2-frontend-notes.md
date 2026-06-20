# SW-2 Frontend implementation notes — TASK-034 (Workstream C + A3 + B5)

Frontend-led scope only, completing the UI polish deferred from the backend SW-2 pass
(`sw2-backend-notes.md`). **No backend semantics changed** — no `.rs` files and no spec files touched.
All edits are additive, secret-free, and preserve the load-bearing guarantees (no-auto-post,
absence ≠ zero, never-zero span echo).

## What changed

### A3 — edit-panel End default reflects backend same-minute rounding (DEC-034 echo)
- `src/suggestions-ui.ts`
  - New exported pure helper `addMinutesHHMM(hhmm, mins)` — adds minutes to an `HH:MM` wall-clock
    string and **clamps a same-day midnight cross to `23:59`**, mirroring the backend
    `bump_end_if_not_after` (lib.rs) so the visible default equals what accept will store.
  - `suggestionRow`: when a **timed** block's start and end render in the same clock minute
    (`startVal === endVal`, `duration_minutes != null`), the End `<input>` default is pre-filled to
    `start + duration` (min 1) instead of `start`. Untimed blocks (empty start) are left blank — the
    backend never invents a duration. Only the editable default changes; the displayed block span and
    `spanLabel` are untouched (they still reflect raw evidence).

### B5 — AI cost line on Today + Reports summary cards (DEC-003 completion)
- `src/summary-cards.ts` **(new, pure/testable module)** — `summaryCards(summaries, lead, emptyMsg)`
  extracted out of `main.ts` so the card markup (incl. the new cost sub-line) is unit-testable, matching
  the project's existing pure-builder convention (suggestions-ui / env-mapping-ui / import-report).
  - AI-suggested sub-line now appends cost via the shared `costLabel` from `suggestions-ui.ts`:
    `AI-suggested {h} · {cost}` (per-project) and `… , reported separately` (lead "Total tracked").
  - `cost` is `costLabel(ai_cost_total, ai_cost_currency)` → renders **"—" when NULL** (absence ≠ zero),
    never "0". Sub-line still appears only when `ai_minutes > 0`; AI time is never folded into the human
    total.
  - `aggregateAiCost` (lead card): sums AI cost across projects; a **mixed-currency** set (or no cost)
    renders **"—"** (a cross-currency sum is not a meaningful single figure), else the single currency.
- `src/main.ts` — import `summaryCards` from `./summary-cards`; removed the inline copy; both call sites
  (`renderToday` lead=true, `renderReports` lead=false) now pass `summaries`. `h()` retained for `table`.

### C — environment trackability explained (DEC-004 posture), `src/suggestions-ui.ts`
- **C1** `unmappedNotice`: copy tightened to *"not trackable until mapped"* + action button relabelled
  *"Map in Settings"* (keeps the per-env trace counts and `data-goto-view="Settings"`).
- **C2** untimed rows now carry an on-row badge *"not auto-trackable — add time manually"*
  (`<span class="hint">` in the Duration cell) so the state is visible on the row, not only inside the
  edit panel. Timed rows carry no badge.
- **C3** `emptyState` rewritten to name **every** candidate cause as a `<ul>`, each with an action:
  nothing-imported/all-decided (→ Open Settings to import), unmapped (→ Map in Settings, only when
  unmapped present), untimed/not-auto-trackable, and source-disabled/down (only when degraded; → Open
  Settings). `suggestionsBody(list, { sourceDegraded })` threads the flag; `renderSuggestions` computes
  it from `sourceHealth` (degraded **or** `disabled`). The source banner already renders above the body.
- No new CSS classes added (reused `.hint`/`.empty`/`.banner`/`.card`) — no blind visual change.

## Tests
- `tests/suggestionsUi.test.mjs` (extended): `addMinutesHHMM` (add + 23:59 clamp); A3 same-minute End
  default `09:01` and **not** `09:00`; A3 normal block keeps its real End; C2 badge on untimed row +
  ordering before the edit panel, and absent on timed rows; C1 trackability copy + "Map in Settings";
  C3 empty state names every cause with action, source-down cause only when `sourceDegraded`, no "0".
- `tests/summaryCards.test.mjs` **(new)**: cost on the project card; "—" when NULL (never "0");
  no AI sub-line when `ai_minutes == 0`; lead card aggregates AI cost; mixed-currency → "—"; empty
  message when no cards; project-name escaping.

## Gate results (run on `feat/task-034-suggestions-uat-polish`)
- **G1** `cargo test` — **not re-run; no backend/Rust change in this pass.** Established green in
  `sw2-backend-notes.md` (164 pass, fmt/clippy clean).
- **G2** `npm run build` (tsc + vite) → **green**. `npm run test:frontend` → **99 pass / 2 fail**.
  The 2 failures are the **pre-existing environmental** `tests/pi-observe.security.test.mjs` cases
  (Langfuse env contamination); re-running that file with `LANGFUSE_*` unset → **10/10 pass**. My-domain
  files pass: `suggestionsUi` + `summaryCards` → **21/21**.
- **G3** `openspec validate task-034-suggestions-uat-polish --strict` → **valid (exit 0)**.
- No eslint script exists in `package.json` (only `build` / `test:frontend`); none run.

## Guarantees held
- **never-zero echo (A3)** — same-minute End default is `start + duration` (≥ 1), strictly after Start;
  backend remains authoritative.
- **absence ≠ zero (B5/C)** — NULL AI cost renders "—" (per-project, aggregate, and mixed-currency);
  untimed evidence surfaced as "not auto-trackable", never zeroed/dropped.
- **AI ≠ human (B5)** — AI cost shown on its own sub-line, never folded into the human total.
- **no auto-post (C)** — render-only; Accept is still the sole writer; untimed blocks still need an
  edited span.
- **secret-free (SEC-012)** — only aggregate numbers / counts / labels rendered; covered by the existing
  SEC-012 render test. No new egress; engine untouched.
</content>
</invoke>
