# Architecture review — TASK-056 active-window capture Settings + privacy/status UI

**Verdict: PASS — one TASK, not split-required.** The change is architecturally consistent: it is the
**permissions/privacy-UI leg** that TASK-046 (`design.md` §7: *"the permissions/privacy-UI task edits the
retention/title-mode policy and surfaces capture health"*) and TASK-048 (`design.md` §1/§7: *"the
permissions/privacy-UI task — user-facing capture toggle + transparency, owns FB-002"*) explicitly named as
downstream and deferred. It exposes the **already-built** capture config (TASK-048) and store (TASK-046)
through a thin, validated IPC seam + one Settings panel, reusing the shipped Langfuse/env-mapping settings
pattern. It introduces **no new component boundary**, **no new native/TCC risk**, and **no schema change**.
Design is `design.md`; scope proposal is `proposal.md`; spec delta validates.

## Scope — why ONE task (not split-required)

The split rule (TASK-035 §3): split only when scope **crosses a `03_architecture_plan.md` component
boundary** or **isolates materially different risk**. Neither applies.

- **No boundary crossing.** Everything lives within the **Rust-core/IPC** + **renderer** + (read-only)
  **Local-SQLite-Store / settings** components already in use: two IPC commands, one read-only store query,
  one Settings panel, and existing-`settings`-row writes. No native code, no macOS/TCC surface, no capture
  logic is touched — contrast TASK-046/048, which *were* split because they crossed the Store↔macOS-Capture
  boundary and carried native packaging/TCC risk. **None of that is present here.**
- **Same precedent as TASK-032 / TASK-055.** Both packaged backend engine/IPC + a renderer view as **one
  capability, one change** with sequenced internal workstreams, and TASK-055 was ruled *not split-required*
  for the identical shape. TASK-056 is smaller (no engine, no new table) — a straightforward settings
  surface over existing config. Fragmenting it would split one user-facing contract across changes for no
  risk-isolation benefit.
- **Proportional.** Three thin workstreams (A IPC, B read-only status, C frontend). A+B are a few dozen
  lines over existing config/tables; C is one panel modelled on `langfusePanel()`/`mappingPanel()`. This is
  a small task, not a program.

No split is *required*; STATUS is `pass`.

## Sequencing vs TASK-055 — independent & parallelizable

- **Clean logical separation.** TASK-055 (suggestions) reads `active_window_evidence` → app→project
  mapping → reviewable suggestions, and its frontend adds a *Suggestions* section + a *Settings app-mapping*
  surface. It **explicitly excludes** *"the `capture_status` string or a capture toggle"* (`task-055/
  proposal.md` §Out of scope). TASK-056 owns exactly that excluded surface: the capture enable/interval/idle/
  retention controls, the health/status readout, and the transparency copy. **No overlap in data, backend
  modules, or IPC** (055: `app_mapping` + `active_window_suggestions`; 056: `settings` keys +
  `capture_status_snapshot`). Neither blocks the other → **parallelizable**.
- **Soft one-way UX linkage (non-blocking).** TASK-055's absence copy points the user to a capture health/
  settings surface (*"capture disabled/degraded → point to Settings/health"*, `task-055/design.md` §5). That
  destination is what TASK-056 builds. 055 can ship its copy regardless; the affordance simply resolves to a
  real surface once 056 lands. **Recommendation: run in parallel; prefer TASK-056 to land at or just before
  TASK-055's frontend (Workstream D)** so 055's "see Settings/health" link is not a dangling reference.
- **File-level merge coordination (trivial, additive).** Both add entries to `generate_handler!`
  (`lib.rs:1205`) and a panel to `renderSettings()` (`src/main.ts:75`). These are **purely additive list/
  composition edits with no shared logic** — whichever lands second rebases by appending its commands and
  its panel. Flag to whichever developer merges second; there is no design conflict.
- **ID-collision note.** TASK-055's handoff proposed *"TASK-056 (`active-window-suggestion-split`)"* as a
  **placeholder, not allocated** for its severable Workstream E. That number is now **this** task. If
  TASK-055's split-E is severed, it needs a **fresh id (e.g. TASK-057)** — the 056 placeholder is void.

## Architecture-fit findings

1. **Truthful transparency is a required correctness fix, not polish.** `renderSettings()` still prints the
   pre-capture claim *"This app does not collect … active windows, idle state …"* (`src/main.ts:75`). Once
   TASK-048's capture is enabled that statement is **false**. Correcting it — and driving the Today
   `capture()` banner from the real state — is the FB-002 transparency obligation TASK-048 deferred to this
   task. Flagged to SW-3/QA as the load-bearing item to verify. (`design.md` §3.)
2. **No new permission; `title_mode` stays redacted and unexposed.** The store's `title_mode='stored'`
   opt-in exists (TASK-046 §3) but only matters under an Accessibility grant that neither TASK-048 nor this
   task takes. Exposing a "store titles" toggle here would imply a permission the task must not request and
   would be misleading (nothing writes a title). Keeping it read-only ("never captured") preserves C2/C5 and
   the zero-grant baseline (SEC-007). The AX-title task (FB-048) owns both title capture and its consent UI.
   (`design.md` §4.)
3. **Config exposure only — capture behavior unchanged, no restart.** The commands read/write the exact
   `settings` keys `CaptureConfig::from_settings` already resolves, and the loop re-reads config every tick,
   so a change applies on the next sample with no thread lifecycle control. This is the minimal correct
   design and matches the shipped settings→loop contract. (`design.md` §0/§5.)
4. **Validation prevents an ill-defined state machine.** `set` enforces `idle_away_seconds >
   idle_candidate_seconds` and positive cadence/retention. The backend already floors `≤0` values, so this
   is defense-in-depth + good UX; a bad write can never disable sampling via a zero cadence. Safe defaults
   are preserved: capture stays OFF unless the user explicitly enables it. (`design.md` §1.)
5. **Status/health surfaced without new storage, gaps always explained.** A single read-only
   `capture_status_snapshot` over the three existing tables yields last-sample/counts/open+recent health.
   No schema change, no new column; `detail` is the already-bounded coarse code (title-free). This realizes
   the "every gap explained, never silent" posture (TASK-002 §4) at the UI. (`design.md` §2.)
6. **Privacy preserved structurally, not by promise.** Titles/screenshots/URLs are already *structurally*
   uncollectable upstream (TASK-046 schema; TASK-048 writes `window_title=NULL` always). This task adds no
   field that could carry them; the view/status/health carry only bundle id/name, coarse idle, timestamps,
   counts, and bounded state codes. SEC-012 extended; SEC-001 allowlist + SEC-007 zero-grant intact.
   (`spec.md` final requirement.)

## feedback_to_ba[] (routed to ba-architect — non-blocking, do not gate implementation)

Proposed DEC number; **BA owns canonical numbering** (repo max is DEC-039; DEC-040..043 are proposed by
TASK-055, so this is the next-free candidate). It *extends* already-ratified decisions rather than
contradicting them — no reality/BA divergence blocks this task.

- **FB-002 realization (transparency + consent UI).** TASK-056 is the user-facing capture toggle +
  transparency surface TASK-048 deferred under FB-002 / DEC-019. It also corrects the now-inaccurate
  pre-capture privacy copy. Confirm ownership; no scope change to DEC-019 — this fulfills it.
- **DEC-044 (proposed).** Capture settings UI exposes **enable / sample interval / idle thresholds /
  retention only**; `title_mode` stays `redacted` and is **not** user-togglable — **no window-title opt-in
  without an Accessibility grant** (that belongs to the AX-title task, FB-048). *Extends C5/DEC-019; keeps
  the zero-grant baseline (SEC-007).*

## Handoff

Route next to a **frontend-capable developer** who can also add the two thin backend commands + the
read-only status query (A+B are small, gate C). No blockers. Finding 1 (truthful copy) and finding 4
(validation/ordering) are the two review-sensitive spots for SW-3/SW-4. Coordinate the trivial additive
merge with TASK-055 on `generate_handler!` and `renderSettings()` (whichever lands second appends).
