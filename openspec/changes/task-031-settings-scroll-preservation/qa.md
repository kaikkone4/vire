# QA Report — TASK-031 settings-scroll-preservation

**Gate**: SW-3 · **Tier**: L1-equivalent (frontend-only)
**Branch**: `feat/task-031-settings-scroll-preservation`
**Date**: 2026-06-20

---

## Scenario Coverage Matrix

| # | Scenario (spec.md) | Coverage type | Result |
|---|---|---|---|
| S1 | Settings control below the fold keeps its place (same-view re-render preserves scroll) | Unit: `scroll.test.mjs` — `nextScrollTop(true, 420) === 420`; `nextScrollTop(true, 0) === 0` | PASS |
| S2 | Navigating to another view starts at the top | Unit: `scroll.test.mjs` — `nextScrollTop(false, 420) === 0`; `nextScrollTop(false, 0) === 0` | PASS |
| S3 | First render starts at the top (no prior view → `sameView=false`) | Unit: `scroll.test.mjs` — `nextScrollTop(false, 0) === 0` (null `lastRenderedView` makes `sameView=false`) | PASS |
| S4 | Help text and button label agree ("Create & map") | Unit: `envMappingUi.test.mjs` line 112 — matches `/Create &amp; map/`, does not match `/Create project for/` | PASS |
| M1 | Manual UAT 4.1 — scroll viewport stays put in Settings (Test connection / Save range / Import now / Map) | Human-only (DOM/webview-bound, not runnable in Node test suite) | UNVERIFIED — human required |
| M2 | Manual UAT 4.2 — switching views resets new view to top | Human-only | UNVERIFIED — human required |
| M3 | Manual UAT 4.3 — mapping panel help reads "Create & map" in the live app | Human-only | UNVERIFIED — human required |

---

## Checks Run

| Check | Result | Detail |
|---|---|---|
| `npm run test:frontend` | 73 pass / 2 fail | 2 pre-existing pi-observe.security failures (Langfuse 401 — requires live local Langfuse, no network in headless; verified pre-existing on `main` HEAD per sw2-impl-notes.md §Tests) |
| `tests/scroll.test.mjs` in isolation | 2/2 pass | `nextScrollTop` helper: same-view returns prevScroll, view-change returns 0 |
| `tests/envMappingUi.test.mjs` in isolation | 12/12 pass | Includes new TASK-031 assertion at line 112 |
| `npm run build` (tsc + vite) | Clean | 0 type errors; 14 modules transformed |

---

## Code Verification

- **`src/scroll.ts`** — pure `nextScrollTop(sameView, prevScroll)` exported helper; correct logic; no side effects.
- **`src/main.ts` line 43** — `shell()` captures `prevScroll` and `sameView` before `app.innerHTML=` swap; restores via `nextScrollTop` after swap; updates `lastRenderedView`. Wiring covers all views via the single chokepoint (not scope creep per arch-review §3).
- **`src/env-mapping-ui.ts` line 97** — help `<p>` reads `use "Create &amp; map"` (was `use "Create project for …"`); button label at line 87 is `Create &amp; map`. Label and copy agree.
- **`tests/scroll.test.mjs`** (new) — 2 meaningful test cases; follows existing `*.test.mjs` pattern.
- **`tests/envMappingUi.test.mjs`** — 1 assertion added (line 112–117); does not break existing 11 tests.

---

## Pre-existing Failures Baseline

`tests/pi-observe.security.test.mjs` — 2 tests fail offline (Langfuse 401 / no ingestion request):
- "safe dotenv parser loads only allowlisted Langfuse keys without shell execution"
- "remote Langfuse host is blocked unless explicitly opted in"

These are Langfuse-network tests requiring a live local Langfuse. Confirmed pre-existing on `main`
before TASK-031 changes (per sw2-impl-notes.md §Tests). Not related to scroll or copy changes.

---

## Manual UAT (Human-Required)

Tasks.md §4 items are not automatable — the scroll fix operates on the live webview DOM which the
Node test suite cannot drive. Awaiting human UAT in the macOS app or Tauri dev mode:

- 4.1 In Settings, scroll down, press any re-rendering control (Test connection, Save settings,
  Save range, Import now, Backfill now, Map, Clear mapping, Create & map): viewport should stay put.
- 4.2 Switch to another view (e.g. Today) and back to Settings: should open at top.
- 4.3 Open Settings → mapping panel: help text should read **"Create & map"**, not "Create project for…".

---

## Gate Verdict

**PASS** (automated gate, L1 criteria met).

All spec scenarios have observable automated coverage. Manual UAT items are recorded above as
human-required (unverifiable in headless). Pre-existing failures are baseline-confirmed and
unrelated to this change.

→ Route to **SW-4 (Code Reviewer)** and **SW-5 (Security Agent)** in parallel.
