# SW-2 Workstream C notes — TASK-032 AI time-entry suggestions

Review/Accept UI. Frontend only. Builds on Workstreams A (engine + `time_entry_suggestions`) and B
(accept/dismiss IPC + `time_entries.origin` + DEC-003 reporting), both SW-3 QA PASS. No backend
semantics changed — C only consumes the existing IPC and the additive `SummaryRow.ai_minutes` /
`TimeEntry.origin` fields.

## What shipped (tasks.md C1–C5)

- **C1** New `'Suggestions'` view: added to `View`/`views` (`src/main.ts:13,24`), routed in `render()`
  (`main.ts:49`), `renderSuggestions()` + `bindSuggestions()` added. Loads the stored pending set via
  `list_time_entry_suggestions({regenerate:false})` on open.
- **C2** New module `src/suggestions-ui.ts` (pure, DOM/IPC-free HTML builders, same pattern as
  `import-report.ts` / `env-mapping-ui.ts`). `suggestionGroups()` groups by project → date (one `.panel`
  per project, rows sorted by date then block start). Each row shows: date, span (`HH:MM–HH:MM` or `—`),
  duration (or **"needs manual time"**), evidence (`N traces · M sessions`), usage (`tokens tok · cost`,
  `—` when unknown), `health · confidence`, reason, and **Accept / Edit / Dismiss**. Edit toggles an
  inline panel with date/start/end/note inputs (reuses the `.lf-form` class + `optionalText` from
  `forms.ts`); **Accept with edits** submits `edits`. Plain **Accept** submits with no edits (backend
  uses the block's own times).
- **C3** **Refresh suggestions** button → `list_time_entry_suggestions({regenerate:true})` then
  re-render. `unmappedNotice()` lists each unmapped env + trace count and a **"Open Settings to map"**
  button (`data-goto-view="Settings"`). Empty/absence state names the candidate causes (no evidence
  imported / nothing mapped / all dismissed) — never a bare empty table. `sourceBanner()` reused at the
  top of the view.
- **C4** Secret-free render (SEC-012): only project name, local date/time, aggregate numbers, counts,
  and fixed health/confidence/reason labels are rendered; the suggestion `id` is used only as an escaped
  `data-*` attribute for action wiring. No payload, session id, prompt, metadata, or credential is ever
  rendered. All caller text is `escapeHtml`-escaped.
- **C5** 10 unit tests (`tests/suggestionsUi.test.mjs`). `npm run test:frontend` and `npm run build`
  (`tsc` strict + `vite build`) green.

## Reporting separation surfaced in existing views (scope: "reflect origin-separated summary fields")

`Summary` type gained `ai_minutes`. A shared `summaryCards()` helper (used by Today + Reports) keeps the
human (`duration_minutes`) figure as the headline total and shows accepted AI-suggested time as a
separate `<small>` sub-line ("AI-suggested Xh Ym, reported separately") only when `ai_minutes > 0` — AI
time is never folded into the human total (DEC-003). The CSV export already carries the backend `origin`
column (Workstream B4); the frontend export button is unchanged (it surfaces the file's labelled rows).

## C-checkpoint evidence — frontend builder tests

`tests/suggestionsUi.test.mjs` — 10/10 pass. Each asserts a guarantee on the pure builders:

| Test | Verifies |
|---|---|
| durationLabel known/unknown | known → `1h 36m`; unknown → `needs manual time`, never `0h 0m` (absence ≠ zero) |
| tokens/cost unknown | `null` → `—`, never `0`; known → `12,345` / `0.42 USD` |
| spanLabel | timed → `09:12–10:48`; untimed → `—` |
| timed row | Accept + Edit + Dismiss present; edit panel `hidden`; figures/counts/health/confidence/reason rendered |
| untimed row | `needs manual time`; **no** plain Accept; edit panel visible; start/end `required`; tokens/cost `—`; "never invents a duration" copy |
| suggestionGroups | grouped by project with `<h2>` heading; rows sorted by date |
| unmappedNotice | env + trace count + Settings link; empty array → `''` |
| empty state | names the candidate causes; not a bare table; with unmapped → mapping cause + notice |
| escaping | `<img>`/`<script>` in project name/reason escaped |
| SEC-012 | render contains no `sk-`/`pk-lf-`/`Bearer`/`Authorization`/`oat01`/`payload`/`metadata`/`prompt` |

Full frontend suite: **85 tests, 83 pass, 2 fail**. The 2 failures are the pre-existing, network-
dependent `pi-observe.security.test.mjs` cases (fail identically 2/10 on `main`; not touched by and
unrelated to Workstream C). My added suite: 10/10 pass; no regressions. `npm run build` green.

## Guarantees (design §5) held by C-scope

- **absence ≠ zero** (DEC-004) — unknown duration → "needs manual time"; unknown tokens/cost → "—";
  unmapped evidence surfaced via `unmappedNotice` (never dropped/zeroed). Asserted in tests.
- **no auto-posting** (DEC-006) — the view only renders affordances; only the Accept handler calls
  `accept_time_entry_suggestion`. An untimed block has no plain Accept and cannot be accepted without an
  edited span.
- **AI ≠ human time** (DEC-003) — `summaryCards()` keeps human total as the headline and reports
  `ai_minutes` separately; never summed in.
- **no secrets** (SEC-012) — only secret-free fields rendered; all escaped; SEC marker test passes.
- **Vire authoritative, no egress** (DEC-001/017) — `suggestions-ui.ts` is pure string-building;
  `main.ts` calls only existing SQLite-backed IPC. No new network.

## Changed / added files

- `src/suggestions-ui.ts` (new) — pure HTML builders + types.
- `src/main.ts` — `'Suggestions'` view + render/bind; `Summary.ai_minutes`; shared `summaryCards()`.
- `tests/suggestionsUi.test.mjs` (new) — 10 builder tests.

## Not in C-scope / notes for QA

- Manual acceptance M1–M3 (packaged macOS app, human-only) remain unchecked — require running the app.
- No new CSS: the view reuses existing classes (`.panel`, `.cards`/`.card`, `table`, `.banner`, `.row`,
  `.lf-form`, `.empty`, `.danger`, `.filters`). `<small>` AI sub-lines use native element styling.
- Interpretation: the empty pending list cannot distinguish "no evidence" from "all dismissed", so the
  empty state names both candidate causes (matches design §4 wording).
