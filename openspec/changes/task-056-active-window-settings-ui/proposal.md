# TASK-056 — Active-window capture Settings + privacy/status UI

- **Change:** `task-056-active-window-settings-ui`
- **Capability:** ADD `active-window-settings` (the **permissions / privacy / transparency UI** leg named
  downstream of TASK-046 storage and TASK-048 capture — *"the permissions/privacy-UI task: user-facing
  capture toggle + transparency, owns FB-002"*, `task-048/design.md` §1/§7).
- **Kind:** Renderer surface + a thin read/write IPC seam over the **already-built** TASK-048 capture
  config and TASK-046 store. Turns the currently env/settings-only zero-permission capture into a
  user-facing, transparent, controllable feature. **No capture change, no new native/TCC work, no window
  titles, no Accessibility, no network.**
- **Tier / gate:** L2 · SEC-001/SEC-007/SEC-012 (field-allowlist + zero-grant baseline + no-secrets
  preserved at the settings/status layer), APP-005 input.

## Why

TASK-046 built the local store and TASK-048 filled it with **zero-permission** frontmost-app + idle
capture, gated by `active_window_capture_enabled` (**default OFF**). But that switch and every knob
(`sample_seconds`, `idle_candidate_seconds`, `idle_away_seconds`, `retention_days`) are reachable **only**
via the raw `settings` table or a `VIRE_*` env var (`docs/active-window-capture.md` §Enabling: *"In-app UI
controls and toggle are not yet available"*). TASK-048 explicitly deferred *"permissions/privacy/retention
UI and any user-facing capture toggle"* to this task (`task-048/design.md` §7), and no IPC exists to read
capture state from the renderer (`task-048/code-to-spec.md`: *"No UI / IPC / capture … additions"*).

Two concrete gaps make this the next needed slice:

1. **No user control or transparency.** A user cannot see whether capture is on, what cadence it runs at,
   how long evidence is kept, or whether it is healthy — nor turn it on/off — without editing a database or
   shell env. That fails the consent/transparency posture the whole feature was designed around (TASK-002
   §4 *"every gap explained, never silent"*; DEC-019/FB-002).
2. **The shipped Settings copy is now inaccurate.** `renderSettings()` still prints the pre-capture claim:
   *"Automatic capture is deferred. This app does not collect … active windows, idle state …"*
   (`src/main.ts:75`). Once capture is enabled that sentence is **false**. Correcting it truthfully is a
   transparency obligation this task owns (FB-002), not cosmetic polish.

This change closes both by exposing the existing config through a small, validated IPC seam and a single
Settings panel — reusing the established Langfuse/env-mapping settings pattern (`langfusePanel()` +
`bindLangfuse()`, `mappingPanel()` + `bindEnvMapping()`). It requests **no** permission and changes **no**
capture behavior: it reads and writes the same settings keys the loop already reads each tick, so a change
takes effect on the next sample **without an app restart**. Capture stays **OFF by default** — this task
never auto-enables it.

## What changes

One OpenSpec change, one cohesive capability, three thin internal workstreams (A backend read/write IPC,
B backend status/health projection, C frontend panel + truthful copy). See `design.md` for the IPC
contract and validation; `arch-review.md` §Scope for why this is **one task, not split-required**.

- **A — Settings read/write IPC (backend).** Two commands in `lib.rs`:
  `get_active_window_capture_settings` (returns the resolved `CaptureConfig` view: enabled, sample/idle/
  away seconds, retention days, plus `platform_supported` and a fixed `title_mode='redacted'` marker) and
  `set_active_window_capture_settings(input)` (**validates** then upserts the exact `settings` keys
  `CaptureConfig::from_settings` already reads). Validation enforces safe bounds: `sample_seconds ≥ 1`,
  `idle_candidate_seconds ≥ 1`, `idle_away_seconds > idle_candidate_seconds`, `retention_days ≥ 1`, and
  sane upper caps. Reuses the existing `settings` table and the `settings > env > default` precedence — no
  new config surface, no schema change.
- **B — Capture status / health projection (backend, read-only).** One new read-only aggregation
  `active_window::store::capture_status_snapshot(conn, …)` over the **existing** three tables (no schema
  change, no new column): last sample timestamp, samples captured today, evidence blocks within the
  retention window, and current **open** + recent `capture_health` states (coarse state codes only, already
  bounded and title-free). Surfaced by `get_active_window_capture_settings` so the UI can show *"capture
  healthy / degraded because …"* — every gap explained, never a silent empty box.
- **C — Settings panel + truthful transparency copy (frontend).** A new **"Active-window capture"** panel
  in `renderSettings()` (`src/main.ts`): an enable toggle, numeric controls (sample interval, idle-candidate
  seconds, away seconds, retention days) with inline validation, a live **capture status/health** readout,
  and a **plain-language privacy table** — *what IS captured* (frontmost app bundle id + name, coarse idle
  state, timestamp) vs *what is NEVER captured* (window titles, Accessibility tree, screen pixels,
  keystrokes, mouse content, URLs, paths, command bodies, clipboard, secrets). The now-inaccurate static
  "Capture status" sentence is **replaced** with source-of-truth copy driven by the real setting, and the
  Today `capture()` banner reflects the actual enabled/deferred state. macOS-only: on non-macOS the panel
  shows *"available on macOS only"* and disables the controls.

## Impact

- **Affected code (A):** `src-tauri/src/lib.rs` — two `#[tauri::command]` fns + two entries in
  `generate_handler!` (`lib.rs:1205`); a small validated settings-write helper. No new module required
  (may add `active_window::settings_api` for the DTOs/validation if it keeps `lib.rs` tidy).
- **Affected code (B):** `src-tauri/src/active_window/store.rs` — one read-only `capture_status_snapshot`
  query + its typed result struct. **No** schema change, **no** write-path change, redaction gate untouched.
- **Affected code (C):** `src/main.ts` (`renderSettings()` panel + `capture()`/status copy + bind fn),
  `src/style.css` if needed. Optionally a small `src/active-window-settings-ui.ts` builder to match the
  `env-mapping-ui.ts` / `langfuse-settings.ts` split and keep pure-builder frontend tests easy.
- **Docs:** `docs/active-window-capture.md` — the §"Enabling capture" note *"In-app UI controls … not yet
  available"* and the §"Not yet available" bullets (*"In-app UI controls and capture status display"*,
  *"IPC commands to read capture state from the frontend"*) are updated to reflect that this task ships them.
- **Data model:** **none.** No table, no column, no migration. Reads/writes only existing `settings` rows
  and reads the existing `active_window_*` tables. Reverting the change leaves those rows harmless.
- **Affected specs:** **ADD `active-window-settings`** (user control of the zero-permission capture,
  validated safe-bounds config, visible health/status with every gap explained, truthful transparency copy,
  no-Accessibility/no-title/no-new-permission guarantee). No existing capability spec modified.
- **Guardrails preserved:** capture stays **OFF by default** (no auto-enable); zero TCC grants requested
  (no AX, no Screen Recording, no event tap); `title_mode` stays `redacted` and is **not** made
  user-togglable (a `stored`/title opt-in belongs to the AX-title task, needs an Accessibility grant);
  local-only; no network client; locked `connect-src ipc:` CSP unchanged; positive field allowlist +
  structural non-collection intact; legacy manual `time_entries` surface untouched.
- **Out of scope:** any capture / macOS / TCC / AX / Screen-Recording change; window titles or a
  `title_mode='stored'` toggle; the suggestion/review pipeline and app→project mapping (TASK-055 owns those);
  writing `time_entries`; CSV/report changes; a Swift sidecar / `externalBin` / codesign / notarization; any
  network egress or CSP change.

## ADRs (proposed — routed to ba-architect via `feedback_to_ba[]`, non-blocking)

Numbers are proposed; **BA owns canonical numbering** (repo max is DEC-039; DEC-040..043 are proposed by
TASK-055). See `arch-review.md`.

- **FB-002 realization (transparency).** This task is the user-facing capture toggle + transparency surface
  TASK-048 deferred (FB-002 / DEC-019). Confirm ownership; correct the inaccurate pre-capture privacy copy.
- **DEC-044 — Capture settings UI exposes enable/interval/idle/retention only.** `title_mode` stays
  `redacted` and unexposed; **no window-title opt-in without an Accessibility grant** (belongs to the
  AX-title task). Extends C5/DEC-019.
